//! 塞壬音乐下载器的 Tauri 桌面后端。
//!
//! 这个二进制 crate 通过 Tauri 命令和播放器事件向前端暴露后端能力。
//!
//! # 命令面
//!
//! Svelte 前端通过 `@tauri-apps/api/core::invoke` 调用下面这些命令：
//!
//! - 目录数据：[`get_albums`]、[`get_album_detail`]、[`get_song_detail`]、
//!   [`get_song_lyrics`]
//! - 播放控制：[`play_song`]、[`pause_playback`]、[`resume_playback`]、
//!   [`seek_current_playback`]、[`play_next`]、[`play_previous`]、
//!   [`stop_playback`]、[`get_player_state`]、[`set_playback_volume`]
//! - 下载和工具：[`download_song`]、[`get_default_output_dir`]、
//!   [`clear_audio_cache`]、[`extract_image_theme`]
//!
//! # 事件
//!
//! - [`player::events::PLAYER_STATE_CHANGED`] 会在播放状态、队列能力或音量
//!   变化时发出完整的 [`PlayerState`] 快照。
//! - [`player::events::PLAYER_PROGRESS`] 会在播放推进过程中持续发出完整的
//!   [`PlayerState`] 快照。
//!
//! # 生成 rustdoc
//!
//! 因为 Tauri 命令定义在二进制目标里，请使用：
//!
//! ```bash
//! cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
//! ```

mod audio_cache;
mod player;
mod theme;

use anyhow::{Context, Result};
use player::stream::{GrowingFileHandle, PlaybackInput, SampleBuffer};
use player::{AudioPlayer, PlaybackContext, PlaybackQueueEntry, PlayerState};
use siren_core::{download_song as download_song_file, MetaOverride, OutputFormat};
use souvlaki::{MediaControlEvent, SeekDirection};
use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::Arc;
use tauri::{LogicalSize, Manager, State, WebviewWindow};

#[derive(Clone)]
struct AppState {
    player: Arc<AudioPlayer>,
    api: Arc<siren_core::ApiClient>,
}

impl AppState {
    fn new(app: tauri::AppHandle) -> Result<Self, String> {
        let player = AudioPlayer::new(app).map_err(|e| e.to_string())?;
        let api = siren_core::ApiClient::new().map_err(|e| e.to_string())?;
        Ok(Self {
            player: Arc::new(player),
            api: Arc::new(api),
        })
    }

    async fn play_song_internal(
        &self,
        song_cid: String,
        cover_url: Option<String>,
        playback_context: Option<PlaybackContext>,
    ) -> Result<f64, String> {
        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

        self.player.prepare_playback_context(
            playback_context,
            PlaybackQueueEntry {
                cid: song_cid.clone(),
                name: song_detail.name.clone(),
                artists: song_detail.artists.clone(),
                cover_url: cover_url.clone(),
            },
        );

        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                cover_url,
                0.0,
                None,
            )
            .map_err(|e| e.to_string())?;

        let result: Result<f64> = async {
            self.start_playback_session(session_id, &song_cid, &song_detail.source_url, 0.0)
                .await
        }
        .await;

