use crate::api::{AlbumDetail, ApiClient, SongDetail};
use crate::audio::{
    detect_image_mime, encode_cover_as_jpeg, sanitize_filename, save_audio, tag_flac, AudioFormat,
    FlacMetadata, OutputFormat,
};
use crate::download::model::DownloadTaskStatus;
use anyhow::{anyhow, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

/// 下载 FLAC 时可选的标签元数据覆盖项。
/// 空字符串或空数组表示“沿用接口返回值”。
pub struct MetaOverride {
    /// 覆盖写入 FLAC 标签的专辑名。
    pub album_name: String,
    /// 覆盖写入 FLAC 标签的艺术家列表。
    pub artists: Vec<String>,
    /// 覆盖写入 FLAC 标签的专辑艺术家列表。
    pub album_artists: Vec<String>,
}

/// 下载过程中产生的进度信息。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DownloadStage {
    Downloading,
    Writing,
}

/// 下载过程中产生的进度信息。
#[derive(Debug, Clone)]
pub struct DownloadProgress {
    /// 当前这条进度对应的歌曲名。
    pub song_name: String,
    /// 当前任务阶段。
    pub status: DownloadTaskStatus,
    /// 底层下载流程阶段。
    pub stage: DownloadStage,
    /// 当前文件已接收的字节数。
    pub bytes_done: u64,
    /// 当前文件的总字节数，未知时为 `None`。
    pub bytes_total: Option<u64>,
    /// 当前歌曲在本批次中的下标，从 0 开始。
    pub song_index: usize,
    /// 本批次的歌曲总数。
    pub song_count: usize,
}

fn lyric_sidecar_path(audio_path: &Path) -> PathBuf {
    audio_path.with_extension("lrc")
}

fn write_lyric_sidecar(audio_path: &Path, lyric_text: &str) -> Result<PathBuf> {
    let lyric_path = lyric_sidecar_path(audio_path);
    std::fs::write(&lyric_path, lyric_text.as_bytes())
        .with_context(|| format!("Failed to write lyric sidecar {}", lyric_path.display()))?;
    Ok(lyric_path)
}

async fn fetch_lyric_text(client: &ApiClient, song: &SongDetail) -> Result<Option<String>> {
    let Some(lyric_url) = song.lyric_url.as_deref().filter(|url| !url.is_empty()) else {
        return Ok(None);
    };

    let lyric_text = client
        .download_text(&lyric_url)
        .await
        .with_context(|| format!("Failed to download lyric text for {}", song.name))?;
    if lyric_text.trim().is_empty() {
        return Ok(None);
    }

    Ok(Some(lyric_text))
}

fn ensure_not_cancelled(cancellation_flag: Option<&Arc<AtomicBool>>) -> Result<()> {
    if cancellation_flag
        .map(|flag| flag.load(Ordering::SeqCst))
        .unwrap_or(false)
    {
        return Err(anyhow!("download cancelled"));
    }

    Ok(())
}

fn emit_progress(
    on_progress: &impl Fn(DownloadProgress),
    song_name: &str,
    stage: DownloadStage,
    bytes_done: u64,
    bytes_total: Option<u64>,
    song_index: usize,
    song_count: usize,
) {
    let status = match stage {
        DownloadStage::Downloading => DownloadTaskStatus::Downloading,
        DownloadStage::Writing => DownloadTaskStatus::Writing,
    };

    on_progress(DownloadProgress {
        song_name: song_name.to_string(),
        status,
        stage,
        bytes_done,
        bytes_total,
        song_index,
        song_count,
    });
}

pub fn album_output_dir(base_out_dir: &Path, album_name: &str) -> PathBuf {
    base_out_dir.join(sanitize_filename(album_name))
}

fn cover_extension_from_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/gif" => "gif",
        "image/webp" => "webp",
        _ => "jpg",
    }
}

pub fn write_album_cover_bytes(album_dir: &Path, cover_bytes: &[u8]) -> Result<PathBuf> {
    std::fs::create_dir_all(album_dir)?;

    let mime = detect_image_mime(cover_bytes).unwrap_or("image/jpeg");
    let extension = cover_extension_from_mime(mime);
    let cover_path = album_dir.join(format!("cover.{extension}"));

    std::fs::write(&cover_path, cover_bytes)
        .with_context(|| format!("Failed to write album cover {}", cover_path.display()))?;

    Ok(cover_path)
}

