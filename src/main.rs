mod keyboard;
mod view;

use keyboard::{KeyBinds, KeyPress};

use iced::{
    button, executor, text_input, window, Align, Application, Button, Clipboard, Column, Command,
    Container, Element, Length, Settings, Subscription, Text, TextInput,
};

use snew::{
    auth::{Authenticator, Credentials, ScriptAuthenticator},
    reddit::{self, Reddit},
    things::Post,
};

fn main() -> iced::Result {
    let settings = Settings {
        window: window::Settings {
            size: (800, 500),
            ..Default::default()
        },
        antialiasing: true,
        ..Default::default()
    };

    RedditUI::<ScriptAuthenticator>::run(settings)
}

#[derive(Debug)]
struct RedditUI<'a, T: Authenticator> {
    /// The reddit client to make requests with
    reddit_client: Option<Reddit<T>>,
    /// Posts that are fetched and can be displayed
    posts: Vec<Post<'a, T>>,
    /// Keybinds that can perform som [`Action`]
    keybinds: KeyBinds,
    /// Current state of the application
    state: UIState<'a, T>,
}

#[derive(Debug)]
enum UIState<'a, T: Authenticator> {
    Unathenticated {
        client_id_input_box: InputBox,
        client_secret_input_box: InputBox,
        username_input_box: InputBox,
        password_input_box: InputBox,
        submit_button: button::State,
        errored: Option<String>,
    },
    LoggingIn,
    ViewingPostFeed {
        highlighted: usize,
    },
    ViewingPost(&'a Post<'a, T>),
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
    LoggedIn(Result<Reddit<ScriptAuthenticator>, Error>),
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

impl<'a> Application for RedditUI<'a, ScriptAuthenticator> {
    type Executor = executor::Default;
    type Message = Message;
    type Flags = ();

    fn new(_: ()) -> (Self, Command<Message>) {
        (
            Self {
                reddit_client: None,
                keybinds: KeyBinds::default(),
                posts: Vec::new(),
                state: UIState::Unathenticated {
                    client_id_input_box: InputBox::default(),
                    client_secret_input_box: InputBox::default(),
                    username_input_box: InputBox::default(),
                    password_input_box: InputBox::default(),
                    submit_button: button::State::default(),
                    errored: None,
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
                    println!("Should've taken action: {:?}", action);
                }
            }
            Message::InputSubmitted => match &mut self.state {
                UIState::Unathenticated {
                    client_id_input_box: ci_box,
                    client_secret_input_box: cs_box,
                    username_input_box: u_box,
                    password_input_box: p_box,
                    submit_button: _,
                    errored: _,
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
                    errored: _,
                } => match input {
                    Input::Username(value) => u_box.value = value,
                    Input::Password(value) => p_box.value = value,
                    Input::ClientID(value) => ci_box.value = value,
                    Input::ClientSecret(value) => cs_box.value = value,
                },
                _ => (),
            },
            Message::LoggedIn(result) => {
                let client = result.unwrap();
                println!("{:?}", client.me());
                self.reddit_client = Some(client);
                self.state = UIState::ViewingPostFeed { highlighted: 0 }
            }
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
                errored,
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
                if let Some(error_str) = &errored {
                    column = column.push(Text::new(error_str));
                }
                column
            }
            UIState::LoggingIn => Column::new().push(Text::new("Logging in...")),
            UIState::ViewingPostFeed { highlighted } => Column::new().push(Text::new(format!(
                "Viewing posts, highlighted: {}",
                highlighted
            ))),
            _ => Column::new().push(Text::new("Empty")),
        };

        Container::new(content).center_x().center_y().into()
    }
}

async fn login(creds: Credentials) -> Result<Reddit<ScriptAuthenticator>, Error> {
    let user_agent = format!("windows:snui:v0.1.0 (by /u/{})", &creds.username);

    let script_auth = ScriptAuthenticator::new(creds);

    Ok(Reddit::new(script_auth, &user_agent)?)
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
