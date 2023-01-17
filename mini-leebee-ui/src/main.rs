use clap::Parser;
use eframe::egui::{self, Widget};
use log::*;
use mini_leebee_proto::{
    mini_leebee_client::MiniLeebeeClient, AddPluginToTrackRequest, CreateTrackRequest,
    DeleteTracksRequest, GetMetrenomeRequest, GetPluginsRequest, GetTracksRequest, Metrenome,
    Plugin, SetMetrenomeRequest, Track,
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
    metrenome: Metrenome,
    plugins: Vec<Plugin>,
    selected_track_id: i32,
    tracks: Vec<Track>,
    refresh_tracks: bool,
}

impl App {
    fn new(client: MiniLeebeeClient<Channel>) -> App {
        let mut client = client;
        let metrenome = client
            .get_metrenome(tonic::Request::new(GetMetrenomeRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .metrenome
            .unwrap();
        let plugins = client
            .get_plugins(tonic::Request::new(GetPluginsRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .plugins;
        let tracks = client
            .get_tracks(tonic::Request::new(GetTracksRequest {}))
            .block_on()
            .unwrap()
            .into_inner()
            .tracks;
        App {
            client,
            metrenome,
            plugins,
            selected_track_id: 0,
            tracks,
            refresh_tracks: false,
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        egui::SidePanel::left("left_panel").show(ctx, |ui| {
            let selected_track_id = self
                .tracks
                .iter()
                .find(|t| t.id == self.selected_track_id)
                .map(|t| t.id);
            for (idx, plugin) in self.plugins.iter().enumerate() {
                ui.push_id(idx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(&plugin.name);
                        if ui.button("Create Track").clicked() {
                            self.selected_track_id = self
                                .client
                                .create_track(tonic::Request::new(CreateTrackRequest {
                                    name: String::new(),
                                }))
                                .block_on()
                                .unwrap()
                                .into_inner()
                                .track_id;
                            self.client
                                .add_plugin_to_track(tonic::Request::new(AddPluginToTrackRequest {
                                    track_id: self.selected_track_id,
                                    plugin_id: plugin.id.clone(),
                                }))
                                .block_on()
                                .unwrap();
                            self.refresh_tracks = true;
                        }
                        if let Some(track_id) = selected_track_id {
                            if ui.button("Add To Track").clicked() {
                                self.client
                                    .add_plugin_to_track(tonic::Request::new(
                                        AddPluginToTrackRequest {
                                            track_id: track_id,
                                            plugin_id: plugin.id.clone(),
                                        },
                                    ))
                                    .block_on()
                                    .unwrap();
                            }
                        }
                    });
                });
            }
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                let mut metrenome_is_on = self.metrenome.volume > 0.0;
                if ui.button("Create Track").clicked() {
                    self.selected_track_id = self
                        .client
                        .create_track(tonic::Request::new(CreateTrackRequest {
                            name: String::new(),
                        }))
                        .block_on()
                        .unwrap()
                        .into_inner()
                        .track_id;
                    self.refresh_tracks = true;
                }
                if ui.toggle_value(&mut metrenome_is_on, "metrenome").clicked() {
                    if metrenome_is_on {
                        self.metrenome.volume = 0.5;
                    } else {
                        self.metrenome.volume = 0.0;
                    }
                    self.client
                        .set_metrenome(tonic::Request::new(SetMetrenomeRequest {
                            metrenome: Some(self.metrenome.clone()),
                        }))
                        .block_on()
                        .unwrap();
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
                        if egui::Button::new("delete")
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
                self.refresh_tracks = true;
                self.tracks.retain(|t| !tracks_to_delete.contains(&t.id));
                self.client
                    .delete_tracks(tonic::Request::new(DeleteTracksRequest {
                        track_ids: tracks_to_delete,
                    }))
                    .block_on()
                    .unwrap();
            }
        });
        if self.refresh_tracks {
            self.refresh_tracks = false;
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
