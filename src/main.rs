mod components;
mod config;
mod fetch;
mod image_manager;
mod impl_render;
mod input;

use components::{
    MainContentComponent, PostFeedComponent, PostSummaryComponent, WindowKind, Windows,
};
use config::State;
use fetch::{Fetcher, Message, MorePosts};
use image_manager::ImageManager;
use input::KeyPress;

use lru::LruCache;
use serde::{Deserialize, Serialize};
use snew::{
    auth::{ApplicationAuthenticator, UserAuthenticator},
    reddit::{self, Reddit},
    things::Me,
};

use eframe::{egui, epi};

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
    #[serde(skip)]
    fetcher: Fetcher,
    // /// Current layout of the application
    // layout: SnuiLayout,
    /// Windows that can pop up.
    windows: Windows,
    /// Logged in user, if any.
    #[serde(skip)]
    user: Option<Me>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = SnuiApp::default();

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
                self.state.set_feed(self.client.frontpage().hot());
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
            self.fetcher.reset();

            self.state.mark_for_refresh = false;
            self.get_more_posts();
        }

        self.state.buffer_posts(&mut self.fetcher);

        if self.state.options.immediate_posts {
            self.state.feed_component.set_h_equal_v();
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

        self.state
            .render_summary_component(&ctx, self.user.as_ref());

        self.state
            .feed_component
            .render(&self.state.posts, ctx, &self.state.options, has_moved);

        self.state.render_main_content(&ctx);

        if self.fetcher.is_working() {
            ctx.request_repaint();
        }
    }
}

impl SnuiApp {
    const CLIENT_ID: &'static str = "kt3c_AvYiWqN5dO1lzMbjg";

    fn conditional_get_more_posts(&mut self) {
        if self.state.feed_component.highlighted
            >= self.state.posts.len().checked_sub(10).unwrap_or(0)
        {
            self.get_more_posts()
        }
    }

    fn handle_action(&mut self, action: Action) -> bool {
        let mut has_moved = false;
        match action {
            Action::PostDown => {
                self.state.feed_component.highlighted = self
                    .state
                    .feed_component
                    .highlighted
                    .checked_add(1)
                    .unwrap_or(usize::MAX)
                    .min(self.state.posts.len() - 1);

                self.conditional_get_more_posts();

                has_moved = true;
            }
            Action::PostUp => {
                self.state.feed_component.highlighted = self
                    .state
                    .feed_component
                    .highlighted
                    .checked_sub(1)
                    .unwrap_or(0);

                has_moved = true;
            }
            Action::OpenPost => {
                if !self.state.options.immediate_posts {
                    self.state.feed_component.viewed = self.state.feed_component.highlighted
                }
            }
            Action::Login => {
                self.fetcher.start_login_process();
            }
            Action::TogglePostFeedMode => self.state.feed_component.toggle_mode(),
            Action::ToggleMainContentMode => self.state.main_component.toggle_mode(),
            Action::TogglePostSummaryMode => self.state.summary_component.toggle_mode(),
            Action::OpenSubredditWindow => self.windows.open(WindowKind::Subreddit),
            Action::Frontpage => {
                self.state.mark_for_refresh = true;

                self.state.set_feed(self.client.frontpage().hot());
                self.state.posts.clear();
                self.state.content_cache.clear();
                self.state.feed_component.reset();
            }
        };

        has_moved
    }

    fn try_receive(&mut self, frame: &mut epi::Frame) {
        if let Some(message) = self.fetcher.try_recv() {
            match message {
                Message::PostsReady(posts, feed) => {
                    self.state.set_feed(feed);
                    self.state.extend_posts(posts);
                }
                Message::ContentReady(content, post_id) => match content {
                    snew::content::Content::Text(text) => {
                        self.state.set_content(&post_id, Box::new(text));
                    }
                    snew::content::Content::Image(image) => {
                        self.fetcher.decode_image(image, post_id);
                    }
                    snew::content::Content::Html(_) => {
                        self.state.set_content(
                            &post_id,
                            Box::new(String::from("Sorry, I can't render this yet.")),
                        );
                    }
                },
                Message::ImageDecoded(image, size, url) => {
                    let handle = self.image_manager.store(
                        self.state.feed_component.highlighted,
                        image,
                        size,
                        frame.tex_allocator(),
                    );
                    if let Some(handle) = handle {
                        self.state.set_content(&url, Box::new(handle));
                    }
                }
                Message::UserLoggedIn(auth) => {
                    self.client.set_authenticator(auth);
                }
            }
        }
    }

    fn get_more_posts(&mut self) {
        self.fetcher
            .get::<MorePosts>(self.client.clone(), &mut self.state);
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
    /// Toggle mode for post summary
    TogglePostSummaryMode,
}

impl Default for SnuiApp {
    fn default() -> Self {
        let client = Reddit::new(
            ApplicationAuthenticator::new("kt3c_AvYiWqN5dO1lzMbjg"),
            "windows:snui:v0.1.0 (by snui on behalf of anonymous user)",
        )
        .expect("Failed to create reddit client");

        let mut feed = client.frontpage().hot();
        feed.limit = 15;

        Self {
            client,
            state: State {
                feed_component: PostFeedComponent::new(),
                main_component: MainContentComponent::new(),
                summary_component: PostSummaryComponent::new(),
                feed: Some(feed),
                posts: vec![],
                num_request_disable_binds: 0,
                mark_for_refresh: true,
                content_cache: LruCache::new(250),
                options: Default::default(),
            },
            image_manager: Default::default(),
            fetcher: Fetcher::default(),
            windows: Windows::new(),
            user: None,
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
