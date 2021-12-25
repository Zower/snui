use bytes::Bytes;
use crossbeam_channel::Sender;
use eframe::egui;
use snew::{
    auth::UserAuthenticator,
    content::Content,
    reddit::Reddit,
    things::{Post, PostFeed},
};
use std::{sync::Arc, thread, time::Duration};

use crate::PostId;
// todo: make this module a bit less.. manual

pub enum Message {
    PostsReady(Vec<Post>, PostFeed),
    ContentReady(Content, PostId),
    ImageDecoded(Vec<egui::Color32>, (usize, usize), PostId),
    UserLoggedIn(UserAuthenticator),
}

pub fn get_more_posts(mut feed: PostFeed, s: Sender<Message>) {
    thread::spawn(move || {
        let posts: Vec<Post> = feed.by_ref().filter_map(|p| p.ok()).take(15).collect();

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

pub fn start_login_process(client_id: &'static str, s: Sender<Message>) {
    thread::spawn(move || {
        let auth = Reddit::perform_code_flow(
            client_id,
            "Success. You can now return to SnUI.",
            Some(Duration::from_secs(240)),
        );

        match auth {
            Ok(auth) => {
                let _ = s.send(Message::UserLoggedIn(auth));
            }

            Err(err) => {
                println!("ERROR {}", err)
            }
        };
    });
}
