mod config;
mod fetch;
mod image_manager;
mod impl_render;
mod input;

use std::{fs, sync::Arc, vec};

use config::{Config, FileConfig};
use crossbeam_channel::{unbounded, Receiver, Sender};
use fetch::{decode_image, get_content, get_more_posts, Message};
use image_manager::ImageManager;
use impl_render::ui_post_summary;
use input::KeyPress;

use serde::Deserialize;
use snew::{
    auth::authenticator::ApplicationAuthenticator,
    reddit::{self, Reddit},
    things::{Post, PostFeed},
};

use eframe::{
    egui::{self, menu, CentralPanel, SidePanel, TopBottomPanel},
    epi,
};

#[derive(Debug)]
struct SnuiApp {
    /// The reddit client to make requests with
    client: Reddit,
    // Configuration
    config: Config,
    /// Currently loaded feed.
    feed: Option<PostFeed>,
    /// Posts that are fetched and can be displayed
    posts: Vec<ViewablePost>,
    /// Currently highlighted post in left pane
    highlighted: PostId,
    /// Currently viewed post in left pane
    viewed: PostId,
    /// Image manager
    image_manager: ImageManager,
    /// Receiver of messages created on other threads
    receiver: Receiver<Message>,
    /// Sender for giving out,
    sender: Sender<Message>,
    /// Current layout of the application
    layout: SnuiLayout,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = fs::read_to_string("./config.toml")
        .expect("Error opening config file. Please create ./config.toml");

    let config: Config = toml::from_str::<FileConfig>(&config)
        .expect("Error parsing config file. Please check ./config.toml")
        .into();

    let client = Reddit::new(
        ApplicationAuthenticator::new("kt3c_AvYiWqN5dO1lzMbjg"),
        "windows:snui:v0.1.0 (by snui on behalf of anonymous user)",
    )?;

    let mut feed = client.subreddit("images").hot();

    feed.limit = 35;

    let (s, r) = unbounded();

    let app = SnuiApp {
        client,
        feed: Some(feed),
        posts: vec![],
        highlighted: 0,
        viewed: 0,
        image_manager: ImageManager::default(),
        receiver: r,
        sender: s,
        config,
        layout: SnuiLayout::HorizontalSplit,
    };

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200f32, 800f32)),
        ..Default::default()
    };

    eframe::run_native(Box::new(app), native_options);
}

impl epi::App for SnuiApp {
    fn name(&self) -> &str {
        "SnUI"
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn eframe::epi::Storage>,
    ) {
        self.get_more_posts()
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        if self.config.immediate_posts {
            self.viewed = self.highlighted;
        }

        for event in &ctx.input().events {
            let action = match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers: m,
                } if (!pressed) => self
                    .config
                    .keybinds
                    .action(KeyPress::new((*key).into(), [m.command, m.shift, m.alt])),
                _ => None,
            };

            if let Some(action) = action {
                self.handle_action(action, frame);
            };
        }
        // if self.s.cou
        ctx.request_repaint();

        self.try_receive(frame);

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu(ui, "App", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        if self.posts.len() > 0 {
            SidePanel::left("side_panel")
                .default_width(350f32)
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        ui.vertical_centered_justified(|ui| {
                            for (i, post) in self.posts.iter().enumerate() {
                                ui_post_summary(ui, &*post.inner, self.highlighted == i);
                                if i != self.posts.len() {
                                    ui.separator();
                                }
                            }
                        })
                    });
                });
        }

        CentralPanel::default().show(ctx, |ui| {
            self.main_ui(ui);
        });
    }
}

impl SnuiApp {
    fn main_ui(&mut self, ui: &mut egui::Ui) {
        if let Some(post) = self.posts.get_mut(self.viewed) {
            if let Some(content) = &post.content {
                content.render(ui);
            } else {
                if !post.fetching {
                    post.fetching = true;
                    get_content(post.inner.clone(), self.highlighted, self.sender.clone());
                    nice_message(ui);
                }
            }
        }
    }

    fn conditional_get_more_posts(&mut self) {
        if self.highlighted == self.posts.len().checked_sub(10).unwrap_or(0) {
            self.get_more_posts()
        }
    }

    fn handle_action(&mut self, action: Action, frame: &mut epi::Frame) {
        match action {
            Action::PostDown => {
                self.highlighted = self.highlighted.checked_add(1).unwrap_or(usize::MAX);

                self.conditional_get_more_posts();
            }

            Action::PostUp => {
                self.highlighted = std::cmp::min(
                    self.posts.len(),
                    self.highlighted.checked_sub(1).unwrap_or(0),
                );

                self.conditional_get_more_posts();
            }

            Action::OpenPost => {
                if !self.config.immediate_posts {
                    self.viewed = self.highlighted
                }
            }
        }
    }

    fn try_receive(&mut self, frame: &mut epi::Frame) {
        if let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::PostsReady(mut posts, feed) => {
                    self.feed = Some(feed);
                    self.posts.append(&mut posts);
                }
                Message::ContentReady(content, post_id) => match content {
                    snew::content::Content::Text(text) => {
                        self.posts[post_id].content = Some(Arc::new(text));
                    }
                    snew::content::Content::Image(image) => {
                        decode_image(image, post_id, self.sender.clone());
                    }
                },
                Message::ImageDecoded(image, size, post_id) => {
                    let handle = self.image_manager.store(
                        self.highlighted,
                        image,
                        size,
                        frame.tex_allocator(),
                    );
                    if let Some(handle) = handle {
                        self.posts[post_id].content = Some(Arc::new(handle))
                    }
                }
            }
        }
    }
    fn get_more_posts(&mut self) {
        if let Some(feed) = self.feed.take() {
            get_more_posts(feed, self.sender.clone());
        }
    }
}

type PostId = usize;

#[derive(Debug, Clone)]
pub struct ViewablePost {
    pub fetching: bool,
    pub content: Option<Arc<dyn MainContent + Send + Sync>>,
    pub inner: Arc<Post>,
}

impl From<Post> for ViewablePost {
    fn from(post: Post) -> Self {
        Self {
            fetching: false,
            inner: Arc::new(post),
            content: None,
        }
    }
}

pub trait MainContent: std::fmt::Debug {
    fn render(&self, ui: &mut egui::Ui);
}

#[derive(Debug)]
pub(crate) enum SnuiLayout {
    /// Two or three panes showing posts | current post or comments | optional third pane for comments exclusively
    HorizontalSplit,
}

// #[derive(Debug, Clone)]
// pub enum Input {
//     Username(String),
//     Password(String),
//     ClientID(String),
//     ClientSecret(String),
// }

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Action {
    /// Currently marked post should be one higher
    PostUp,
    /// Currently marked post should be one lower
    PostDown,
    /// Open the currrently marked post
    OpenPost,
}

// fn login(creds: Credentials) -> Result<Reddit, Error> {
//     let user_agent = format!(")", &creds.username);

//     let script_auth = ScriptAuthenticator::new(creds);

//     Ok(Reddit::new(script_auth, &user_agent)?)
// }

#[derive(Debug, Clone)]
pub enum Error {
    AuthenticationError(String),
    RequestError(String),
    Other(String),
}

impl From<reddit::Error> for Error {
    fn from(error: reddit::Error) -> Self {
        match error {
            reddit::Error::AuthenticationError(err) => Self::AuthenticationError(err),
            reddit::Error::RequestError(err) => Self::RequestError(err.to_string()),
            // Implement rest of errors
            _ => panic!("Other error received"),
        }
    }
}

fn nice_message(ui: &mut egui::Ui) {
    ui.label("You're beautiful. We're not ready yet..");
}
