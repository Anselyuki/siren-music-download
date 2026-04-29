//! Tag 注册表服务——本地缓存、内存查询与 i18n 解析。
//!
//! 该模块管理一份从远端或本地缓存加载的标签注册表（tag registry），负责为前端
//! 提供已本地化的 tag 维度列表、专辑/单曲标签查询，以及为搜索索引构建提供
//! 全语种标签文本。
//!
//! # 主要能力
//!
//! - 启动时从本地缓存文件加载注册表；缓存缺失或 schema 不兼容时降级为空注册表。
//! - 内存中以 `Arc<RwLock<TagRegistry>>` 存储，读操作无锁竞争，写操作（`update`）原子替换并持久化。
//! - 支持 zh-CN / en-US 双语解析，缺失时按 locale → zh-CN → en-US → 第一个可用项的顺序回退。
//! - 单曲标签支持继承专辑标签（同维度 values 去重合并）。
//! - `get_all_locale_tag_values_*` 方法为搜索索引提供所有语种的标签值拼接串。

use crate::preferences::Locale;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use siren_core::api::TagEntry;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use tempfile::NamedTempFile;

// ─── 公开常量 ────────────────────────────────────────────────────────────────

/// 当前支持的 tag 注册表 schema 版本。加载缓存时若版本不匹配则拒绝并降级为空注册表。
pub(crate) const CURRENT_SCHEMA_VERSION: u32 = 1;

/// 远端 tag 注册表 JSON 文件地址，用于后台增量拉取与版本比对。
pub(crate) const REMOTE_URL: &str = "https://raw.githubusercontent.com/anselyuki/siren-music-download/main/data/tag_registry.json";

// ─── 缓存文件名 ───────────────────────────────────────────────────────────────

const CACHE_FILE_NAME: &str = "tag_registry.json";

// ─── 数据模型 ─────────────────────────────────────────────────────────────────

/// 远端/本地 JSON 格式的 tag 注册表根结构。
///
/// `schema_version` 用于兼容性校验，`updated_at` 用于增量拉取的版本对比，
/// `tag_dimensions` 定义所有可用的 tag 维度，`albums`/`songs` 存储各自的标签集合。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TagRegistry {
    /// 注册表 schema 版本，当前为 1。
    #[serde(default)]
    pub(crate) schema_version: u32,
    /// 注册表最后更新时间（ISO 8601 字符串），用于远端版本对比。
    #[serde(default)]
    pub(crate) updated_at: String,
    /// 所有 tag 维度定义，包含 key 与各语种的展示名称。
    #[serde(default)]
    pub(crate) tag_dimensions: Vec<TagDimension>,
    /// 专辑 CID → 专辑标签集合的映射。
    #[serde(default)]
    pub(crate) albums: HashMap<String, TagSet>,
    /// 单曲 CID → 单曲标签集合的映射。
    #[serde(default)]
    pub(crate) songs: HashMap<String, TagSet>,
}

/// 单个 tag 维度的定义。
///
/// `key` 是维度的唯一标识符（如 `"faction"`），`label` 是各语种的展示名称映射。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TagDimension {
    /// 维度唯一键，与 [`TagSet`] 中的字段 key 对应。
    pub(crate) key: String,
    /// 各语种的维度展示名称，key 为 BCP 47 语言标签（如 `"zh-CN"`、`"en-US"`）。
    pub(crate) label: HashMap<String, String>,
}

/// 单个实体（专辑或单曲）的标签集合。
///
/// 结构为 `维度 key → LocalizedValue 列表`，每个 `LocalizedValue` 代表一个已本地化的 tag 值。
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub(crate) struct TagSet {
    /// 维度 key → 本地化值列表的映射。
    #[serde(default)]
    pub(crate) tags: HashMap<String, Vec<LocalizedValue>>,
}

/// 单个 tag 值的多语种本地化映射。
///
/// key 为 BCP 47 语言标签（如 `"zh-CN"`、`"en-US"`），value 为对应语种下的文本。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct LocalizedValue(pub HashMap<String, String>);

// ─── 前端展示类型 ─────────────────────────────────────────────────────────────

/// 已解析为单一语种的 tag 维度条目，供前端展示用。
///
/// `key` 为维度唯一标识符，`label` 为当前 locale 下的展示名（经过 locale 回退策略解析）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct TagDimensionResolved {
    /// 维度唯一键。
    pub key: String,
    /// 当前 locale 下的维度展示名。
    pub label: String,
}

