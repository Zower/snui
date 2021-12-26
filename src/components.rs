use std::sync::Arc;

use eframe::egui::{self, CentralPanel, CtxRef, Response, SidePanel, TopBottomPanel, Window};
use serde::{Deserialize, Serialize};
use snew::{
    reddit::Reddit,
    things::{Me, Post},
};

use crate::{config::Options, state::State, Render};

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

impl From<(PostId, Post)> for ViewablePost {
    fn from(post: (PostId, Post)) -> Self {
        Self {
            post_id: post.0,
            inner: Arc::new(post.1),
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
    pub fn render<'a>(
        &mut self,
        posts: impl Iterator<Item = &'a ViewablePost>,
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

    fn posts<'a>(
        &mut self,
        posts: impl Iterator<Item = &'a ViewablePost>,
        ui: &mut egui::Ui,
        auto_scroll: bool,
    ) {
        egui::ScrollArea::vertical()
            .id_source("post_scroller")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.vertical_centered_justified(|ui| {
                    for (i, post) in posts.enumerate() {
                        let is_highlighted = self.highlighted == i;
                        let response = Self::ui_post_summary(ui, &*post.inner, is_highlighted);

                        if response.clicked() {
                            self.highlighted = i;
                        }

                        if (is_highlighted || response.clicked()) && auto_scroll {
                            response.scroll_to_me(egui::Align::Center)
                        }

                        ui.separator();
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
#[derive(Debug)]
pub struct Windows {
    windows: Vec<Box<dyn Show>>,
}

impl Windows {
    pub fn new() -> Self {
        Self {
            windows: vec![
                Box::new(SubredditWindow::new()),
                Box::new(FilterWindow::new()),
            ],
        }
    }

    pub fn open(&mut self, kind: WindowKind) {
        let window = self
            .windows
            .iter_mut()
            .find(|window| window.kind() == kind)
            .expect("Uninitialized window");

        window.toggle_open();
    }

    /// Called every frame
    pub fn update(&mut self, ctx: &CtxRef, reddit: &Reddit, state: &mut State) {
        for window in self.windows.iter_mut() {
            window.show(ctx, reddit, state)
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum WindowKind {
    Subreddit,
    Filter,
}

#[derive(Debug, Serialize, Deserialize)]
struct WindowState {
    open: bool,
    request_focus: bool,
}

impl WindowState {
    fn new() -> Self {
        Self {
            open: false,
            request_focus: true,
        }
    }
}

pub trait Show: std::fmt::Debug {
    fn show(&mut self, ctx: &egui::CtxRef, reddit: &Reddit, state: &mut State);
    fn kind(&self) -> WindowKind;
    fn toggle_open(&mut self);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FilterWindow {
    window: WindowState,
    checked: bool,
}

impl FilterWindow {
    fn new() -> Self {
        Self {
            window: WindowState::new(),
            checked: false,
        }
    }
}

impl Show for FilterWindow {
    fn show(&mut self, ctx: &egui::CtxRef, reddit: &Reddit, state: &mut State) {
        let mut should_close = false;

        if !self.window.open {
            self.window.request_focus = true;
        }

        egui::Window::new("Filters")
            .open(&mut self.window.open)
            .title_bar(state.options.show_title_bars)
            .show(ctx, |ui| {
                ui.add_space(10f32);
                let response = ui
                    .horizontal(|ui| {
                        ui.label("Posts: ");
                        ui.checkbox(&mut self.checked, "Only renderable")
                    })
                    .response;

                ui.add_space(10f32);

                if self.window.request_focus {
                    response.request_focus();
                    self.window.request_focus = false;
                }

                if response.gained_focus() {
                    state.num_request_disable_binds += 1
                }

                if response.lost_focus() {
                    state.num_request_disable_binds -= 1;

                    if ui.input().key_pressed(egui::Key::Enter) {
                        println!("Enter!");
                        should_close = true;
                    }
                }
            });

        if self.checked {
            state.active_filters.insert(0, |p| {
                p.inner.selftext.is_some()
                    || p.inner.url.ends_with(".jpg")
                    || p.inner.url.ends_with(".png")
                    || p.inner.url.ends_with(".jpeg")
            });
        } else {
            state.active_filters.remove(&0);
        }

        if should_close {
            self.window.open = false;
        }
    }

    fn kind(&self) -> WindowKind {
        WindowKind::Filter
    }

    fn toggle_open(&mut self) {
        self.window.open = !self.window.open
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SubredditWindow {
    window: WindowState,
    text: Option<String>,
}

impl SubredditWindow {
    fn new() -> Self {
        Self {
            window: WindowState::new(),
            text: None,
        }
    }
}

impl Show for SubredditWindow {
    fn show(&mut self, ctx: &egui::CtxRef, reddit: &Reddit, state: &mut State) {
        let mut should_close = false;

        if !self.window.open {
            self.window.request_focus = true;
            self.text = None;
        }

        egui::Window::new("Choose subreddit")
            .open(&mut self.window.open)
            .title_bar(state.options.show_title_bars)
            .show(ctx, |ui| {
                let mut text = self.text.take().unwrap_or(String::new());
                let response = ui.add(egui::TextEdit::singleline(&mut text));

                if self.window.request_focus {
                    response.request_focus();
                    self.window.request_focus = false;
                }

                if response.gained_focus() {
                    state.num_request_disable_binds += 1
                }

                self.text = if response.lost_focus() {
                    state.num_request_disable_binds -= 1;

                    if ui.input().key_pressed(egui::Key::Enter) {
                        state.reset_feed(reddit.subreddit(&text).hot());
                        should_close = true;
                        None
                    } else {
                        Some(text)
                    }
                } else {
                    Some(text)
                }
            });

        if should_close {
            self.window.open = false;
        }
    }

    fn kind(&self) -> WindowKind {
        WindowKind::Subreddit
    }

    fn toggle_open(&mut self) {
        self.window.open = !self.window.open
    }
}
