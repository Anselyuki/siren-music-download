mod api;
mod audio;
mod downloader;

use std::sync::{Arc, Mutex};
use std::path::PathBuf;

use anyhow::Result;
use slint::{Model, ModelRc, SharedString, VecModel};

use api::ApiClient;
use audio::OutputFormat;
use downloader::{download_album, download_song, DownloadProgress};

slint::include_modules!();

// ── Shared state between callbacks and async tasks ────────────────────────────

struct AppState {
    albums: Vec<api::Album>,
    songs: Vec<api::SongEntry>,
    album_detail: Option<api::AlbumDetail>,
    selected_album_idx: Option<usize>,
    output_dir: PathBuf,
}

impl AppState {
    fn new() -> Self {
        Self {
            albums: Vec::new(),
            songs: Vec::new(),
            album_detail: None,
            selected_album_idx: None,
            output_dir: dirs::download_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join("SirenMusic"),
        }
    }
}

// ── Helpers: convert domain types → Slint model items ────────────────────────

fn to_slint_album(a: &api::Album) -> AlbumItem {
    AlbumItem {
        cid: SharedString::from(a.cid.as_str()),
        name: SharedString::from(a.name.as_str()),
        artists: SharedString::from(a.artists.join(", ").as_str()),
        cover_url: SharedString::from(a.cover_url.as_str()),
    }
}

fn to_slint_song(s: &api::SongEntry) -> SongItem {
    SongItem {
        cid: SharedString::from(s.cid.as_str()),
        name: SharedString::from(s.name.as_str()),
        artists: SharedString::from(s.artists.join(", ").as_str()),
        selected: false,
    }
}

