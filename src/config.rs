use std::{collections::HashMap, fmt};

use eframe::egui::CtxRef;
use lru::LruCache;
use serde::Serialize;
use serde_derive::Deserialize;
use snew::things::{Me, Post, PostFeed};

use crate::{
    components::{
        MainContentComponent, PostFeedComponent, PostId, PostSummaryComponent, ViewablePost,
    },
    fetch::Fetcher,
    input::{KeyBind, KeyBinds},
    Action, Render,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    /// The post feed, a scrollable view of posts.
    pub feed_component: PostFeedComponent,
    /// The main, center view content.
    pub main_component: MainContentComponent,
    /// The summary of the the current post. Also information about the user, if present.
    pub summary_component: PostSummaryComponent,
    /// Currently loaded feed.
    #[serde(skip)]
    pub feed: Option<PostFeed>,
    /// Posts that are fetched and can be displayed
    #[serde(skip)]
    pub posts: Vec<ViewablePost>,
    /// Cached content
    #[serde(skip)]
    #[serde(default = "empty_map")]
    pub content_cache: LruCache<PostId, Option<Box<dyn Render>>>,
    /// Number of components claiming that keybinds should not be read.
    pub num_request_disable_binds: u32,
    /// Reset posts
    #[serde(skip)]
    pub mark_for_refresh: bool,
    /// User options
    #[serde(skip)]
    pub options: Options,
}

fn current_buffer<'a, T>(
    vec: &'a mut Vec<T>,
    idx: usize,
    amount: usize,
    ratio: f32,
) -> &'a mut [T] {
    let right_side = (ratio * amount as f32).round() as usize;
    let left_side = ((1f32 - ratio) * amount as f32).round() as usize;

    let len = vec.len();

    if len <= amount {
        &mut vec[..]
    } else if idx.checked_sub(left_side).is_none() {
        &mut vec[0..idx + right_side]
    } else if idx + right_side > len {
        &mut vec[idx - left_side..len - 1]
    } else {
        &mut vec[idx - left_side..idx + right_side]
    }
}

impl State {
    pub fn buffer_posts(&mut self, fetcher: &mut Fetcher) {
        let window = current_buffer(
            &mut self.posts,
            self.feed_component.viewed,
            self.options.buffer_amount,
            self.options.buffer_ratio,
        );

        for post in window {
            if !self.content_cache.contains(&post.post_id) {
                self.content_cache.put(post.post_id, None);
                fetcher.get_content(post.inner.clone(), post.post_id)
            }
        }
    }

    pub fn set_feed(&mut self, mut feed: PostFeed) {
        feed.limit = 15;
        self.feed = Some(feed);
    }

    pub fn render_main_content(&mut self, ctx: &CtxRef) {
        let post = self.posts.get(self.feed_component.viewed);

        let content = Box::new(String::from("Loading..")) as Box<dyn Render>;
        let mut content = &content;

        if let Some(post) = post {
            if let Some(maybe_cached) = self.content_cache.get(&post.post_id) {
                if let Some(cached_content) = maybe_cached {
                    content = cached_content;
                }
            }
        }

        self.main_component.render(ctx, &self.options, content);
    }

    pub fn extend_posts(&mut self, posts: Vec<Post>) {
        let mut idx = self.posts.len();

        for post in posts {
            self.posts.push((post, idx).into());
            idx += 1;
        }
    }

    pub fn set_content(&mut self, post_id: &PostId, content: Box<dyn Render>) {
        if let Some(empty_content) = self.content_cache.get_mut(post_id) {
            assert!(empty_content.is_none());
            *empty_content = Some(content);
        }
    }
}

impl State {
    pub fn render_summary_component(&self, ctx: &CtxRef, me: Option<&Me>) {
        self.summary_component.render(
            ctx,
            &self.options,
            self.posts.get(self.feed_component.viewed),
            me,
        );
    }
}

fn empty_map() -> LruCache<PostId, Option<Box<dyn Render>>> {
    LruCache::new(250)
}

