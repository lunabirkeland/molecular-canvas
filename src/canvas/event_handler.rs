use super::{
    HoverSelection, Message, MolCanvas, Scaling, SingleSelection
};
use anyhow::{Context, Result};
use iced::keyboard::key::Named;
use iced::widget::canvas::event::{self, Event};
use iced::{mouse, Point, Size};
use iced::{Rectangle, Vector};

use crate::application;
use crate::molecule::{Atom, AtomId, Bond, BondType, MoleculeId};
use crate::toolbar::ToolAction;

pub fn handle_event(
    mol_canvas: &MolCanvas,
    prev_interaction: &mut MouseInteraction,
    event: Event,
    bounds: Rectangle,
    cursor: mouse::Cursor,
) -> (event::Status, Option<application::Message>) {
    if let Event::Mouse(mouse::Event::WheelScrolled { delta }) = event {
        return (
            event::Status::Captured,
            handle_scrolling(mol_canvas, bounds, cursor, delta).map(Message::into),
        );
    };

    let Some(cursor_position) = cursor.position_in(bounds) else {
        return (event::Status::Ignored, None);
    };

    let canvas_position = mol_canvas.project(cursor_position, bounds.size());
    let hover_selection = match mol_canvas.state.get_hovered(canvas_position) {
        Ok(value) => value,
        Err(error) => return (event::Status::Captured, Some(error.into()))
    };

    let tool_action = tool_action_from_event(mol_canvas, prev_interaction, event, hover_selection);

    let message = match message_from_tool_action(
        mol_canvas,
        tool_action,
        cursor_position,
        canvas_position,
        hover_selection,
    ) {
        Ok(message) => message,
        Err(error) => Some(error.into())
    };

    (event::Status::Captured, message)
}

fn handle_scrolling(
    mol_canvas: &MolCanvas,
    bounds: Rectangle,
    cursor: mouse::Cursor,
    delta: mouse::ScrollDelta,
) -> Option<Message> {
    match delta {
        mouse::ScrollDelta::Lines { y, .. } | mouse::ScrollDelta::Pixels { y, .. } => {
            if y < 0.0 && mol_canvas.scaling > MolCanvas::MIN_SCALING
                || y > 0.0 && mol_canvas.scaling < MolCanvas::MAX_SCALING
            {
                let old_scaling = *mol_canvas.scaling;

                let scaling = (mol_canvas.scaling * (1.0 + y / 30.0))
                    .clamp(*MolCanvas::MIN_SCALING, *MolCanvas::MAX_SCALING);

                let translation =
                    if let Some(cursor_to_center) = cursor.position_from(bounds.center()) {
                        let factor = scaling - old_scaling;

                        Some(
                            mol_canvas.translation
                                - Vector::new(
                                    cursor_to_center.x * factor / (old_scaling * old_scaling),
                                    cursor_to_center.y * factor / (old_scaling * old_scaling),
                                ),
                        )
                    } else {
                        None
                    };

                Some(Message::Scaled(Scaling(scaling), translation))
            } else {
                None
            }
        }
    }
}

fn get_mouse_interaction(
    prev_interaction: &mut MouseInteraction,
    mouse_event: mouse::Event,
) -> MouseInteraction {
    match mouse_event {
        mouse::Event::ButtonPressed(mouse::Button::Left) => {
            *prev_interaction = MouseInteraction::MouseDown;
            MouseInteraction::MouseDown
        }
        mouse::Event::CursorMoved { .. } => match prev_interaction {
            MouseInteraction::MouseDown | MouseInteraction::MouseDragged => {
                *prev_interaction = MouseInteraction::MouseDragged;
                MouseInteraction::MouseDragged
            }
            _ => MouseInteraction::None,
        },
        mouse::Event::ButtonReleased(mouse::Button::Left) => {
            *prev_interaction = match prev_interaction {
                MouseInteraction::MouseDown => MouseInteraction::MouseTapped,
                MouseInteraction::MouseDragged => MouseInteraction::MouseReleased,
                _ => MouseInteraction::None,
            };

            *prev_interaction
        }
        _ => MouseInteraction::None,
    }
}

