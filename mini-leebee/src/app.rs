use std::{
    collections::{HashMap, HashSet},
    sync::{atomic::AtomicBool, Arc},
};

use eframe::egui::{self, Widget};
use log::*;
use mini_leebee_state::{Plugin, State};

#[derive(Debug)]
pub struct App {
    /// The arguments passed to the application.
    args: crate::args::Arguments,
    /// A connection to a MiniLeebee audio server.
    ///
    /// TODO: Figure out why using client may sometimes permanently stall the
    /// program.
    state: State,
    /// The value of the BPM text. This is not necessarily the currently set BPM.
    bpm_text: String,
    /// The set of plugins.
    plugins: Vec<Plugin>,
    /// A mapping from a plugin id to its index in the plugins vector.
    plugin_to_index: HashMap<String, usize>,
    /// The id of the selected track. If invalid, then it is assumed no track is
    /// selected.
    selected_track_id: i32,
    /// If true, the UI should be refreshed using the client.
    refresh: bool,
    /// If true, the UI has requested a performance profile from the server and
    /// is still waiting.
    profile_in_progress: Arc<AtomicBool>,
}

impl App {
    /// Create a new application from a client.
    pub fn new(args: crate::args::Arguments, state: State) -> App {
        let metronome = state.metronome().clone();
        let plugins = state.get_plugins();
        for plugin in plugins.iter() {
            info!("{:?}", plugin);
        }
        let plugin_to_index = plugins
            .iter()
            .enumerate()
            .map(|(idx, p)| (p.id.clone(), idx))
            .collect();
        App {
            args,
            state,
            bpm_text: metronome.beats_per_minute.to_string(),
            plugins,
            plugin_to_index,
            selected_track_id: 0,
            refresh: false,
            profile_in_progress: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _: &mut eframe::Frame) {
        self.state.update();
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
            ctx.request_repaint_after(std::time::Duration::from_millis(10));
            return;
        }
        ctx.request_repaint();
        self.refresh = false;
    }

    fn update_plugin_panel(&mut self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            let selected_track_id = self
                .state
                .iter_tracks()
                .find(|t| t.id == self.selected_track_id)
                .map(|t| t.id);
            for (idx, plugin) in self.plugins.iter().enumerate() {
                ui.push_id(idx, |ui| {
                    ui.label(&plugin.name);
                    ui.horizontal(|ui| {
                        if ui.button("Create Track").clicked() {
                            let track_name = plugin.name.clone();
                            let track_id = self.state.create_track(Some(track_name)).unwrap();
                            self.selected_track_id = track_id;
                            self.state
                                .add_plugin_to_track(track_id, &plugin.id)
                                .unwrap();
                            self.state.set_armed(Some(self.selected_track_id));
                            self.refresh = true;
                        }
                        if let Some(track_id) = selected_track_id {
                            if ui.button("Add To Track").clicked() {
                                self.state
                                    .add_plugin_to_track(track_id, &plugin.id)
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
            let mut metronome_is_on = self.state.metronome().volume > 0.0;
            if ui.button("New Track").clicked() {
                let track_id = self.state.create_track(None).unwrap();
                self.selected_track_id = track_id;
                self.state.set_armed(Some(self.selected_track_id));
                self.refresh = true;
            }
            ui.spacing();
            let bpm_text = egui::TextEdit::singleline(&mut self.bpm_text)
                .desired_width(48.0)
                .ui(ui);
            if bpm_text.lost_focus() {
                match self.bpm_text.parse::<f32>() {
                    Ok(bpm) if bpm == self.state.metronome().beats_per_minute => {
                        info!("BPM is unchanged.");
                    }
                    Ok(bpm) => {
                        let mut metronome = self.state.metronome().clone();
                        metronome.beats_per_minute = bpm;
                        self.state.set_metronome(metronome);
                    }
                    Err(err) => {
                        warn!("{:?} is not a valid bpm: {}", self.bpm_text, err);
                        self.bpm_text = self.state.metronome().beats_per_minute.to_string();
                    }
                }
            }
            ui.label(format!("{}", self.state.time_info()));
            if ui.toggle_value(&mut metronome_is_on, "metronome").clicked() {
                let volume = if metronome_is_on { 0.5 } else { 0.0 };
                let mut metronome = self.state.metronome().clone();
                metronome.volume = volume;
                self.state.set_metronome(metronome);
            }
            ui.label(self.state.cpu_load());
            if self.args.enable_profiling {
                if self
                    .profile_in_progress
                    .load(std::sync::atomic::Ordering::Relaxed)
                {
                    ui.label("profiling in progress...");
                } else if ui.link("perf profile").clicked() {
                    self.profile_in_progress
                        .store(true, std::sync::atomic::Ordering::Relaxed);
                    let profile_in_progress = self.profile_in_progress.clone();
                    let ctx = ui.ctx().clone();
                    self.state.pprof_report(
                        std::time::Duration::from_secs(10),
                        Box::new(move |svg_report| {
                            profile_and_show(&ctx, svg_report);
                            profile_in_progress.store(false, std::sync::atomic::Ordering::Relaxed);
                        }),
                    );
                }
            }
        });
    }

    fn update_track_list(&mut self, ui: &mut egui::Ui) {
        let mut tracks_to_delete = HashSet::new();
        let tracks = self.state.iter_tracks().cloned().collect::<Vec<_>>();
        for (idx, track) in tracks.iter().enumerate() {
            ui.push_id(idx, |ui| {
                ui.horizontal(|ui| {
                    let mut is_selected = self.selected_track_id == track.id;
                    if ui.toggle_value(&mut is_selected, &track.name).clicked() {
                        self.selected_track_id = if is_selected { track.id } else { 0 };
                        self.state.set_armed(Some(self.selected_track_id));
                    }
                    if egui::Button::new("🗑")
                        .fill(eframe::epaint::Color32::DARK_RED)
                        .ui(ui)
                        .clicked()
                    {
                        tracks_to_delete.insert(track.id);
                    }
                });
            });
        }
        if !tracks_to_delete.is_empty() {
            self.refresh = true;
            self.state.delete_tracks(tracks_to_delete).unwrap();
        }
    }

    fn update_track(&mut self, ui: &mut egui::Ui) {
        let track = match self
            .state
            .iter_tracks()
            .find(|t| t.id == self.selected_track_id)
        {
            Some(t) => t.clone(),
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
                        self.state.remove_plugin_from_track(track.id, idx).unwrap();
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
fn profile_and_show(ctx: &egui::Context, flamegraph_svg: Vec<u8>) {
    let flamegraph_path = "/tmp/mini-leebee-flamegraph.svg";
    if let Err(err) = std::fs::write(flamegraph_path, flamegraph_svg) {
        error!(
            "Failed to write performance profile flamegraph to {:?}: {}",
            flamegraph_path, err
        );
        return;
    }
    ctx.output_mut(|o| o.open_url(flamegraph_path));
}
