use std::f32::consts::PI;
use std::iter;

use anyhow::Context;
use anyhow::Result;
use iced::widget::canvas::path::lyon_path::math::Transform;
use iced::widget::canvas::Frame;
use iced::widget::canvas::Path;
use iced::widget::canvas::Stroke;
use iced::Color;
use iced::Point;
use iced::Radians;
use iced::Size;
use iced::Vector;
use rustc_hash::FxHashMap;

use crate::bounds::Bounds;
use crate::canvas::MolCanvas;

use super::Atom;
use super::AtomId;

#[derive(Debug, Clone)]
pub struct Bond {
    start: AtomId,
    end: AtomId,
    bond_type: BondType,
}

impl Bond {
    pub fn new(start: AtomId, end: AtomId, bond_type: BondType) -> Bond {
        Bond {
            start,
            end,
            bond_type,
        }
    }

    pub fn change_type(&mut self, bond_type: BondType) {
        self.bond_type = bond_type;
    }

    pub fn flip(&mut self) {
        (self.start, self.end) = (self.end, self.start);
    }

    pub fn draw(
        &self,
        frame: &mut Frame,
        transform: &Transform,
        atoms: &FxHashMap<AtomId, Atom>,
        stroke: &Stroke,
        color: &Color
    ) -> Result<()> {
        let start_atom = atoms.get(&self.start).ok_or(super::Error::AtomMissing(self.start)).context("while drawing bond")?;
        let end_atom = atoms.get(&self.end).ok_or(super::Error::AtomMissing(self.end)).context("while drawing bond")?;

        let start: Point = start_atom.bond_start(end_atom.position()).into();
        let end: Point = end_atom.bond_start(start_atom.position()).into();


        draw_bond(frame, transform, start, end, &self.bond_type, stroke, color)
    }

    pub fn bounds(&self, atoms: &FxHashMap<AtomId, Atom>) -> Result<Bounds> {
        let start_atom = atoms.get(&self.start).ok_or(super::Error::AtomMissing(self.start)).context("while calculating bond bounds")?;
        let end_atom = atoms.get(&self.end).ok_or(super::Error::AtomMissing(self.end)).context("while calculating bond bounds")?;

        let start: Point = start_atom.bond_start(end_atom.position()).into();
        let end: Point = end_atom.bond_start(start_atom.position()).into();

        let direction: Vector = end - start;
        let length = (direction.x.powi(2) + direction.y.powi(2)).sqrt();
        let unit_normal = Vector::new(direction.y, -direction.x) * length.powi(-1);

        let width = match self.bond_type {
            BondType::Normal(strength) => (strength as f32 - 1.0) * MolCanvas::BOND_OFFSETS + MolCanvas::BOND_WIDTH,
            BondType::Hydrogen => MolCanvas::H_BOND_WIDTH,
            BondType::Wedge => MolCanvas::WEDGE_END_WIDTH,
            BondType::Dash => MolCanvas::DASH_END_WIDTH,
        };

        let offset = start + unit_normal * (width / 2.0);

        let size = Size::new(length, width);

        let angle = Radians(f32::atan(direction.y / direction.x) + if direction.x < 0.0 { PI } else { 0.0 });

        let mut bounds = Bounds::new(offset, size, angle);
        bounds.add_padding(MolCanvas::BOND_PADDING);

        Ok(bounds)
    }

    pub fn fixed_length(start: Point, direction: Vector, length: f32) -> Point {
        let magnitude = f32::sqrt(direction.x.powi(2) + direction.y.powi(2));

        if magnitude > 0.0001 {
            start + (direction * (length / magnitude))
        } else {
            start + Vector::new(length, 0.0)
        }
    }

    pub fn center(&self, atoms: &FxHashMap<AtomId, Atom>) -> Result<Point> {
        let start_atom = atoms.get(&self.start).ok_or(super::Error::AtomMissing(self.start)).context("while calculating bond bounds")?;
        let end_atom = atoms.get(&self.end).ok_or(super::Error::AtomMissing(self.end)).context("while calculating bond bounds")?;

        let start: Point = start_atom.bond_start(end_atom.position()).into();
        let end: Point = end_atom.bond_start(start_atom.position()).into();

        let direction: Vector = end - start;

        Ok(start + direction * 0.5)
    }

    pub fn start(&self) -> AtomId {
        self.start
    }

    pub fn end(&self) -> AtomId {
        self.end
    }

    pub fn bond_type(&self) -> BondType {
        self.bond_type
    }

    pub fn atom_ids(&self) -> impl Iterator<Item = AtomId> {
        [self.start, self.end].into_iter()
    }
}

