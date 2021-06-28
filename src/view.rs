use crate::Message;
use iced::{Element, Row, Text};
use snew::{auth::Authenticator, things::Post};

/// Something that can be rendered in the UI.
trait View {
    fn view(&self) -> Element<Message>;
}

impl<'a, T: Authenticator> View for Post<'a, T> {
    fn view(&self) -> Element<Message> {
        Row::new().push(Text::new("Post!")).into()
    }
}
