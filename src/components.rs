use std::sync::Arc;

use eframe::egui::{self, CentralPanel, CtxRef, Layout, Resize, Response, SidePanel, Window};
use serde::{Deserialize, Serialize};
use snew::{
    reddit::Reddit,
    things::{Me, Post, PostFeed},
};

use crate::{
    config::{Options, State},
    PostId, Render, ViewablePost,
};

#[derive(Debug, Serialize, Deserialize)]
pub enum ComponentMode {
    Snapped,
    Floating,
    Closed,
}

impl ComponentMode {
    pub fn next(&self) -> Self {
        match self {
            ComponentMode::Snapped => ComponentMode::Floating,
            ComponentMode::Floating => ComponentMode::Closed,
            ComponentMode::Closed => ComponentMode::Snapped,
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MainContentComponent {
    #[serde(skip)]
    pub content: Option<Arc<dyn Render>>,
    pub mode: ComponentMode,
}

impl MainContentComponent {
    pub fn new(content: Option<Arc<dyn Render>>) -> Self {
        Self {
            content,
            mode: ComponentMode::Snapped,
        }
    }

    pub fn set_content(&mut self, content: Arc<dyn Render>) {
        self.content = Some(content);
    }

    pub fn render(&mut self, ctx: &CtxRef, options: &Options) {
        match self.mode {
            ComponentMode::Snapped => {
                CentralPanel::default().show(&ctx, |ui| {
                    self.render_if_some(ui);
                });
            }
            ComponentMode::Floating => {
                Window::new("Main view")
                    .title_bar(options.show_title_bars)
                    .default_width(800f32)
                    .default_height(800f32)
                    .show(&ctx, |ui| {
                        self.render_if_some(ui);
                    });
            }
            ComponentMode::Closed => {}
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }

    fn render_if_some(&self, ui: &mut egui::Ui) -> bool {
        if let Some(content) = &self.content {
            content.render(ui);
            true
        } else {
            false
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostFeedComponent {
    /// Currently loaded feed.
    #[serde(skip)]
    pub feed: Option<PostFeed>,
    /// Posts that are fetched and can be displayed
    #[serde(skip)]
    pub posts: Vec<ViewablePost>,
    /// Currently highlighted post in left pane
    #[serde(skip)]
    pub highlighted: PostId,
    /// Currently viewed post in left pane
    #[serde(skip)]
    pub viewed: PostId,
    pub mode: ComponentMode,
    just_dragged: bool,
}

impl PostFeedComponent {
    pub fn new(mut feed: PostFeed) -> Self {
        feed.limit = 15;

        Self {
            feed: Some(feed),
            posts: vec![],
            highlighted: 0,
            viewed: 0,
            mode: ComponentMode::Snapped,
            just_dragged: false,
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }

    pub fn set_feed(&mut self, mut feed: PostFeed) {
        feed.limit = 15;
        self.feed = Some(feed);
    }

    pub fn set_h_equal_v(&mut self) {
        self.viewed = self.highlighted;
    }

    pub fn take(&mut self) -> Option<PostFeed> {
        self.feed.take()
    }
}

impl PostFeedComponent {
    pub fn render(&mut self, ctx: &CtxRef, options: &Options, auto_scroll: bool) {
        match self.mode {
            ComponentMode::Snapped => {
                SidePanel::left("Posts")
                    .default_width(350f32)
                    .show(ctx, |ui| {
                        self.posts(ui, auto_scroll);
                    });
            }
            ComponentMode::Floating => {
                Window::new("Posts")
                    .default_width(350f32)
                    .default_height(800f32)
                    .title_bar(options.show_title_bars)
                    .show(&ctx, |ui| {
                        self.posts(ui, auto_scroll);
                    });
            }
            ComponentMode::Closed => {}
        }
    }

    pub fn render_summary(&self, ui: &mut egui::Ui, user: &Option<Me>) {
        ui.centered_and_justified(|ui| {
            if self.posts.len() > 0 {
                let post = &self.posts[self.highlighted].inner;
                ui.label(format!("{} by /u/{}", &post.title, &post.author));
                ui.add_space(3f32);
                ui.label(format!("{} points\t\t/r/{}", &post.score, &post.subreddit));
                ui.add_space(3f32);
            } else {
                ui.label("Loading..");
            }
        });
        if let Some(user) = user {
            ui.with_layout(Layout::right_to_left(), |ui| {
                ui.add_space(10f32);
                ui.label(format!("Logged in as /u/{}", &user.name));
            });
        }
    }

    fn posts(&mut self, ui: &mut egui::Ui, auto_scroll: bool) {
        egui::ScrollArea::vertical()
            .id_source("post_scroller")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    for (i, post) in self.posts.iter().enumerate() {
                        let is_highlighted = self.highlighted == i;
                        let response = Self::ui_post_summary(ui, &*post.inner, is_highlighted);

                        if response.clicked() {
                            self.highlighted = i;
                        }

                        if (is_highlighted || response.clicked()) && auto_scroll {
                            response.scroll_to_me(egui::Align::Center)
                        }

                        if i != self.posts.len() {
                            ui.separator();
                        }
                    }
                });
            });
    }

    fn ui_post_summary(ui: &mut egui::Ui, post: &Post, highlight: bool) -> Response {
        let response = ui.vertical(|ui| {
            if highlight {
                ui.visuals_mut().widgets.noninteractive.fg_stroke =
                    egui::Stroke::new(10f32, egui::Color32::WHITE);
            }

            let max_chars = (ui.available_width() / 10f32) as usize;

            let title = PostFeedComponent::create_display_string(&post.title, max_chars);
            let url = PostFeedComponent::create_display_string(&post.url, max_chars);

            let title = egui::Label::new(title)
                .sense(egui::Sense::click())
                .wrap(true)
                .heading();
            let response = ui.add(title);

            ui.horizontal(|ui| {
                ui.label(url);
                ui.with_layout(egui::Layout::bottom_up(egui::Align::RIGHT), |ui| {
                    ui.label(post.num_comments)
                });
            });
            ui.label(post.score.to_string() + " points");

            response
        });

        response.inner
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
}

/// Floatable, potentially open, windows.
#[derive(Debug, Serialize, Deserialize)]
pub struct Windows {
    subreddit: SubredditWindow,
}

impl Windows {
    pub fn new() -> Self {
        Self {
            subreddit: SubredditWindow {
                request_focus: true,
                window: WindowState::new(None),
            },
        }
    }

    pub fn open(&mut self, kind: WindowKind) {
        match kind {
            WindowKind::Subreddit => {
                self.subreddit.window.open = true;
            }
        }
    }

    /// Called every frame
    pub fn update(&mut self, ctx: &CtxRef, reddit: &Reddit, state: &mut State) {
        self.show_subreddit(ctx, reddit, state);
    }

    fn show_subreddit(&mut self, ctx: &CtxRef, reddit: &Reddit, state: &mut State) {
        let sub = self.subreddit.show(ctx, state);

        if let Some(sub) = sub {
            SubredditWindow::handle(&sub, reddit, state)
        }
    }
}

pub enum WindowKind {
    Subreddit,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowState<T> {
    open: bool,
    inner: T,
}

impl<T> WindowState<T> {
    fn new(t: T) -> Self {
        Self {
            open: false,
            inner: t,
        }
    }
}

pub trait Handle {
    type Input;
    fn handle(input: &Self::Input, reddit: &Reddit, state: &mut State);
}

impl Handle for SubredditWindow {
    type Input = String;
    fn handle(input: &Self::Input, reddit: &Reddit, state: &mut State) {
        state.feed = PostFeedComponent::new(reddit.subreddit(&input).hot());
    }
}

pub trait Show {
    type Output;
    fn show(&mut self, ctx: &egui::CtxRef, state: &mut State) -> Self::Output;
}

impl Show for SubredditWindow {
    type Output = Option<String>;

    fn show(&mut self, ctx: &egui::CtxRef, state: &mut State) -> Self::Output {
        let mut should_close = false;

        if !self.window.open {
            self.request_focus = true;
            self.window.inner = None;
        }

        egui::Window::new("Choose subreddit")
            .open(&mut self.window.open)
            .title_bar(state.options.show_title_bars)
            .show(ctx, |ui| {
                let mut text = self.window.inner.take().unwrap_or(String::new());
                let response = ui.add(egui::TextEdit::singleline(&mut text));

                if self.request_focus {
                    response.request_focus();
                    self.request_focus = false;
                }

                if response.gained_focus() {
                    state.num_request_disable_binds += 1
                }

                self.window.inner = Some(text);

                if response.lost_focus() {
                    state.num_request_disable_binds -= 1;

                    if ui.input().key_pressed(egui::Key::Enter) {
                        should_close = true;
                    }
                }
            });

        return if should_close {
            self.window.open = false;
            self.request_focus = true;
            self.window.inner.take()
        } else {
            None
        };
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubredditWindow {
    request_focus: bool,
    window: WindowState<Option<String>>,
}
