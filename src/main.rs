#![windows_subsystem = "windows"]

mod keyboard;
mod view;

use keyboard::{KeyBinds, KeyPress};

use iced::{
    button, executor, scrollable, text_input, window, Align, Application, Button, Clipboard,
    Column, Command, Container, Element, Length, Scrollable, Settings, Subscription, Text,
    TextInput,
};

use snew::{
    auth::{Credentials, ScriptAuthenticator},
    reddit::{self, Reddit},
    things::Post,
};

use view::View;

use std::sync::Arc;

fn main() -> iced::Result {
    let settings = Settings {
        window: window::Settings {
            size: (800, 500),
            ..Default::default()
        },
        antialiasing: true,
        ..Default::default()
    };

    RedditUI::run(settings)
}

#[derive(Debug)]
struct RedditUI {
    /// The reddit client to make requests with
    reddit_client: Option<Arc<Reddit>>,
    /// Keybinds that can perform som [`Action`]
    keybinds: KeyBinds,
    /// Current state of the application
    state: UIState,
}

#[derive(Debug)]
enum UIState {
    Unathenticated {
        client_id_input_box: InputBox,
        client_secret_input_box: InputBox,
        username_input_box: InputBox,
        password_input_box: InputBox,
        submit_button: button::State,
        error_message: Option<Error>,
    },
    LoggingIn,
    ViewingPostFeed {
        /// Posts that are fetched and can be displayed
        posts: Vec<Post>,
        /// Currently highlighted post, that is it is marked in the UI and pressing the keybind to open a post will open this post.
        /// References an index in [`Self::ViewingPostFeed::posts`]
        highlighted: usize,
        /// State of the scrollbar
        scrollbar: scrollable::State,
        /// Currently opened post
        opened: Option<Post>,
    },
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
    /// The user logged in succesfully
    LoggedIn(Result<Arc<Reddit>, Error>),
}