// ─── 服务结构 ─────────────────────────────────────────────────────────────────

/// Tag 注册表服务。
///
/// 负责从本地缓存文件加载 tag 注册表，并以读多写少的 `Arc<RwLock<TagRegistry>>` 模式
/// 支持多线程并发访问。典型用法：
///
/// 1. 启动时调用 [`TagRegistryService::new`] 初始化（尝试从缓存加载）。
/// 2. 在 Tauri command 中调用 `get_album_tags` / `get_song_tags` 获取已本地化的标签。
/// 3. 后台拉取到新版本后调用 [`TagRegistryService::update`] 原子替换并持久化。
///
/// # 线程安全
///
/// 该类型实现了 `Clone`，克隆后共享同一份底层数据，适合通过 `Arc` 注入 Tauri 状态。
///
/// # 错误处理
///
/// - 缓存缺失或 schema 版本不兼容时，静默降级为空注册表（无任何 tag 数据）。
/// - `update` 原子写入失败时返回 `Err`，但内存中的注册表状态已更新；调用方应记录日志后继续运行。
#[derive(Clone)]
pub(crate) struct TagRegistryService {
    registry: Arc<RwLock<TagRegistry>>,
    cache_path: PathBuf,
}

impl TagRegistryService {
    /// 创建服务实例，并尝试从 `app_data_dir/tag_registry.json` 加载缓存。
    ///
    /// 若缓存文件不存在、读取失败或 schema 版本不匹配，则静默使用空注册表初始化。
    ///
    /// # 参数
    ///
    /// - `app_data_dir`：应用数据目录路径，注册表缓存文件将存储在该目录下。
    pub(crate) fn new(app_data_dir: &PathBuf) -> Self {
        let cache_path = app_data_dir.join(CACHE_FILE_NAME);
        let registry = load_from_cache(&cache_path).unwrap_or_default();
        Self {
            registry: Arc::new(RwLock::new(registry)),
            cache_path,
        }
    }

    /// 获取所有 tag 维度，按指定 locale 解析展示名。
    ///
    /// 返回值顺序与注册表中 `tagDimensions` 数组顺序一致。
    ///
    /// # 参数
    ///
    /// - `locale`：目标语种，解析时按 locale → zh-CN → en-US → 第一个可用项的顺序回退。
    pub(crate) fn get_dimensions(&self, locale: Locale) -> Vec<TagDimensionResolved> {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        registry
            .tag_dimensions
            .iter()
            .map(|dim| TagDimensionResolved {
                key: dim.key.clone(),
                label: resolve_locale_str(&dim.label, locale),
            })
            .collect()
    }

    /// 获取指定专辑的 tag 列表，按指定 locale 解析。
    ///
    /// 若该专辑在注册表中无对应数据，返回空列表。
    ///
    /// # 参数
    ///
    /// - `album_cid`：专辑 CID。
    /// - `locale`：目标语种。
    pub(crate) fn get_album_tags(&self, album_cid: &str, locale: Locale) -> Vec<TagEntry> {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        resolve_tag_set(
            registry.albums.get(album_cid),
            &registry.tag_dimensions,
            locale,
        )
    }

    /// 获取指定单曲的 tag 列表，合并专辑继承标签后按指定 locale 解析。
    ///
    /// 合并规则：先取专辑 tag，再用单曲 tag 追加（同维度 values 去重）。
    /// 若专辑或单曲均无数据，返回空列表。
    ///
    /// # 参数
    ///
    /// - `song_cid`：单曲 CID。
    /// - `album_cid`：所属专辑 CID，用于继承专辑 tag。
    /// - `locale`：目标语种。
    pub(crate) fn get_song_tags(
        &self,
        song_cid: &str,
        album_cid: &str,
        locale: Locale,
    ) -> Vec<TagEntry> {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        resolve_merged_tag_set(
            registry.albums.get(album_cid),
            registry.songs.get(song_cid),
            &registry.tag_dimensions,
            locale,
        )
    }

    /// 获取指定专辑的全语种 tag 值拼接串，供搜索索引使用。
    ///
    /// 收集该专辑所有维度、所有语种下的 tag 值，以空格连接为单一字符串。
    /// 若无数据，返回空字符串。
    ///
    /// # 参数
    ///
    /// - `album_cid`：专辑 CID。
    pub(crate) fn get_all_locale_tag_values_for_album(&self, album_cid: &str) -> String {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        collect_all_locale_values(registry.albums.get(album_cid), None)
    }

