use crate::{MainContent, image_manager::Image};
use eframe::egui::{self, ScrollArea};
use snew::things::Post;

impl MainContent for Post {
    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            if let Some(content) = &self.selftext {
                ui.label(content);
            }
        });
    }
}

impl MainContent for Image {
    fn render(&self, ui: &mut egui::Ui) {
        ui.image(self.id, egui::Vec2::new(self.size.0 as f32, self.size.1 as f32));
    }
}

pub fn ui_post_summary(ui: &mut egui::Ui, post: &Post, highlight: bool) {
    ui.vertical(|ui| {
        if highlight {
            ui.visuals_mut().widgets.noninteractive.fg_stroke =
                egui::Stroke::new(10f32, egui::Color32::WHITE);
        }

        let max_chars = (ui.available_width() / 10f32) as usize;

        let title = create_display_string(&post.title, max_chars);
        let url = create_display_string(&post.url, max_chars);

        let title = egui::Label::new(title).wrap(true).heading();
        ui.add(title);

        ui.horizontal(|ui| {
            ui.label(url);
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                ui.label("0")
            });
        });
        ui.label(post.score.to_string() + " points");
    });
}

impl MainContent for String {
    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.label(self);
        });
    }
}

fn create_display_string(original: &String, max_chars: usize) -> String {
    let mut new: String = original
        .chars()
        .filter(|c| c.is_ascii_graphic() || *c == ' ')
        .take(max_chars as usize)
        .collect();

    if original.len() > max_chars {
        new.push_str("...");
    }

    new
}
