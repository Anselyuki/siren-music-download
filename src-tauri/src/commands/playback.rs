use crate::app_state::AppState;
use crate::player::{PlaybackContext, PlayerState};
use tauri::State;

#[tauri::command]
pub async fn play_song(
    state: State<'_, AppState>,
    song_cid: String,
    cover_url: Option<String>,
    playback_context: Option<PlaybackContext>,
) -> Result<f64, String> {
    state
        .play_song_internal(song_cid, cover_url, playback_context)
        .await
}

#[tauri::command]
pub fn stop_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.stop().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn pause_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.pause().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn resume_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.resume().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn seek_current_playback(
    state: State<'_, AppState>,
    position_secs: f64,
) -> Result<f64, String> {
    state.seek_current_internal(position_secs).await
}

#[tauri::command]
pub async fn play_next(state: State<'_, AppState>) -> Result<f64, String> {
    state.play_next_internal().await
}

#[tauri::command]
pub async fn play_previous(state: State<'_, AppState>) -> Result<f64, String> {
    state.play_previous_internal().await
}

#[tauri::command]
pub fn get_player_state(state: State<'_, AppState>) -> Result<PlayerState, String> {
    Ok(state.player.get_state())
}

#[tauri::command]
pub fn set_playback_volume(state: State<'_, AppState>, volume: f64) -> Result<f64, String> {
    Ok(state.player.set_volume(volume))
}
