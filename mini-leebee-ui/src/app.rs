use std::{
    collections::HashMap,
    sync::{atomic::AtomicBool, Arc},
};

use eframe::egui::{self, Widget};
use log::*;
use mini_leebee_proto::{
    mini_leebee_client::MiniLeebeeClient, AddPluginToTrackRequest, CreateTrackRequest,
    DeleteTracksRequest, GetMetronomeRequest, GetPluginsRequest, GetTracksRequest, Metronome,
    Plugin, PprofReportRequest, RemovePluginFromTrackRequest, SetMetronomeRequest, Track,
};
use pollster::FutureExt;
use tonic::transport::Channel;

#[derive(Debug)]
pub struct App {
    /// A connection to a MiniLeebee audio server.
    ///
    /// TODO: Figure out why using client may sometimes permanently stall the
    /// program.
    client: MiniLeebeeClient<Channel>,
    /// The value of the BPM text. This is not necessarily the currently set BPM.
    bpm_text: String,
    /// The state of the metronome.
    metronome: Metronome,
    /// The set of plugins.
    plugins: Vec<Plugin>,
    /// A mapping from a plugin id to its index in the plugins vector.
    plugin_to_index: HashMap<String, usize>,
    /// The index of the selected track. If invalid, then it is assumed no track
    /// is selected.
    selected_track_id: i32,
    /// The tracks.
    tracks: Vec<Track>,
    /// If true, the UI should be refreshed using the client.
    refresh: bool,
    /// If true, the UI has requested a performance profile from the server and
    /// is still waiting.
    profile_in_progress: Arc<AtomicBool>,
}

