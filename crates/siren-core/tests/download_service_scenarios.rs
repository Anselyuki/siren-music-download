use siren_core::audio::OutputFormat;
use siren_core::download::model::{
    DownloadErrorCode, DownloadErrorInfo, DownloadJobKind, DownloadJobSnapshot, DownloadJobStatus,
    DownloadManagerSnapshot, DownloadOptions, DownloadTaskSnapshot, DownloadTaskStatus,
};
use siren_core::DownloadService;

fn make_task_snapshot(status: DownloadTaskStatus) -> DownloadTaskSnapshot {
    DownloadTaskSnapshot {
        id: "task-1".to_string(),
        job_id: "job-1".to_string(),
        song_cid: "song-1".to_string(),
        song_name: "Song".to_string(),
        artists: vec!["Artist".to_string()],
        album_cid: "album-1".to_string(),
        album_name: "Album".to_string(),
        status,
        bytes_done: 128,
        bytes_total: Some(512),
        output_path: Some("Album/Song.flac".to_string()),
        error: Some(DownloadErrorInfo {
            code: DownloadErrorCode::Internal,
            message: "persisted".to_string(),
            retryable: true,
            details: None,
        }),
        attempt: 2,
        song_index: 0,
        song_count: 1,
    }
}

fn make_job_snapshot(
    status: DownloadJobStatus,
    tasks: Vec<DownloadTaskSnapshot>,
) -> DownloadJobSnapshot {
    let completed_task_count = tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Completed)
        .count();
    let failed_task_count = tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Failed)
        .count();
    let cancelled_task_count = tasks
        .iter()
        .filter(|task| task.status == DownloadTaskStatus::Cancelled)
        .count();

    DownloadJobSnapshot {
        id: "job-1".to_string(),
        kind: DownloadJobKind::Album,
        status,
        created_at: "2026-04-15T00:00:00Z".to_string(),
        started_at: Some("2026-04-15T00:00:10Z".to_string()),
        finished_at: Some("2026-04-15T00:01:00Z".to_string()),
        options: DownloadOptions {
            output_dir: "/tmp".to_string(),
            format: OutputFormat::Flac,
            download_lyrics: true,
        },
        title: "Album".to_string(),
        task_count: tasks.len(),
        completed_task_count,
        failed_task_count,
        cancelled_task_count,
        tasks,
        error: None,
    }
}

#[test]
fn restores_service_from_manager_snapshot() {
    let snapshot = DownloadManagerSnapshot {
        jobs: vec![DownloadJobSnapshot {
            id: "job-1".to_string(),
            kind: DownloadJobKind::Album,
            status: DownloadJobStatus::Running,
            created_at: "2026-04-15T00:00:00Z".to_string(),
            started_at: Some("2026-04-15T00:00:10Z".to_string()),
            finished_at: None,
            options: DownloadOptions {
                output_dir: "/tmp".to_string(),
                format: OutputFormat::Mp3,
                download_lyrics: false,
            },
            title: "Album".to_string(),
            task_count: 1,
            completed_task_count: 0,
            failed_task_count: 0,
            cancelled_task_count: 0,
            tasks: vec![make_task_snapshot(DownloadTaskStatus::Downloading)],
            error: None,
        }],
        active_job_id: Some("job-1".to_string()),
        queued_job_ids: vec!["job-1".to_string()],
    };

    let service = DownloadService::from_manager_snapshot(snapshot);
    let restored = service.manager_snapshot();

    assert!(restored.active_job_id.is_none());
    assert!(restored.queued_job_ids.is_empty());
    assert_eq!(restored.jobs.len(), 1);
    assert!(matches!(
        restored.jobs[0].status,
        DownloadJobStatus::Running
    ));
    assert!(matches!(
        restored.jobs[0].tasks[0].status,
        DownloadTaskStatus::Downloading
    ));
    assert_eq!(restored.jobs[0].tasks[0].attempt, 2);
}

#[test]
fn recomputes_job_status_from_restored_tasks() {
    let snapshot = DownloadManagerSnapshot {
        jobs: vec![make_job_snapshot(
            DownloadJobStatus::Completed,
            vec![
                make_task_snapshot(DownloadTaskStatus::Completed),
                DownloadTaskSnapshot {
                    id: "task-2".to_string(),
                    ..make_task_snapshot(DownloadTaskStatus::Failed)
                },
            ],
        )],
        active_job_id: None,
        queued_job_ids: Vec::new(),
    };

    let service = DownloadService::from_manager_snapshot(snapshot);
    let restored = service.manager_snapshot();

    assert!(matches!(
        restored.jobs[0].status,
        DownloadJobStatus::PartiallyFailed
    ));
}

#[test]
fn can_retry_restored_failed_task() {
    let snapshot = DownloadManagerSnapshot {
        jobs: vec![DownloadJobSnapshot {
            error: Some(DownloadErrorInfo {
                code: DownloadErrorCode::Internal,
                message: "failed".to_string(),
                retryable: true,
                details: None,
            }),
            ..make_job_snapshot(
                DownloadJobStatus::Failed,
                vec![make_task_snapshot(DownloadTaskStatus::Failed)],
            )
        }],
        active_job_id: None,
        queued_job_ids: Vec::new(),
    };

    let mut service = DownloadService::from_manager_snapshot(snapshot);
    let retried = service
        .retry_task("job-1", "task-1")
        .expect("job should exist");

    assert!(matches!(retried.status, DownloadJobStatus::Queued));
    assert!(matches!(
        retried.tasks[0].status,
        DownloadTaskStatus::Queued
    ));
    assert_eq!(retried.tasks[0].attempt, 3);
    assert_eq!(retried.tasks[0].bytes_done, 0);
    assert!(retried.tasks[0].error.is_none());
}
