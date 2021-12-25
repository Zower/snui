mod components;
mod config;
mod fetch;
mod image_manager;
mod impl_render;
mod input;

use std::sync::Arc;

use components::{PostFeedComponent, WindowKind, Windows};
use config::State;
use crossbeam_channel::{unbounded, Receiver, Sender};
use fetch::{decode_image, get_content, get_more_posts, start_login_process, Message};
use image_manager::ImageManager;
use input::KeyPress;

use serde::{Deserialize, Serialize};
use snew::{
    auth::{ApplicationAuthenticator, UserAuthenticator},
    reddit::{self, Reddit},
    things::{Me, Post},
};

use eframe::{
    egui::{self, TopBottomPanel},
    epi,
};

#[derive(Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct SnuiApp {
    /// The reddit client to make requests with
    #[serde(skip)]
    client: Reddit,
    // App state
    state: State,
    /// Image manager
    #[serde(skip)]
    image_manager: ImageManager,
    /// Receiver of messages created on other threads
    #[serde(skip)]
    receiver: Receiver<Message>,
    /// Sender for giving out
    #[serde(skip)]
    sender: Sender<Message>,
    // /// Current layout of the application
    // layout: SnuiLayout,
    /// Windows that can pop up.
    windows: Windows,
    /// Logged in user, if any.
    #[serde(skip)]
    user: Option<Me>,
    /// Number of active senders.
    #[serde(skip)]
    num_senders: u32,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = SnuiApp::default();

    let native_options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(1200f32, 800f32)),
        ..Default::default()
    };

    eframe::run_native(Box::new(app), native_options);
}

fn current_buffer<'a, T>(vec: &'a mut Vec<T>, idx: usize, amount: usize) -> &'a mut [T] {
    if vec.len() <= amount * 2 {
        &mut vec[..]
    } else if idx.checked_sub(amount).is_none() {
        &mut vec[..idx + amount]
    } else if idx + amount > vec.len() {
        &mut vec[idx - amount..]
    } else {
        &mut vec[idx - amount..idx + amount]
    }
}

impl epi::App for SnuiApp {
    fn name(&self) -> &str {
        "SnUI"
    }

    fn setup(
        &mut self,
        _ctx: &egui::CtxRef,
        _frame: &mut epi::Frame<'_>,
        _storage: Option<&dyn epi::Storage>,
    ) {
        if let Some(storage) = _storage {
            if let Some(app) = eframe::epi::get_value(storage, epi::APP_KEY) {
                *self = app;
            }

            if let Some(token) =
                eframe::epi::get_value::<SerializeRefreshToken>(storage, SerializeRefreshToken::ID)
            {
                self.client.set_authenticator(UserAuthenticator::new(
                    token.refresh_token,
                    Self::CLIENT_ID,
                ));

                self.user = self.client.me().ok();
                self.state.feed.set_feed(self.client.frontpage().hot());
                self.state.mark_for_refresh = true;
            }
        }
    }

    fn save(&mut self, storage: &mut dyn epi::Storage) {
        epi::set_value(storage, epi::APP_KEY, self);
        if let Some(refresh_token) = self.client.refresh_token() {
            epi::set_value(
                storage,
                SerializeRefreshToken::ID,
                &SerializeRefreshToken::new(refresh_token),
            );
        }
    }

    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        if self.state.mark_for_refresh {
            let (s, r) = unbounded();
            self.sender = s;
            self.receiver = r;

            self.state.mark_for_refresh = false;
            self.get_more_posts();
        }
        let current_buffer = current_buffer(
            &mut self.state.feed.posts,
            self.state.feed.viewed,
            self.state.options.buffer_amount,
        );

        for post in current_buffer {
            if !post.fetching {
                post.fetching = true;
                get_content(post.inner.clone(), post.post_id, self.sender.clone());
                self.num_senders += 1;
            }
        }

        if self.state.options.immediate_posts {
            self.state.feed.set_h_equal_v();
        }

        if self.num_senders > 0 {
            ctx.request_repaint();
        }

        if let Some(post) = self.state.feed.posts.get(self.state.feed.viewed) {
            if let Some(content) = &post.content {
                self.state.main_content.set_content(content.clone());
            }
        }

        let mut has_moved = false;

        if self.state.num_request_disable_binds == 0 {
            for event in &ctx.input().events {
                let action = match event {
                    egui::Event::Key {
                        key,
                        pressed,
                        modifiers: m,
                    } if (!pressed) => self
                        .state
                        .options
                        .keybinds
                        .action(KeyPress::new((*key).into(), [m.command, m.shift, m.alt])),
                    _ => None,
                };

                if let Some(action) = action {
                    has_moved = self.handle_action(action);
                };
            }
        }

        self.try_receive(frame);
        self.windows.update(ctx, &self.client, &mut self.state);

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                self.state.feed.render_summary(ui, &self.user);
            });
        });

        self.state.feed.render(ctx, &self.state.options, has_moved);

        self.state.main_content.render(ctx, &self.state.options);
    }
}

impl SnuiApp {
    const CLIENT_ID: &'static str = "kt3c_AvYiWqN5dO1lzMbjg";

