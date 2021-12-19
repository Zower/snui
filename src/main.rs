mod impl_render;
mod input;

use input::{KeyBinds, KeyPress};

use snew::{
    auth::{
        authenticator::{AnonymousAuthenticator, Authenticator, ScriptAuthenticator},
        Credentials,
    },
    reddit::{self, Reddit},
    things::Post,
};

use eframe::{
    egui::{self, menu, widgets, Align, CentralPanel, Layout, SidePanel, Slider, TopBottomPanel},
    epi,
};

#[derive(Debug)]
struct SnuiApp {
    /// The reddit client to make requests with
    client: Reddit,
    /// Posts that are fetched and can be displayed
    posts: Vec<Post>,
    /// Currently highlighted post in left pane
    highlighted: usize,
    /// Content currently in the center pane
    content: Box<dyn Render>,
    /// Keybinds that can perform som [`Action`]
    keybinds: KeyBinds,
    /// Current layout of the application
    layout: SnuiLayout,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Reddit::new(
        AnonymousAuthenticator::new(),
        "windows:snui:v0.1.0 (by /u/zower98",
    )?;

    let posts: Vec<Post> = client
        .subreddit("rust")
        .hot()
        .filter_map(|p| p.ok())
        .take(50)
        .collect();

    let app = SnuiApp {
        client,
        highlighted: 0,
        content: Box::new(posts[0].clone()),
        posts,
        keybinds: KeyBinds::default(),
        layout: SnuiLayout::HorizontalSplit,
    };
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

    /// Called each time the UI needs repainting, which may be many times per second.
    /// Put your widgets into a `SidePanel`, `TopPanel`, `CentralPanel`, `Window` or `Area`.
    fn update(&mut self, ctx: &egui::CtxRef, frame: &mut epi::Frame<'_>) {
        for event in &ctx.input().events {
            let action = match event {
                egui::Event::Key {
                    key,
                    pressed,
                    modifiers: _,
                } if (!pressed) => self.keybinds.action(KeyPress::basic(*key)),
                _ => None,
            };

            if let Some(action) = action {
                match action {
                    Action::PostUp => {
                        self.highlighted = self.highlighted.checked_add(1).unwrap_or(usize::MAX)
                    }
                    Action::PostDown => {
                        self.highlighted = self.highlighted.checked_sub(1).unwrap_or(0)
                    }
                    Action::OpenPost => {
                        self.content = Box::new(self.posts[self.highlighted].clone())
                    }
                }
            };
        }

        TopBottomPanel::top("top_panel").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                menu::menu(ui, "App", |ui| {
                    if ui.button("Quit").clicked() {
                        frame.quit();
                    }
                });
            });
        });

        SidePanel::left("side_panel")
            .default_width(500f32)
            .max_width(800f32)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for post in &self.posts {
                        post.render_summary(ui);
                    }
                });
            });

        CentralPanel::default().show(ctx, |ui| {
            self.content.render(ui);
        });
    }
}

pub(crate) trait Render: std::fmt::Debug {
    fn render_summary(&self, ui: &mut egui::Ui) {}
    fn render(&self, ui: &mut egui::Ui);
}

#[derive(Debug)]
pub(crate) enum SnuiLayout {
    /// Two or three panes showing posts | current post or comments | optional third pane for comments exclusively
    HorizontalSplit,
}

/// Messages that may be sent when the user performs certain actions.
#[derive(Debug, Clone)]
pub enum Message {
    /// User submitted some input. This is agnostic and each state may interpret this differently.
    InputSubmitted,
    /// User typed something into a textbox, but have not submitted yet.
    InputChanged(Input),
    /// A key was pressed
    KeyPressed(KeyPress),
}

#[derive(Debug, Clone)]
pub enum Input {
    Username(String),
    Password(String),
    ClientID(String),
    ClientSecret(String),
}

///
#[derive(Debug, Clone, Copy)]
pub enum Action {
    /// Currently marked post should be one higher
    PostUp,
    /// Currently marked post should be one lower
    PostDown,
    /// Open the currrently marked post
    OpenPost,
}

// fn login(creds: Credentials) -> Result<Reddit, Error> {
//     let user_agent = format!(")", &creds.username);

//     let script_auth = ScriptAuthenticator::new(creds);

//     Ok(Reddit::new(script_auth, &user_agent)?)
// }

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
