use std::sync::Arc;

use audio_engine::{commands::Command, track::Track};
use eframe::egui::{self, Event, Key};
use jack_adapter::JackAdapter;
use log::*;

/// The Mini Leebee app.
pub struct App {
    /// The main audio and midi engine.
    jack_adapter: JackAdapter,
    /// A widget for selecting a plugin.
    plugin_selector: PluginSelector,
}

impl App {
    /// Create a new app.
    pub fn new(jack_adapter: JackAdapter) -> App {
        let plugin_selector = PluginSelector {
            livi: jack_adapter.audio_engine.livi.clone(),
            selected_index: 0,
        };
        App {
            jack_adapter,
            plugin_selector,
        }
    }
}

impl eframe::App for App {
    /// Update the UI and handle inputs.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(plugin) = self.plugin_selector.show(ctx, ui) {
                info!("Creating track with plugin {:?}", plugin);
                match unsafe {
                    plugin.instantiate(
                        self.jack_adapter.audio_engine.lv2_features.clone(),
                        self.jack_adapter.sample_rate(),
                    )
                } {
                    Ok(instance) => {
                        let mut track = Track::new(self.jack_adapter.buffer_size());
                        track.push_plugin(instance);
                        self.jack_adapter
                            .audio_engine
                            .commands
                            .send(Command::AddTrack(track))
                            .unwrap();
                    }
                    Err(err) => error!("Failed to instantiate plugin {:?}: {:?}", plugin, err),
                }
            }
        });
    }
}

/// A widget for selecting plugins.
struct PluginSelector {
    /// The LV2 plugin manager.
    livi: Arc<livi::World>,
    /// The selected index of the plugin.
    selected_index: usize,
}

impl PluginSelector {
    /// Show the widget and handle inputs. Returns a plugin if a plugin is
    /// selected.
    fn show(&mut self, ctx: &egui::Context, ui: &mut egui::Ui) -> Option<livi::Plugin> {
        let plugins_count = self.livi.iter_plugins().len();
        if plugins_count == 0 {
            error!("No plugins found!");
            return None;
        }
        let mut enter_pressed = false;
        for event in ctx.input().events.iter() {
            if let Event::Key {
                key, pressed: true, ..
            } = event
            {
                match key {
                    Key::ArrowLeft => self.selected_index += plugins_count - 1,
                    Key::ArrowRight => self.selected_index += 1,
                    Key::Enter => enter_pressed = true,
                    _ => {}
                }
            }
        }
        self.selected_index %= plugins_count;
        let plugin = self.livi.iter_plugins().nth(self.selected_index).unwrap();

        ui.label("Select Plugin");
        ui.strong(plugin.name());
        if enter_pressed {
            Some(plugin)
        } else {
            None
        }
    }
}
