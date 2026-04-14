use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

const APP_CACHE_DIR: &str = "siren-music-download";
const AUDIO_CACHE_DIR: &str = "audio";

pub fn audio_cache_dir() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| std::env::temp_dir().join("cache"))
        .join(APP_CACHE_DIR)
        .join(AUDIO_CACHE_DIR)
}

pub fn ensure_audio_cache_dir() -> Result<PathBuf> {
    let dir = audio_cache_dir();
    fs::create_dir_all(&dir).context("Failed to create audio cache directory")?;
    Ok(dir)
}

pub fn cached_song_path(song_cid: &str, source_url: &str) -> Result<PathBuf> {
    let extension = Path::new(source_url.split('?').next().unwrap_or(source_url))
        .extension()
        .and_then(|value| value.to_str())
        .filter(|value| !value.is_empty())
        .unwrap_or("bin");

    Ok(ensure_audio_cache_dir()?.join(format!("{song_cid}.{extension}")))
}

pub fn pending_marker_path(cache_path: &Path) -> PathBuf {
    let mut marker_name = cache_path
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("audio")
        .to_string();
    marker_name.push_str(".pending");
    cache_path.with_file_name(marker_name)
}

pub fn is_song_cached(cache_path: &Path) -> bool {
    cache_path.is_file() && !pending_marker_path(cache_path).exists()
}

pub fn clear_audio_cache() -> Result<u64> {
    let dir = ensure_audio_cache_dir()?;
    let mut removed = 0_u64;

    for entry in fs::read_dir(&dir).context("Failed to read audio cache directory")? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            fs::remove_dir_all(&path)
                .with_context(|| format!("Failed to remove cache directory {}", path.display()))?;
        } else {
            fs::remove_file(&path)
                .with_context(|| format!("Failed to remove cache file {}", path.display()))?;
        }
        removed += 1;
    }

    Ok(removed)
}
