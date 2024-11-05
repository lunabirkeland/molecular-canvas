use anyhow::{Context, Result};
use derive_more::derive::{Add, AddAssign, Deref, Mul, MulAssign};
use iced::mouse;
use iced::widget::canvas;
use iced::widget::canvas::event::{self, Event};
use iced::widget::canvas::Stroke;
use iced::widget::canvas::Style;
use iced::widget::canvas::{Cache, Canvas, Frame, Geometry, Path};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Size, Theme, Vector};

mod event_handler;
mod selection;
mod state;

use crate::application;
use crate::bounds::Bounds;
use crate::molecule::{AtomId, AtomPosition, Bond, BondId, BondType, Molecule, MoleculeId};
use crate::toolbar::Tool;
use event_handler::handle_event;
pub use event_handler::{Action, MouseInteraction};
pub use selection::{HoverSelection, Selection, SingleSelection};
use state::State;

#[derive(Default, Debug)]
pub struct MolCanvas {
    state: State,
    cache: Cache,
    tool: Tool,
    action: Action,
    translation: Vector,
    scaling: Scaling,
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Add, AddAssign, Mul, MulAssign, Deref)]
pub struct Scaling(f32);

impl Default for Scaling {
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Debug, Clone)]
pub enum Message {
    AddMoleculeWithAtom(MoleculeId, AtomId, String, Point),
    AddAtom(MoleculeId, AtomId, String, Point),
    FinishBond(MoleculeId, AtomId, Point, BondType),
    NewBond(MoleculeId, AtomId, AtomId, BondType),
    ChangeBondType(MoleculeId, BondId, BondType),
    FlipBond(MoleculeId, BondId),
    ConnectMolecules(MoleculeId, AtomId, MoleculeId, AtomId, BondType),
    RelabelAtom(MoleculeId, AtomId, String),
    DeleteMolecule(MoleculeId),
    DeleteAtom(MoleculeId, AtomId),
    DeleteBond(MoleculeId, BondId),
    MoveSelection(Point),
    NewSelection(Selection),
    // MoveMolecule(MoleculeId, Point),
    // MoveAtom(MoleculeId, AtomId, Point),
    // MoveBond(MoleculeId, BondId, Point),
    ToolChanged(Tool),
    ActionChanged(Action),
    Translated(Vector),
    Scaled(Scaling, Option<Vector>),
}

impl MolCanvas {
    const MIN_SCALING: Scaling = Scaling(0.1);
    const MAX_SCALING: Scaling = Scaling(5.0);

    pub const MOLECULE_PADDING: f32 = 3.0;
    pub const ATOM_PADDING: f32 = 3.0;
    pub const BOND_PADDING: f32 = 3.0;

    pub const BOND_LENGTH: f32 = 30.0;
    pub const BOND_WIDTH: f32 = 1.0;
    pub const BOND_OFFSETS: f32 = 2.0;
    pub const WEDGE_START_WIDTH: f32 = 1.0;
    pub const WEDGE_END_WIDTH: f32 = 4.0;
    pub const DASH_START_WIDTH: f32 = 1.0;
    pub const DASH_END_WIDTH: f32 = 4.0;
    pub const DASH_BOND_OFFSETS: f32 = 4.0;
    pub const H_BOND_WIDTH: f32 = 3.0;
    pub const H_BOND_OFFSETS: f32 = 4.0;

