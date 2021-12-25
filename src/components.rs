use std::sync::Arc;

use eframe::egui::{
    self, CentralPanel, CtxRef, Layout, Response, SidePanel, TopBottomPanel, Window,
};
use serde::{Deserialize, Serialize};
use snew::{
    reddit::Reddit,
    things::{Me, Post},
};

use crate::{
    config::{Options, State},
    Render,
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
    pub mode: ComponentMode,
}

impl MainContentComponent {
    pub fn new() -> Self {
        Self {
            mode: ComponentMode::Snapped,
        }
    }

    pub fn render(&self, ctx: &CtxRef, options: &Options, content: &Box<dyn Render>) {
        match self.mode {
            ComponentMode::Snapped => {
                CentralPanel::default().show(&ctx, |ui| {
                    content.render(ui);
                });
            }
            ComponentMode::Floating => {
                Window::new("Main view")
                    .title_bar(options.show_title_bars)
                    .default_width(800f32)
                    .default_height(600f32)
                    .show(&ctx, |ui| {
                        content.render(ui);
                    });
            }
            ComponentMode::Closed => {}
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }
}

pub type PostId = usize;

#[derive(Debug, Clone)]
pub struct ViewablePost {
    pub post_id: PostId,
    pub inner: Arc<Post>,
}

impl From<(Post, PostId)> for ViewablePost {
    fn from(post: (Post, PostId)) -> Self {
        Self {
            post_id: post.1,
            inner: Arc::new(post.0),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PostFeedComponent {
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
    pub fn new() -> Self {
        Self {
            // feed: Some(feed),
            // posts: vec![],
            highlighted: 0,
            viewed: 0,
            mode: ComponentMode::Snapped,
            just_dragged: false,
        }
    }

    pub fn reset(&mut self) {
        self.viewed = 0;
        self.highlighted = 0;
    }

    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }

    pub fn set_h_equal_v(&mut self) {
        self.viewed = self.highlighted;
    }
}

impl PostFeedComponent {
    pub fn render(
        &mut self,
        posts: &Vec<ViewablePost>,
        ctx: &CtxRef,
        options: &Options,
        auto_scroll: bool,
    ) {
        match self.mode {
            ComponentMode::Snapped => {
                SidePanel::left("Posts")
                    .default_width(350f32)
                    .show(ctx, |ui| {
                        self.posts(posts, ui, auto_scroll);
                    });
            }
            ComponentMode::Floating => {
                Window::new("Posts")
                    .default_width(350f32)
                    .default_height(800f32)
                    .title_bar(options.show_title_bars)
                    .show(&ctx, |ui| {
                        self.posts(posts, ui, auto_scroll);
                    });
            }
            ComponentMode::Closed => {}
        }
    }

    fn posts(&mut self, posts: &Vec<ViewablePost>, ui: &mut egui::Ui, auto_scroll: bool) {
        egui::ScrollArea::vertical()
            .id_source("post_scroller")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    for (i, post) in posts.iter().enumerate() {
                        let is_highlighted = self.highlighted == i;
                        let response = Self::ui_post_summary(ui, &*post.inner, is_highlighted);

                        if response.clicked() {
                            self.highlighted = i;
                        }

                        if (is_highlighted || response.clicked()) && auto_scroll {
                            response.scroll_to_me(egui::Align::Center)
                        }

                        if i != posts.len() {
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

#[derive(Debug, Deserialize, Serialize)]
pub struct PostSummaryComponent {
    pub mode: ComponentMode,
}

impl PostSummaryComponent {
    pub fn new() -> Self {
        Self {
            mode: ComponentMode::Snapped,
        }
    }
    pub fn render(
        &self,
        ctx: &CtxRef,
        options: &Options,
        post: Option<&ViewablePost>,
        user: Option<&Me>,
    ) {
        match self.mode {
            ComponentMode::Snapped => {
                TopBottomPanel::top("top_panel").show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        Self::render_summary(post, ui, user);
                    });
                });
            }
            ComponentMode::Floating => {
                Window::new("Viewed post")
                    .title_bar(options.show_title_bars)
                    .default_width(500f32)
                    .default_height(100f32)
                    .resizable(true)
                    .show(&ctx, |ui| {
                        Self::render_summary(post, ui, user);
                    });
            }
            ComponentMode::Closed => {}
        }
    }

    pub fn toggle_mode(&mut self) {
        self.mode = self.mode.next();
    }

    fn render_summary(post: Option<&ViewablePost>, ui: &mut egui::Ui, user: Option<&Me>) {
        ui.centered_and_justified(|ui| {
            if let Some(post) = post {
                let post = &post.inner;
                let user_string = if let Some(user) = user {
                    format!("Logged in as /u/{}", user.name)
                } else {
                    String::from("")
                };

                ui.label(format!(
                    "{} by /u/{}\n{} points\t\t/r/{}\t\t\t\t\t{}",
                    &post.title, &post.author, &post.score, &post.subreddit, user_string
                ));
            } else {
                ui.label("Loading..");
            }
        });
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
        state.feed = Some(reddit.subreddit(input).hot());
        state.posts.clear();
        state.content_cache.clear();
        state.feed_component.reset();

        state.mark_for_refresh = true;
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