#[derive(Debug, Default)]
struct InputBox {
    state: text_input::State,
    value: String,
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

impl<'a> Application for RedditUI {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_: ()) -> (Self, Command<Message>) {
        (
            Self {
                reddit_client: None,
                keybinds: KeyBinds::default(),
                state: UIState::Unathenticated {
                    client_id_input_box: InputBox::default(),
                    client_secret_input_box: InputBox::default(),
                    username_input_box: InputBox::default(),
                    password_input_box: InputBox::default(),
                    submit_button: button::State::default(),
                    error_message: None,
                },
            },
            Command::none(),
        )
    }

    fn title(&self) -> String {
        String::from("Snui")
    }

    fn update(&mut self, message: Message, _: &mut Clipboard) -> Command<Message> {
        match message {
            Message::KeyPressed(press) => {
                if let Some(action) = self.keybinds.action(press) {
                    match &mut self.state {
                        UIState::ViewingPostFeed {
                            highlighted,
                            scrollbar: _,
                            posts: _,
                            opened: _,
                        } => match action {
                            Action::PostUp => *highlighted -= 1,
                            Action::PostDown => *highlighted += 1,
                            Action::OpenPost => (),
                        },
                        _ => (),
                    }
                }
            }
            Message::InputSubmitted => match &mut self.state {
                UIState::Unathenticated {
                    client_id_input_box: ci_box,
                    client_secret_input_box: cs_box,
                    username_input_box: u_box,
                    password_input_box: p_box,
                    submit_button: _,
                    error_message: _,
                } => {
                    let command = Command::perform(
                        login(Credentials::new(
                            &ci_box.value,
                            &cs_box.value,
                            &u_box.value,
                            &p_box.value,
                        )),
                        Message::LoggedIn,
                    );
                    self.state = UIState::LoggingIn;
                    return command;
                }
                _ => (),
            },
            Message::InputChanged(input) => match &mut self.state {
                UIState::Unathenticated {
                    client_id_input_box: ci_box,
                    client_secret_input_box: cs_box,
                    username_input_box: u_box,
                    password_input_box: p_box,
                    submit_button: _,
                    error_message: _,
                } => match input {
                    Input::Username(value) => u_box.value = value,
                    Input::Password(value) => p_box.value = value,
                    Input::ClientID(value) => ci_box.value = value,
                    Input::ClientSecret(value) => cs_box.value = value,
                },
                _ => (),
            },
            Message::LoggedIn(result) => match result {
                Ok(reddit) => {
                    let mut rust = reddit.subreddit("rust").hot();

                    rust.limit = 50;
                    let posts = rust.take(50).filter_map(|p| p.ok()).collect();

                    self.reddit_client = Some(reddit);

                    self.state = UIState::ViewingPostFeed {
                        posts,
                        highlighted: 0,
                        opened: None,
                        scrollbar: scrollable::State::new(),
                    };
                }
                Err(err) => {
                    self.state = UIState::Unathenticated {
                        client_id_input_box: InputBox::default(),
                        client_secret_input_box: InputBox::default(),
                        username_input_box: InputBox::default(),
                        password_input_box: InputBox::default(),
                        submit_button: button::State::default(),
                        error_message: Some(err),
                    }
                }
            },
        }
        Command::none()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced_native::subscription::events_with(|event, _| match event {
            iced_native::event::Event::Keyboard(iced::keyboard::Event::KeyPressed {
                key_code,
                modifiers,
            }) => Some(Message::KeyPressed(KeyPress::from((key_code, modifiers)))),
            _ => None,
        })
    }

    fn view(&mut self) -> Element<Self::Message> {
        let content = match &mut self.state {
            UIState::Unathenticated {
                client_id_input_box,
                client_secret_input_box,
                username_input_box,
                password_input_box,
                submit_button,
                error_message,
            } => {
                let mut column = Column::new()
                    .padding(20)
                    .spacing(10)
                    .align_items(Align::Center)
                    .push(
                        TextInput::new(
                            &mut username_input_box.state,
                            "Username",
                            &username_input_box.value,
                            |input| Message::InputChanged(Input::Username(input)),
                        )
                        .width(Length::Units(200))
                        .padding(10)
                        .size(20)
                        .on_submit(Message::InputSubmitted),
                    )
                    .push(
                        TextInput::new(
                            &mut password_input_box.state,
                            "Password",
                            &password_input_box.value,
                            |input| Message::InputChanged(Input::Password(input)),
                        )
                        .width(Length::Units(200))
                        .padding(10)
                        .size(20)
                        .password()
                        .on_submit(Message::InputSubmitted),
                    )
                    .push(
                        TextInput::new(
                            &mut client_id_input_box.state,
                            "Client ID",
                            &client_id_input_box.value,
                            |input| Message::InputChanged(Input::ClientID(input)),
                        )
                        .width(Length::Units(200))
                        .padding(10)
                        .size(20)
                        .password()
                        .on_submit(Message::InputSubmitted),
                    )
                    .push(
                        TextInput::new(
                            &mut client_secret_input_box.state,
                            "Client Secret",
                            &client_secret_input_box.value,
                            |input| Message::InputChanged(Input::ClientSecret(input)),
                        )
                        .width(Length::Units(200))
                        .padding(10)
                        .size(20)
                        .password()
                        .on_submit(Message::InputSubmitted),
                    )
                    .push(
                        Button::new(submit_button, Text::new("Submit"))
                            .on_press(Message::InputSubmitted),
                    );
                if let Some(error) = &error_message {
                    let error_str = match error {
                        Error::AuthenticationError(s) => {
                            format!("Failed to authenticate, try again. Reason:\n{}", s)
                        }
                        Error::RequestError(s) => {
                            format!("An error occured sending the HTTPS request. Reason:\n{}", s)
                        }
                        Error::Other(s) => format!("An error occured:\n{}", s),
                    };
                    column = column.push(Text::new(error_str));
                }
                column
            }
            UIState::LoggingIn => Column::new().push(Text::new("Logging in...")),
            UIState::ViewingPostFeed {
                posts,
                highlighted: _,
                opened: _,
                scrollbar,
            } => {
                let mut main_view = Scrollable::new(scrollbar)
                    .spacing(10)
                    // .width(Length::Fill)
                    .align_items(Align::Start);

                for post in posts.iter() {
                    main_view = main_view.push(post.view());
                }

                Column::new().push(main_view).width(Length::Fill)
            }
        };

        Container::new(content)
            .width(Length::Fill)
            .center_x()
            .center_y()
            .into()
    }
}

async fn login(creds: Credentials) -> Result<Arc<Reddit>, Error> {
    let user_agent = format!("windows:snui:v0.1.0 (by /u/{})", &creds.username);

    let script_auth = ScriptAuthenticator::new(creds);

    Ok(Arc::new(Reddit::new(script_auth, &user_agent)?))
}

#[derive(Debug, Clone)]
pub enum Error {
    AuthenticationError(String),
    RequestError(String),
    Other(String),
}

impl From<reddit::Error> for Error {
    fn from(error: reddit::Error) -> Self {
        match error {
            reddit::Error::AuthenticationError(err) => Self::AuthenticationError(err.to_string()),
            reddit::Error::RequestError(err) => Self::RequestError(err.to_string()),
            // Implement rest of errors
            _ => panic!("Other error received"),
        }
    }
}
