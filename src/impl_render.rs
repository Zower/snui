use crate::{image_manager::Image, Render};
use eframe::egui::{self, ScrollArea};

impl Render for Image {
    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::both().show(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                let mut size = egui::Vec2::new(self.size.0 as f32, self.size.1 as f32);
                size *= (ui.available_width() / size.x).min(1.0);
                size *= (ui.available_height() / size.y).min(1.0);
                ui.image(self.id, size);
            });
        })
    }
}

impl Render for String {
    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.vertical(|ui| {
                ui.label(self);
            });
        });
    }
}
