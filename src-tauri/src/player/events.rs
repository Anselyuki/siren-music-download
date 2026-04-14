use crate::player::state::PlayerState;
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter};

/// Tauri event emitted when playback state, queue flags, or volume changes.
///
/// The payload is a full [`PlayerState`] snapshot.
pub const PLAYER_STATE_CHANGED: &str = "player-state-changed";
/// Tauri event emitted while playback is advancing.
///
/// The payload is also a full [`PlayerState`] snapshot so the frontend can
/// update time, duration, and active song metadata from a single shape.
pub const PLAYER_PROGRESS: &str = "player-progress";

/// Emit [`PLAYER_STATE_CHANGED`] with the current player snapshot.
pub fn emit_state(app: &AppHandle, state: &Arc<Mutex<PlayerState>>) {
    let _ = app.emit(PLAYER_STATE_CHANGED, state.lock().unwrap().clone());
}

/// Emit [`PLAYER_PROGRESS`] with the current player snapshot.
pub fn emit_progress(app: &AppHandle, state: &Arc<Mutex<PlayerState>>) {
    let _ = app.emit(PLAYER_PROGRESS, state.lock().unwrap().clone());
}
