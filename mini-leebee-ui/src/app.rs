use std::collections::HashMap;

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
    client: MiniLeebeeClient<Channel>,
    metronome: Metronome,
    plugins: Vec<Plugin>,
    plugin_to_index: HashMap<String, usize>,
    selected_track_id: i32,
    tracks: Vec<Track>,
    refresh: bool,
}

impl App {
    pub fn new(client: MiniLeebeeClient<Channel>) -> App {
        let mut client = client;
        let metronome = client
            .get_metronome(tonic::Request::new(GetMetronomeRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .metronome
            .unwrap();
        let plugins = client
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
            .get_tracks(tonic::Request::new(GetTracksRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .tracks;
        App {
            client,
            metronome,
            plugins,
            plugin_to_index,
            selected_track_id: 0,
            tracks,
            refresh: false,
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
        info!("Refreshing state.");
        ctx.request_repaint();
        let mut request = tonic::Request::new(GetMetronomeRequest {});
        request.set_timeout(std::time::Duration::from_secs(1));
        self.metronome = self
            .client
            .get_metronome(request)
            .block_on()
            .unwrap()
            .into_inner()
            .metronome
            .unwrap();
        let mut request = tonic::Request::new(GetTracksRequest {});
        request.set_timeout(std::time::Duration::from_secs(1));
        self.tracks = self
            .client
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
                    .create_track(tonic::Request::new(request))
                    .block_on()
                    .unwrap()
                    .into_inner()
                    .track_id;
                self.refresh = true;
            }
            ui.spacing();
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
                    .set_metronome(tonic::Request::new(request))
                    .block_on()
                    .unwrap();
            }
            if ui.button("Performance Profile").clicked() {
                // TODO: Do not block UI updates as profile is happening. Do it
                // asynchronously.
                let request = PprofReportRequest {
                    // 0 falls back to the server's default.
                    duration_secs: 0,
                };
                info!("{:?}", request);
                let response = self
                    .client
                    .pprof_report(tonic::Request::new(request))
                    .block_on()
                    .unwrap()
                    .into_inner();
                handle_profile(response);
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

fn handle_profile(response: mini_leebee_proto::PprofReportResponse) {
    let flamegraph_path = "/tmp/mini-leebee-flamegraph.svg";
    std::fs::write(flamegraph_path, &response.flamegraph_svg).unwrap();
    // TODO: Default to the OS's preferred file opener.
    std::process::Command::new("google-chrome")
        .arg(flamegraph_path)
        .spawn()
        .unwrap();
    // TODO: Support pprof.
    // let path = "/tmp/mini-leebee-profile.pb";
    // std::fs::write(path, response.encode_to_vec()).unwrap();
    // std::process::Command::new("pprof")
    //     .arg("--http=localhost:8080")
    //     .arg(path)
    //     .spawn()
    //     .unwrap();
}