fn tool_action_from_event(
    mol_canvas: &MolCanvas,
    prev_interaction: &mut MouseInteraction,
    event: Event,
    hover_selection: HoverSelection,
) -> ToolAction {
    match event {
        Event::Mouse(mouse_event) => {
            let interaction = get_mouse_interaction(prev_interaction, mouse_event);

            mol_canvas
                .tool
                .action(interaction, mol_canvas.state.selection(), &hover_selection)
        }
        Event::Keyboard(iced::keyboard::Event::KeyPressed { key, .. }) => match key {
            iced::keyboard::Key::Named(Named::Enter) => ToolAction::Rename,
            iced::keyboard::Key::Named(Named::Delete) => ToolAction::Erase,
            _ => ToolAction::None,
        },
        _ => ToolAction::None,
    }
}

fn cursor_dragged(
    mol_canvas: &MolCanvas,
    cursor_position: Point,
    canvas_position: Point,
    _hover_selection: HoverSelection,
) -> Result<Vec<Message>> {
    Ok(match mol_canvas.action {
        Action::Panning { translation, start } => {
            vec![
                Message::Translated(
                    translation + (cursor_position - start) * (1.0 / *mol_canvas.scaling),
                )
            ]
        }
        Action::MovingSelection { last: _ } => {
            vec![
                Message::MoveSelection(canvas_position)
            ]
        }
        Action::DrawingSelection { start } => {
            let rect = Rectangle::new(
                Point::new(f32::min(start.x, canvas_position.x), f32::min(start.y, canvas_position.y)),
                Size::new(f32::abs(start.x - canvas_position.x), f32::abs(start.y - canvas_position.y)));

            vec![Message::NewSelection(mol_canvas.state.get_selection(rect)?)]
        }
        Action::Erasing | Action::DrawingBond { .. } | Action::None => vec![]
    })
}

