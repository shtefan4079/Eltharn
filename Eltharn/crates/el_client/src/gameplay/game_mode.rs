use el_protocol::packets::game as protocol_game;
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum GameType {
    Survival,
    Creative,
    Adventure,
    Spectator,
}
impl GameType {
    pub const DEFAULT_MODE: Self = Self::Survival;
    pub fn id(self) -> i32 {
        match self {
            Self::Survival => 0,
            Self::Creative => 1,
            Self::Adventure => 2,
            Self::Spectator => 3,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Survival => "survival",
            Self::Creative => "creative",
            Self::Adventure => "adventure",
            Self::Spectator => "spectator",
        }
    }
    pub fn by_id(id: i32) -> Self {
        match id {
            1 => Self::Creative,
            2 => Self::Adventure,
            3 => Self::Spectator,
            _ => Self::Survival,
        }
    }
    pub fn by_name(name: &str) -> Option<Self> {
        match name {
            "survival" | "s" | "0" => Some(Self::Survival),
            "creative" | "c" | "1" => Some(Self::Creative),
            "adventure" | "a" | "2" => Some(Self::Adventure),
            "spectator" | "sp" | "3" => Some(Self::Spectator),
            _ => None,
        }
    }
    pub fn nullable_id(mode: Option<Self>) -> i32 {
        mode.map(Self::id).unwrap_or(-1)
    }
    pub fn by_nullable_id(id: i32) -> Option<Self> {
        if id == -1 {
            None
        } else {
            Some(Self::by_id(id))
        }
    }
    pub fn is_valid_id(id: i32) -> bool {
        (0..=3).contains(&id)
    }
    pub fn is_block_placing_restricted(self) -> bool {
        matches!(self, Self::Adventure | Self::Spectator)
    }
    pub fn is_creative(self) -> bool {
        matches!(self, Self::Creative)
    }
    pub fn is_survival(self) -> bool {
        matches!(self, Self::Survival | Self::Adventure)
    }
    pub fn update_player_abilities(self, abilities: &mut Abilities) {
        match self {
            Self::Creative => {
                abilities.mayfly = true;
                abilities.instabuild = true;
                abilities.invulnerable = true;
            }
            Self::Spectator => {
                abilities.mayfly = true;
                abilities.instabuild = false;
                abilities.invulnerable = true;
                abilities.flying = true;
            }
            Self::Survival | Self::Adventure => {
                abilities.mayfly = false;
                abilities.instabuild = false;
                abilities.invulnerable = false;
                abilities.flying = false;
            }
        }
        abilities.may_build = !self.is_block_placing_restricted();
    }
}
impl From<protocol_game::GameType> for GameType {
    fn from(value: protocol_game::GameType) -> Self {
        match value {
            protocol_game::GameType::Survival => Self::Survival,
            protocol_game::GameType::Creative => Self::Creative,
            protocol_game::GameType::Adventure => Self::Adventure,
            protocol_game::GameType::Spectator => Self::Spectator,
        }
    }
}
impl From<GameType> for protocol_game::GameType {
    fn from(value: GameType) -> Self {
        match value {
            GameType::Survival => Self::Survival,
            GameType::Creative => Self::Creative,
            GameType::Adventure => Self::Adventure,
            GameType::Spectator => Self::Spectator,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Abilities {
    pub invulnerable: bool,
    pub flying: bool,
    pub mayfly: bool,
    pub instabuild: bool,
    pub may_build: bool,
    pub flying_speed: f32,
    pub walking_speed: f32,
}
impl Default for Abilities {
    fn default() -> Self {
        Self {
            invulnerable: false,
            flying: false,
            mayfly: false,
            instabuild: false,
            may_build: true,
            flying_speed: 0.05,
            walking_speed: 0.1,
        }
    }
}
