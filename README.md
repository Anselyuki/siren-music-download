# 塞壬音乐下载器

基于 [Rust](https://www.rust-lang.org/) + [Tauri 2](https://tauri.app/) + Svelte 5 的桌面应用，用来浏览 [塞壬唱片](https://monster-siren.hypergryph.com/)公开曲库、在线播放，并将当前曲目下载为 WAV / FLAC / MP3。

## 当前实现

- 已完成：专辑列表、曲目详情、在线播放、暂停 / 继续、拖动进度、上一首 / 下一首、系统媒体会话
- 已完成：播放器歌词面板、播放队列面板、封面取色动态主题、流式音频缓存与缓存清理
- 已完成：当前播放曲目的单曲下载，支持 `wav` / `flac` / `mp3`
- 已完成：FLAC 输出写入标题、艺术家、专辑名、专辑艺术家、曲序和封面元数据；可选同目录生成同名 `.lrc`
- 未完成：工具栏里的“批量下载选中曲目”仍是占位交互，暂无批量下载与下载进度 UI

## 依赖

| 工具 | 说明 |
| --- | --- |
| Rust 1.70+ | Rust 编译环境，建议通过 [rustup](https://rustup.rs/) 安装 |
| Node 18+ | 前端构建与 Tauri CLI 运行环境 |

说明：
- FLAC 编码使用纯 Rust `flacenc`，不依赖 `ffmpeg`
- WAV 和 MP3 输出也不需要外部转码工具

## 开发与构建

```bash
npm install

# 前端 + Tauri 开发模式
npm run tauri:dev

# 仅构建前端
npm run build

# 打包桌面应用
npm run tauri:build

# Rust 检查
cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets
```

## 使用方式

1. 启动应用后会自动拉取专辑列表。
2. 顶部工具栏可切换“下载器”或“播放器”模式。
3. 在“播放器”模式中单击曲目开始播放，可使用底部播放器进行暂停、继续、拖动进度、上一首、下一首。
4. 底部播放器可展开歌词面板和当前播放队列。
5. 右侧设置面板可选择输出格式和下载目录，默认目录为 `~/Downloads/SirenMusic`。
6. 当前播放曲目可通过播放器上的下载按钮直接下载。
7. “下载器”模式中的多选与批量下载按钮尚未接通实际下载逻辑。

## 后端 API 概览

前端通过 `@tauri-apps/api/core` 的 `invoke()` 调用 Rust 后端。当前暴露的 command 如下：

| Command | 参数 | 返回 | 说明 |
| --- | --- | --- | --- |
| `get_albums` | 无 | `Album[]` | 获取全部专辑列表 |
| `get_album_detail` | `albumCid: string` | `AlbumDetail` | 获取专辑详情和曲目列表 |
| `get_song_detail` | `cid: string` | `SongDetail` | 获取单曲详情与 `sourceUrl` |
| `extract_image_theme` | `imageUrl: string` | `ThemePalette` | 从封面提取动态主题色 |
| `get_default_output_dir` | 无 | `string` | 获取默认下载目录 |
| `play_song` | `songCid`, `coverUrl?`, `playbackContext?` | `number` | 开始播放并建立播放上下文，返回时长秒数 |
| `pause_playback` | 无 | `void` | 暂停播放 |
| `resume_playback` | 无 | `void` | 恢复播放 |
| `seek_current_playback` | `positionSecs: number` | `number` | 跳转当前播放位置，返回时长秒数 |
| `play_next` | 无 | `number` | 播放当前上下文中的下一首 |
| `play_previous` | 无 | `number` | 播放当前上下文中的上一首 |
| `stop_playback` | 无 | `void` | 停止播放并重置状态 |
| `get_player_state` | 无 | `PlayerState` | 获取当前播放器状态快照 |
| `set_playback_volume` | `volume: number` | `number` | 设置音量，后端会钳制到 `0.0..=1.0` |
| `download_song` | `songCid`, `outputDir`, `format` | `string` | 下载单曲并返回输出路径 |
| `clear_audio_cache` | 无 | `number` | 清空播放缓存并返回删除文件数 |

后端会发出两个播放器事件：

| 事件名 | 载荷 | 说明 |
| --- | --- | --- |
| `player-state-changed` | `PlayerState` | 播放状态、队列能力、音量等发生变化时触发 |
| `player-progress` | `PlayerState` | 播放进度推进时持续触发 |

## 上游 HTTP API

`siren_core::ApiClient` 封装了塞壬唱片公开 REST API：

| 接口 | 说明 |
| --- | --- |
| `GET /api/albums` | 获取全部专辑列表 |
| `GET /api/album/:cid/detail` | 获取专辑详情及曲目列表 |
| `GET /api/song/:cid` | 获取单曲详情，包含 `sourceUrl` |

## 生成 Rust 文档

```bash
# 共享核心库文档
cargo doc -p siren_core --no-deps

# Tauri 后端 command 文档（包含私有 command 函数）
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

`siren_core` 的文档主要覆盖上游 API、音频处理和下载流程；`src-tauri` 的文档主要覆盖 Tauri command、播放器事件和前后端共享的后端数据结构。

## 项目结构

```text
.
├── Cargo.toml
├── README.md
├── UI_DESIGN.md
├── src/
│   ├── App.svelte
│   ├── app.css
│   ├── main.ts
│   └── lib/
│       ├── api.ts
│       ├── cache.ts
│       ├── theme.ts
│       ├── types.ts
│       └── components/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   └── src/
│       ├── main.rs
│       ├── audio_cache.rs
│       ├── theme.rs
│       └── player/
└── crates/
    └── siren-core/
        └── src/
            ├── lib.rs
            ├── api.rs
            ├── audio.rs
            └── downloader.rs
```

## 许可证

MIT
