use std::collections::HashMap;

use clap::Parser;
use eframe::egui::{self, Widget};
use log::*;
use mini_leebee_proto::{
    mini_leebee_client::MiniLeebeeClient, AddPluginToTrackRequest, CreateTrackRequest,
    DeleteTracksRequest, GetPluginsRequest, GetTracksRequest, GetmetronomeRequest, Metronome,
    Plugin, SetmetronomeRequest, Track,
};
use pollster::FutureExt;
use tonic::transport::Channel;

pub mod args;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = args::Arguments::parse();
    env_logger::builder().filter_level(args.log_level).init();
    info!("{:?}", args);
    info!("Working directory: {:?}", std::env::current_dir());

    let addr = format!("http://[::1]:{}", args.port);
    info!("Connecting to {}", addr);
    let client = MiniLeebeeClient::connect(addr).await?;
    eframe::run_native(
        "Mini LeeBee",
        eframe::NativeOptions::default(),
        Box::new(|_| Box::new(App::new(client))),
    );
    Ok(())
}

#[derive(Debug)]
struct App {
    client: MiniLeebeeClient<Channel>,
    metronome: Metronome,
    plugins: Vec<Plugin>,
    plugin_to_index: HashMap<String, usize>,
    selected_track_id: i32,
    tracks: Vec<Track>,
    refresh: bool,
}

impl App {
    fn new(client: MiniLeebeeClient<Channel>) -> App {
        let mut client = client;
        let metronome = client
            .get_metronome(tonic::Request::new(GetmetronomeRequest {}))
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
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
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
                                self.selected_track_id = self
                                    .client
                                    .create_track(tonic::Request::new(CreateTrackRequest {
                                        name: plugin.name.clone(),
                                    }))
                                    .block_on()
                                    .unwrap()
                                    .into_inner()
                                    .track_id;
                                self.client
                                    .add_plugin_to_track(tonic::Request::new(
                                        AddPluginToTrackRequest {
                                            track_id: self.selected_track_id,
                                            plugin_id: plugin.id.clone(),
                                        },
                                    ))
                                    .block_on()
                                    .unwrap();
                                self.refresh = true;
                            }
                            if let Some(track_id) = selected_track_id {
                                if ui.button("Add To Track").clicked() {
                                    self.client
                                        .add_plugin_to_track(tonic::Request::new(
                                            AddPluginToTrackRequest {
                                                track_id,
                                                plugin_id: plugin.id.clone(),
                                            },
                                        ))
                                        .block_on()
                                        .unwrap();
                                    self.refresh = true;
                                }
                            }
                        });
                    });
                }
            });
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut metronome_is_on = self.metronome.volume > 0.0;
                if ui.button("New Track").clicked() {
                    self.selected_track_id = self
                        .client
                        .create_track(tonic::Request::new(CreateTrackRequest {
                            name: String::new(),
                        }))
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
                    self.client
                        .set_metronome(tonic::Request::new(SetmetronomeRequest {
                            metronome: Some(self.metronome.clone()),
                        }))
                        .block_on()
                        .unwrap();
                }
                if ui.button("↻").clicked() {
                    self.refresh = true;
                }
            });

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
                self.tracks.retain(|t| !tracks_to_delete.contains(&t.id));
                self.client
                    .delete_tracks(tonic::Request::new(DeleteTracksRequest {
                        track_ids: tracks_to_delete,
                    }))
                    .block_on()
                    .unwrap();
            }
            if let Some(track) = self.tracks.iter().find(|t| t.id == self.selected_track_id) {
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
                            ui.label(&plugin.name);
                        });
                    });
                }
            }
        });
        if self.refresh {
            self.refresh = false;
            self.metronome = self
                .client
                .get_metronome(tonic::Request::new(GetmetronomeRequest {}))
                .block_on()
                .unwrap()
                .into_inner()
                .metronome
                .unwrap();
            self.tracks = self
                .client
                .get_tracks(tonic::Request::new(GetTracksRequest {}))
                .block_on()
                .unwrap()
                .into_inner()
                .tracks;
        }
    }
}
