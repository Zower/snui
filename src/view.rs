use crate::Message;
use iced::{Align, Column, Element, HorizontalAlignment, Row, Text, VerticalAlignment};
use snew::things::Post;

/// Something that can be rendered in the UI.
pub trait View {
    fn view(&self) -> Element<Message>;
}

impl View for Post {
    fn view(&self) -> Element<Message> {
        Row::new()
            .push(
                Column::new()
                    .push(
                        Text::new(self.ups.to_string())
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .vertical_alignment(VerticalAlignment::Center)
                            .size(12),
                    )
                    .push(
                        Text::new(self.downs.to_string())
                            .horizontal_alignment(HorizontalAlignment::Center)
                            .vertical_alignment(VerticalAlignment::Center)
                            .size(12),
                    ),
            )
            .push(Text::new(&self.title).size(20))
            .push(Text::new(&self.author).size(15))
            .spacing(15)
            .align_items(Align::Center)
            .into()
    }
}
