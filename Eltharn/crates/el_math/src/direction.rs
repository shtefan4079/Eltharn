use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Direction {
    Down,
    Up,
    North,
    South,
    West,
    East,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Axis {
    X,
    Y,
    Z,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AxisDirection {
    Positive,
    Negative,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Plane {
    Horizontal,
    Vertical,
}

impl Direction {
    pub const VALUES: [Direction; 6] = [
        Direction::Down,
        Direction::Up,
        Direction::North,
        Direction::South,
        Direction::West,
        Direction::East,
    ];

    const BY_2D_DATA: [Direction; 4] = [
        Direction::South,
        Direction::West,
        Direction::North,
        Direction::East,
    ];

    pub fn data_3d(self) -> i32 {
        match self {
            Direction::Down => 0,
            Direction::Up => 1,
            Direction::North => 2,
            Direction::South => 3,
            Direction::West => 4,
            Direction::East => 5,
        }
    }

    pub fn data_2d(self) -> i32 {
        match self {
            Direction::Down | Direction::Up => -1,
            Direction::North => 2,
            Direction::South => 0,
            Direction::West => 1,
            Direction::East => 3,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Direction::Down => "down",
            Direction::Up => "up",
            Direction::North => "north",
            Direction::South => "south",
            Direction::West => "west",
            Direction::East => "east",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "down" => Some(Direction::Down),
            "up" => Some(Direction::Up),
            "north" => Some(Direction::North),
            "south" => Some(Direction::South),
            "west" => Some(Direction::West),
            "east" => Some(Direction::East),
            _ => None,
        }
    }

    pub fn axis(self) -> Axis {
        match self {
            Direction::Down | Direction::Up => Axis::Y,
            Direction::North | Direction::South => Axis::Z,
            Direction::West | Direction::East => Axis::X,
        }
    }

    pub fn axis_direction(self) -> AxisDirection {
        match self {
            Direction::Down | Direction::North | Direction::West => AxisDirection::Negative,
            Direction::Up | Direction::South | Direction::East => AxisDirection::Positive,
        }
    }

    pub fn step_x(self) -> i32 {
        match self {
            Direction::West => -1,
            Direction::East => 1,
            _ => 0,
        }
    }

    pub fn step_y(self) -> i32 {
        match self {
            Direction::Down => -1,
            Direction::Up => 1,
            _ => 0,
        }
    }

    pub fn step_z(self) -> i32 {
        match self {
            Direction::North => -1,
            Direction::South => 1,
            _ => 0,
        }
    }

    pub fn step(self) -> (i32, i32, i32) {
        (self.step_x(), self.step_y(), self.step_z())
    }

    pub fn opposite(self) -> Self {
        Self::from_3d_data_value(match self {
            Direction::Down => 1,
            Direction::Up => 0,
            Direction::North => 3,
            Direction::South => 2,
            Direction::West => 5,
            Direction::East => 4,
        })
    }

    pub fn from_3d_data_value(data: i32) -> Self {
        let index = (data % Self::VALUES.len() as i32).abs() as usize;
        Self::VALUES[index]
    }

    pub fn from_2d_data_value(data: i32) -> Self {
        let index = (data % Self::BY_2D_DATA.len() as i32).abs() as usize;
        Self::BY_2D_DATA[index]
    }

    pub fn from_y_rot(y_rot: f64) -> Self {
        Self::from_2d_data_value((crate::mth::floor_f64(y_rot / 90.0 + 0.5)) & 3)
    }

    pub fn from_axis_and_direction(axis: Axis, direction: AxisDirection) -> Self {
        match axis {
            Axis::X => {
                if direction == AxisDirection::Positive {
                    Direction::East
                } else {
                    Direction::West
                }
            }
            Axis::Y => {
                if direction == AxisDirection::Positive {
                    Direction::Up
                } else {
                    Direction::Down
                }
            }
            Axis::Z => {
                if direction == AxisDirection::Positive {
                    Direction::South
                } else {
                    Direction::North
                }
            }
        }
    }

    pub fn try_clockwise(self) -> Option<Self> {
        match self {
            Direction::North => Some(Direction::East),
            Direction::South => Some(Direction::West),
            Direction::West => Some(Direction::North),
            Direction::East => Some(Direction::South),
            _ => None,
        }
    }

    pub fn clockwise(self) -> Self {
        self.try_clockwise()
            .unwrap_or_else(|| panic!("Unable to get Y-rotated facing of {}", self))
    }

    pub fn try_counter_clockwise(self) -> Option<Self> {
        match self {
            Direction::North => Some(Direction::West),
            Direction::South => Some(Direction::East),
            Direction::West => Some(Direction::South),
            Direction::East => Some(Direction::North),
            _ => None,
        }
    }

    pub fn counter_clockwise(self) -> Self {
        self.try_counter_clockwise()
            .unwrap_or_else(|| panic!("Unable to get CCW facing of {}", self))
    }

    pub fn clockwise_around(self, axis: Axis) -> Self {
        match axis {
            Axis::X => {
                if self == Direction::West || self == Direction::East {
                    self
                } else {
                    self.clockwise_x()
                }
            }
            Axis::Y => {
                if self == Direction::Up || self == Direction::Down {
                    self
                } else {
                    self.clockwise()
                }
            }
            Axis::Z => {
                if self == Direction::North || self == Direction::South {
                    self
                } else {
                    self.clockwise_z()
                }
            }
        }
    }

    pub fn counter_clockwise_around(self, axis: Axis) -> Self {
        match axis {
            Axis::X => {
                if self == Direction::West || self == Direction::East {
                    self
                } else {
                    self.counter_clockwise_x()
                }
            }
            Axis::Y => {
                if self == Direction::Up || self == Direction::Down {
                    self
                } else {
                    self.counter_clockwise()
                }
            }
            Axis::Z => {
                if self == Direction::North || self == Direction::South {
                    self
                } else {
                    self.counter_clockwise_z()
                }
            }
        }
    }

    fn clockwise_x(self) -> Self {
        match self {
            Direction::Down => Direction::South,
            Direction::Up => Direction::North,
            Direction::North => Direction::Down,
            Direction::South => Direction::Up,
            _ => panic!("Unable to get X-rotated facing of {}", self),
        }
    }

    fn counter_clockwise_x(self) -> Self {
        match self {
            Direction::Down => Direction::North,
            Direction::Up => Direction::South,
            Direction::North => Direction::Up,
            Direction::South => Direction::Down,
            _ => panic!("Unable to get X-rotated facing of {}", self),
        }
    }

    fn clockwise_z(self) -> Self {
        match self {
            Direction::Down => Direction::West,
            Direction::Up => Direction::East,
            Direction::West => Direction::Up,
            Direction::East => Direction::Down,
            _ => panic!("Unable to get Z-rotated facing of {}", self),
        }
    }

    fn counter_clockwise_z(self) -> Self {
        match self {
            Direction::Down => Direction::East,
            Direction::Up => Direction::West,
            Direction::West => Direction::Down,
            Direction::East => Direction::Up,
            _ => panic!("Unable to get Z-rotated facing of {}", self),
        }
    }

    pub fn to_y_rot(self) -> f32 {
        ((self.data_2d() & 3) * 90) as f32
    }

    pub fn try_y_rot(self) -> Option<f32> {
        match self {
            Direction::North => Some(180.0),
            Direction::South => Some(0.0),
            Direction::West => Some(90.0),
            Direction::East => Some(-90.0),
            _ => None,
        }
    }

    pub fn y_rot(self) -> f32 {
        self.try_y_rot()
            .unwrap_or_else(|| panic!("No y-Rot for vertical axis: {}", self))
    }

    pub fn approximate_nearest(dx: f32, dy: f32, dz: f32) -> Self {
        let mut result = Direction::North;
        let mut highest_dot = f32::from_bits(1);

        for direction in Self::VALUES {
            let dot = dx * direction.step_x() as f32
                + dy * direction.step_y() as f32
                + dz * direction.step_z() as f32;
            if dot > highest_dot {
                highest_dot = dot;
                result = direction;
            }
        }

        result
    }

    pub fn nearest(x: i32, y: i32, z: i32, or_else: Option<Self>) -> Option<Self> {
        let abs_x = java_abs_i32(x);
        let abs_y = java_abs_i32(y);
        let abs_z = java_abs_i32(z);

        if abs_x > abs_z && abs_x > abs_y {
            Some(if x < 0 {
                Direction::West
            } else {
                Direction::East
            })
        } else if abs_z > abs_x && abs_z > abs_y {
            Some(if z < 0 {
                Direction::North
            } else {
                Direction::South
            })
        } else if abs_y > abs_x && abs_y > abs_z {
            Some(if y < 0 {
                Direction::Down
            } else {
                Direction::Up
            })
        } else {
            or_else
        }
    }
}

impl Axis {
    pub const VALUES: [Axis; 3] = [Axis::X, Axis::Y, Axis::Z];

    pub fn name(self) -> &'static str {
        match self {
            Axis::X => "x",
            Axis::Y => "y",
            Axis::Z => "z",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "x" => Some(Axis::X),
            "y" => Some(Axis::Y),
            "z" => Some(Axis::Z),
            _ => None,
        }
    }

    pub fn is_vertical(self) -> bool {
        self == Axis::Y
    }

    pub fn is_horizontal(self) -> bool {
        self == Axis::X || self == Axis::Z
    }

    pub fn positive(self) -> Direction {
        match self {
            Axis::X => Direction::East,
            Axis::Y => Direction::Up,
            Axis::Z => Direction::South,
        }
    }

    pub fn negative(self) -> Direction {
        match self {
            Axis::X => Direction::West,
            Axis::Y => Direction::Down,
            Axis::Z => Direction::North,
        }
    }

    pub fn directions(self) -> [Direction; 2] {
        [self.positive(), self.negative()]
    }

    pub fn plane(self) -> Plane {
        match self {
            Axis::X | Axis::Z => Plane::Horizontal,
            Axis::Y => Plane::Vertical,
        }
    }

    pub fn choose_i32(self, x: i32, y: i32, z: i32) -> i32 {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }

    pub fn choose_f64(self, x: f64, y: f64, z: f64) -> f64 {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }

    pub fn choose_bool(self, x: bool, y: bool, z: bool) -> bool {
        match self {
            Axis::X => x,
            Axis::Y => y,
            Axis::Z => z,
        }
    }
}

impl AxisDirection {
    pub const VALUES: [AxisDirection; 2] = [AxisDirection::Positive, AxisDirection::Negative];

    pub fn id(self) -> &'static str {
        match self {
            AxisDirection::Positive => "positive",
            AxisDirection::Negative => "negative",
        }
    }

    pub fn step(self) -> i32 {
        match self {
            AxisDirection::Positive => 1,
            AxisDirection::Negative => -1,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            AxisDirection::Positive => "Towards positive",
            AxisDirection::Negative => "Towards negative",
        }
    }

    pub fn opposite(self) -> Self {
        match self {
            AxisDirection::Positive => AxisDirection::Negative,
            AxisDirection::Negative => AxisDirection::Positive,
        }
    }
}

impl Plane {
    pub fn name(self) -> &'static str {
        match self {
            Plane::Horizontal => "horizontal",
            Plane::Vertical => "vertical",
        }
    }
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for Axis {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl fmt::Display for AxisDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

fn java_abs_i32(value: i32) -> i32 {
    if value == i32::MIN {
        i32::MIN
    } else {
        value.abs()
    }
}
