use crate::framework::pose::transform_point;
use el_math::Matrix3x2f;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenAxis {
    Horizontal,
    Vertical,
}
impl ScreenAxis {
    pub fn orthogonal(self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
    pub fn positive(self) -> ScreenDirection {
        match self {
            Self::Horizontal => ScreenDirection::Right,
            Self::Vertical => ScreenDirection::Down,
        }
    }
    pub fn negative(self) -> ScreenDirection {
        match self {
            Self::Horizontal => ScreenDirection::Left,
            Self::Vertical => ScreenDirection::Up,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScreenDirection {
    Up,
    Down,
    Left,
    Right,
}
impl ScreenDirection {
    pub fn axis(self) -> ScreenAxis {
        match self {
            Self::Up | Self::Down => ScreenAxis::Vertical,
            Self::Left | Self::Right => ScreenAxis::Horizontal,
        }
    }
    pub fn opposite(self) -> Self {
        match self {
            Self::Up => Self::Down,
            Self::Down => Self::Up,
            Self::Left => Self::Right,
            Self::Right => Self::Left,
        }
    }
    pub fn is_positive(self) -> bool {
        matches!(self, Self::Down | Self::Right)
    }
    pub fn is_after(self, a: i32, b: i32) -> bool {
        if self.is_positive() { a > b } else { b > a }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ScreenPosition {
    pub x: i32,
    pub y: i32,
}
impl ScreenPosition {
    pub fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }
    pub fn coordinate(self, axis: ScreenAxis) -> i32 {
        match axis {
            ScreenAxis::Horizontal => self.x,
            ScreenAxis::Vertical => self.y,
        }
    }
    pub fn step(self, direction: ScreenDirection) -> Self {
        match direction {
            ScreenDirection::Up => Self::new(self.x, self.y - 1),
            ScreenDirection::Down => Self::new(self.x, self.y + 1),
            ScreenDirection::Left => Self::new(self.x - 1, self.y),
            ScreenDirection::Right => Self::new(self.x + 1, self.y),
        }
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct ScreenRectangle {
    pub position: ScreenPosition,
    pub width: i32,
    pub height: i32,
}
impl ScreenRectangle {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            position: ScreenPosition::new(x, y),
            width,
            height,
        }
    }
    pub fn empty() -> Self {
        Self::new(0, 0, 0, 0)
    }
    pub fn left(self) -> i32 {
        self.position.x
    }
    pub fn top(self) -> i32 {
        self.position.y
    }
    pub fn right(self) -> i32 {
        self.position.x + self.width
    }
    pub fn bottom(self) -> i32 {
        self.position.y + self.height
    }
    pub fn contains_point(self, x: i32, y: i32) -> bool {
        x >= self.left() && x < self.right() && y >= self.top() && y < self.bottom()
    }
    pub fn get_border(self, direction: ScreenDirection) -> Self {
        match direction {
            ScreenDirection::Left => Self::new(self.left() - 1, self.top(), 1, self.height),
            ScreenDirection::Right => Self::new(self.right(), self.top(), 1, self.height),
            ScreenDirection::Up => Self::new(self.left(), self.top() - 1, self.width, 1),
            ScreenDirection::Down => Self::new(self.left(), self.bottom(), self.width, 1),
        }
    }
    pub fn intersects(self, other: Self) -> bool {
        self.left() < other.right()
            && self.right() > other.left()
            && self.top() < other.bottom()
            && self.bottom() > other.top()
    }
    pub fn encompasses(self, other: Self) -> bool {
        other.left() >= self.left()
            && other.top() >= self.top()
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }
    pub fn intersection(self, other: Self) -> Option<Self> {
        let left = self.left().max(other.left());
        let top = self.top().max(other.top());
        let right = self.right().min(other.right());
        let bottom = self.bottom().min(other.bottom());
        (left < right && top < bottom).then(|| Self::new(left, top, right - left, bottom - top))
    }
    pub fn transform_axis_aligned(self, matrix: Matrix3x2f) -> Self {
        let (x0, y0) = transform_point(matrix, self.left() as f32, self.top() as f32);
        let (x1, y1) = transform_point(matrix, self.right() as f32, self.bottom() as f32);
        Self::new(
            x0.floor() as i32,
            y0.floor() as i32,
            (x1 - x0).floor() as i32,
            (y1 - y0).floor() as i32,
        )
    }
    pub fn transform_max_bounds(self, matrix: Matrix3x2f) -> Self {
        let points = [
            transform_point(matrix, self.left() as f32, self.top() as f32),
            transform_point(matrix, self.right() as f32, self.top() as f32),
            transform_point(matrix, self.left() as f32, self.bottom() as f32),
            transform_point(matrix, self.right() as f32, self.bottom() as f32),
        ];
        let min_x = points.iter().map(|p| p.0).fold(f32::INFINITY, f32::min);
        let max_x = points.iter().map(|p| p.0).fold(f32::NEG_INFINITY, f32::max);
        let min_y = points.iter().map(|p| p.1).fold(f32::INFINITY, f32::min);
        let max_y = points.iter().map(|p| p.1).fold(f32::NEG_INFINITY, f32::max);
        Self::new(
            min_x.floor() as i32,
            min_y.floor() as i32,
            (max_x - min_x).ceil() as i32,
            (max_y - min_y).ceil() as i32,
        )
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FocusNavigationEvent {
    InitialFocus,
    TabNavigation { forward: bool },
    ArrowNavigation { direction: ScreenDirection },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ComponentPath {
    Leaf(usize),
    Path {
        component: usize,
        child: Box<ComponentPath>,
    },
}
impl ComponentPath {
    pub fn leaf(component: usize) -> Self {
        Self::Leaf(component)
    }
}
