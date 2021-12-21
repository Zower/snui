use crate::{config::Key, Action};
use std::collections::HashMap;

use eframe::egui;

/// A map from keys to actions.
#[derive(Debug)]
pub struct KeyBinds {
    pub binds: HashMap<KeyBind, Action>,
}

impl KeyBinds {
    pub fn action(&self, key: KeyPress) -> Option<Action> {
        self.binds.get(&KeyBind::from(key)).map(|value| *value)
    }
}

/// A keypress, but one that can be validly mapped to an action.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct KeyBind(KeyPress);

impl KeyBind {
    pub fn basic(key: Key) -> Self {
        Self(KeyPress::basic(key))
    }

    /// modifiers: [CTRL, SHIFT, ALT]
    pub fn new(key: Key, modifiers: [bool; 3]) -> Self {
        Self(KeyPress::new(key, modifiers))
    }
}

/// Press of a key along with potential modifiers.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq)]
pub struct KeyPress {
    pub key: Key,
    pub control: bool,
    pub shift: bool,
    pub alt: bool,
}

impl KeyPress {
    /// modifiers: [CTRL, SHIFT, ALT]
    pub fn new(key: Key, modifiers: [bool; 3]) -> Self {
        Self {
            key,
            control: modifiers[0],
            shift: modifiers[1],
            alt: modifiers[2],
        }
    }

    pub fn basic(key: Key) -> Self {
        Self {
            key,
            shift: false,
            control: false,
            alt: false,
        }
    }
}

impl Default for KeyBinds {
    fn default() -> Self {
        let mut binds = HashMap::new();
        binds.insert(KeyBind::basic(Key::J), Action::PostDown);
        binds.insert(KeyBind::basic(Key::K), Action::PostUp);
        binds.insert(KeyBind::basic(Key::Enter), Action::OpenPost);

        Self { binds }
    }
}

impl From<KeyPress> for KeyBind {
    fn from(key: KeyPress) -> Self {
        Self(key)
    }
}
