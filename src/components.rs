use eframe::egui::{self, CtxRef};
use snew::reddit::Reddit;

use crate::config::State;

/// Floatable, potentially open, windows.
#[derive(Debug)]
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
            WindowKind::Subreddit => self.subreddit.window.open = true,
        }
    }

    pub fn show(&mut self, ctx: &CtxRef, reddit: &Reddit, state: &mut State) {
        let sub = self.subreddit.show(ctx);

        if let Some(sub) = sub {
            SubredditWindow::handle(&sub, reddit, state)
        }
    }
}

pub enum WindowKind {
    Subreddit,
}

#[derive(Debug)]
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
        state.posts.clear();

        state.feed = Some(reddit.subreddit(&input).hot());

        state.highlighted = 0;
        state.viewed = 0;
    }
}

pub trait Show {
    type Output;
    fn show(&mut self, ctx: &egui::CtxRef) -> Self::Output;
}

impl Show for SubredditWindow {
    type Output = Option<String>;

    fn show(&mut self, ctx: &egui::CtxRef) -> Self::Output {
        let mut should_close = false;
        egui::Window::new("Choose subreddit")
            .open(&mut self.window.open)
            .show(ctx, |ui| {
                let mut text = self.window.inner.take().unwrap_or(String::new());
                let response = ui.add(egui::TextEdit::singleline(&mut text));

                if self.request_focus {
                    response.request_focus();
                    self.request_focus = false;
                }

                self.window.inner = Some(text);

                if response.lost_focus() && ui.input().key_pressed(egui::Key::Enter) {
                    should_close = true;
                }
            });

        return if should_close {
            self.window.open = false;
            self.window.inner.take()
        } else {
            None
        };
    }
}

#[derive(Debug)]
pub struct SubredditWindow {
    request_focus: bool,
    window: WindowState<Option<String>>,
}
