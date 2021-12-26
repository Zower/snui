use crate::{image_manager::Image, Render};
use eframe::egui::{self, ScrollArea};

impl Render for Image {
    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::both().show(ui, |ui| {
            ui.vertical_centered_justified(|ui| {
                let size = egui::Vec2::new(self.size.0 as f32, self.size.1 as f32);
                let size1 = size * (ui.available_width() / size.x);
                let size2 = size * (ui.available_height() / size.y);
                ui.image(self.id, size1.min(size2));
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
