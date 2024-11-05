use iced::widget::svg::Handle;
use iced::widget::{button, center, column, svg, Button};
use iced::{Border, Element, Length, Padding, Theme};

use crate::canvas::{HoverSelection, MouseInteraction, Selection};
use crate::molecule::BondType;

#[derive(Debug, Default, Clone)]
pub struct Toolbar {
    selected: Tool,
}

#[derive(Debug, Default, Clone, Copy, PartialEq)]
pub enum Tool {
    #[default] Cursor,
    Select,
    Pan,
    Erase,
    Bond(BondType),
    Rename,
    C,
}

impl Tool {
    pub fn action(&self, interaction: MouseInteraction, selection: &Selection, hover_selection: &HoverSelection) -> ToolAction {
        if matches!(interaction, MouseInteraction::MouseDragged) { return ToolAction::CursorDragged }

        match self {
            Tool::Cursor => {
                match interaction {
                    MouseInteraction::MouseDown => match hover_selection.is_empty() {
                        true => ToolAction::StartPan,
                        false => ToolAction::ClickSelect,
                    }
                    MouseInteraction::MouseTapped => ToolAction::ClickSelect,
                    _ => ToolAction::None
                }
            }
            Tool::Select => {
                match interaction {
                    // MouseInteraction::MouseTapped => ToolAction::ClickSelect,
                    MouseInteraction::MouseDown => match selection.contains(hover_selection) {
                        true => ToolAction::StartMove,
                        false => ToolAction::DragSelectStart,
                    }
                    MouseInteraction::MouseReleased => ToolAction::DragSelectFinish,
                    MouseInteraction::MouseTapped => ToolAction::ClickSelect,
                    _ => ToolAction::None
                }
            }
            Tool::Pan => {
                match interaction {
                    MouseInteraction::MouseDown => ToolAction::StartPan,
                    _ => ToolAction::None
                }
            }
            Tool::Erase => {
                match interaction {
                    MouseInteraction::MouseDown => ToolAction::Erase,
                    _ => ToolAction::None
                }
            }
            Tool::Bond(bond_type) => {
                match interaction {
                    MouseInteraction::MouseDown => ToolAction::BondStart(*bond_type),
                    MouseInteraction::MouseReleased | MouseInteraction::MouseTapped => ToolAction::BondFinish,
                    _ => ToolAction::None
                }
            }
            Tool::Rename => {
                match interaction {
                    MouseInteraction::MouseTapped => ToolAction::Rename,
                    MouseInteraction::MouseDown => ToolAction::StartPan,
                    _ => ToolAction::None,
                }
            }
            Tool::C => {
                match interaction {
                    MouseInteraction::MouseTapped => ToolAction::AtomDraw("C".to_string()),
                    MouseInteraction::MouseDown => ToolAction::StartPan,
                    _ => ToolAction::None
                }
            }
        }
    }
}

#[derive(Debug, Default, Clone)]
pub enum ToolAction {
    #[default] None,
    CursorDragged,
    ClickSelect,
    DragSelectStart,
    DragSelectFinish,
    StartPan,
    StartMove,
    Erase,
    BondStart(BondType),
    BondFinish,
    Rename,
    AtomDraw(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    ToolChanged(Tool),
}

impl Default for Message {
    fn default() -> Self {
        Message::ToolChanged(Tool::default())
    }
}


impl Toolbar {
    pub fn update(&mut self, message: Message) -> Message {
        match &message {
            Message::ToolChanged(tool) => {
                self.selected = *tool;
            }
        }

        message
    }

    fn svg_button(&self, name: &str, tool: Tool) -> Button<Message> {
        let selected = self.selected == tool;

        let svg = svg(Handle::from_path(format!(
            "{}/resources/{}.svg",
            env!("CARGO_MANIFEST_DIR"),
            name
        )))
        .style(|theme: &Theme, _status| svg::Style {
            color: Some(theme.palette().text),
        });

        let button = button(center(svg));
        
        button
            .style(move |theme: &Theme, _status| button::Style { 
                background: Some(iced::Background::Color(if selected {
                    theme.extended_palette().background.weak.color 
                } else {
                    theme.extended_palette().background.base.color
                })),
                text_color: theme.palette().text,
                border: Border {
                    ..Default::default()
                },
                shadow: iced::Shadow { 
                    ..Default::default()
                }
            })
            .padding(Padding::new(5.0))
            .width(Length::Fixed(30.0))
            .height(Length::Fixed(30.0))
            .on_press(Message::ToolChanged(tool))
}


    pub fn view(&self) -> Element<Message> {
        Into::<Element<Message>>::into(column![
                self.svg_button("cursor-pointer", Tool::Cursor),
                self.svg_button("square-dashed", Tool::Select),
                self.svg_button("drag-hand-gesture", Tool::Pan),
                self.svg_button("erase-solid", Tool::Erase),
                self.svg_button("single", Tool::Bond(BondType::Normal(1))),
                self.svg_button("double", Tool::Bond(BondType::Normal(2))),
                self.svg_button("triple", Tool::Bond(BondType::Normal(3))),
                self.svg_button("wedge", Tool::Bond(BondType::Wedge)),
                self.svg_button("dash", Tool::Bond(BondType::Dash)),
                self.svg_button("hydrogen-bond", Tool::Bond(BondType::Hydrogen)),
                self.svg_button("input-field", Tool::Rename),
                self.svg_button("letters/c", Tool::C),
            ]
            .width(Length::Fixed(30.0))
        )
    }
}