        match result {
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    async fn seek_current_internal(&self, position_secs: f64) -> Result<f64, String> {
        let current_state = self.player.get_state();
        let song_cid = current_state
            .song_cid
            .clone()
            .ok_or_else(|| "No active track".to_string())?;

        if current_state.is_loading {
            return Err("Playback is still loading".to_string());
        }

        let target_position = normalize_seek_position(position_secs, current_state.duration);
        if (current_state.progress - target_position).abs() < 0.05 {
            return Ok(current_state.duration);
        }

        let song_detail = self
            .api
            .get_song_detail(&song_cid)
            .await
            .map_err(|e| e.to_string())?;

        let session_id = self
            .player
            .begin_loading_session(
                song_cid.clone(),
                song_detail.name.clone(),
                song_detail.artists.clone(),
                current_state.cover_url.clone(),
                target_position,
                (current_state.duration > 0.0).then_some(current_state.duration),
            )
            .map_err(|e| e.to_string())?;

        let should_pause_after_seek = current_state.is_paused;
        let result: Result<f64> = async {
            let duration = self
                .start_playback_session(
                    session_id,
                    &song_cid,
                    &song_detail.source_url,
                    target_position,
                )
                .await?;

            if should_pause_after_seek {
                self.player.pause()?;
            }

            Ok(duration)
        }
        .await;

        match result {
            Ok(duration) => Ok(duration),
            Err(error) => {
                self.player.fail_session(session_id);
                Err(error.to_string())
            }
        }
    }

    async fn start_playback_session(
        &self,
        session_id: u64,
        song_cid: &str,
        source_url: &str,
        start_position_secs: f64,
    ) -> Result<f64> {
        let stop_flag = self.player.stop_signal();
        let pause_flag = self.player.pause_signal();
        let cache_path = audio_cache::cached_song_path(song_cid, source_url)?;
        let pending_marker = audio_cache::pending_marker_path(&cache_path);

        let input = if audio_cache::is_song_cached(&cache_path) {
            PlaybackInput::cached_file(cache_path)
        } else {
            let _ = std::fs::remove_file(&cache_path);
            let _ = std::fs::remove_file(&pending_marker);
            std::fs::write(&pending_marker, b"pending").with_context(|| {
                format!("Failed to create cache marker {}", pending_marker.display())
            })?;

            let (handle, mut writer) = GrowingFileHandle::new(cache_path.clone())?;
            let api = Arc::clone(&self.api);
            let stop_for_download = Arc::clone(&stop_flag);
            let handle_for_download = handle.clone();
            let source_url = source_url.to_string();
            let cache_path_for_cleanup = cache_path.clone();
            let pending_for_cleanup = pending_marker.clone();

            tokio::spawn(async move {
                let download_result = api
                    .download_stream(&source_url, |chunk, _, _| {
                        if stop_for_download.load(Ordering::SeqCst) {
                            return Ok(false);
                        }
                        handle_for_download.append_chunk(&mut writer, chunk)?;
                        Ok(true)
                    })
                    .await;

                match download_result {
                    Ok(()) if !stop_for_download.load(Ordering::SeqCst) => {
                        handle_for_download.mark_complete();
                        let _ = std::fs::remove_file(&pending_for_cleanup);
                    }
                    Ok(()) => {
                        handle_for_download.mark_error("Playback stopped");
                        let _ = std::fs::remove_file(&pending_for_cleanup);
                        let _ = std::fs::remove_file(&cache_path_for_cleanup);
                    }
                    Err(error) => {
                        eprintln!("[player] download error: {error:#}");
                        handle_for_download.mark_error(error.to_string());
                        let _ = std::fs::remove_file(&pending_for_cleanup);
                        let _ = std::fs::remove_file(&cache_path_for_cleanup);
                    }
                }
            });

            PlaybackInput::growing_file(handle)
        };

        let inspect_input = input.clone();
        let source_format = tokio::task::spawn_blocking(move || inspect_input.inspect_format())
            .await
            .map_err(|error| anyhow::anyhow!(error.to_string()))??;

        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        let output_format = self.player.negotiate_output_format(source_format)?;
        let start_position_secs =
            normalize_seek_position(start_position_secs, source_format.duration_secs);
        let sample_buffer = SampleBuffer::new();
        let _decode_worker = input.spawn_decode_worker(
            source_format,
            output_format,
            sample_buffer.clone(),
            Arc::clone(&stop_flag),
            Arc::clone(&pause_flag),
            start_position_secs,
        )?;

        let minimum_samples =
            ((output_format.sample_rate as usize * output_format.channels as usize) / 3)
                .max(output_format.channels as usize * 4096)
                .min(output_format.channels as usize * 32_768);

        let wait_buffer = sample_buffer.clone();
        let wait_stop = Arc::clone(&stop_flag);
        tokio::task::spawn_blocking(move || {
            wait_buffer.wait_for_samples(minimum_samples, &wait_stop)
        })
        .await
        .map_err(|error| anyhow::anyhow!(error.to_string()))??;

        anyhow::ensure!(
            self.player.is_session_active(session_id),
            "Playback stopped"
        );

        self.player.start_stream_playback(
            session_id,
            output_format,
            sample_buffer,
            start_position_secs,
        )
    }

    async fn play_next_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_next_entry()
            .ok_or_else(|| "No next track available".to_string())?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    async fn play_previous_internal(&self) -> Result<f64, String> {
        let target = self
            .player
            .select_previous_entry()
            .ok_or_else(|| "No previous track available".to_string())?;
        self.play_song_internal(target.cid, target.cover_url, None)
            .await
    }

    fn handle_media_control(&self, event: MediaControlEvent) {
        match event {
            MediaControlEvent::Play => {
                if let Err(error) = self.player.resume() {
                    eprintln!("[media-session] failed to resume playback: {error:#}");
                }
            }
            MediaControlEvent::Pause => {
                if let Err(error) = self.player.pause() {
                    eprintln!("[media-session] failed to pause playback: {error:#}");
                }
            }
            MediaControlEvent::Toggle => {
                if let Err(error) = self.player.toggle_playback() {
                    eprintln!("[media-session] failed to toggle playback: {error:#}");
                }
            }
            MediaControlEvent::Stop | MediaControlEvent::Quit => {
                if let Err(error) = self.player.stop() {
                    eprintln!("[media-session] failed to stop playback: {error:#}");
                }
            }
            MediaControlEvent::Next => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = state.play_next_internal().await {
                        eprintln!("[media-session] failed to play next track: {error}");
                    }
                });
            }
            MediaControlEvent::Previous => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = state.play_previous_internal().await {
                        eprintln!("[media-session] failed to play previous track: {error}");
                    }
                });
            }
            MediaControlEvent::SetPosition(position) => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(error) = state.seek_current_internal(position.0.as_secs_f64()).await
                    {
                        eprintln!("[media-session] failed to seek playback: {error}");
                    }
                });
            }
            MediaControlEvent::SeekBy(direction, delta) => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    let current = state.player.get_state();
                    let delta_secs = delta.as_secs_f64();
                    let target = match direction {
                        SeekDirection::Forward => current.progress + delta_secs,
                        SeekDirection::Backward => current.progress - delta_secs,
                    };
                    if let Err(error) = state.seek_current_internal(target).await {
                        eprintln!("[media-session] failed to seek playback by delta: {error}");
                    }
                });
            }
            MediaControlEvent::Seek(direction) => {
                let state = self.clone();
                tauri::async_runtime::spawn(async move {
                    let current = state.player.get_state();
                    let target = match direction {
                        SeekDirection::Forward => current.progress + 10.0,
                        SeekDirection::Backward => current.progress - 10.0,
                    };
                    if let Err(error) = state.seek_current_internal(target).await {
                        eprintln!("[media-session] failed to seek playback: {error}");
                    }
                });
            }
            _ => {}
        }
    }
}

