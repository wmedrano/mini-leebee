use std::sync::Arc;

use eframe::egui::{self, Event, Key};
use log::*;

pub struct App {
    plugin_selector: PluginSelector,
}

impl App {
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(plugin) = self.plugin_selector.show(ctx, ui) {
                info!("Selected {:?}", plugin);
            }
        });
    }
}

struct PluginSelector {
    livi: Arc<livi::World>,
    selected_index: usize,
}

impl PluginSelector {
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
