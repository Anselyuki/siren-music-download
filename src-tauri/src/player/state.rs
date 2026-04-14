use serde::Serialize;

/// Snapshot emitted to the frontend for playback state updates.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    /// Currently loaded song identifier, or `None` when idle.
    pub song_cid: Option<String>,
    /// Currently loaded song name, or `None` when idle.
    pub song_name: Option<String>,
    /// Artists for the current song.
    pub artists: Vec<String>,
    /// Current artwork URL used by the UI and system media session.
    pub cover_url: Option<String>,
    /// Whether audio is actively playing.
    pub is_playing: bool,
    /// Whether playback is paused without unloading the current song.
    pub is_paused: bool,
    /// Whether the backend is still preparing audio for playback.
    pub is_loading: bool,
    /// Whether the current queue can move to a previous entry.
    pub has_previous: bool,
    /// Whether the current queue can move to a next entry.
    pub has_next: bool,
    /// Playback progress in seconds.
    pub progress: f64,
    /// Total track duration in seconds when known.
    pub duration: f64,
    /// Playback volume clamped to the `0.0..=1.0` range.
    pub volume: f64,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            song_cid: None,
            song_name: None,
            artists: Vec::new(),
            cover_url: None,
            is_playing: false,
            is_paused: false,
            is_loading: false,
            has_previous: false,
            has_next: false,
            progress: 0.0,
            duration: 0.0,
            volume: 1.0,
        }
    }
}