pub fn album_cover_exists(album_dir: &Path) -> bool {
    ["jpg", "png", "gif", "webp"]
        .iter()
        .map(|extension| album_dir.join(format!("cover.{extension}")))
        .any(|path| path.exists())
}

pub async fn download_album_cover(
    client: &ApiClient,
    album: &AlbumDetail,
    album_dir: &Path,
    cancellation_flag: Option<&Arc<AtomicBool>>,
) -> Result<Option<PathBuf>> {
    ensure_not_cancelled(cancellation_flag)?;

    let cover_bytes = match client.download_bytes(&album.cover_url, |_, _| {}).await {
        Ok(bytes) => bytes,
        Err(_) => return Ok(None),
    };

    ensure_not_cancelled(cancellation_flag)?;

    write_album_cover_bytes(album_dir, &cover_bytes).map(Some)
}

/// 下载单首歌曲到磁盘，并在可能时为 FLAC 文件写入标签。
///
/// `format` 用于指定目标输出格式。除非源文件是 WAV 且目标格式为
/// [`OutputFormat::Flac`]，否则 WAV 和 MP3 会按原始格式直接写出；
/// 如果满足上述条件，则会先用纯 Rust 方案转码为 FLAC，再按需写入
/// FLAC 元数据。
///
/// 回调会在每个下载分块结束后触发，并始终把当前任务视为单文件批次，
/// 因此 `song_index = 0`、`song_count = 1`。
///
/// 返回最终写入的文件路径。
pub async fn download_song(
    client: &ApiClient,
    song: &SongDetail,
    album: &AlbumDetail,
    out_dir: &Path,
    format: OutputFormat,
    download_lyrics: bool,
    meta: &MetaOverride,
    cancellation_flag: Option<Arc<AtomicBool>>,
    on_progress: impl Fn(DownloadProgress) + Send + 'static,
) -> Result<PathBuf> {
    ensure_not_cancelled(cancellation_flag.as_ref())?;

    // 尝试拉取封面图，失败不影响主流程
    let cover_bytes: Option<Vec<u8>> = client
        .download_bytes(&album.cover_url, |_, _| {})
        .await
        .ok();
    let embedded_cover = cover_bytes
        .as_deref()
        .and_then(|bytes| encode_cover_as_jpeg(bytes).ok());
    ensure_not_cancelled(cancellation_flag.as_ref())?;
    let lyric_text = if download_lyrics {
        fetch_lyric_text(client, song).await?
    } else {
        None
    };

    let name_for_progress = song.name.clone();
    let progress_fn = Arc::new(on_progress);
    let pfn = Arc::clone(&progress_fn);
    let cancellation_flag_for_download = cancellation_flag.clone();

    let mut audio_bytes = Vec::new();
    client
        .download_stream(&song.source_url, |chunk, done, total| {
            if cancellation_flag_for_download
                .as_ref()
                .map(|flag| flag.load(Ordering::SeqCst))
                .unwrap_or(false)
            {
                return Ok(false);
            }

            audio_bytes.extend_from_slice(chunk);
            emit_progress(
                pfn.as_ref(),
                &name_for_progress,
                DownloadStage::Downloading,
                done,
                total,
                0,
                1,
            );

            Ok(true)
        })
        .await?;

    ensure_not_cancelled(cancellation_flag.as_ref())?;
    emit_progress(
        progress_fn.as_ref(),
        &song.name,
        DownloadStage::Writing,
        audio_bytes.len() as u64,
        Some(audio_bytes.len() as u64),
        0,
        1,
    );

    let out_path = save_audio(&audio_bytes, out_dir, &song.name, format)
        .with_context(|| format!("Failed to save audio file for {}", song.name))?;

    // 为 FLAC 文件写入标签，并在有覆盖项时优先使用覆盖值
    if AudioFormat::detect(&audio_bytes) == AudioFormat::Flac
        || (AudioFormat::detect(&audio_bytes) == AudioFormat::Wav && format == OutputFormat::Flac)
    {
        if out_path.extension().and_then(|e| e.to_str()) == Some("flac") {
            let eff_album = if meta.album_name.is_empty() {
                &album.name
            } else {
                &meta.album_name
            };
            let eff_artists = if meta.artists.is_empty() {
                &song.artists
            } else {
                &meta.artists
            };
            let eff_album_artists = if meta.album_artists.is_empty() {
                album
                    .artists
                    .as_deref()
                    .filter(|artists| !artists.is_empty())
                    .unwrap_or(eff_artists)
            } else {
                &meta.album_artists
            };
            let track_number = album
                .songs
                .iter()
                .position(|entry| entry.cid == song.cid)
                .map(|index| (index + 1) as u32);
            let total_tracks = (!album.songs.is_empty()).then_some(album.songs.len() as u32);
            let cover = embedded_cover.as_deref().map(|bytes| ("image/jpeg", bytes));

            ensure_not_cancelled(cancellation_flag.as_ref())?;
            tag_flac(
                &out_path,
                &FlacMetadata {
                    title: &song.name,
                    artists: eff_artists,
                    album: eff_album,
                    album_artists: eff_album_artists,
                    track_number,
                    total_tracks,
                    disc_number: Some(1),
                    total_discs: Some(1),
                    cover,
                },
            )
            .with_context(|| format!("Failed to write FLAC metadata for {}", song.name))?;
        }
    }

    if let Some(lyric_text) = lyric_text.as_deref() {
        ensure_not_cancelled(cancellation_flag.as_ref())?;
        write_lyric_sidecar(&out_path, lyric_text)
            .with_context(|| format!("Failed to save lyric sidecar for {}", song.name))?;
    }

    Ok(out_path)
}