    fn conditional_get_more_posts(&mut self) {
        if self.state.feed.highlighted >= self.state.feed.posts.len().checked_sub(10).unwrap_or(0) {
            self.get_more_posts()
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        let mut has_moved = false;
        match action {
            Action::PostDown => {
                self.state.feed.highlighted = self
                    .state
                    .feed
                    .highlighted
                    .checked_add(1)
                    .unwrap_or(usize::MAX)
                    .min(self.state.feed.posts.len() - 1);

                self.conditional_get_more_posts();

                has_moved = true;
            }
            Action::PostUp => {
                self.state.feed.highlighted =
                    self.state.feed.highlighted.checked_sub(1).unwrap_or(0);

                has_moved = true;
            }
            Action::OpenPost => {
                if !self.state.options.immediate_posts {
                    self.state.feed.viewed = self.state.feed.highlighted
                }
            }
            Action::Login => {
                start_login_process(Self::CLIENT_ID, self.sender.clone());
            }
            Action::TogglePostFeedMode => self.state.feed.toggle_mode(),
            Action::ToggleMainContentMode => self.state.main_content.toggle_mode(),
            Action::OpenSubredditWindow => self.windows.open(WindowKind::Subreddit),
            Action::Frontpage => {
                self.state.mark_for_refresh = true;

                self.state.feed = PostFeedComponent::new(self.client.frontpage().hot());
            }
        };

        has_moved
    }

    fn try_receive(&mut self, frame: &mut epi::Frame) {
        if let Ok(message) = self.receiver.try_recv() {
            match message {
                Message::PostsReady(posts, feed) => {
                    self.state.feed.set_feed(feed);
                    let mut idx = self.state.feed.posts.len();

                    for post in posts {
                        self.state.feed.posts.push((post, idx).into());
                        idx += 1;
                    }

                    self.num_senders -= 1;
                }
                Message::ContentReady(content, post_id) => match content {
                    snew::content::Content::Text(text) => {
                        self.state.feed.posts[post_id].content = Some(Arc::new(text));
                        self.num_senders -= 1;
                    }
                    snew::content::Content::Image(image) => {
                        decode_image(image, post_id, self.sender.clone());
                        self.num_senders += 1;
                        self.num_senders -= 1;
                    }
                    snew::content::Content::Html(_) => {
                        self.state.feed.posts[post_id].content = Some(Arc::new(
                            "Sorry, this is a webpage. I can't render that yet.".to_string(),
                        ));
                    }
                },
                Message::ImageDecoded(image, size, post_id) => {
                    let handle = self.image_manager.store(
                        self.state.feed.highlighted,
                        image,
                        size,
                        frame.tex_allocator(),
                    );
                    if let Some(handle) = handle {
                        self.state.feed.posts[post_id].content = Some(Arc::new(handle))
                    }
                    self.num_senders -= 1;
                }
                Message::UserLoggedIn(auth) => {
                    self.client.set_authenticator(auth);
                }
            }
        }
    }
    fn get_more_posts(&mut self) {
        if let Some(feed) = self.state.feed.take() {
            get_more_posts(feed, self.sender.clone());
            self.num_senders += 1;
        }
    }
}

type PostId = usize;

#[derive(Debug, Clone)]
pub struct ViewablePost {
    pub post_id: PostId,
    pub fetching: bool,
    pub content: Option<Arc<dyn Render + Send + Sync>>,
    pub inner: Arc<Post>,
}

impl From<(Post, PostId)> for ViewablePost {
    fn from(post: (Post, PostId)) -> Self {
        Self {
            post_id: post.1,
            fetching: false,
            inner: Arc::new(post.0),
            content: None,
        }
    }
}

/// Something that can be rendered.
/// If it makes sense to render something in multiple ways, this should be the "main", most common sense way.
pub trait Render: std::fmt::Debug {
    fn render(&self, ui: &mut egui::Ui);
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Action {
    /// Currently marked post should be one higher
    PostUp,
    /// Currently marked post should be one lower
    PostDown,
    /// Open the currrently marked post
    OpenPost,
    /// Go to frontpage
    Frontpage,
    /// Open subreddit window
    OpenSubredditWindow,
    /// Start login process
    Login,
    /// Toggle mode for the post feed
    TogglePostFeedMode,
    /// Toggle mode for the main content
    ToggleMainContentMode,
}

impl Default for SnuiApp {
    fn default() -> Self {
        let client = Reddit::new(
            ApplicationAuthenticator::new("kt3c_AvYiWqN5dO1lzMbjg"),
            "windows:snui:v0.1.0 (by snui on behalf of anonymous user)",
        )
        .expect("Failed to create reddit client");

        let feed = client.frontpage().hot();

        let (s, r) = unbounded();

        Self {
            client,
            state: State {
                feed: PostFeedComponent::new(feed),
                main_content: components::MainContentComponent::new(None),
                num_request_disable_binds: 0,
                mark_for_refresh: true,
                options: Default::default(),
            },
            image_manager: Default::default(),
            receiver: r,
            sender: s,
            windows: Windows::new(),
            user: None,
            num_senders: 0,
        }
    }
}

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

#[derive(Debug, Serialize, Deserialize)]
struct SerializeRefreshToken {
    refresh_token: String,
}

impl SerializeRefreshToken {
    const ID: &'static str = "Refresh token";

    fn new(token: String) -> Self {
        Self {
            refresh_token: token,
        }
    }
}
