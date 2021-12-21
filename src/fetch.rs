use bytes::Bytes;
use crossbeam_channel::Sender;
use eframe::egui;
use snew::{
    content::Content,
    things::{Post, PostFeed},
};
use std::{sync::Arc, thread};

use crate::{PostId, ViewablePost};
// todo: make this module a bit less.. manual

pub enum Message {
    PostsReady(Vec<ViewablePost>, PostFeed),
    ContentReady(Content, PostId),
    ImageDecoded(Vec<egui::Color32>, (usize, usize), PostId),
}

pub fn get_more_posts(mut feed: PostFeed, s: Sender<Message>) {
    thread::spawn(move || {
        let posts: Vec<ViewablePost> = feed
            .by_ref()
            .filter_map(|p| p.ok())
            .map(|p| p.into())
            .take(35)
            .collect();

        let _ = s.send(Message::PostsReady(posts, feed));
    });
}

pub fn get_content(post: Arc<Post>, post_id: PostId, s: Sender<Message>) {
    thread::spawn(move || {
        if let Ok(content) = post.get_content() {
            let _ = s.send(Message::ContentReady(content, post_id));
        }
    });
}

pub fn decode_image(image: Bytes, post_id: PostId, s: Sender<Message>) {
    thread::spawn(move || {
        let image = image::load_from_memory(&image).unwrap();
        let image = image.to_rgba8();

        let size = (image.width() as usize, image.height() as usize);

        let image = image
            .chunks(4)
            .map(|pixel| {
                egui::Color32::from_rgba_unmultiplied(pixel[0], pixel[1], pixel[2], pixel[3])
            })
            .collect::<Vec<egui::Color32>>();

        let _ = s.send(Message::ImageDecoded(image, size, post_id));
    });
}