pub fn draw_bond(frame: &mut Frame, transform: &Transform, start: Point, end: Point, bond_type: &BondType, stroke: &Stroke, color: &Color) -> Result<()> {
    let direction: Vector = end - start;
    let length = (direction.x.powi(2) + direction.y.powi(2)).sqrt();
    let normal = Vector::new(direction.y, -direction.x);
    let unit_direction = direction * length.powi(-1);
    let unit_normal = normal * length.powi(-1);

    match bond_type {
        BondType::Normal(strength) => {
            // offsets is an iterator of either (1, -1, 3, -3, 5, -5, ...) for even strength
            // or (0, 2, -2, 4, -4, 6, -6, ...) for odd strength
            let offsets = {
                let mut curr_offset: i32 = match strength % 2 {
                    0 => 1,
                    1 => 0,
                    _ => unreachable!()
                };

                iter::from_fn(move || {
                    let old_offset = curr_offset;

                    curr_offset = match curr_offset.is_positive() {
                        _ if curr_offset == 0 => 2,
                        true => -curr_offset,
                        false => -curr_offset + 2
                    };

                    if old_offset > *strength as i32 {
                        None
                    } else {
                        Some(old_offset)
                    }
                })
            };

            for offset in offsets {
                // divided by 2 to account for existing spacing of 2 between bonds
                let offset = unit_normal * (offset as f32 * MolCanvas::BOND_OFFSETS / 2.0);
                let path = Path::line(start + offset, end + offset).transform(transform);

                frame.stroke(&path, *stroke);
            }
        }
        BondType::Wedge => {
            let path = Path::new(|builder| {
                builder.move_to(start - unit_normal * (MolCanvas::WEDGE_START_WIDTH / 2.0));
                builder.line_to(start + unit_normal * (MolCanvas::WEDGE_START_WIDTH / 2.0));
                builder.line_to(end + unit_normal * (MolCanvas::WEDGE_END_WIDTH / 2.0));
                builder.line_to(end - unit_normal * (MolCanvas::WEDGE_END_WIDTH / 2.0));
                builder.close();
            }).transform(transform);

            frame.fill(&path, *color);
        }
        BondType::Dash => {
            let start = start + unit_direction * (MolCanvas::BOND_WIDTH / 2.0);
            let length = length - MolCanvas::BOND_WIDTH;
            // aim to have a dash every MolCanvas::DASH_BOND_OFFSETS add small offset to prevent
            // jittering caused by floating point arithmetic errors
            let dashes: u32 = f32::round(length / MolCanvas::DASH_BOND_OFFSETS + 0.01) as u32;
            let true_spacing = length / dashes as f32;
            // offsets is an iterator of either (1, -1, 3, -3, 5, -5, ...) for even strength
            // or (0, 2, -2, 4, -4, 6, -6, ...) for odd strength
            let offsets = 0..=dashes;

            let width = |n: u32| {
                MolCanvas::DASH_START_WIDTH + (n as f32 / dashes as f32) * MolCanvas::DASH_END_WIDTH
            };

            for n in offsets {
                let offset = unit_direction * (n as f32 * true_spacing);
                let path = Path::line(
                    start + offset + unit_normal * (width(n) / 2.0),
                    start + offset - unit_normal * (width(n) / 2.0)
                ).transform(transform);

                frame.stroke(&path, *stroke);
            }
        }
        BondType::Hydrogen => {
            let start = start + unit_direction * (MolCanvas::BOND_WIDTH / 2.0);
            let length = length - MolCanvas::BOND_WIDTH;
            // aim to have a dash every MolCanvas::H_BOND_OFFSETS add small offset to prevent
            // jittering caused by floating point arithmetic errors
            let dashes: u32 = f32::round(length / MolCanvas::H_BOND_OFFSETS + 0.01) as u32;
            let true_spacing = length / dashes as f32;
            // offsets is an iterator of either (1, -1, 3, -3, 5, -5, ...) for even strength
            // or (0, 2, -2, 4, -4, 6, -6, ...) for odd strength
            let offsets = 0..=dashes;

            for offset in offsets {
                let offset = unit_direction * (offset as f32 * true_spacing);
                let path = Path::line(
                    start + offset + unit_normal * (MolCanvas::H_BOND_WIDTH / 2.0),
                    start + offset - unit_normal * (MolCanvas::H_BOND_WIDTH / 2.0)
                ).transform(transform);

                frame.stroke(&path, *stroke);
            }

        }
    }
    
    Ok(())
}


#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BondType {
    Normal(u8),
    Wedge,
    Dash,
    Hydrogen,
}

impl Default for BondType {
    fn default() -> Self {
        Self::Normal(1)
    }
}
