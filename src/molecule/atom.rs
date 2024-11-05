use anyhow::Result;
use iced::alignment::Horizontal;
use iced::alignment::Vertical;
use iced::widget::canvas::path::lyon_path::geom::euclid::Point2D;
use iced::widget::canvas::path::lyon_path::geom::Transform;
use iced::widget::canvas::path::lyon_path::traits::PathIterator;
use iced::widget::canvas::path::lyon_path::PathEvent;
use iced::widget::canvas::Frame;
use iced::widget::canvas::{Path, Text};
use iced::widget::text::LineHeight;
use iced::widget::text::Shaping;
use iced::Vector;
use iced::{Color, Font, Pixels, Point, Rectangle, Size};

use crate::bounds::Bounds;
use crate::canvas::MolCanvas;

use super::atom_position::AtomPosition;

#[derive(Debug, Clone)]
pub struct Atom {
    label: Label,
    position: AtomPosition,
}

impl Atom {
    pub fn new(label: String, position: AtomPosition, direction: Direction) -> Atom {
        Self {
            label: Label::new(label, direction),
            position,
        }
    }

    pub fn draw(&self, frame: &mut Frame, transform: &Transform<f32>, color: &Color) -> Result<()> {
        let transform = <AtomPosition as Into<Transform<f32>>>::into(self.position).then(transform);

        if self.label.is_empty() {
            let path =
                Path::circle(Point::ORIGIN, MolCanvas::BOND_WIDTH / 2.0).transform(&transform);

            frame.fill(&path, *color);
        }

        self.label.draw(frame, &transform, color);

        Ok(())
    }

    pub fn bounds(&self) -> Bounds {
        Bounds::from(self.label.bounds().expand(MolCanvas::ATOM_PADDING)) + self.position().into()
    }

    pub fn rename(&mut self, text: String) {
        self.label = Label::new(text, self.label.direction);
    }

    pub fn update_label_direction(&mut self, direction: Direction) {
        self.label.update_direction(direction);
    }

    pub fn label(&self) -> String {
        self.label.input_string.clone()
    }

    pub fn position(&self) -> AtomPosition {
        self.position
    }

    pub fn translate(&mut self, translation: Vector) {
        self.position += translation;
    }

    /// Returns the start point for a bond.
    pub fn bond_start(&self, end: AtomPosition) -> AtomPosition {
        if self.label.is_empty() {
            return self.position;
        }

        let direction: Vector = (end - self.position).into();

        // distance along direction until you reach bounds on the x axis
        let t_xmin = {
            if direction.x > 0.0 {
                (self.label.bounds().x + self.label.bounds().width) / direction.x
            } else if direction.x < 0.0 {
                (self.label.bounds().x) / direction.x
            } else {
                0.0
            }
        };

        // distance along direction until you reach bounds on the y axis
        let t_ymin = {
            if direction.y > 0.0 {
                (self.label.bounds().y + self.label.bounds().height) / direction.y
            } else if direction.y < 0.0 {
                (self.label.bounds().y) / direction.y
            } else {
                0.0
            }
        };

        let t_min = t_xmin.min(t_ymin);

        self.position + (direction * t_min)
    }
}

#[derive(Debug, Clone)]
struct Token {
    paths: Vec<Path>,
    bounds: Rectangle,
}

impl Token {
    pub fn new(content: String) -> Token {
        let paths = Self::calculate_paths(&content);
        let bounds = Self::calculate_bounds(&paths);

        Self { paths, bounds }
    }

    fn calculate_paths(content: &String) -> Vec<Path> {
        let text = Text {
            content: content.to_string(),
            color: Color::default(),
            position: Point::default(),
            font: Font::DEFAULT,
            size: Pixels(10.0),
            line_height: LineHeight::Relative(1.2),
            horizontal_alignment: Horizontal::Center,
            vertical_alignment: Vertical::Center,
            shaping: Shaping::Basic,
        };

        let mut paths = Vec::<Path>::new();

        let draw_path = |path, _| {
            paths.push(path);
        };

        text.draw_with(draw_path);

        paths
    }

