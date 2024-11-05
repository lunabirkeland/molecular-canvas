use anyhow::{Context, Result};
use iced::widget::text_input::Id;
use iced::widget::{
    container, row, text_input, Stack
};
use iced::{Element, Subscription, Task, Theme};

use crate::molecule::{AtomId, MoleculeId};
use crate::{canvas, toolbar};

pub fn main() -> iced::Result {
    iced::application(
        "MolCanvas",
        Application::update,
        Application::view,
        )
        .subscription(Application::subscription)
        .theme(|_| Theme::Dark)
        .antialiasing(true)
        .centered()
        .run()
}


struct Application {
    mol_canvas: canvas::MolCanvas,
    toolbar: toolbar::Toolbar,
    text_input: Option<InputHandler>,
    text_input_id: Id,
}

#[derive(Debug, Clone)]
struct InputHandler {
    placeholder: String,
    value: String,
    molecule_id: MoleculeId,
    atom_id: AtomId,
    callback: fn(MoleculeId, AtomId, String) -> canvas::Message,
}

#[derive(Debug, Clone)]
pub enum Message {
    MolCanvas(Vec<canvas::Message>),
    Toolbar(toolbar::Message),
    TextInputSpawn(String, MoleculeId, AtomId, fn(MoleculeId, AtomId, String) -> canvas::Message),
    TextInputChange(String),
    TextInputSubmit,
    Error(String)
}

impl From<canvas::Message> for Message {
    fn from(message: canvas::Message) -> Self {
        Self::MolCanvas(vec![message])
    }
}

impl From<Vec<canvas::Message>> for Message {
    fn from(messages: Vec<canvas::Message>) -> Self {
        Self::MolCanvas(messages)
    }
}

impl From<anyhow::Error> for Message {
    fn from(error: anyhow::Error) -> Self {
        Self::Error(error.to_string())
    }
}

impl Application {
    fn new() -> Self {
        Self {
            mol_canvas: canvas::MolCanvas::default(),
            toolbar: toolbar::Toolbar::default(),
            text_input: None,
            text_input_id: Id::unique(),
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        fn handle_message(application: &mut Application, message: Message) -> Result<Task<Message>> {
            match message {
                Message::MolCanvas(message) => {
                    application.mol_canvas.update(message).context("while handling application message MolCanvas")?;
                }
                Message::Toolbar(message) => {
                    application.toolbar.update(message.clone());
                    let toolbar::Message::ToolChanged(tool) = &message;

                    application.mol_canvas.update(vec![canvas::Message::ToolChanged(*tool)]).context("while handling application message Toolbar")?;
                }
                Message::TextInputSpawn(value, molecule_id, atom_id, callback) => {
                    // if let Some(InputHandler { value, molecule_id, atom_id, callback, .. }) = &application.text_input {
                    //     application.mol_canvas.update(callback(*molecule_id, *atom_id, value.to_string()));
                    // };

                    application.text_input = Some(InputHandler { placeholder: "label: ".to_string(), value, molecule_id, atom_id, callback });
                    return Ok(text_input::focus(application.text_input_id.clone()));
                }
                Message::TextInputChange(text) => {
                    if let Some(InputHandler { value, molecule_id, atom_id, callback, .. }) = application.text_input.as_mut() {
                        *value = text;
                        application.mol_canvas.update(vec![callback(*molecule_id, *atom_id, value.to_string())])
                            .context("while handling application message TextInputChange")?;
                    };
                }
                Message::TextInputSubmit => {
                    // if let Some(InputHandler { value, molecule_id, atom_id, callback, .. }) = &application.text_input {
                    //     // application.mol_canvas.update(callback(*molecule_id, *atom_id, value.to_string()));
                    //     application.text_input = None;
                    // };
                    application.text_input = None;
                }
                Message::Error(error) => {
                    panic!("{}", error)
                }
            }

            Ok(Task::none())
        }

        match handle_message(self, message) {
            Ok(task) => task,
            Err(error) => Task::done(error.into())
        }
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::none()
    }

    fn view(&self) -> Element<Message> {
        let canvas = match &self.text_input {
            Some(InputHandler { placeholder, value, .. }) => {
                let text_input = text_input(placeholder, value)
                    .on_input(Message::TextInputChange)
                    .on_submit(Message::TextInputSubmit)
                    .id(self.text_input_id.clone());

                Stack::with_children(vec!(
                        self.mol_canvas.view(),
                        text_input.into()
                )).into()
            }
            None => {
                self.mol_canvas.view()
            }
        };

        let toolbar = self.toolbar.view().map(Message::Toolbar);

        let content = row![toolbar, canvas];

        container(content).padding(5).into()
    }
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}