fn output_format_from_index(idx: i32) -> OutputFormat {
    match idx {
        1 => OutputFormat::Flac,
        2 => OutputFormat::Mp3,
        _ => OutputFormat::Wav,
    }
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let rt = tokio::runtime::Runtime::new()?;
    let client = Arc::new(ApiClient::new()?);
    let state = Arc::new(Mutex::new(AppState::new()));

    let ui = AppWindow::new()?;
    let ui_handle = ui.as_weak();

    // Initialise output_dir label
    {
        let dir = state.lock().unwrap().output_dir.display().to_string();
        ui.set_output_dir(SharedString::from(dir));
    }

    // ── load_albums callback ──────────────────────────────────────────────────
    {
        let client = Arc::clone(&client);
        let state = Arc::clone(&state);
        let ui_handle = ui_handle.clone();

        ui.on_load_albums(move || {
            let client = Arc::clone(&client);
            let state = Arc::clone(&state);
            let ui_handle = ui_handle.clone();

            let _ = slint::spawn_local(async move {
                let ui = ui_handle.upgrade().unwrap();
                ui.set_loading_albums(true);
                ui.set_status_text(SharedString::from("Fetching album list…"));

                match client.get_albums().await {
                    Ok(albums) => {
                        let slint_albums: Vec<AlbumItem> =
                            albums.iter().map(to_slint_album).collect();
                        let model = ModelRc::new(VecModel::from(slint_albums));
                        ui.set_albums(model);
                        ui.set_status_text(SharedString::from(format!(
                            "Loaded {} albums.",
                            albums.len()
                        )));
                        state.lock().unwrap().albums = albums;
                    }
                    Err(e) => {
                        ui.set_status_text(SharedString::from(format!("Error: {e}")));
                    }
                }
                ui.set_loading_albums(false);
            });
        });
    }

    // ── select_album callback ─────────────────────────────────────────────────
    {
        let client = Arc::clone(&client);
        let state = Arc::clone(&state);
        let ui_handle = ui_handle.clone();

        ui.on_select_album(move |idx| {
            let client = Arc::clone(&client);
            let state = Arc::clone(&state);
            let ui_handle = ui_handle.clone();

            let album_cid = {
                let st = state.lock().unwrap();
                st.albums.get(idx as usize).map(|a| a.cid.clone())
            };

            let Some(cid) = album_cid else { return };

            let _ = slint::spawn_local(async move {
                let ui = ui_handle.upgrade().unwrap();
                ui.set_selected_album_index(idx);
                ui.set_loading_songs(true);
                ui.set_status_text(SharedString::from("Loading album details…"));

                match client.get_album_detail(&cid).await {
                    Ok(detail) => {
                        let slint_songs: Vec<SongItem> =
                            detail.songs.iter().map(to_slint_song).collect();
                        let model = ModelRc::new(VecModel::from(slint_songs));
                        ui.set_songs(model);
                        ui.set_selected_album_name(SharedString::from(detail.name.as_str()));
                        ui.set_status_text(SharedString::from(format!(
                            "{} songs in this album.",
                            detail.songs.len()
                        )));

                        let mut st = state.lock().unwrap();
                        st.songs = detail.songs.clone();
                        st.album_detail = Some(detail);
                        st.selected_album_idx = Some(idx as usize);
                    }
                    Err(e) => {
                        ui.set_status_text(SharedString::from(format!("Error: {e}")));
                    }
                }
                ui.set_loading_songs(false);
            });
        });
    }

    // ── toggle_song callback ──────────────────────────────────────────────────
    {
        let ui_handle = ui_handle.clone();

        ui.on_toggle_song(move |idx, checked| {
            let ui = ui_handle.upgrade().unwrap();
            let songs_model = ui.get_songs();
            if let Some(model) = songs_model.as_any().downcast_ref::<VecModel<SongItem>>() {
                let mut item = model.row_data(idx as usize).unwrap();
                item.selected = checked;
                model.set_row_data(idx as usize, item);
            }
        });
    }

    // ── select_all / deselect_all ─────────────────────────────────────────────
    {
        let ui_handle = ui_handle.clone();
        ui.on_select_all_songs(move || {
            let ui = ui_handle.upgrade().unwrap();
            let songs_model = ui.get_songs();
            if let Some(model) = songs_model.as_any().downcast_ref::<VecModel<SongItem>>() {
                for i in 0..model.row_count() {
                    let mut item = model.row_data(i).unwrap();
                    item.selected = true;
                    model.set_row_data(i, item);
                }
            }
        });
    }
    {
        let ui_handle = ui_handle.clone();
        ui.on_deselect_all_songs(move || {
            let ui = ui_handle.upgrade().unwrap();
            let songs_model = ui.get_songs();
            if let Some(model) = songs_model.as_any().downcast_ref::<VecModel<SongItem>>() {
                for i in 0..model.row_count() {
                    let mut item = model.row_data(i).unwrap();
                    item.selected = false;
                    model.set_row_data(i, item);
                }
            }
        });
    }

    // ── choose_output_dir callback ────────────────────────────────────────────
    {
        let state = Arc::clone(&state);
        let ui_handle = ui_handle.clone();

        ui.on_choose_output_dir(move || {
            // Native file dialog — use rfd crate in a future iteration.
            // For now, open a terminal prompt or skip.
            let ui = ui_handle.upgrade().unwrap();
            ui.set_status_text(SharedString::from(
                "Tip: set output directory via command-line (--output) for now. \
                 Native dialog coming soon.",
            ));
            let _ = state; // suppress warning
        });
    }

    // ── start_download callback ───────────────────────────────────────────────
    {
        let client = Arc::clone(&client);
        let state = Arc::clone(&state);
        let ui_handle = ui_handle.clone();

        ui.on_start_download(move || {
            let client = Arc::clone(&client);
            let state = Arc::clone(&state);
            let ui_handle = ui_handle.clone();

            let _ = slint::spawn_local(async move {
                let ui = ui_handle.upgrade().unwrap();

                // Collect selected songs & album detail
                let (selected_cids, album_detail, out_dir, fmt_idx) = {
                    let st = state.lock().unwrap();
                    let songs_model = ui.get_songs();
                    let selected: Vec<String> = (0..songs_model.row_count())
                        .filter_map(|i| {
                            let item = songs_model.row_data(i).unwrap();
                            if item.selected {
                                Some(item.cid.to_string())
                            } else {
                                None
                            }
                        })
                        .collect();
                    (
                        selected,
                        st.album_detail.clone(),
                        st.output_dir.clone(),
                        ui.get_output_format_index(),
                    )
                };

                if selected_cids.is_empty() {
                    ui.set_status_text(SharedString::from("No songs selected."));
                    return;
                }
                let Some(album) = album_detail else {
                    ui.set_status_text(SharedString::from("No album selected."));
                    return;
                };

                let format = output_format_from_index(fmt_idx);
                ui.set_downloading(true);

                // Clear existing tasks
                ui.set_tasks(ModelRc::new(VecModel::from(vec![])));

                for cid in &selected_cids {
                    // Resolve full song detail
                    match client.get_song_detail(cid).await {
                        Ok(song) => {
                            // Add a task row
                            let task_name = SharedString::from(song.name.as_str());
                            let tasks_model = ui.get_tasks();
                            if let Some(model) =
                                tasks_model.as_any().downcast_ref::<VecModel<DownloadTask>>()
                            {
                                model.push(DownloadTask {
                                    song_name: task_name.clone(),
                                    progress: 0.0,
                                    done: false,
                                });
                            }
                            let task_idx = ui.get_tasks().row_count() - 1;
                            let ui_handle2 = ui_handle.clone();

                            // Download
                            let album_dir = out_dir
                                .join(audio::sanitize_filename(&album.name));
                            match download_song(
                                &client,
                                &song,
                                &album,
                                &album_dir,
                                format,
                                move |prog: DownloadProgress| {
                                    let frac = prog.bytes_total
                                        .map(|t| prog.bytes_done as f32 / t as f32)
                                        .unwrap_or(0.0);
                                    let ui2 = ui_handle2.upgrade().unwrap();
                                    let tasks_model = ui2.get_tasks();
                                    if let Some(model) = tasks_model
                                        .as_any()
                                        .downcast_ref::<VecModel<DownloadTask>>()
                                    {
                                        if let Some(mut t) = model.row_data(task_idx) {
                                            t.progress = frac;
                                            model.set_row_data(task_idx, t);
                                        }
                                    }
                                },
                            )
                            .await
                            {
                                Ok(path) => {
                                    let tasks_model = ui.get_tasks();
                                    if let Some(model) = tasks_model
                                        .as_any()
                                        .downcast_ref::<VecModel<DownloadTask>>()
                                    {
                                        if let Some(mut t) = model.row_data(task_idx) {
                                            t.progress = 1.0;
                                            t.done = true;
                                            model.set_row_data(task_idx, t);
                                        }
                                    }
                                    ui.set_status_text(SharedString::from(format!(
                                        "Saved: {}",
                                        path.display()
                                    )));
                                }
                                Err(e) => {
                                    ui.set_status_text(SharedString::from(format!(
                                        "Download error: {e}"
                                    )));
                                }
                            }
                        }
                        Err(e) => {
                            ui.set_status_text(SharedString::from(format!(
                                "Failed to fetch song detail: {e}"
                            )));
                        }
                    }
                }

                ui.set_downloading(false);
                ui.set_status_text(SharedString::from("All downloads complete."));
            });
        });
    }

    // ── Initial load & run ────────────────────────────────────────────────────
    let ui_handle2 = ui.as_weak();
    let _ = slint::spawn_local(async move {
        if let Some(ui) = ui_handle2.upgrade() {
            ui.invoke_load_albums();
        }
    });

    ui.run()?;
    Ok(())
}