impl App {
    /// Create a new application from a client.
    pub fn new(client: MiniLeebeeClient<Channel>) -> App {
        let metronome = client
            .clone()
            .get_metronome(tonic::Request::new(GetMetronomeRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .metronome
            .unwrap();
        let plugins = client
            .clone()
            .get_plugins(tonic::Request::new(GetPluginsRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .plugins;
        let plugin_to_index = plugins
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id.clone(), idx))
            .collect();
        let tracks = client
            .clone()
            .get_tracks(tonic::Request::new(GetTracksRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .tracks;
        App {
            client,
            bpm_text: metronome.beats_per_minute.to_string(),
            metronome,
            plugins,
            plugin_to_index,
            selected_track_id: 0,
            tracks,
            refresh: false,
            profile_in_progress: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| self.update_plugin_panel(ui));
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            self.update_top_bar(ui);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            self.update_track_list(ui);
            self.update_track(ui);
        });
        self.maybe_refresh(ctx);
    }
}

impl App {
    fn maybe_refresh(&mut self, ctx: &egui::Context) {
        if !self.refresh {
            return;
        }
        ctx.request_repaint();
        let mut request = tonic::Request::new(GetMetronomeRequest {});
        info!("{:?}", request);
        request.set_timeout(std::time::Duration::from_secs(1));
        self.metronome = self
            .client
            .clone()
            .get_metronome(request)
            .block_on()
            .unwrap()
            .into_inner()
            .metronome
            .unwrap();
        let mut request = tonic::Request::new(GetTracksRequest {});
        info!("{:?}", request);
        request.set_timeout(std::time::Duration::from_secs(1));
        self.tracks = self
            .client
            .clone()
            .get_tracks(request)
            .block_on()
            .unwrap()
            .into_inner()
            .tracks;
        self.refresh = false;
    }

    fn update_plugin_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let selected_track_id = self
                .tracks
                .iter()
                .find(|t| t.id == self.selected_track_id)
                .map(|t| t.id);
            for (idx, plugin) in self.plugins.iter().enumerate() {
                ui.push_id(idx, |ui| {
                    ui.label(&plugin.name);
                    ui.horizontal(|ui| {
                        if ui.button("Create Track").clicked() {
                            let request = CreateTrackRequest {
                                name: plugin.name.clone(),
                            };
                            info!("{:?}", request);
                            self.selected_track_id = self
                                .client
                                .clone()
                                .create_track(tonic::Request::new(request))
                                .block_on()
                                .unwrap()
                                .into_inner()
                                .track_id;
                            let request = AddPluginToTrackRequest {
                                track_id: self.selected_track_id,
                                plugin_id: plugin.id.clone(),
                            };
                            info!("{:?}", request);
                            self.client
                                .clone()
                                .add_plugin_to_track(tonic::Request::new(request))
                                .block_on()
                                .unwrap();
                            self.refresh = true;
                        }
                        if let Some(track_id) = selected_track_id {
                            if ui.button("Add To Track").clicked() {
                                let request = AddPluginToTrackRequest {
                                    track_id,
                                    plugin_id: plugin.id.clone(),
                                };
                                info!("{:?}", request);
                                self.client
                                    .clone()
                                    .add_plugin_to_track(tonic::Request::new(request))
                                    .block_on()
                                    .unwrap();
                                self.refresh = true;
                            }
                        }
                    });
                });
            }
        });
    }

    fn update_top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let mut metronome_is_on = self.metronome.volume > 0.0;
            if ui.button("New Track").clicked() {
                let request = CreateTrackRequest {
                    name: String::new(),
                };
                self.selected_track_id = self
                    .client
                    .clone()
                    .create_track(tonic::Request::new(request))
                    .block_on()
                    .unwrap()
                    .into_inner()
                    .track_id;
                self.refresh = true;
            }
            ui.spacing();
            let bpm_text = egui::TextEdit::singleline(&mut self.bpm_text)
                .desired_width(48.0)
                .ui(ui);
            if bpm_text.lost_focus() {
                match self.bpm_text.parse::<f32>() {
                    Ok(bpm) if bpm == self.metronome.beats_per_minute => {
                        info!("BPM is unchanged.");
                    }
                    Ok(bpm) => {
                        self.metronome.beats_per_minute = bpm;
                        let request = SetMetronomeRequest {
                            metronome: Some(self.metronome.clone()),
                        };
                        info!("{:?}", request);
                        self.client
                            .clone()
                            .set_metronome(tonic::Request::new(request))
                            .block_on()
                            .unwrap();
                    }
                    Err(err) => {
                        warn!("{:?} is not a valid bpm: {}", self.bpm_text, err);
                        self.bpm_text = self.metronome.beats_per_minute.to_string();
                    }
                }
            }
            if ui.toggle_value(&mut metronome_is_on, "metronome").clicked() {
                if metronome_is_on {
                    self.metronome.volume = 0.5;
                } else {
                    self.metronome.volume = 0.0;
                }
                let request = SetMetronomeRequest {
                    metronome: Some(self.metronome.clone()),
                };
                info!("{:?}", request);
                self.client
                    .clone()
                    .set_metronome(tonic::Request::new(request))
                    .block_on()
                    .unwrap();
            }
            if self
                .profile_in_progress
                .load(std::sync::atomic::Ordering::Relaxed)
            {
                ui.label("profiling in progress...");
            } else if ui.link("perf profile").clicked() {
                self.profile_in_progress
                    .store(true, std::sync::atomic::Ordering::Relaxed);
                let profile_in_progress = self.profile_in_progress.clone();
                let client = self.client.clone();
                let ctx = ui.ctx().clone();
                std::thread::spawn(move || {
                    profile_and_show(&ctx, client);
                    ctx.request_repaint();
                    profile_in_progress.store(false, std::sync::atomic::Ordering::Relaxed);
                });
            }
        });
    }

    fn update_track_list(&mut self, ui: &mut egui::Ui) {
        let mut tracks_to_delete = Vec::new();
        for (idx, track) in self.tracks.iter().enumerate() {
            ui.push_id(idx, |ui| {
                ui.horizontal(|ui| {
                    let mut is_selected = self.selected_track_id == track.id;
                    if ui.toggle_value(&mut is_selected, &track.name).clicked() {
                        self.selected_track_id = if is_selected { track.id } else { 0 };
                    }
                    if egui::Button::new("🗑")
                        .fill(eframe::epaint::Color32::DARK_RED)
                        .ui(ui)
                        .clicked()
                    {
                        tracks_to_delete.push(track.id);
                    }
                });
            });
        }
        if !tracks_to_delete.is_empty() {
            self.refresh = true;
            self.client
                .clone()
                .delete_tracks(tonic::Request::new(DeleteTracksRequest {
                    track_ids: tracks_to_delete,
                }))
                .block_on()
                .unwrap();
        }
    }

    fn update_track(&mut self, ui: &mut egui::Ui) {
        let track = match self.tracks.iter().find(|t| t.id == self.selected_track_id) {
            Some(t) => t,
            None => return,
        };
        ui.separator();
        ui.label(&track.name);
        for (idx, track_plugin) in track.plugins.iter().enumerate() {
            let plugin_index = match self.plugin_to_index.get(&track_plugin.plugin_id) {
                Some(idx) => idx,
                None => {
                    error!(
                        "Could not find plugin with id {:?}.",
                        track_plugin.plugin_id
                    );
                    continue;
                }
            };
            let plugin = match self.plugins.get(*plugin_index) {
                Some(p) => p,
                None => {
                    error!("Could not find plugin with index {:?}.", plugin_index);
                    continue;
                }
            };
            ui.push_id(idx, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing();
                    ui.separator();
                    if egui::Button::new("🗑")
                        .fill(eframe::epaint::Color32::DARK_RED)
                        .ui(ui)
                        .clicked()
                    {
                        let request = RemovePluginFromTrackRequest {
                            track_id: track.id,
                            plugin_index: idx as i32,
                        };
                        info!("{:?}", request);
                        self.client
                            .clone()
                            .remove_plugin_from_track(request)
                            .block_on()
                            .unwrap();
                        self.refresh = true;
                    }
                    ui.label(&plugin.name);
                });
            });
        }
    }
}

/// Request and retrieve a profile from client and open the results in a
/// browser.
fn profile_and_show(ctx: &egui::Context, client: MiniLeebeeClient<Channel>) {
    let mut client = client;
    let request = PprofReportRequest {
        duration_secs: 15,
        want_flamegraph: true,
        want_report_proto: false,
    };
    info!("{:?}", request);
    let response = client
        .pprof_report(tonic::Request::new(request))
        .block_on()
        .unwrap()
        .into_inner();
    let flamegraph_path = "/tmp/mini-leebee-flamegraph.svg";
    if let Err(err) = std::fs::write(flamegraph_path, response.flamegraph_svg) {
        error!(
            "Failed to write performance profile flamegraph to {:?}: {}",
            flamegraph_path, err
        );
        return;
    }
    ctx.output().open_url(flamegraph_path);
}