    pub fn update(&mut self, messages: Vec<Message>) -> Result<()> {
        for message in messages {
            match message {
                Message::AddMoleculeWithAtom(molecule_id, atom_id, label, position) => {
                    self.state
                        .add_molecule_with_atom(molecule_id, atom_id, label, position)?;

                    self.cache.clear();
                }
                Message::AddAtom(molecule_id, atom_id, label, position) => {
                    let molecule = self
                        .state
                        .get_molecule_mut(&molecule_id)
                        .context("while handling AddAtom message")?;
                    molecule.add_atom(atom_id, label, position)?;

                    self.cache.clear();
                }
                Message::FinishBond(molecule_id, start_atom_id, position, bond_type) => {
                    let molecule = self
                        .state
                        .get_molecule_mut(&molecule_id)
                        .context("while handling FinishBond message")?;
                    let end_atom_id = AtomId::new();

                    molecule.add_atom(end_atom_id, "".to_string(), position)?;
                    molecule.add_bond(start_atom_id, end_atom_id, bond_type)?;

                    self.cache.clear();
                }
                Message::NewBond(molecule_id, start_atom_id, end_atom_id, bond_type) => {
                    let molecule = self
                        .state
                        .get_molecule_mut(&molecule_id)
                        .context("while handling ChangeBondType message")?;
                    molecule.add_bond(start_atom_id, end_atom_id, bond_type)?;

                    self.cache.clear();
                }
                Message::ChangeBondType(molecule_id, bond_id, bond_type) => {
                    let molecule = self
                        .state
                        .get_molecule_mut(&molecule_id)
                        .context("while handling ChangeBondType message")?;
                    molecule.change_bond_type(&bond_id, bond_type);

                    self.cache.clear();
                }
                Message::FlipBond(molecule_id, bond_id) => {
                    let molecule = self
                        .state
                        .get_molecule_mut(&molecule_id)
                        .context("while handling FlipBond message")?;
                    molecule.flip_bond(&bond_id);

                    self.cache.clear();
                }
                Message::ConnectMolecules(
                    molecule_id1,
                    atom_id1,
                    molecule_id2,
                    atom_id2,
                    bond_type,
                ) => {
                    let molecule2 = self
                        .state
                        .remove_molecule(&molecule_id2)
                        .context("while getting molecule2")
                        .context("while handling ConnectMolecules message")?;
                    let molecule1 = self
                        .state
                        .get_molecule_mut(&molecule_id1)
                        .context("while getting molecule1")
                        .context("while handling ConnectMolecules message")?;

                    molecule1.extend(molecule2);

                    molecule1.add_bond(atom_id1, atom_id2, bond_type)?;

                    self.cache.clear();
                }
                Message::RelabelAtom(mol_id, atom_id, text) => {
                    let molecule = self
                        .state
                        .get_molecule_mut(&mol_id)
                        .context("while handling RelabelAtom message")?;
                    molecule.rename_atom(&atom_id, text)?;
                    self.cache.clear();
                }
                Message::DeleteMolecule(molecule_id) => {
                    self.state.remove_molecule(&molecule_id)?;

                    self.cache.clear();
                }
                Message::DeleteAtom(molecule_id, atom_id) => {
                    self.state.delete_atom(&molecule_id, atom_id)?;

                    self.cache.clear();
                }
                Message::DeleteBond(molecule_id, bond_id) => {
                    self.state.delete_bond(&molecule_id, bond_id)?;

                    self.cache.clear();
                }
                Message::MoveSelection(position) => {
                    if let Action::MovingSelection { last } = &mut self.action {
                        self.state.move_selection(position - *last)?;
                        *last = position;

                        self.cache.clear();
                    }
                }
                Message::NewSelection(selection) => {
                    self.state.new_selection(selection);
                }
                Message::ToolChanged(tool) => {
                    self.tool = tool;
                }
                Message::ActionChanged(action) => {
                    self.action = action;
                }
                Message::Translated(translation) => {
                    self.translation = translation;

                    self.cache.clear();
                }
                Message::Scaled(scaling, translation) => {
                    self.scaling = scaling;

                    if let Some(translation) = translation {
                        self.translation = translation;
                    }

                    self.cache.clear();
                }
            }
        }

        Ok(())
    }

    pub fn view(&self) -> Element<application::Message> {
        Canvas::new(self).width(Fill).height(Fill).into()
    }

    fn visible_region(&self, size: Size) -> Region {
        let width = size.width / *self.scaling;
        let height = size.height / *self.scaling;

        Region {
            rect: Rectangle {
                x: -self.translation.x - width / 2.0,
                y: -self.translation.y - height / 2.0,
                width,
                height,
            },
        }
    }

    fn project(&self, position: Point, size: Size) -> Point {
        let region = self.visible_region(size);

        Point::new(
            position.x / *self.scaling + region.rect.x,
            position.y / *self.scaling + region.rect.y,
        )
    }

    fn draw_pending_bond(
        &self,
        canvas_position: Option<Point>,
        hover_selection: HoverSelection,
        center: Vector,
        frame: &mut Frame,
        stroke: &Stroke,
        color: &Color,
    ) -> Result<()> {
        let Action::DrawingBond {
            molecule_id,
            atom_id,
            start,
            bond_type,
        } = self.action
        else {
            return Ok(());
        };
        let molecule = self
            .state
            .get_molecule(&molecule_id)
            .context("while drawing pending bond")?;
        let atom = molecule
            .get_atom(&atom_id)
            .context("while drawing pending bond")?;
        let Some(canvas_position) = canvas_position else {
            return Ok(());
        };
        let end = match hover_selection.selection() {
            Some(SingleSelection::Atom(hov_molecule_id, hov_atom_id)) if hov_atom_id != atom_id => {
                let hov_molecule = self
                    .state
                    .get_molecule(&hov_molecule_id)
                    .context("while getting hovered molecule")
                    .context("while drawing pending bond")?;
                let hov_atom = hov_molecule
                    .get_atom(&hov_atom_id)
                    .context("while getting hovered atom")
                    .context("while drawing pending bond")?;

                hov_molecule.position()
                    + hov_atom.bond_start(AtomPosition::from(hov_molecule.position(), start))
            }
            _ => Bond::fixed_length(
                molecule.position() + atom.position(),
                canvas_position - start,
                Self::BOND_LENGTH,
            ),
        };

        let bond_start =
            molecule.position() + atom.bond_start(AtomPosition::from(molecule.position(), end));

        frame.with_save(|frame| {
            frame.translate(center);
            frame.scale(*self.scaling);
            frame.translate(self.translation);

            molecule
                .draw_pending_bond(frame, bond_start, end, &bond_type, stroke, color)
                .expect("error in frame with_save")
        });

        Ok(())
    }
}

