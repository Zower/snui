use std::collections::HashMap;

use eframe::egui;

#[derive(Debug, Clone, Copy)]
pub struct Image {
    pub id: egui::TextureId,
    pub size: (usize, usize),
}

impl Image {
    pub fn new(id: egui::TextureId, size: (usize, usize)) -> Self {
        Self { id, size }
    }
}

#[derive(Debug, Default)]
pub struct ImageManager {
    images: HashMap<usize, Image>,
}

impl ImageManager {
    pub fn store(
        &mut self,
        post_id: usize,
        image: Vec<egui::Color32>,
        size: (usize, usize),
        allocator: &mut dyn eframe::epi::TextureAllocator,
    ) -> Option<Image> {
        // let size = (image.width() as usize, image.height() as usize);
        let id = allocator.alloc_srgba_premultiplied(size, &image);
        let image = Image::new(id, size);
        self.images.insert(post_id, image);

        return Some(image);

        None
    }

    pub fn get(&self, post_id: &usize) -> Option<&Image> {
        self.images.get(post_id)
    }
}