#[derive(Debug)]
pub struct Options {
    /// Keybinds that can perform som [`Action`]
    pub keybinds: KeyBinds,
    /// Whether the post is immediately rendered upon highlight, or if [`Action::OpenPost`] must be performed
    pub immediate_posts: bool,
    /// Whether title bars are rendered. Probably want this on until Esc closes current window.
    pub show_title_bars: bool,
    /// Number of posts to buffer. Max 50.
    pub buffer_amount: usize,
    /// The ratio of the buffer above and below the currently viewed post.
    /// If buffer_amount is 10, and this is 0.8, 8 posts will be buffered in front of current, and one behind.
    pub buffer_ratio: f32,
}

impl From<FileConfig> for Options {
    fn from(fc: FileConfig) -> Self {
        let mut keybinds = KeyBinds::default();
        for (key, details) in fc.binds.into_iter() {
            match details {
                ConfigKey::Simple(action) => {
                    keybinds.binds.insert(KeyBind::basic(key.into()), action)
                }
                ConfigKey::Detailed(config) => {
                    let m = config.modifiers;
                    let ctrl = m.iter().any(|m| *m == Mods::Ctrl);
                    let shift = m.iter().any(|m| *m == Mods::Shift);
                    let alt = m.iter().any(|m| *m == Mods::Alt);

                    keybinds
                        .binds
                        .insert(KeyBind::new(key.into(), [ctrl, shift, alt]), config.action)
                }
            };
        }

        Self {
            keybinds,
            immediate_posts: fc.immediate_posts.unwrap_or(false),
            show_title_bars: fc.show_title_bars.unwrap_or(true),
            buffer_amount: fc.buffer_amount.unwrap_or(25).min(50).max(1),
            buffer_ratio: fc.buffer_ratio.unwrap_or(0.75).min(1f32).max(0f32),
        }
    }
}

impl Default for Options {
    // todo dont crash
    fn default() -> Self {
        let config = std::fs::read_to_string("./config.toml")
            .expect("Error opening config file. Please create ./config.toml");

        toml::from_str::<FileConfig>(&config)
            .expect("Error parsing config file. Please check ./config.toml")
            .into()
    }
}

#[derive(Debug, Deserialize)]
pub struct FileConfig {
    pub binds: HashMap<Key, ConfigKey>,
    pub immediate_posts: Option<bool>,
    pub show_title_bars: Option<bool>,
    pub buffer_amount: Option<usize>,
    pub buffer_ratio: Option<f32>,
}

#[derive(Debug, Deserialize)]
#[serde(untagged)]
pub enum ConfigKey {
    Simple(Action),
    Detailed(ConfigAction),
}

#[derive(Debug, Deserialize)]
pub struct ConfigAction {
    pub action: Action,
    #[serde(default = "no_mods")]
    pub modifiers: Vec<Mods>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub enum Mods {
    #[serde(alias = "alt")]
    Alt,
    #[serde(alias = "shift")]
    Shift,
    #[serde(alias = "ctrl")]
    Ctrl,
}

fn no_mods() -> Vec<Mods> {
    vec![]
}

#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Deserialize)]
#[serde(try_from = "String")]
pub enum Key {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,

    Escape,
    Tab,
    Backspace,
    Enter,
    Space,

    Insert,
    Delete,
    Home,
    End,
    PageUp,
    PageDown,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
}

pub struct NoMatchingKey(String);

impl fmt::Display for NoMatchingKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "No matching keybind: {} found", self.0)
    }
}

