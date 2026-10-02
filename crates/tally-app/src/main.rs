use iced::widget::{column, text, text_input};
use iced::{Element, Size};
use tally_core::Activity;

fn main() -> iced::Result {
    iced::application(Tally::default, Tally::update, Tally::view)
        .title("Tally")
        .window_size(Size::new(400.0, 160.0))
        .run()
}

#[derive(Default)]
struct Tally {
    input: String,
    last: Option<Activity>,
}

#[derive(Debug, Clone)]
enum Message {
    InputChanged(String),
    Submitted,
}

impl Tally {
    fn update(&mut self, message: Message) {
        match message {
            Message::InputChanged(input) => self.input = input,
            Message::Submitted => {
                let name = self.input.trim();
                if !name.is_empty() {
                    self.last = Some(Activity::new(name));
                    self.input.clear();
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let last = match &self.last {
            Some(activity) => format!("Last: {}", activity.name),
            None => String::from("Nothing logged yet"),
        };

        column![
            text("What are you doing?").size(20),
            text_input("e.g. Writing docs", &self.input)
                .on_input(Message::InputChanged)
                .on_submit(Message::Submitted),
            text(last),
        ]
        .spacing(10)
        .padding(20)
        .into()
    }
}
