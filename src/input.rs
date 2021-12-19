use crate::Action;
use std::collections::HashMap;

use eframe::egui;

/// A map from keys to actions.
#[derive(Debug)]
pub struct KeyBinds {
    binds: HashMap<KeyBind, Action>,
}

impl KeyBinds {
    pub fn action(&self, key: KeyPress) -> Option<Action> {
        self.binds.get(&KeyBind::from(key)).map(|value| *value)
    }
}

/// A keypress that has an action tied to it.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct KeyBind(KeyPress);

impl KeyBind {
    fn basic(key: egui::Key) -> Self {
        Self(KeyPress::basic(key))
    }
}

/// Press of a key along with potential modifiers.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct KeyPress {
    pub key: egui::Key,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

impl KeyPress {
    pub fn basic(key: egui::Key) -> Self {
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
        binds.insert(KeyBind::basic(egui::Key::J), Action::PostUp);
        binds.insert(KeyBind::basic(egui::Key::K), Action::PostDown);
        binds.insert(KeyBind::basic(egui::Key::Enter), Action::OpenPost);

        Self { binds }
    }
}

impl From<KeyPress> for KeyBind {
    fn from(key: KeyPress) -> Self {
        Self(key)
    }
}