    /// 获取指定单曲（含继承专辑 tag）的全语种 tag 值拼接串，供搜索索引使用。
    ///
    /// 收集专辑 tag 与单曲 tag 所有维度、所有语种下的值，以空格连接为单一字符串。
    /// 若均无数据，返回空字符串。
    ///
    /// # 参数
    ///
    /// - `song_cid`：单曲 CID。
    /// - `album_cid`：所属专辑 CID。
    pub(crate) fn get_all_locale_tag_values_for_song(
        &self,
        song_cid: &str,
        album_cid: &str,
    ) -> String {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        collect_all_locale_values(
            registry.albums.get(album_cid),
            registry.songs.get(song_cid),
        )
    }

    /// 按维度 key 聚合专辑 CID，返回 tag 值 → 专辑 CID 列表的映射。
    ///
    /// 遍历所有专辑，找出包含指定维度的专辑，并按当前 locale 下的 tag 值分组。
    /// 同一专辑可能出现在多个 tag 值下（该维度有多个值时）。
    ///
    /// # 参数
    ///
    /// - `dimension_key`：目标维度的唯一键（如 `"faction"`）。
    /// - `locale`：目标语种，用于解析 tag 值展示名。
    pub(crate) fn get_album_cids_by_dimension(
        &self,
        dimension_key: &str,
        locale: Locale,
    ) -> HashMap<String, Vec<String>> {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        let mut result: HashMap<String, Vec<String>> = HashMap::new();
        for (album_cid, tag_set) in &registry.albums {
            if let Some(values) = tag_set.tags.get(dimension_key) {
                for localized in values {
                    let label = resolve_localized_value(&localized.0, locale);
                    result
                        .entry(label)
                        .or_default()
                        .push(album_cid.clone());
                }
            }
        }
        result
    }

    /// 用新的注册表替换当前内存状态，并原子写入缓存文件。
    ///
    /// 内存状态在持久化成功前即已更新；若写盘失败，返回 `Err` 但内存中已使用新数据。
    /// 调用方应在 `Err` 时记录日志并继续运行（降级策略：内存数据仍可用）。
    ///
    /// # 参数
    ///
    /// - `new_registry`：新版本的 tag 注册表。
    ///
    /// # 错误
    ///
    /// 若创建临时文件、写入内容或原子重命名失败，返回 `Err`。
    pub(crate) fn update(&self, new_registry: TagRegistry) -> Result<()> {
        {
            let mut registry = self.registry.write().expect("tag registry RwLock poisoned");
            *registry = new_registry.clone();
        }
        persist_to_cache(&self.cache_path, &new_registry)
    }

    /// 获取当前内存中注册表的 `updated_at` 字段（ISO 8601 字符串）。
    ///
    /// 适用于与远端版本进行对比，判断是否需要拉取更新。若注册表为空（初始状态），
    /// 返回空字符串。
    pub(crate) fn current_updated_at(&self) -> String {
        let registry = self.registry.read().expect("tag registry RwLock poisoned");
        registry.updated_at.clone()
    }
}

// ─── 私有辅助函数 ─────────────────────────────────────────────────────────────

/// 将 `Locale` 枚举转换为 BCP 47 语言标签字符串。
fn locale_to_key(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCN => "zh-CN",
        Locale::EnUS => "en-US",
    }
}

/// 从多语种字符串 map 中按 locale 回退策略取值。
///
/// 回退顺序：locale 对应 key → "zh-CN" → "en-US" → map 中第一个可用值 → 空字符串。
fn resolve_locale_str(map: &HashMap<String, String>, locale: Locale) -> String {
    let key = locale_to_key(locale);
    if let Some(v) = map.get(key) {
        return v.clone();
    }
    if let Some(v) = map.get("zh-CN") {
        return v.clone();
    }
    if let Some(v) = map.get("en-US") {
        return v.clone();
    }
    map.values().next().cloned().unwrap_or_default()
}

/// 从单个 `LocalizedValue` 内部的 map 中按 locale 回退策略取值。
///
/// 与 [`resolve_locale_str`] 逻辑相同，但针对 `LocalizedValue` 包装的内部 map。
fn resolve_localized_value(map: &HashMap<String, String>, locale: Locale) -> String {
    resolve_locale_str(map, locale)
}

