use std::{collections::HashMap, fmt};

use serde::Serialize;
use serde_derive::Deserialize;

use crate::{
    components::{MainContentComponent, PostFeedComponent},
    input::{KeyBind, KeyBinds},
    Action,
};

#[derive(Debug, Serialize, Deserialize)]
pub struct State {
    /// The post feed, a scrollable view of posts.
    pub feed: PostFeedComponent,
    /// The main, center view content.
    pub main_content: MainContentComponent,
    /// Number of components claiming that keybinds should not be read.
    pub num_request_disable_binds: u32,
    /// User options
    #[serde(skip)]
    pub options: Options,
}

#[derive(Debug)]
pub struct Options {
    /// Keybinds that can perform som [`Action`]
    pub keybinds: KeyBinds,
    /// Whether the post is immediately rendered upon highlight, or if [`Action::OpenPost`] must be performed
    pub immediate_posts: bool,
    /// Whether title bars are rendered. Probably want this on until Esc closes current window.
    pub show_title_bars: bool,
    /// Number of posts to buffer on either side of current post. Setting this to 10 will at most buffer 20 posts. Max 50.
    pub buffer_amount: usize,
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
            buffer_amount: fc.buffer_amount.unwrap_or(10).min(50),
        }
    }
}

impl Default for Options {
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
