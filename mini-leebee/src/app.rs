use std::sync::Arc;

use eframe::egui::{self, Event, Key};
use log::*;

/// The Mini Leebee app.
pub struct App {
    /// A widget for selecting a plugin.
    plugin_selector: PluginSelector,
}

impl App {
    /// Create a new app.
    pub fn new() -> App {
        let livi = Arc::new(livi::World::new());
        let plugin_selector = PluginSelector {
            livi,
            selected_index: 0,
        };
        App { plugin_selector }
    }
}

impl Default for App {
    fn default() -> App {
        App::new()
    }
}

impl eframe::App for App {
    /// Update the UI and handle inputs.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(plugin) = self.plugin_selector.show(ctx, ui) {
                info!("Selected {:?}", plugin);
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
