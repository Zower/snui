use std::collections::HashMap;

use iced::keyboard::KeyCode;

use crate::Action;

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

/// Wrapper around a keypress, showing that it is persistant and indicates some action to be taken.
#[derive(Debug, Hash, Eq, PartialEq)]
pub struct KeyBind(KeyPress);

impl KeyBind {
    fn basic(key: KeyCode) -> Self {
        Self(KeyPress::basic(key))
    }
}

/// Press of a key along with potential modifiers.
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct KeyPress {
    pub key: KeyCode,
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
}

impl KeyPress {
    fn basic(key: KeyCode) -> Self {
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
        binds.insert(KeyBind::basic(KeyCode::J), Action::PostUp);
        binds.insert(KeyBind::basic(KeyCode::K), Action::PostDown);
        binds.insert(KeyBind::basic(KeyCode::Enter), Action::OpenPost);

        Self { binds }
    }
}

impl From<(iced::keyboard::KeyCode, iced::keyboard::Modifiers)> for KeyPress {
    fn from(combos: (iced::keyboard::KeyCode, iced::keyboard::Modifiers)) -> Self {
        let (key, modifiers) = combos;
        Self {
            key,
            shift: modifiers.shift,
            control: modifiers.control,
            alt: modifiers.alt,
        }
    }
}

impl From<(iced::keyboard::KeyCode, iced::keyboard::Modifiers)> for KeyBind {
    fn from(combos: (iced::keyboard::KeyCode, iced::keyboard::Modifiers)) -> Self {
        Self(KeyPress::from(combos))
    }
}

impl From<KeyPress> for KeyBind {
    fn from(key: KeyPress) -> Self {
        Self(key)
    }
}