impl From<eframe::egui::Key> for Key {
    fn from(k: eframe::egui::Key) -> Self {
        match k {
            eframe::egui::Key::A => Self::A,
            eframe::egui::Key::B => Self::B,
            eframe::egui::Key::C => Self::C,
            eframe::egui::Key::D => Self::D,
            eframe::egui::Key::E => Self::E,
            eframe::egui::Key::F => Self::F,
            eframe::egui::Key::G => Self::G,
            eframe::egui::Key::H => Self::H,
            eframe::egui::Key::I => Self::I,
            eframe::egui::Key::J => Self::J,
            eframe::egui::Key::K => Self::K,
            eframe::egui::Key::L => Self::L,
            eframe::egui::Key::M => Self::M,
            eframe::egui::Key::N => Self::N,
            eframe::egui::Key::O => Self::O,
            eframe::egui::Key::P => Self::P,
            eframe::egui::Key::Q => Self::Q,
            eframe::egui::Key::R => Self::R,
            eframe::egui::Key::S => Self::S,
            eframe::egui::Key::T => Self::T,
            eframe::egui::Key::U => Self::U,
            eframe::egui::Key::V => Self::V,
            eframe::egui::Key::W => Self::W,
            eframe::egui::Key::X => Self::X,
            eframe::egui::Key::Y => Self::Y,
            eframe::egui::Key::Z => Self::Z,
            eframe::egui::Key::ArrowDown => Self::ArrowDown,
            eframe::egui::Key::ArrowLeft => Self::ArrowLeft,
            eframe::egui::Key::ArrowRight => Self::ArrowRight,
            eframe::egui::Key::ArrowUp => Self::ArrowUp,
            eframe::egui::Key::Escape => Self::Escape,
            eframe::egui::Key::Tab => Self::Tab,
            eframe::egui::Key::Backspace => Self::Backspace,
            eframe::egui::Key::Enter => Self::Enter,
            eframe::egui::Key::Space => Self::Space,
            eframe::egui::Key::Num0 => Self::Num0,
            eframe::egui::Key::Num1 => Self::Num1,
            eframe::egui::Key::Num2 => Self::Num2,
            eframe::egui::Key::Num3 => Self::Num3,
            eframe::egui::Key::Num4 => Self::Num4,
            eframe::egui::Key::Num5 => Self::Num5,
            eframe::egui::Key::Num6 => Self::Num6,
            eframe::egui::Key::Num7 => Self::Num7,
            eframe::egui::Key::Num8 => Self::Num8,
            eframe::egui::Key::Num9 => Self::Num9,
            eframe::egui::Key::Insert => Self::Insert,
            eframe::egui::Key::Delete => Self::Delete,
            eframe::egui::Key::Home => Self::Home,
            eframe::egui::Key::End => Self::End,
            eframe::egui::Key::PageUp => Self::PageUp,
            eframe::egui::Key::PageDown => Self::PageDown,
        }
    }
}

impl TryFrom<String> for Key {
    type Error = NoMatchingKey;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        match value.as_str().to_lowercase().as_str() {
            "a" => Ok(Self::A),
            "b" => Ok(Self::B),
            "c" => Ok(Self::C),
            "d" => Ok(Self::D),
            "e" => Ok(Self::E),
            "f" => Ok(Self::F),
            "g" => Ok(Self::G),
            "h" => Ok(Self::H),
            "i" => Ok(Self::I),
            "j" => Ok(Self::J),
            "k" => Ok(Self::K),
            "l" => Ok(Self::L),
            "m" => Ok(Self::M),
            "n" => Ok(Self::N),
            "o" => Ok(Self::O),
            "p" => Ok(Self::P),
            "q" => Ok(Self::Q),
            "r" => Ok(Self::R),
            "s" => Ok(Self::S),
            "t" => Ok(Self::T),
            "u" => Ok(Self::U),
            "v" => Ok(Self::V),
            "w" => Ok(Self::W),
            "x" => Ok(Self::X),
            "y" => Ok(Self::Y),
            "z" => Ok(Self::Z),
            "0" => Ok(Self::Num0),
            "1" => Ok(Self::Num1),
            "2" => Ok(Self::Num2),
            "3" => Ok(Self::Num3),
            "4" => Ok(Self::Num4),
            "5" => Ok(Self::Num5),
            "6" => Ok(Self::Num6),
            "7" => Ok(Self::Num7),
            "8" => Ok(Self::Num8),
            "9" => Ok(Self::Num9),
            "escape" => Ok(Self::Escape),
            "tab" => Ok(Self::Tab),
            "backspace" => Ok(Self::Backspace),
            "enter" => Ok(Self::Enter),
            "space" => Ok(Self::Space),
            "down" => Ok(Self::ArrowDown),
            "up" => Ok(Self::ArrowUp),
            "left" => Ok(Self::ArrowLeft),
            "right" => Ok(Self::ArrowRight),
            "insert" => Ok(Self::Insert),
            "home" => Ok(Self::Home),
            "end" => Ok(Self::End),
            "pageup" => Ok(Self::PageUp),
            "pagedown" => Ok(Self::Insert),
            _ => Err(NoMatchingKey(value)),
        }
    }
}
