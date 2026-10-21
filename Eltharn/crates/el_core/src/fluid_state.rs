use crate::block_state::BlockState;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct FluidState {
    pub kind: FluidKind,
    pub amount: u8,
    pub falling: bool,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum FluidKind {
    #[default]
    Empty,
    Water,
    Lava,
}
impl FluidState {
    pub const EMPTY: Self = Self {
        kind: FluidKind::Empty,
        amount: 0,
        falling: false,
    };
    pub const fn source(kind: FluidKind) -> Self {
        Self {
            kind,
            amount: 8,
            falling: false,
        }
    }
    pub const fn flowing(kind: FluidKind, amount: u8, falling: bool) -> Self {
        Self {
            kind,
            amount: if amount > 8 { 8 } else { amount },
            falling,
        }
    }
    pub fn from_block_state(state: &BlockState) -> Self {
        let kind = match state.block.path.as_str() {
            "water" | "bubble_column" => FluidKind::Water,
            "lava" => FluidKind::Lava,
            path if is_water_containing_plant(path) => {
                if state
                    .get_property("waterlogged")
                    .is_some_and(|value| value == "false")
                {
                    return Self::EMPTY;
                }
                FluidKind::Water
            }
            _ if state
                .get_property("waterlogged")
                .is_some_and(|value| value == "true") =>
            {
                FluidKind::Water
            }
            _ => return Self::EMPTY,
        };
        if state.block.path == "bubble_column"
            || is_water_containing_plant(state.block.path.as_str())
            || (state.block.path != "water" && state.block.path != "lava")
        {
            return Self::source(kind);
        }
        let legacy_level = state
            .get_property("level")
            .and_then(|value| value.parse::<u8>().ok())
            .unwrap_or(0)
            .min(15);
        if legacy_level == 0 {
            Self::source(kind)
        } else if legacy_level >= 8 {
            Self::flowing(kind, 8, true)
        } else {
            Self::flowing(kind, 8 - legacy_level, false)
        }
    }
    pub const fn is_empty(self) -> bool {
        matches!(self.kind, FluidKind::Empty)
    }
    pub const fn is_water(self) -> bool {
        matches!(self.kind, FluidKind::Water)
    }
    pub const fn is_lava(self) -> bool {
        matches!(self.kind, FluidKind::Lava)
    }
    pub const fn is_source(self) -> bool {
        !self.is_empty() && self.amount == 8 && !self.falling
    }
    pub const fn is_full(self) -> bool {
        self.amount == 8
    }
    pub fn own_height(self) -> f32 {
        self.amount as f32 / 9.0
    }
    pub fn height_with_same_fluid_above(self, above: Self) -> f32 {
        if !self.is_empty() && self.kind == above.kind {
            1.0
        } else {
            self.own_height()
        }
    }
    pub const fn tick_delay(self) -> u64 {
        match self.kind {
            FluidKind::Water => 5,
            FluidKind::Lava => 30,
            FluidKind::Empty => 0,
        }
    }
    pub const fn legacy_level(self) -> u8 {
        if self.is_empty() || self.is_source() {
            0
        } else {
            let amount = if self.amount > 8 { 8 } else { self.amount };
            8 - amount + if self.falling { 8 } else { 0 }
        }
    }
}
pub fn is_water_containing_plant(path: &str) -> bool {
    matches!(
        path,
        "kelp"
            | "kelp_plant"
            | "seagrass"
            | "tall_seagrass"
            | "sea_pickle"
            | "tube_coral"
            | "brain_coral"
            | "bubble_coral"
            | "fire_coral"
            | "horn_coral"
            | "tube_coral_fan"
            | "brain_coral_fan"
            | "bubble_coral_fan"
            | "fire_coral_fan"
            | "horn_coral_fan"
            | "tube_coral_wall_fan"
            | "brain_coral_wall_fan"
            | "bubble_coral_wall_fan"
            | "fire_coral_wall_fan"
            | "horn_coral_wall_fan"
            | "dead_tube_coral"
            | "dead_brain_coral"
            | "dead_bubble_coral"
            | "dead_fire_coral"
            | "dead_horn_coral"
            | "dead_tube_coral_fan"
            | "dead_brain_coral_fan"
            | "dead_bubble_coral_fan"
            | "dead_fire_coral_fan"
            | "dead_horn_coral_fan"
            | "dead_tube_coral_wall_fan"
            | "dead_brain_coral_wall_fan"
            | "dead_bubble_coral_wall_fan"
            | "dead_fire_coral_wall_fan"
            | "dead_horn_coral_wall_fan"
    )
}
