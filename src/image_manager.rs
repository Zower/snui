use std::collections::HashMap;

use eframe::egui;
use image;

#[derive(Debug, Clone, Copy)]
pub struct Image {
    pub id: egui::TextureId,
    pub size: (usize, usize),
}

impl Image {
    pub fn new(id: egui::TextureId, size: (usize, usize)) -> Self {
       Self {
           id,
           size
       } 
    }
}

#[derive(Debug, Default)]
pub struct ImageManager {
    images: HashMap<usize, Image>,
}

impl ImageManager {
    pub fn store(&mut self, post_id: usize, image: &[u8], allocator: &mut dyn eframe::epi::TextureAllocator) -> Option<Image> {
        let image = image::load_from_memory(image);

        if let Ok(image) = image {
            let image = image.to_rgba8();
            let size = (image.width() as usize, image.height() as usize);
            let id = allocator.alloc_srgba_premultiplied(
                size,
                &image
                    .chunks(4)
                    .map(|pixel| {
                        egui::Color32::from_rgba_unmultiplied(
                            pixel[0], pixel[1], pixel[2], pixel[3],
                        )
                    })
                    .collect::<Vec<egui::Color32>>(),
            );
            let image = Image::new(id, size);
            self.images.insert(post_id, image);

            return Some(image);
        }

        None

    }
    
    pub fn get(&self, post_id: &usize) -> Option<&Image> {
        self.images.get(post_id)
    }
}
