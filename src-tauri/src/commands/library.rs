use crate::app_state::AppState;
use crate::theme;
use tauri::State;

#[tauri::command]
pub async fn get_albums(state: State<'_, AppState>) -> Result<Vec<siren_core::api::Album>, String> {
    state.api.get_albums().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_album_detail(
    state: State<'_, AppState>,
    album_cid: String,
) -> Result<siren_core::api::AlbumDetail, String> {
    state
        .api
        .get_album_detail(&album_cid)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_song_detail(
    state: State<'_, AppState>,
    cid: String,
) -> Result<siren_core::api::SongDetail, String> {
    state
        .api
        .get_song_detail(&cid)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_song_lyrics(
    state: State<'_, AppState>,
    cid: String,
) -> Result<Option<String>, String> {
    let song_detail = state
        .api
        .get_song_detail(&cid)
        .await
        .map_err(|e| e.to_string())?;

    let Some(lyric_url) = song_detail.lyric_url else {
        return Ok(None);
    };

    state
        .api
        .download_text(&lyric_url)
        .await
        .map(Some)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn extract_image_theme(
    state: State<'_, AppState>,
    image_url: String,
) -> Result<theme::ThemePalette, String> {
    let bytes = state
        .api
        .download_bytes(&image_url, |_, _| {})
        .await
        .map_err(|e| e.to_string())?;

    tokio::task::spawn_blocking(move || theme::extract_theme_palette(&bytes))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_default_output_dir() -> String {
    dirs::download_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("SirenMusic")
        .to_string_lossy()
        .to_string()
}