fn normalize_seek_position(position_secs: f64, duration_secs: f64) -> f64 {
    let position_secs = position_secs.max(0.0);
    if duration_secs > 0.0 {
        position_secs.min((duration_secs - 0.05).max(0.0))
    } else {
        position_secs
    }
}

fn parse_output_format(format: &str) -> Result<OutputFormat, String> {
    match format {
        "wav" => Ok(OutputFormat::Wav),
        "flac" => Ok(OutputFormat::Flac),
        "mp3" => Ok(OutputFormat::Mp3),
        _ => Err(format!("Unsupported output format: {format}")),
    }
}

#[tauri::command]
/// 获取 Monster Siren 公开接口返回的完整专辑列表。
///
/// # 前端调用
///
/// `invoke("get_albums")`
///
/// # 返回值
///
/// 返回 `Vec<siren_core::api::Album>`，包含专辑标识、名称、封面地址和
/// 艺术家列表。
async fn get_albums(state: State<'_, AppState>) -> Result<Vec<siren_core::api::Album>, String> {
    state.api.get_albums().await.map_err(|e| e.to_string())
}

#[tauri::command]
/// 获取单张专辑及其歌曲列表。
///
/// # 前端调用
///
/// `invoke("get_album_detail", { albumCid })`
///
/// # 参数
///
/// - `album_cid`：由 [`get_albums`] 返回的专辑标识。
///
/// # 返回值
///
/// 返回一个 [`siren_core::api::AlbumDetail`]。
async fn get_album_detail(
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
/// 获取单首歌曲的完整元数据。
///
/// # 前端调用
///
/// `invoke("get_song_detail", { cid })`
///
/// # 返回值
///
/// 返回一个 [`siren_core::api::SongDetail`]，其中包含播放和下载地址。
async fn get_song_detail(
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
/// 在歌曲存在歌词时，返回后端拉取到的歌词原文。
///
/// # 前端调用
///
/// `invoke("get_song_lyrics", { cid })`
///
/// # 返回值
///
/// 如果歌曲没有歌词地址，则返回 `None`；否则返回后端下载到的原始歌词文本。
async fn get_song_lyrics(
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
/// 根据图片地址提取一组用于界面的主题色。
///
/// # 前端调用
///
/// `invoke("extract_image_theme", { imageUrl })`
///
/// # 返回值
///
/// 返回一个 [`theme::ThemePalette`]，前端会用它给当前专辑头图和播放器控件着色。
async fn extract_image_theme(
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
/// 获取后端建议的默认下载目录。
///
/// # 前端调用
///
/// `invoke("get_default_output_dir")`
///
/// # 返回值
///
/// 返回平台相关的目录字符串，末尾固定为 `SirenMusic`。
fn get_default_output_dir() -> String {
    dirs::download_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join("SirenMusic")
        .to_string_lossy()
        .to_string()
}

#[tauri::command]
/// 开始播放一首歌曲，并可选地初始化后端播放队列。
///
/// # 前端调用
///
/// `invoke("play_song", { songCid, coverUrl, playbackContext })`
///
/// # 参数
///
/// - `song_cid`：需要通过上游接口解析的歌曲标识。
/// - `cover_url`：可选封面地址，会继续传给媒体会话元数据。
/// - `playback_context`：可选的 [`PlaybackContext`]，供 [`play_next`] 和
///   [`play_previous`] 复用队列信息。
///
/// # 返回值
///
/// 返回解析后的歌曲时长，单位为秒。
///
/// # 副作用
///
/// 会启动或替换当前播放会话、预热音频缓存，并发出
/// [`player::events::PLAYER_STATE_CHANGED`] 和
/// [`player::events::PLAYER_PROGRESS`] 事件。
async fn play_song(
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
/// 停止当前播放，并把播放器状态重置为空闲态。
fn stop_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.stop().map_err(|e| e.to_string())
}

#[tauri::command]
/// 如果当前正在播放音频，则暂停当前播放会话。
fn pause_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.pause().map_err(|e| e.to_string())
}

#[tauri::command]
/// 如果当前加载的是已暂停歌曲，则恢复播放。
fn resume_playback(state: State<'_, AppState>) -> Result<(), String> {
    state.player.resume().map_err(|e| e.to_string())
}

#[tauri::command]
/// 把当前歌曲跳转到指定播放位置。
///
/// # 前端调用
///
/// `invoke("seek_current_playback", { positionSecs })`
///
/// # 参数
///
/// - `position_secs`：目标时间，单位为秒。后端会把它限制在当前已知时长范围内。
///
/// # 返回值
///
/// 返回当前歌曲总时长，单位为秒。
///
/// 如果跳转前处于暂停状态，后端会在重建播放管线后恢复暂停态。
async fn seek_current_playback(
    state: State<'_, AppState>,
    position_secs: f64,
) -> Result<f64, String> {
    state.seek_current_internal(position_secs).await
}

#[tauri::command]
/// 切到最近一次 [`play_song`] 携带的 [`PlaybackContext`] 中的下一首。
///
/// 返回下一首歌曲的时长，单位为秒。
async fn play_next(state: State<'_, AppState>) -> Result<f64, String> {
    state.play_next_internal().await
}

#[tauri::command]
/// 切到最近一次 [`play_song`] 携带的 [`PlaybackContext`] 中的上一首。
///
/// 返回上一首歌曲的时长，单位为秒。
async fn play_previous(state: State<'_, AppState>) -> Result<f64, String> {
    state.play_previous_internal().await
}

#[tauri::command]
/// 获取后端最新的播放器状态快照。
///
/// 返回结构与播放事件中发出的状态快照一致。
fn get_player_state(state: State<'_, AppState>) -> Result<PlayerState, String> {
    Ok(state.player.get_state())
}

#[tauri::command]
/// 更新播放音量，并返回限制后的实际值。
///
/// # 前端调用
///
/// `invoke("set_playback_volume", { volume })`
///
/// # 参数
///
/// - `volume`：期望设置的音量，范围为 `0.0..=1.0`。超出范围的值会先被限制再保存。
fn set_playback_volume(state: State<'_, AppState>, volume: f64) -> Result<f64, String> {
    Ok(state.player.set_volume(volume))
}

#[tauri::command]
/// 把单首歌曲下载到用户选择的目录。
///
/// # 前端调用
///
/// `invoke("download_song", { songCid, outputDir, format, downloadLyrics })`
///
/// # 参数
///
/// - `song_cid`：需要下载的歌曲标识。
/// - `output_dir`：用户选择的目标目录。
/// - `format`：小写输出格式字符串，可选值为 `wav`、`flac`、`mp3`。
/// - `download_lyrics`：如果歌曲存在歌词，是否同时下载同名 `.lrc` 附带文件。
///
/// # 返回值
///
/// 返回最终写入文件的绝对路径字符串。
///
/// # 说明
///
/// 当源文件可转换为 FLAC 或本身就是 FLAC 时，输出文件会尽量保留标题、
/// 艺术家、专辑、专辑艺术家、曲序和封面等元数据。
/// 如果启用了 `download_lyrics` 且歌词可用，会在音频文件旁边写入同名 `.lrc`。
async fn download_song(
    state: State<'_, AppState>,
    song_cid: String,
    output_dir: String,
    format: String,
    download_lyrics: bool,
) -> Result<String, String> {
    let song = state
        .api
        .get_song_detail(&song_cid)
        .await
        .map_err(|e| e.to_string())?;
    let album = state
        .api
        .get_album_detail(&song.album_cid)
        .await
        .map_err(|e| e.to_string())?;
    let output_format = parse_output_format(&format)?;
    let output_path = download_song_file(
        state.api.as_ref(),
        &song,
        &album,
        Path::new(&output_dir),
        output_format,
        download_lyrics,
        &MetaOverride {
            album_name: String::new(),
            artists: Vec::new(),
            album_artists: Vec::new(),
        },
        |_| {},
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
/// 停止播放，并清理为流式播放生成的音频缓存文件。
///
/// 返回本次删除的缓存条目数量。
fn clear_audio_cache(state: State<'_, AppState>) -> Result<u64, String> {
    state.player.stop().map_err(|e| e.to_string())?;
    audio_cache::clear_audio_cache().map_err(|e| e.to_string())
}

const PLAYER_BAR_SAFE_WINDOW_WIDTH: f64 = 1120.0;
const MIN_LAYOUT_WINDOW_WIDTH: f64 = 1120.0;
const DEFAULT_WINDOW_HEIGHT: f64 = 800.0;
const MIN_WINDOW_HEIGHT: f64 = 600.0;
const WINDOW_MARGIN_X: f64 = 48.0;
const WINDOW_MARGIN_Y: f64 = 72.0;

fn fit_main_window_to_monitor<R: tauri::Runtime>(window: &WebviewWindow<R>) -> tauri::Result<()> {
    let monitor = window.current_monitor()?.or(window.primary_monitor()?);
    let Some(monitor) = monitor else {
        return Ok(());
    };

    let work_area = monitor.work_area();
    let scale_factor = monitor.scale_factor().max(1.0);
    let available_width = work_area.size.width as f64 / scale_factor;
    let available_height = work_area.size.height as f64 / scale_factor;
    if available_width <= 0.0 || available_height <= 0.0 {
        return Ok(());
    }

    let max_width = if available_width > WINDOW_MARGIN_X {
        available_width - WINDOW_MARGIN_X
    } else {
        available_width
    };
    let max_height = if available_height > WINDOW_MARGIN_Y {
        available_height - WINDOW_MARGIN_Y
    } else {
        available_height
    };

    let width = PLAYER_BAR_SAFE_WINDOW_WIDTH.min(max_width).round();
    let height = DEFAULT_WINDOW_HEIGHT.min(max_height).round();
    let min_width = MIN_LAYOUT_WINDOW_WIDTH.min(width).round();
    let min_height = MIN_WINDOW_HEIGHT.min(height).round();

    window.set_min_size(Some(LogicalSize::new(min_width, min_height)))?;
    window.set_size(LogicalSize::new(width, height))?;
    window.center()?;

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let window = app
                .get_webview_window("main")
                .context("Failed to locate main window")?;
            if let Err(error) = fit_main_window_to_monitor(&window) {
                eprintln!("[window] failed to fit main window to monitor: {error}");
            }

            let state =
                AppState::new(app.handle().clone()).expect("Failed to initialize app state");
            let media_state = state.clone();
            if let Err(error) = state
                .player
                .bind_media_controls(move |event| media_state.handle_media_control(event))
            {
                eprintln!("[media-session] disabled: {error:#}");
            }
            app.manage(state);

            #[cfg(debug_assertions)]
            {
                window.open_devtools();
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_albums,
            get_album_detail,
            get_song_detail,
            get_song_lyrics,
            extract_image_theme,
            get_default_output_dir,
            play_song,
            stop_playback,
            pause_playback,
            resume_playback,
            seek_current_playback,
            play_next,
            play_previous,
            get_player_state,
            set_playback_volume,
            download_song,
            clear_audio_cache,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
