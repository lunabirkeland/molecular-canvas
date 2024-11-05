use std::ops::{Add, AddAssign, Sub};
use derive_more::derive::{Add, AddAssign, Mul, MulAssign};
use iced::widget::canvas::path::lyon_path::geom::Transform;
use iced::{Point, Rectangle, Vector};

use super::AtomPosition;

#[derive(Debug, Clone, Copy, PartialEq)]
#[derive(Add, Mul, AddAssign, MulAssign)]
pub struct MoleculePosition {
    pub x: f32,
    pub y: f32,
}

impl From<Point> for MoleculePosition {
    fn from(point: Point) -> Self {
        Self { x: point.x, y: point.y }
    }
}

impl From<MoleculePosition> for Point {
    fn from(val: MoleculePosition) -> Self {
        Point { x: val.x, y: val.y }
    }
}

impl From<MoleculePosition> for Vector {
    fn from(val: MoleculePosition) -> Self {
        Vector { x: val.x, y: val.y }
    }
}

impl From<MoleculePosition> for Transform<f32> {
    fn from(val: MoleculePosition) -> Self {
        Transform::translation(val.x, val.y)
    }
}

impl Add<AtomPosition> for MoleculePosition {
    type Output = Point;

    fn add(self, rhs: AtomPosition) -> Self::Output {
        Self::Output {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
        }
    }
}

impl Add<Rectangle> for MoleculePosition {
    type Output = Rectangle;

    fn add(self, rhs: Rectangle) -> Self::Output {
        Rectangle {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            ..rhs
        }
    }
}

impl Sub<Point> for MoleculePosition {
    type Output = Self;

    fn sub(self, rhs: Point) -> Self::Output {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

impl AddAssign<Vector> for MoleculePosition {
    fn add_assign(&mut self, rhs: Vector) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}
