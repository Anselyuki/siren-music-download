use siren_core::local_inventory::is_downloaded_status;
use siren_core::{
    aggregate_album_download_badge, album_badge_from_evidence, badge_for_detected_file,
    badge_for_status, candidate_relative_paths, has_detected_track, matched_track_evidence,
    track_badge_from_matches, AlbumDownloadBadge, LocalAudioFileEvidence,
    LocalAudioFileVerificationState, LocalTrackDownloadStatus, VerificationMode,
};
use std::collections::HashSet;

#[test]
fn builds_root_and_album_candidates_for_all_audio_extensions() {
    let candidates = candidate_relative_paths("A/B:C?D", "Track/01");

    assert!(candidates.contains(&"Track_01.flac".to_string()));
    assert!(candidates.contains(&"Track_01.wav".to_string()));
    assert!(candidates.contains(&"Track_01.mp3".to_string()));
    assert!(candidates.contains(&"A_B_C_D/Track_01.flac".to_string()));
}

#[test]
fn detects_track_from_single_song_or_album_layout() {
    let mut files = HashSet::new();
    files.insert("Album/Track.flac".to_string());
    files.insert("Other.wav".to_string());

    assert!(has_detected_track(&files, "Album", "Track"));
    assert!(has_detected_track(&files, "Anything", "Other"));
    assert!(!has_detected_track(&files, "Album", "Missing"));
}

#[test]
fn collects_matched_track_evidence_with_match_rules() {
    let audio_files = vec![
        LocalAudioFileEvidence {
            relative_path: "Track.flac".to_string(),
            file_size: 12,
            modified_at_ms: Some(10),
            candidate_checksum: None,
            is_in_album_directory: false,
            verification_state: LocalAudioFileVerificationState::Unchecked,
        },
        LocalAudioFileEvidence {
            relative_path: "Album/Track.wav".to_string(),
            file_size: 24,
            modified_at_ms: Some(20),
            candidate_checksum: None,
            is_in_album_directory: true,
            verification_state: LocalAudioFileVerificationState::Unchecked,
        },
        LocalAudioFileEvidence {
            relative_path: "Album/Other.wav".to_string(),
            file_size: 24,
            modified_at_ms: Some(20),
            candidate_checksum: None,
            is_in_album_directory: true,
            verification_state: LocalAudioFileVerificationState::Unchecked,
        },
    ];

    let matches = matched_track_evidence(&audio_files, "Album", "Track");

    assert_eq!(matches.len(), 2);
    assert!(matches
        .iter()
        .any(|item| item.relative_path == "Track.flac"));
    assert!(matches
        .iter()
        .any(|item| item.relative_path == "Album/Track.wav"));
}

#[test]
fn maps_detected_files_to_unverifiable_in_strict_mode() {
    let strict_badge = badge_for_detected_file(VerificationMode::Strict, "v1");
    let relaxed_badge = badge_for_detected_file(VerificationMode::WhenAvailable, "v1");

    assert_eq!(
        strict_badge.download_status,
        LocalTrackDownloadStatus::Unverifiable
    );
    assert_eq!(
        relaxed_badge.download_status,
        LocalTrackDownloadStatus::Detected
    );
    assert!(strict_badge.is_downloaded);
}

#[test]
fn downloaded_status_mapping_matches_contract() {
    assert!(is_downloaded_status(LocalTrackDownloadStatus::Detected));
    assert!(is_downloaded_status(LocalTrackDownloadStatus::Verified));
    assert!(is_downloaded_status(LocalTrackDownloadStatus::Partial));
    assert!(is_downloaded_status(LocalTrackDownloadStatus::Unverifiable));
    assert!(!is_downloaded_status(LocalTrackDownloadStatus::Missing));
    assert!(!is_downloaded_status(LocalTrackDownloadStatus::Mismatch));
    assert!(!is_downloaded_status(LocalTrackDownloadStatus::Unknown));
}