/// 将 `TagSet` 解析为 `Vec<TagEntry>`，按 locale 解析维度名与 tag 值。
///
/// 遍历所有维度定义，若 `tag_set` 中存在该维度则生成对应的 [`TagEntry`]；
/// 值列表去重后按出现顺序保留。
fn resolve_tag_set(
    tag_set: Option<&TagSet>,
    dimensions: &[TagDimension],
    locale: Locale,
) -> Vec<TagEntry> {
    let Some(tag_set) = tag_set else {
        return Vec::new();
    };
    build_tag_entries(&tag_set.tags, dimensions, locale)
}

/// 合并专辑 tag 与单曲 tag 后解析为 `Vec<TagEntry>`。
///
/// 合并规则：先取专辑 tag 中的所有维度值，再追加单曲 tag 中同维度的新值（去重）。
/// 整体按维度定义顺序输出。
fn resolve_merged_tag_set(
    album: Option<&TagSet>,
    song: Option<&TagSet>,
    dims: &[TagDimension],
    locale: Locale,
) -> Vec<TagEntry> {
    // 构建合并后的 dimension_key → LocalizedValue 列表映射
    let mut merged: HashMap<String, Vec<LocalizedValue>> = HashMap::new();

    for set_opt in [album, song] {
        let Some(set) = set_opt else { continue };
        for (dim_key, values) in &set.tags {
            let entry = merged.entry(dim_key.clone()).or_default();
            for v in values {
                // 去重：若内部 map 完全相同则跳过
                let duplicate = entry
                    .iter()
                    .any(|existing| existing.0 == v.0);
                if !duplicate {
                    entry.push(v.clone());
                }
            }
        }
    }

    build_tag_entries(&merged, dims, locale)
}

/// 从 dimension_key → LocalizedValue 列表映射构建 `Vec<TagEntry>`。
///
/// 按 `dims` 顺序输出，仅包含 `tags` 中存在且值非空的维度。
fn build_tag_entries(
    tags: &HashMap<String, Vec<LocalizedValue>>,
    dims: &[TagDimension],
    locale: Locale,
) -> Vec<TagEntry> {
    let mut result = Vec::new();
    for dim in dims {
        let Some(values) = tags.get(&dim.key) else {
            continue;
        };
        let resolved_values: Vec<String> = values
            .iter()
            .map(|lv| resolve_localized_value(&lv.0, locale))
            .collect();
        if resolved_values.is_empty() {
            continue;
        }
        result.push(TagEntry {
            dimension: resolve_locale_str(&dim.label, locale),
            values: resolved_values,
        });
    }
    result
}

/// 收集专辑和/或单曲 tag 中所有语种、所有维度的 tag 值，以空格拼接。
///
/// 用于为搜索索引生成全语种标签文本，使搜索能够命中任意语种的 tag 值。
fn collect_all_locale_values(album: Option<&TagSet>, song: Option<&TagSet>) -> String {
    let mut all_values: Vec<String> = Vec::new();
    for set_opt in [album, song] {
        let Some(set) = set_opt else { continue };
        for values in set.tags.values() {
            for lv in values {
                for v in lv.0.values() {
                    if !v.is_empty() {
                        all_values.push(v.clone());
                    }
                }
            }
        }
    }
    all_values.join(" ")
}

/// 从缓存文件加载 tag 注册表。
///
/// 若文件不存在、读取失败、JSON 解析失败或 schema 版本不匹配，返回 `None`。
fn load_from_cache(path: &PathBuf) -> Option<TagRegistry> {
    if !path.exists() {
        return None;
    }
    let content = std::fs::read_to_string(path).ok()?;
    let registry: TagRegistry = serde_json::from_str(&content).ok()?;
    if registry.schema_version != CURRENT_SCHEMA_VERSION {
        eprintln!(
            "[tag_registry] schema version mismatch: expected {}, got {}; falling back to empty registry",
            CURRENT_SCHEMA_VERSION, registry.schema_version
        );
        return None;
    }
    Some(registry)
}

