mod application;
mod canvas;
mod molecule;
mod toolbar;
mod bounds;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();

    application::main()
}