fn message_from_tool_action(
    mol_canvas: &MolCanvas,
    tool_action: ToolAction,
    cursor_position: Point,
    canvas_position: Point,
    hover_selection: HoverSelection,
) -> Result<Option<application::Message>> {
    let mut messages: Vec<Message> = vec![];

    match tool_action {
        ToolAction::None => {
            messages.push(Message::ActionChanged(Action::None))
        }
        ToolAction::CursorDragged => {
            let message = cursor_dragged(mol_canvas, cursor_position, canvas_position, hover_selection)?;
            messages.extend(message);
        },
        ToolAction::ClickSelect => {
            messages.push(Message::ActionChanged(Action::MovingSelection {
                last: canvas_position
            }));

            if !mol_canvas.state.selection().contains(&hover_selection) {
                messages.push(Message::NewSelection(hover_selection.into()));
            }
        }
        ToolAction::DragSelectStart => {
            messages.push(Message::ActionChanged(Action::DrawingSelection {
                start: canvas_position,
            }))
        }
        ToolAction::DragSelectFinish => {
            if let Action::DrawingSelection { .. } = mol_canvas.action {
                messages.push(Message::ActionChanged(Action::None));
            }
        }
        ToolAction::StartPan => {
            messages.push(Message::ActionChanged(Action::Panning {
                translation: mol_canvas.translation,
                start: cursor_position,
            }));
        }
        ToolAction::StartMove => {
            messages.push(Message::ActionChanged(Action::MovingSelection {
                last: canvas_position
            }));
        }
        ToolAction::Erase => {
            messages.push(Message::ActionChanged(Action::Erasing));
            match hover_selection.selection() {
                Some(SingleSelection::Atom(molecule_id, atom_id)) => {
                    messages.push(Message::DeleteAtom(molecule_id, atom_id))
                }
                Some(SingleSelection::Molecule(molecule_id)) => {
                    messages.push(Message::DeleteMolecule(molecule_id))
                }
                Some(SingleSelection::Bond(molecule_id, bond_id)) => {
                    messages.push(Message::DeleteBond(molecule_id, bond_id))
                }
                None => (),
            }
        }
        ToolAction::BondStart(bond_type) => match hover_selection.selection() {
            Some(SingleSelection::Atom(molecule_id, atom_id)) => {
                let atom_position = mol_canvas
                    .state
                    .get_molecule(&molecule_id).context("while getting message from BondStart tool action")?
                    .atom_position(&atom_id).context("while getting message from BondStart tool action")?;

                messages.push(Message::ActionChanged(Action::DrawingBond {
                    molecule_id,
                    atom_id,
                    start: atom_position,
                    bond_type,
                }));
            }
            Some(SingleSelection::Bond(molecule_id, bond_id)) => {
                let bond = mol_canvas.state.get_bond(&molecule_id, &bond_id)?;

                match bond.bond_type() {
                    BondType::Normal(1) if bond_type == BondType::Normal(1) => messages.push(Message::ChangeBondType(molecule_id, bond_id, BondType::Normal(2))),
                    BondType::Wedge | BondType::Dash if bond_type == bond.bond_type() => messages.push(Message::FlipBond(molecule_id, bond_id)), 
                    _ => messages.push(Message::ChangeBondType(molecule_id, bond_id, bond_type))
                }
            }
            None | Some(SingleSelection::Molecule(_)) => {
                let molecule_id = MoleculeId::new();
                let atom_id = AtomId::new();

                messages.push(Message::ActionChanged(Action::DrawingBond {
                    molecule_id,
                    atom_id,
                    start: canvas_position,
                    bond_type,
                }));
                messages.push(Message::AddMoleculeWithAtom(molecule_id, atom_id, "".to_string(), canvas_position));
            }
        },
        ToolAction::BondFinish => {
            if let Action::DrawingBond {
                molecule_id,
                atom_id,
                start,
                bond_type,
            } = mol_canvas.action {
                match hover_selection.selection() {
                    Some(SingleSelection::Atom(hov_molecule_id, hov_atom_id))
                        if hov_atom_id != atom_id =>
                        {
                            if hov_molecule_id == molecule_id {
                                messages.push(Message::NewBond(molecule_id, atom_id, hov_atom_id, bond_type));
                            } else {
                                messages.push(
                                    Message::ConnectMolecules(
                                        molecule_id,
                                        atom_id,
                                        hov_molecule_id,
                                        hov_atom_id,
                                        bond_type,
                                    ),
                                );
                            }
                        }
                    _ => {
                        let end = Bond::fixed_length(
                            start,
                            canvas_position - start,
                            MolCanvas::BOND_LENGTH,
                        );

                        messages.push(Message::FinishBond(molecule_id, atom_id, end, bond_type))
                    }
                }
            }
        }
        ToolAction::Rename => match hover_selection.selection() {
            Some(SingleSelection::Atom(hov_molecule_id, hov_atom_id)) => {
                let label = mol_canvas
                    .state
                    .get_atom(&hov_molecule_id, &hov_atom_id)
                    .map(Atom::label);

                return Ok(Some(application::Message::TextInputSpawn(
                        label.unwrap_or_default(),
                        hov_molecule_id,
                        hov_atom_id,
                        Message::RelabelAtom,
                )))
            }
            _ => return Ok(Some(application::Message::TextInputSubmit)),
        },
        ToolAction::AtomDraw(label) => match hover_selection.selection() {
            Some(SingleSelection::Atom(hov_molecule_id, hov_atom_id)) => {
                messages.push(Message::RelabelAtom(hov_molecule_id, hov_atom_id, label));
            }
            _ => {
                messages.push(Message::AddMoleculeWithAtom(MoleculeId::new(), AtomId::new(), label, canvas_position));
            }
        },
    }

    Ok(Some(messages.into()))
}

#[derive(Debug, Default, Clone, Copy)]
pub enum MouseInteraction {
    #[default] None,
    MouseDown,
    MouseDragged,
    MouseReleased,
    MouseTapped,
}

#[derive(Debug, Default, Clone)]
pub enum Action {
    #[default] None,
    Panning {
        translation: Vector,
        start: Point,
    },
    MovingSelection {
        last: Point,
    },
    DrawingSelection {
        start: Point,
    },
    Erasing,
    DrawingBond {
        molecule_id: MoleculeId,
        atom_id: AtomId,
        start: Point,
        bond_type: BondType,
    },
}