impl canvas::Program<application::Message> for MolCanvas {
    type State = MouseInteraction;

    fn update(
        &self,
        state: &mut MouseInteraction,
        event: Event,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> (event::Status, Option<application::Message>) {
        handle_event(self, state, event, bounds, cursor)
    }

    fn draw(
        &self,
        _state: &MouseInteraction,
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let cursor_position = cursor.position_in(bounds);
        let canvas_position = cursor_position.map(|point| self.project(point, bounds.size()));
        let hover_selection = canvas_position
            .map(|point| self.state.get_hovered(point).expect("error while drawing"))
            .unwrap_or_default();

        let center = Vector::new(bounds.width / 2.0, bounds.height / 2.0);

        let color = theme.palette().text;
        let stroke = Stroke::default()
            .with_color(color)
            .with_width(Self::BOND_WIDTH * *self.scaling);

        let molecules = self.cache.draw(renderer, bounds.size(), |frame| {
            let background = Path::rectangle(Point::ORIGIN, frame.size());
            frame.fill(&background, theme.palette().background);

            frame.with_save(|frame| {
                frame.translate(center);
                frame.scale(*self.scaling);
                frame.translate(self.translation);

                let region = self.visible_region(frame.size());

                for (_id, molecule) in region.cull(self.state.molecules()) {
                    molecule
                        .draw(frame, &theme.palette().text, &stroke, &color)
                        .expect("error in frame with_save");
                }
            });
        });

        let overlay = {
            let mut frame = Frame::new(renderer, bounds.size());

            self.draw_pending_bond(
                canvas_position,
                hover_selection,
                center,
                &mut frame,
                &stroke,
                &color,
            )
            .expect("error while drawing");

            let draw_from_bounds = |frame: &mut Frame, bounds: Bounds, stroke: Stroke| {
                frame.with_save(|frame| {
                    frame.translate(center);
                    frame.scale(*self.scaling);
                    frame.translate(self.translation);

                    bounds.draw(frame, stroke);
                });
            };

            for bounds in self
                .state
                .selection()
                .bounds(&self.state)
                .expect("error while drawing")
            {
                draw_from_bounds(
                    &mut frame,
                    bounds,
                    Stroke {
                        style: Style::Solid(Color {
                            a: 0.5,
                            ..theme.palette().text
                        }),
                        width: 2.0,
                        ..Default::default()
                    },
                );
            }

            if let Action::DrawingSelection { start } = self.action {
                if let Some(canvas_position) = canvas_position {
                    // draw outline of selecting rectangle
                    frame.with_save(|frame| {
                        frame.translate(center);
                        frame.scale(*self.scaling);
                        frame.translate(self.translation);

                        frame.stroke_rectangle(
                            Point::new(
                                f32::min(start.x, canvas_position.x),
                                f32::min(start.y, canvas_position.y),
                            ),
                            Size::new(
                                f32::abs(start.x - canvas_position.x),
                                f32::abs(start.y - canvas_position.y),
                            ),
                            Stroke {
                                style: Style::Solid(Color {
                                    a: 0.5,
                                    ..theme.palette().primary
                                }),
                                width: 1.0,
                                ..Default::default()
                            },
                        );
                    });
                }
            } else {
                let hover_bounds = hover_selection
                    .bounds(&self.state)
                    .expect("error while drawing");

                if let Some(bounds) = hover_bounds {
                    draw_from_bounds(
                        &mut frame,
                        bounds,
                        Stroke {
                            style: Style::Solid(Color {
                                a: 0.5,
                                ..theme.palette().primary
                            }),
                            width: 1.0,
                            ..Default::default()
                        },
                    );
                }
            }

            frame.into_geometry()
        };

        vec![molecules, overlay]
    }

    fn mouse_interaction(
        &self,
        _state: &MouseInteraction,
        bounds: Rectangle,
        cursor: mouse::Cursor,
    ) -> mouse::Interaction {
        match self.action {
            _ if !cursor.is_over(bounds) => mouse::Interaction::default(),
            Action::Panning { .. } => mouse::Interaction::Grabbing,
            _ => mouse::Interaction::default(), // Action::MovingAtom { .. } | Action::MovingMolecule { .. } => {
                                                //     mouse::Interaction::Move
                                                // }
        }
    }
}

pub struct Region {
    rect: Rectangle,
}

impl Region {
    fn cull<'a, 'b>(
        &'b self,
        molecules: impl Iterator<Item = (&'a MoleculeId, &'a Molecule)> + 'b,
    ) -> impl Iterator<Item = (&'a MoleculeId, &'a Molecule)> + 'b {
        molecules.filter(move |(_molecule_id, molecule)| molecule.bounds().intersects(&self.rect))
    }
}
