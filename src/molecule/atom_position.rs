use std::ops::{Add, AddAssign, Sub};
use derive_more::derive::{Add, AddAssign, Mul, MulAssign, Sub};
use iced::widget::canvas::path::lyon_path::geom::Transform;
use iced::{Point, Rectangle, Vector};

use super::molecule_position::MoleculePosition;

#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Add, Sub, Mul, AddAssign, MulAssign)]
pub struct AtomPosition {
    pub x: f32,
    pub y: f32,
}

impl Default for AtomPosition {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0
        }
    }
}

impl AtomPosition {
    pub fn from(molecule_position: MoleculePosition, canvas_position: Point) -> Self {
        Self { 
            x: canvas_position.x - molecule_position.x,
            y: canvas_position.y - molecule_position.y,
        }
    }
}

impl From<Point> for AtomPosition {
    fn from(point: Point) -> Self {
        Self { x: point.x, y: point.y }
    }
}

impl From<AtomPosition> for Point {
    fn from(val: AtomPosition) -> Self {
        Point { x: val.x, y: val.y }
    }
}

impl From<AtomPosition> for Vector {
    fn from(val: AtomPosition) -> Self {
        Vector { x: val.x, y: val.y }
    }
}

impl From<AtomPosition> for Transform<f32> {
    fn from(val: AtomPosition) -> Self {
        Transform::translation(val.x, val.y)
    }
}

impl Add<MoleculePosition> for AtomPosition {
    type Output = Point;

    fn add(self, rhs: MoleculePosition) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<Vector<f32>> for AtomPosition {
    type Output = Self;

    fn add(self, rhs: Vector) -> Self::Output {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<Rectangle> for AtomPosition {
    type Output = Rectangle;

    fn add(self, rhs: Rectangle) -> Self::Output {
        Rectangle {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            ..rhs
        }
    }
}

impl Sub<Point> for AtomPosition {
    type Output = Self;

    fn sub(self, rhs: Point) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl AddAssign<Vector> for AtomPosition {
    fn add_assign(&mut self, rhs: Vector) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

