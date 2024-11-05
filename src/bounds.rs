use std::ops::Add;

use iced::{
    widget::canvas::{
        path::lyon_path::{
            geom::Vector,
            math::{Angle, Point, Transform},
        },
        Frame, Path, Stroke,
    },
    Radians, Rectangle, Size,
};


/// rectanglular bounding box with arbitrary rotation
#[derive(Debug, Default, Clone, Copy)]
pub struct Bounds {
    offset: Vector<f32>,
    size: Size,
    angle: Angle,
}

impl Bounds {
    pub fn new(top_left: iced::Point, size: Size, angle: Radians) -> Self {
        Self {
            offset: Vector::new(top_left.x, top_left.y),
            size,
            angle: Angle::radians(angle.into()),
        }
    }

    fn transform(&self) -> Transform {
        Transform::rotation(self.angle).then_translate(self.offset)
    }

    pub fn add_padding(&mut self, padding: f32) {
        self.offset -= self.transform().transform_vector(Vector::new(padding, padding));
        self.size = Size::new(self.size.width + 2.0 * padding, self.size.height + 2.0 * padding);
    }

    fn points(&self) -> impl Iterator<Item = Point> {
        let transform = self.transform();

        [
            Point::new(0.0, 0.0),
            Point::new(self.size.width, 0.0),
            Point::new(self.size.width, self.size.height),
            Point::new(0.0, self.size.height),
        ]
        .into_iter()
        .map(move |point| transform.transform_point(point))
    }

    pub fn center(&self) -> iced::Point {
        let rect_center = Point::new(self.size.width / 2.0, self.size.height / 2.0);
        let center = self.transform().transform_point(rect_center);

        iced::Point::new(center.x, center.y)
    }

    pub fn contains(&self, point: iced::Point) -> bool {
        let point = Point::new(point.x, point.y);

        let point = self.transform().inverse().unwrap().transform_point(point);

        Rectangle::with_size(self.size)
            .contains(iced::Point::new(point.x, point.y))
    }

    pub fn draw(&self, frame: &mut Frame, stroke: Stroke) {
        let path =
            Path::rectangle(iced::Point::ORIGIN, self.size).transform(&self.transform());

        frame.stroke(&path, stroke);
    }

    /// Returns the smallest axis-aligned rectangle that contains both bounds
    pub fn union(&self, bounds: &Bounds) -> Bounds {
        let mut points = self.points().chain(bounds.points());

        let mut min_corner = points.next().unwrap();
        let mut max_corner = min_corner;

        let expand_bounds = |min_corner: &mut Point, max_corner: &mut Point, point: Point| {
            if point.x < min_corner.x {
                min_corner.x = point.x;
            } else if point.y < min_corner.y {
                min_corner.y = point.y;
            }

            if point.x > max_corner.x {
                max_corner.x = point.x;
            } else if point.y > max_corner.y {
                max_corner.y = point.y;
            }
        };

        for point in points {
            expand_bounds(&mut min_corner, &mut max_corner, point);
        }

        let size = Size::new(max_corner.x - min_corner.x, max_corner.y - min_corner.y);

        Bounds::from(Rectangle::new(
            iced::Point::new(min_corner.x, min_corner.y),
            size,
        ))
    }

    pub fn intersects(&self, rect: &Rectangle) -> bool {
        // comparisons to tell if point is inside rect in an axis
        for cmp in [
            |point: Point, rect: &Rectangle| point.x > rect.x,
            |point: Point, rect: &Rectangle| point.y > rect.y,
            |point: Point, rect: &Rectangle| point.x < rect.x + rect.width,
            |point: Point, rect: &Rectangle| point.y < rect.y + rect.height,
        ] {
            let mut outside_line = true;
            for point in self.points() {
                if cmp(point, rect) {
                    outside_line = false;
                }
            }

            if outside_line {
                return false;
            };
        }

        true
    }

    pub fn is_contained(&self, rect: &Rectangle) -> bool {
        // comparisons to tell if point is outside rect in an axis
        for cmp in [
            |point: Point, rect: &Rectangle| point.x < rect.x,
            |point: Point, rect: &Rectangle| point.y < rect.y,
            |point: Point, rect: &Rectangle| point.x > rect.x + rect.width,
            |point: Point, rect: &Rectangle| point.y > rect.y + rect.height,
        ] {
            for point in self.points() {
                if cmp(point, rect) {
                    return false;
                }
            }
        }

        true
    }
}

impl From<Rectangle> for Bounds {
    fn from(rectangle: Rectangle) -> Self {
        Self {
            offset: Vector::new(rectangle.x, rectangle.y),
            size: rectangle.size(),
            angle: Angle::zero(),
        }
    }
}

impl Add<iced::Vector> for Bounds {
    type Output = Bounds;

    fn add(self, vector: iced::Vector) -> Self::Output {
        Self::Output {
            offset: self.offset + Vector::new(vector.x, vector.y),
            size: self.size,
            angle: self.angle,
        }
    }
}