#[test]
fn marks_multiple_matches_as_partial() {
    let matches = vec![
        matched_track_evidence(
            &[LocalAudioFileEvidence {
                relative_path: "Track.flac".to_string(),
                file_size: 12,
                modified_at_ms: Some(10),
                candidate_checksum: None,
                is_in_album_directory: false,
                verification_state: LocalAudioFileVerificationState::Unchecked,
            }],
            "Album",
            "Track",
        )
        .pop()
        .expect("root match"),
        matched_track_evidence(
            &[LocalAudioFileEvidence {
                relative_path: "Album/Track.wav".to_string(),
                file_size: 24,
                modified_at_ms: Some(20),
                candidate_checksum: None,
                is_in_album_directory: true,
                verification_state: LocalAudioFileVerificationState::Unchecked,
            }],
            "Album",
            "Track",
        )
        .pop()
        .expect("album match"),
    ];

    let badge = track_badge_from_matches(&matches, VerificationMode::WhenAvailable, "v1");
    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Partial);
    assert!(badge.is_downloaded);
}

#[test]
fn album_badge_is_missing_when_aggregate_input_is_empty() {
    let badge = aggregate_album_download_badge(&[], "v1");

    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Missing);
    assert!(!badge.is_downloaded);
}

#[test]
fn album_badge_is_verified_when_all_tracks_are_verified() {
    let badge = aggregate_album_download_badge(
        &[
            badge_for_status(LocalTrackDownloadStatus::Verified, "v1"),
            badge_for_status(LocalTrackDownloadStatus::Verified, "v1"),
        ],
        "v1",
    );

    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Verified);
    assert!(badge.is_downloaded);
}

#[test]
fn album_badge_is_mismatch_when_any_track_is_mismatch() {
    let badge = aggregate_album_download_badge(
        &[
            badge_for_status(LocalTrackDownloadStatus::Mismatch, "v1"),
            badge_for_status(LocalTrackDownloadStatus::Detected, "v1"),
        ],
        "v1",
    );

    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Mismatch);
    assert!(!badge.is_downloaded);
}

#[test]
fn album_badge_is_partial_when_only_some_tracks_are_downloaded() {
    let badge = aggregate_album_download_badge(
        &[
            badge_for_status(LocalTrackDownloadStatus::Detected, "v1"),
            badge_for_status(LocalTrackDownloadStatus::Missing, "v1"),
        ],
        "v1",
    );

    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Partial);
    assert!(badge.is_downloaded);
}

#[test]
fn album_badge_is_unverifiable_when_all_tracks_are_downloaded_but_not_verifiable() {
    let badge = aggregate_album_download_badge(
        &[
            badge_for_status(LocalTrackDownloadStatus::Unverifiable, "v1"),
            badge_for_status(LocalTrackDownloadStatus::Detected, "v1"),
        ],
        "v1",
    );

    assert_eq!(
        badge.download_status,
        LocalTrackDownloadStatus::Unverifiable
    );
    assert!(badge.is_downloaded);
}

#[test]
fn album_badge_is_missing_when_all_tracks_are_missing() {
    let badge = aggregate_album_download_badge(
        &[
            badge_for_status(LocalTrackDownloadStatus::Missing, "v1"),
            badge_for_status(LocalTrackDownloadStatus::Missing, "v1"),
        ],
        "v1",
    );

    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Missing);
    assert!(!badge.is_downloaded);
}

#[test]
fn album_list_badge_uses_album_directory_evidence_conservatively() {
    let badge: AlbumDownloadBadge = album_badge_from_evidence(
        &[LocalAudioFileEvidence {
            relative_path: "Album/Track.flac".to_string(),
            file_size: 12,
            modified_at_ms: Some(10),
            candidate_checksum: None,
            is_in_album_directory: true,
            verification_state: LocalAudioFileVerificationState::Unchecked,
        }],
        "Album",
        "v1",
    );

    assert_eq!(badge.download_status, LocalTrackDownloadStatus::Partial);
    assert!(badge.is_downloaded);
}
