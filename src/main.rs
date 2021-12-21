mod fetch;
mod image_manager;
mod impl_render;
mod input;

use std::{sync::Arc, vec};

use crossbeam_channel::{unbounded, Receiver, Sender};
use fetch::{spawn_more, Message};
use image_manager::ImageManager;
use impl_render::ui_post_summary;
use input::{KeyBinds, KeyPress};

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
    /// Currently loaded feed.
    feed: Option<PostFeed>,
    /// Posts that are fetched and can be displayed
    posts: Vec<Arc<Post>>,
    /// Currently highlighted post in left pane
    highlighted: usize,
    /// Content currently in the center pane
    content: Option<Arc<dyn MainContent>>,
    /// Image manager
    image_manager: ImageManager,
    /// Receiver of messages created on other threads
    receiver: Receiver<Message>,
    /// Sender for giving out,
    sender: Sender<Message>,
    /// Keybinds that can perform som [`Action`]
    keybinds: KeyBinds,
    /// Current layout of the application
    layout: SnuiLayout,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Reddit::new(
        ApplicationAuthenticator::new("kt3c_AvYiWqN5dO1lzMbjg"),
        "windows:snui:v0.1.0 (by /u/zower98",
    )?;

    let mut feed = client.subreddit("images").hot();

    feed.limit = 35;

    let (s, r) = unbounded();

    let app = SnuiApp {
        client,
        feed: Some(feed),
        posts: vec![],
        highlighted: 0,
        content: None,
        image_manager: ImageManager::default(),
        receiver: r,
        sender: s,
        keybinds: KeyBinds::default(),
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
        self.spawn_more()
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        if let None = self.feed {
            self.try_extend()
        }

        for event in &ctx.input().events {
            let action = match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers: _,
                } if (!pressed) => self.keybinds.action(KeyPress::basic(*key)),
                _ => None,
            };

            if let Some(action) = action {
                match action {
                    Action::PostUp => {
                        self.highlighted = self.highlighted.checked_add(1).unwrap_or(usize::MAX)
                    }
                    Action::PostDown => {
                        self.highlighted = std::cmp::min(
                            self.posts.len(),
                            self.highlighted.checked_sub(1).unwrap_or(0),
                        )
                    }
                    Action::OpenPost => {
                        let post = &self.posts[self.highlighted];

                        if post.is_self {
                            self.content = Some(post.clone())
                        } else {
                            if let Some(image) = self.image_manager.get(&self.highlighted) {
                                self.content = Some(Arc::new(image.clone()))
                            } else {
                                if let Ok(content) = post.get_content() {
                                    match content {
                                        snew::content::Content::Text(text) => {
                                            self.content = Some(Arc::new(text))
                                        }
                                        snew::content::Content::Image(image) => {
                                            let handle = self.image_manager.store(
                                                self.highlighted,
                                                &image,
                                                frame.tex_allocator(),
                                            );
                                            if let Some(handle) = handle {
                                                self.content = Some(Arc::new(handle))
                                            }
                                        }
                                    }
                                } else {
                                    println!("{:?}", post.get_content());
                                }
                            }
                        }
                    }
                }
            };
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu(ui, "App", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        SidePanel::left("side_panel")
            .default_width(350f32)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .always_show_scroll(true)
                    .show(ui, |ui| {
                        if self.highlighted == self.posts.len().checked_sub(10).unwrap_or(0) {
                            self.spawn_more()
                        }

                        ui.vertical_centered_justified(|ui| {
                            for (i, post) in self.posts.iter().enumerate() {
                                ui_post_summary(ui, &*post, self.highlighted == i);
                                if i != self.posts.len() {
                                    ui.separator();
                                }
                            }
                        })
                    });
            });

        CentralPanel::default().show(ctx, |ui| {
            if let Some(content) = &self.content {
                content.render(ui);
            } else {
                nice_message(ui);
            }
        });
    }
}

impl SnuiApp {
    fn try_extend(&mut self) {
        if let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::PostsReady(mut posts, feed) => {
                    self.feed = Some(feed);
                    self.posts.append(&mut posts);
                }
            }
        }
    }
    fn spawn_more(&mut self) {
        if let Some(feed) = self.feed.take() {
            spawn_more(feed, self.sender.clone());
        }
    }
}

pub(crate) trait MainContent: std::fmt::Debug {
    fn render(&self, ui: &mut egui::Ui);
}

#[derive(Debug)]
pub(crate) enum SnuiLayout {
    /// Two or three panes showing posts | current post or comments | optional third pane for comments exclusively
    HorizontalSplit,
}

#[derive(Debug, Clone)]
pub enum Input {
    Username(String),
    Password(String),
    ClientID(String),
    ClientSecret(String),
}

#[derive(Debug, Clone, Copy)]
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
