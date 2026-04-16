//! Tauri commands for the download task system.
//!
//! Phase 1 exposes only job/task CRUD operations (create, list, get, cancel, retry, clear).
//! Actual download execution and progress events are wired in Phase 2.

use crate::app_state::AppState;
use crate::audio_cache;
use crate::downloads::events::{emit_download_job_updated, emit_download_manager_state_changed};
use siren_core::download::model::{
    CreateDownloadJobRequest, DownloadJobSnapshot, DownloadManagerSnapshot,
};
use siren_core::{download_song as download_song_file, MetaOverride};
use std::path::Path;
use tauri::{AppHandle, State};

fn emit_download_state(app: &AppHandle, manager_snapshot: &DownloadManagerSnapshot) {
    emit_download_manager_state_changed(app, manager_snapshot);
    for job in &manager_snapshot.jobs {
        emit_download_job_updated(app, job);
    }
}

// ---------------------------------------------------------------------------
// Legacy single-song download commands (maintained for backward compatibility)
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn download_song(
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
    let output_format = crate::app_state::parse_output_format(&format)?;
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
        None,
        |_| {},
    )
    .await
    .map_err(|e| e.to_string())?;

    Ok(output_path.to_string_lossy().to_string())
}

#[tauri::command]
pub fn clear_audio_cache(state: State<'_, AppState>) -> Result<u64, String> {
    state.player.stop().map_err(|e| e.to_string())?;
    audio_cache::clear_audio_cache().map_err(|e| e.to_string())
}

// ---------------------------------------------------------------------------
// Phase 1: Download job management commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub async fn create_download_job(
    app: AppHandle,
    state: State<'_, AppState>,
    request: CreateDownloadJobRequest,
) -> Result<DownloadJobSnapshot, String> {
    let api = state.api.clone();
    let (job_snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let job_snapshot = service
            .create_job(&api, request)
            .await
            .map_err(|e| e.to_string())?;
        let manager_snapshot = service.manager_snapshot();
        (job_snapshot, manager_snapshot)
    };

    emit_download_job_updated(&app, &job_snapshot);
    emit_download_manager_state_changed(&app, &manager_snapshot);

    Ok(job_snapshot)
}

#[tauri::command]
pub async fn list_download_jobs(
    state: State<'_, AppState>,
) -> Result<DownloadManagerSnapshot, String> {
    let service = state.download_service.lock().await;
    Ok(service.snapshot())
}

#[tauri::command]
pub async fn get_download_job(
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let service = state.download_service.lock().await;
    Ok(service.get_job(&job_id))
}

#[tauri::command]
pub async fn cancel_download_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.cancel_job(&job_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

#[tauri::command]
pub async fn cancel_download_task(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    task_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.cancel_task(&job_id, &task_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

#[tauri::command]
pub async fn retry_download_job(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.retry_job(&job_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

#[tauri::command]
pub async fn retry_download_task(
    app: AppHandle,
    state: State<'_, AppState>,
    job_id: String,
    task_id: String,
) -> Result<Option<DownloadJobSnapshot>, String> {
    let (snapshot, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let snapshot = service.retry_task(&job_id, &task_id);
        let manager_snapshot = service.manager_snapshot();
        (snapshot, manager_snapshot)
    };

    if let Some(job_snapshot) = &snapshot {
        emit_download_job_updated(&app, job_snapshot);
        emit_download_manager_state_changed(&app, &manager_snapshot);
    }

    Ok(snapshot)
}

#[tauri::command]
pub async fn clear_download_history(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let (removed_count, manager_snapshot) = {
        let mut service = state.download_service.lock().await;
        let removed_count = service.clear_history();
        let manager_snapshot = service.manager_snapshot();
        (removed_count, manager_snapshot)
    };

    emit_download_state(&app, &manager_snapshot);

    Ok(removed_count)
}
