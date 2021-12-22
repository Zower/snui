use crate::{image_manager::Image, MainContent};
use eframe::egui::{self, ScrollArea};
use snew::things::Post;

impl MainContent for Image {
    fn render(&self, ui: &mut egui::Ui) {
        ScrollArea::both().show(ui, |ui| {
            let mut size = egui::Vec2::new(self.size.0 as f32, self.size.1 as f32);
            size *= (ui.available_width() / size.x).min(1.0);
            size *= (ui.available_height() / size.y).min(1.0);
            ui.image(self.id, size);
        })
    }
}

pub fn ui_post_summary(ui: &mut egui::Ui, post: &Post, highlight: bool) {
    let response = ui.vertical(|ui| {
        if highlight {
            ui.visuals_mut().widgets.noninteractive.fg_stroke =
                egui::Stroke::new(10f32, egui::Color32::WHITE);
        }

        let max_chars = (ui.available_width() / 10f32) as usize;

        let title = create_display_string(&post.title, max_chars);
        let url = create_display_string(&post.url, max_chars);

        let title = egui::Label::new(title).wrap(true).heading();
        let response = ui.add(title);

        if highlight {
            response.scroll_to_me(egui::Align::Center)
        }

        ui.horizontal(|ui| {
            ui.label(url);
            ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                // ui.label(post.num_comments)
            });
        });
        ui.label(post.score.to_string() + " points");
    });

    if highlight {
        response.response.scroll_to_me(egui::Align::Center)
    }
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