    fn calculate_bounds(paths: &[Path]) -> Rectangle {
        let mut points = paths.iter().flat_map(|path| {
            path.raw().iter().flattened(0.1).flat_map(|evt| match evt {
                PathEvent::Begin { at } => {
                    vec![at]
                }
                PathEvent::Line { from, to } => {
                    vec![from, to]
                }
                PathEvent::End {
                    first,
                    last,
                    close: _,
                } => {
                    vec![first, last]
                }
                _ => unreachable!(),
            })
        });

        let mut min_corner = points.next().unwrap_or(Point2D::zero());
        let mut max_corner = min_corner;

        let expand_bounds = |min_corner: &mut Point2D<f32, _>,
                             max_corner: &mut Point2D<f32, _>,
                             point: Point2D<f32, _>| {
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

        points.for_each(|point| expand_bounds(&mut min_corner, &mut max_corner, point));

        Rectangle::new(
            Point::new(min_corner.x, min_corner.y),
            Size::new(max_corner.x - min_corner.x, max_corner.y - min_corner.y),
        )
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    #[default]
    Right,
}

#[derive(Debug, Clone)]
struct Label {
    input_string: String,
    tokens: Vec<Token>,
    bounds: Rectangle,
    direction: Direction,
}

impl Label {
    const TOKEN_SEPARATION: f32 = 1.0;

    pub fn new(input_string: String, direction: Direction) -> Self {
        let tokens = Self::tokenize(&input_string);
        let mut label = Self {
            input_string,
            tokens,
            direction,
            bounds: Rectangle::default(),
        };

        label.calculate_bounds();
        label
    }

    pub fn draw(&self, frame: &mut Frame, transform: &Transform<f32>, color: &Color) {
        let mut shift = match self.direction {
            Direction::Right => Vector::new(self.bounds.x, 0.0),
            Direction::Left => Vector::new(self.bounds.x + self.bounds.width, 0.0),
            Direction::Down => Vector::new(0.0, self.bounds.y),
            Direction::Up => Vector::new(0.0, self.bounds.y + self.bounds.height),
        };

        for Token { paths, bounds } in &self.tokens {
            // shift such that drawing starts at x = shift
            let new_shift = match self.direction {
                Direction::Right => shift - Vector::new(bounds.x, 0.0),
                Direction::Left => shift - Vector::new(bounds.x + bounds.width, 0.0),
                Direction::Down => shift - Vector::new(0.0, bounds.y),
                Direction::Up => shift - Vector::new(0.0, bounds.y + bounds.height),
            };

            let transform = Transform::translation(new_shift.x, new_shift.y).then(transform);

            for path in paths {
                let path = path.transform(&transform);

                frame.fill(&path, *color);
            }

            shift = shift
                + match self.direction {
                    Direction::Right => Vector::new(bounds.width + Self::TOKEN_SEPARATION, 0.0),
                    Direction::Left => -Vector::new(bounds.width + Self::TOKEN_SEPARATION, 0.0),
                    Direction::Down => Vector::new(0.0, bounds.height + Self::TOKEN_SEPARATION),
                    Direction::Up => -Vector::new(0.0, bounds.height + Self::TOKEN_SEPARATION),
                }
        }
    }

    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    pub fn bounds(&self) -> Rectangle {
        self.bounds
    }

    pub fn update_direction(&mut self, direction: Direction) {
        if direction != self.direction {
            self.direction = direction;
            self.calculate_bounds();
        }
    }

    fn calculate_bounds(&mut self) {
        if self.tokens.is_empty() {
            self.bounds = Rectangle::default();
            return;
        }

        let Token {
            bounds: label_bounds,
            ..
        } = self.tokens[0];

        let mut label_bounds = label_bounds;

        for Token { bounds, .. } in &self.tokens[1..] {
            let position = match self.direction {
                Direction::Right => Point::new(
                    label_bounds.width + label_bounds.x + Self::TOKEN_SEPARATION,
                    bounds.y,
                ),
                Direction::Left => Point::new(
                    label_bounds.x - bounds.width + Self::TOKEN_SEPARATION,
                    bounds.y,
                ),
                Direction::Down => Point::new(
                    bounds.x,
                    label_bounds.height + label_bounds.y + Self::TOKEN_SEPARATION,
                ),
                Direction::Up => Point::new(
                    bounds.x,
                    label_bounds.y - bounds.height - Self::TOKEN_SEPARATION,
                ),
            };

            let bounds = Rectangle::new(position, bounds.size());
            label_bounds = label_bounds.union(&bounds);
        }

        self.bounds = label_bounds;
    }

    fn tokenize(input_string: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let mut current_token = String::new();

        for c in input_string.chars() {
            match c {
                _ if c.is_uppercase() => {
                    if !current_token.is_empty() {
                        tokens.push(Token::new(current_token));
                        current_token = String::new();
                    }
                    current_token.push(c);
                }
                _ if c.is_ascii_digit() => current_token.push(match c {
                    '0' => '₀',
                    '1' => '₁',
                    '2' => '₂',
                    '3' => '₃',
                    '4' => '₄',
                    '5' => '₅',
                    '6' => '₆',
                    '7' => '₇',
                    '8' => '₈',
                    '9' => '₉',
                    _ => unreachable!(),
                }),
                _ => current_token.push(c),
            }
        }
        if !current_token.is_empty() {
            tokens.push(Token::new(current_token));
        }

        tokens
    }
}