/// 将 tag 注册表原子写入缓存文件（临时文件 + rename）。
///
/// 写入步骤：序列化为 JSON → 写入同目录临时文件 → 调用 `sync_all` → `persist`（rename）。
/// 若任意步骤失败，返回 `Err` 且原缓存文件不被破坏。
///
/// # 错误
///
/// 创建临时文件、序列化、写盘、sync 或 persist 失败时均返回 `Err`。
fn persist_to_cache(path: &PathBuf, registry: &TagRegistry) -> Result<()> {
    let parent = path
        .parent()
        .context("缓存文件路径无父目录，无法创建临时文件")?;
    std::fs::create_dir_all(parent).context("创建缓存目录失败")?;
    let content = serde_json::to_vec_pretty(registry).context("序列化 tag 注册表失败")?;
    let mut temp_file = NamedTempFile::new_in(parent).context("创建临时文件失败")?;
    temp_file
        .write_all(&content)
        .context("写入临时文件失败")?;
    temp_file
        .as_file()
        .sync_all()
        .context("sync 临时文件失败")?;
    temp_file.persist(path).map_err(|e| e.error).context("原子重命名缓存文件失败")?;
    Ok(())
}

// ─── 单元测试 ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_registry() -> TagRegistry {
        let mut albums = HashMap::new();
        let mut album_tags = HashMap::new();
        album_tags.insert(
            "faction".to_string(),
            vec![LocalizedValue({
                let mut m = HashMap::new();
                m.insert("zh-CN".to_string(), "罗德岛".to_string());
                m.insert("en-US".to_string(), "Rhodes Island".to_string());
                m
            })],
        );
        albums.insert(
            "ALBUM_CID".to_string(),
            TagSet { tags: album_tags },
        );

        let mut songs = HashMap::new();
        let mut song_tags = HashMap::new();
        song_tags.insert(
            "character".to_string(),
            vec![LocalizedValue({
                let mut m = HashMap::new();
                m.insert("zh-CN".to_string(), "陈".to_string());
                m.insert("en-US".to_string(), "Ch'en".to_string());
                m
            })],
        );
        songs.insert(
            "SONG_CID".to_string(),
            TagSet { tags: song_tags },
        );

        TagRegistry {
            schema_version: CURRENT_SCHEMA_VERSION,
            updated_at: "2026-04-29T12:00:00Z".to_string(),
            tag_dimensions: vec![
                TagDimension {
                    key: "faction".to_string(),
                    label: {
                        let mut m = HashMap::new();
                        m.insert("zh-CN".to_string(), "阵营".to_string());
                        m.insert("en-US".to_string(), "Faction".to_string());
                        m
                    },
                },
                TagDimension {
                    key: "character".to_string(),
                    label: {
                        let mut m = HashMap::new();
                        m.insert("zh-CN".to_string(), "角色".to_string());
                        m.insert("en-US".to_string(), "Character".to_string());
                        m
                    },
                },
            ],
            albums,
            songs,
        }
    }

    fn make_service_with(registry: TagRegistry) -> TagRegistryService {
        TagRegistryService {
            registry: Arc::new(RwLock::new(registry)),
            cache_path: PathBuf::from("/tmp/test_tag_registry.json"),
        }
    }

    #[test]
    fn get_dimensions_returns_localized_labels() {
        let svc = make_service_with(make_registry());
        let dims = svc.get_dimensions(Locale::ZhCN);
        assert_eq!(dims.len(), 2);
        assert_eq!(dims[0].key, "faction");
        assert_eq!(dims[0].label, "阵营");
        assert_eq!(dims[1].key, "character");
        assert_eq!(dims[1].label, "角色");
    }

    #[test]
    fn get_dimensions_en_us() {
        let svc = make_service_with(make_registry());
        let dims = svc.get_dimensions(Locale::EnUS);
        assert_eq!(dims[0].label, "Faction");
        assert_eq!(dims[1].label, "Character");
    }

    #[test]
    fn get_album_tags_resolves_zh_cn() {
        let svc = make_service_with(make_registry());
        let tags = svc.get_album_tags("ALBUM_CID", Locale::ZhCN);
        assert_eq!(tags.len(), 1);
        assert_eq!(tags[0].dimension, "阵营");
        assert_eq!(tags[0].values, vec!["罗德岛"]);
    }

    #[test]
    fn get_album_tags_returns_empty_for_unknown() {
        let svc = make_service_with(make_registry());
        let tags = svc.get_album_tags("NO_SUCH_CID", Locale::ZhCN);
        assert!(tags.is_empty());
    }

    #[test]
    fn get_song_tags_merges_album_tags() {
        let svc = make_service_with(make_registry());
        let tags = svc.get_song_tags("SONG_CID", "ALBUM_CID", Locale::ZhCN);
        // 应包含 faction（来自专辑）和 character（来自单曲）
        assert_eq!(tags.len(), 2);
        let factions: Vec<_> = tags.iter().filter(|t| t.dimension == "阵营").collect();
        assert_eq!(factions.len(), 1);
        assert_eq!(factions[0].values, vec!["罗德岛"]);
        let chars: Vec<_> = tags.iter().filter(|t| t.dimension == "角色").collect();
        assert_eq!(chars.len(), 1);
        assert_eq!(chars[0].values, vec!["陈"]);
    }

    #[test]
    fn get_all_locale_tag_values_for_album_contains_all_locales() {
        let svc = make_service_with(make_registry());
        let text = svc.get_all_locale_tag_values_for_album("ALBUM_CID");
        // 应同时包含中文和英文值
        assert!(text.contains("罗德岛"), "should contain zh-CN value");
        assert!(text.contains("Rhodes Island"), "should contain en-US value");
    }

    #[test]
    fn get_all_locale_tag_values_for_song_includes_album_tags() {
        let svc = make_service_with(make_registry());
        let text = svc.get_all_locale_tag_values_for_song("SONG_CID", "ALBUM_CID");
        assert!(text.contains("罗德岛"));
        assert!(text.contains("陈"));
    }

    #[test]
    fn get_album_cids_by_dimension_groups_correctly() {
        let svc = make_service_with(make_registry());
        let map = svc.get_album_cids_by_dimension("faction", Locale::ZhCN);
        assert!(map.contains_key("罗德岛"));
        assert!(map["罗德岛"].contains(&"ALBUM_CID".to_string()));
    }

    #[test]
    fn current_updated_at_returns_registry_value() {
        let svc = make_service_with(make_registry());
        assert_eq!(svc.current_updated_at(), "2026-04-29T12:00:00Z");
    }

    #[test]
    fn load_from_cache_rejects_wrong_schema_version() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tag_registry.json");
        let bad = serde_json::json!({
            "schemaVersion": 99,
            "updatedAt": "",
            "tagDimensions": [],
            "albums": {},
            "songs": {}
        });
        std::fs::write(&path, serde_json::to_vec(&bad).unwrap()).unwrap();
        // schema_version 字段对应 camelCase，但我们的结构体用 rename_all = "camelCase"
        // 此处写 schemaVersion 即为 JSON key；load_from_cache 应该 parse 成功但 schema 不匹配返回 None
        let result = load_from_cache(&path.to_path_buf());
        // version=99 与 CURRENT_SCHEMA_VERSION=1 不同，应返回 None
        assert!(result.is_none());
    }

    #[test]
    fn persist_and_reload_round_trip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("tag_registry.json");
        let registry = make_registry();
        persist_to_cache(&path.to_path_buf(), &registry).unwrap();
        let loaded = load_from_cache(&path.to_path_buf()).unwrap();
        assert_eq!(loaded.updated_at, "2026-04-29T12:00:00Z");
        assert!(loaded.albums.contains_key("ALBUM_CID"));
    }

    #[test]
    fn resolve_locale_str_falls_back_to_zh_cn() {
        let mut map = HashMap::new();
        map.insert("zh-CN".to_string(), "中文".to_string());
        // EnUS 不存在，回退到 zh-CN
        let result = resolve_locale_str(&map, Locale::EnUS);
        assert_eq!(result, "中文");
    }

    #[test]
    fn merge_deduplicates_same_localized_values() {
        // 专辑和单曲都有 faction=罗德岛，合并后应只有一个
        let mut albums = HashMap::new();
        let mut album_tags = HashMap::new();
        let shared_value = LocalizedValue({
            let mut m = HashMap::new();
            m.insert("zh-CN".to_string(), "罗德岛".to_string());
            m
        });
        album_tags.insert("faction".to_string(), vec![shared_value.clone()]);
        albums.insert("A".to_string(), TagSet { tags: album_tags });

        let mut songs = HashMap::new();
        let mut song_tags = HashMap::new();
        song_tags.insert("faction".to_string(), vec![shared_value]);
        songs.insert("S".to_string(), TagSet { tags: song_tags });

        let dims = vec![TagDimension {
            key: "faction".to_string(),
            label: {
                let mut m = HashMap::new();
                m.insert("zh-CN".to_string(), "阵营".to_string());
                m
            },
        }];

        let entries = resolve_merged_tag_set(
            albums.get("A"),
            songs.get("S"),
            &dims,
            Locale::ZhCN,
        );
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].values.len(), 1, "重复值应被去重");
    }
}