/// 下载整张专辑到 `out_dir/<album_name>/` 目录下。
///
/// 会在每个文件的每个下载分块后调用 `on_progress`，并按专辑曲序返回
/// 最终写入的文件路径列表。
pub async fn download_album(
    client: &ApiClient,
    album_cid: &str,
    base_out_dir: &Path,
    format: OutputFormat,
    download_lyrics: bool,
    on_progress: impl Fn(DownloadProgress) + Send + Clone + 'static,
) -> Result<Vec<PathBuf>> {
    let album = client.get_album_detail(album_cid).await?;
    let song_count = album.songs.len();

    let album_dir = album_output_dir(base_out_dir, &album.name);
    std::fs::create_dir_all(&album_dir)?;

    let cover_bytes: Option<Vec<u8>> = client
        .download_bytes(&album.cover_url, |_, _| {})
        .await
        .ok();
    if let Some(cover_bytes) = cover_bytes.as_deref() {
        let _ = write_album_cover_bytes(&album_dir, cover_bytes);
    }
    let embedded_cover = cover_bytes
        .as_deref()
        .and_then(|bytes| encode_cover_as_jpeg(bytes).ok());

    let mut paths = Vec::new();

    for (idx, song_entry) in album.songs.iter().enumerate() {
        let song_detail = client.get_song_detail(&song_entry.cid).await?;
        let lyric_text = if download_lyrics {
            fetch_lyric_text(client, &song_detail).await?
        } else {
            None
        };
        let prog = on_progress.clone();
        let song_name = song_detail.name.clone();

        let audio_bytes = client
            .download_bytes(&song_detail.source_url, move |done, total| {
                emit_progress(
                    &prog,
                    &song_name,
                    DownloadStage::Downloading,
                    done,
                    total,
                    idx,
                    song_count,
                );
            })
            .await?;

        emit_progress(
            &on_progress,
            &song_detail.name,
            DownloadStage::Writing,
            audio_bytes.len() as u64,
            Some(audio_bytes.len() as u64),
            idx,
            song_count,
        );

        let out_path = save_audio(&audio_bytes, &album_dir, &song_detail.name, format)
            .with_context(|| format!("Failed to save audio file for {}", song_detail.name))?;

        if out_path.extension().and_then(|e| e.to_str()) == Some("flac") {
            let album_artists = album
                .artists
                .as_deref()
                .filter(|artists| !artists.is_empty())
                .unwrap_or(&song_detail.artists);
            let cover = embedded_cover.as_deref().map(|bytes| ("image/jpeg", bytes));

            tag_flac(
                &out_path,
                &FlacMetadata {
                    title: &song_detail.name,
                    artists: &song_detail.artists,
                    album: &album.name,
                    album_artists,
                    track_number: Some((idx + 1) as u32),
                    total_tracks: Some(song_count as u32),
                    disc_number: Some(1),
                    total_discs: Some(1),
                    cover,
                },
            )
            .with_context(|| format!("Failed to write FLAC metadata for {}", song_detail.name))?;
        }

        if let Some(lyric_text) = lyric_text.as_deref() {
            write_lyric_sidecar(&out_path, lyric_text)
                .with_context(|| format!("Failed to save lyric sidecar for {}", song_detail.name))?;
        }

        paths.push(out_path);
    }

    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::{
        album_output_dir, download_song, lyric_sidecar_path, write_album_cover_bytes,
        write_lyric_sidecar, MetaOverride,
    };
    use crate::api::ApiClient;
    use crate::audio::OutputFormat;
    use anyhow::Result;
    use std::path::Path;

    #[test]
    fn builds_album_output_dir_from_sanitized_album_name() {
        let base_dir = Path::new("/tmp/downloads");
        let album_dir = album_output_dir(base_dir, "A/B:C?D");

        assert_eq!(album_dir, Path::new("/tmp/downloads").join("A_B_C_D"));
    }

    #[test]
    fn writes_album_cover_with_stable_name_and_detected_extension() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let album_dir = temp_dir.path().join("album");
        let png_bytes = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0x00];

        let cover_path = write_album_cover_bytes(&album_dir, &png_bytes)?;

        assert_eq!(cover_path, album_dir.join("cover.png"));
        assert!(cover_path.exists(), "cover file should exist");
        assert_eq!(std::fs::read(&cover_path)?, png_bytes);

        Ok(())
    }

    #[test]
    fn writes_lrc_sidecar_next_to_audio_file() -> Result<()> {
        let temp_dir = tempfile::tempdir()?;
        let audio_path = temp_dir.path().join("In Due Time.flac");

        std::fs::write(&audio_path, b"fLaC")?;
        let lyric_path = write_lyric_sidecar(&audio_path, "[00:01.000]In Due Time")?;

        assert_eq!(lyric_path, lyric_sidecar_path(&audio_path));
        assert_eq!(
            std::fs::read_to_string(&lyric_path)?,
            "[00:01.000]In Due Time"
        );

        Ok(())
    }

    #[tokio::test]
    #[ignore = "hits the live Monster Siren API"]
    async fn downloads_real_flac_with_metadata() -> Result<()> {
        let client = ApiClient::new()?;
        let song = client.get_song_detail("048760").await?;
        let album = client.get_album_detail(&song.album_cid).await?;
        let temp_dir = tempfile::tempdir()?;

        let output_path = download_song(
            &client,
            &song,
            &album,
            temp_dir.path(),
            OutputFormat::Flac,
            true,
            &MetaOverride {
                album_name: String::new(),
                artists: Vec::new(),
                album_artists: Vec::new(),
            },
            None,
            |_| {},
        )
        .await?;

        let tag = metaflac::Tag::read_from_path(&output_path)?;
        let comments = tag
            .vorbis_comments()
            .ok_or_else(|| anyhow::anyhow!("missing vorbis comments"))?;

        assert_eq!(
            comments.title().map(|items| items.as_slice()),
            Some([song.name.clone()].as_slice())
        );
        assert_eq!(
            comments.artist().map(|items| items.as_slice()),
            Some(song.artists.as_slice())
        );
        assert_eq!(
            comments.album().map(|items| items.as_slice()),
            Some([album.name.clone()].as_slice())
        );
        assert_eq!(
            comments.album_artist().map(|items| items.as_slice()),
            Some(
                album
                    .artists
                    .as_deref()
                    .filter(|artists| !artists.is_empty())
                    .unwrap_or(&song.artists)
            )
        );
        assert_eq!(comments.track(), Some(1));
        assert_eq!(comments.total_tracks(), Some(album.songs.len() as u32));
        let picture = tag
            .pictures()
            .next()
            .ok_or_else(|| anyhow::anyhow!("expected embedded cover art"))?;
        assert_eq!(picture.mime_type, "image/jpeg");

        let lyric_path = output_path.with_extension("lrc");
        assert!(lyric_path.exists(), "expected lyric sidecar file");
        let lyric_text = std::fs::read_to_string(&lyric_path)?;
        assert!(
            lyric_text.contains("[00:"),
            "expected synchronized LRC lyric content"
        );

        Ok(())
    }
}
