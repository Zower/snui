use crate::Render;
use eframe::egui::{self, ScrollArea};
use snew::things::Post;

impl Render for Post {
    fn render_summary(&self, ui: &mut egui::Ui) {
        ui.vertical(|ui| {
            let max_chars = (ui.available_width() / 10f32) as usize;

            let title = create_display_string(&self.title, max_chars);
            let url = create_display_string(&self.url, max_chars);

            ui.add(egui::Label::new(title).wrap(true).heading());

            ui.horizontal(|ui| {
                ui.label(url);
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.label("0")
                });
            });
            ui.label(self.score.to_string() + " points");
        });
    }

    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.label(&self.selftext);
        });
    }
}

fn create_display_string(original: &String, max_chars: usize) -> String {
    let mut new: String = original.chars().take(max_chars as usize).collect();

    if original.len() > max_chars {
        new.push_str("...");
    }

    new
}