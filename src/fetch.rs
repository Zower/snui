use bytes::Bytes;
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui;
use snew::{
    auth::UserAuthenticator,
    content::Content,
    reddit::Reddit,
    things::{Post, PostFeed},
};
use std::{sync::Arc, thread, time::Duration};

use crate::{components::PostId, state::State, SnuiApp};
// todo: make this module a bit less.. manual

pub enum Message {
    PostsReady(Vec<Post>, PostFeed),
    ContentReady(Content, PostId),
    ImageDecoded(Vec<egui::Color32>, (usize, usize), PostId),
    UserLoggedIn(UserAuthenticator),
}

pub trait Fetch {
    fn fetch(reddit: Reddit, state: &mut State, sender: Sender<Message>);
}

#[derive(Debug)]
pub struct MorePosts;

impl Fetch for MorePosts {
    fn fetch(
        _reddit: snew::reddit::Reddit,
        state: &mut State,
        sender: crossbeam_channel::Sender<super::Message>,
    ) {
        if let Some(mut feed) = state.feed.take() {
            thread::spawn(move || {
                let posts: Vec<Post> = feed.by_ref().filter_map(|p| p.ok()).take(15).collect();

                let _ = sender.send(Message::PostsReady(posts, feed));
            });
        }
    }
}

#[derive(Debug)]
pub struct Fetcher {
    /// Receiver of messages created on other threads
    pub receiver: Receiver<Message>,
    /// Sender for giving out
    pub sender: Sender<Message>,
    num_senders: u32,
}

impl Default for Fetcher {
    fn default() -> Self {
        let (sender, receiver) = unbounded();

        Self {
            receiver,
            sender,
            num_senders: Default::default(),
        }
    }
}

impl Fetcher {
    pub fn try_recv(&mut self) -> Option<Message> {
        match self.receiver.try_recv() {
            Ok(msg) => {
                self.num_senders -= 1;
                Some(msg)
            }
            Err(_) => None,
        }
    }

    pub fn reset(&mut self) {
        let (sender, receiver) = unbounded();

        *self = Self {
            receiver,
            sender,
            num_senders: 0,
        }
    }

    pub fn is_working(&self) -> bool {
        self.num_senders > 0
    }

    pub fn get<T: Fetch>(&mut self, reddit: Reddit, state: &mut State) {
        self.num_senders += 1;
        T::fetch(reddit, state, self.sender.clone());
    }

    pub fn get_content(&mut self, post: Arc<Post>, id: PostId) {
        let s = self.sender.clone();
        self.num_senders += 1;

        thread::spawn(move || {
            if let Ok(content) = post.get_content() {
                let _ = s.send(Message::ContentReady(content, id));
            }
        });
    }

    pub fn decode_image(&mut self, image: Bytes, post_id: PostId) {
        let s = self.sender.clone();
        self.num_senders += 1;
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

    pub fn start_login_process(&mut self) {
        let s = self.sender.clone();
        self.num_senders += 1;

        thread::spawn(move || {
            let auth = Reddit::perform_code_flow(
                SnuiApp::CLIENT_ID,
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
}
