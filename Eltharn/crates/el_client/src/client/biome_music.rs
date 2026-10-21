use crate::client::music_manager::{BackgroundMusic, Music};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BiomeMusic {
    pub background: BackgroundMusic,
    pub music_volume: Option<f32>,
}
const fn game_music(sound: &'static str) -> Music {
    Music {
        sound,
        min_delay: 12000,
        max_delay: 24000,
        replace_current_music: false,
    }
}
const OVERWORLD_WATER_MUSIC: BiomeMusic = BiomeMusic {
    background: BackgroundMusic {
        default_music: Some(game_music("minecraft:music.game")),
        creative_music: Some(game_music("minecraft:music.creative")),
        underwater_music: Some(game_music("minecraft:music.under_water")),
    },
    music_volume: None,
};
const fn single_track(sound: &'static str) -> BiomeMusic {
    BiomeMusic {
        background: BackgroundMusic {
            default_music: Some(game_music(sound)),
            creative_music: None,
            underwater_music: None,
        },
        music_volume: None,
    }
}
pub fn biome_music(biome_id: &str) -> Option<BiomeMusic> {
    match biome_id {
        "minecraft:badlands" | "minecraft:eroded_badlands" | "minecraft:wooded_badlands" => {
            Some(single_track("minecraft:music.overworld.badlands"))
        }
        "minecraft:bamboo_jungle" => Some(single_track("minecraft:music.overworld.bamboo_jungle")),
        "minecraft:basalt_deltas" => Some(single_track("minecraft:music.nether.basalt_deltas")),
        "minecraft:birch_forest"
        | "minecraft:dark_forest"
        | "minecraft:forest"
        | "minecraft:old_growth_birch_forest" => {
            Some(single_track("minecraft:music.overworld.forest"))
        }
        "minecraft:cherry_grove" => Some(single_track("minecraft:music.overworld.cherry_grove")),
        "minecraft:cold_ocean"
        | "minecraft:deep_cold_ocean"
        | "minecraft:deep_lukewarm_ocean"
        | "minecraft:deep_ocean"
        | "minecraft:frozen_river"
        | "minecraft:lukewarm_ocean"
        | "minecraft:ocean"
        | "minecraft:river"
        | "minecraft:warm_ocean" => Some(OVERWORLD_WATER_MUSIC),
        "minecraft:crimson_forest" => Some(single_track("minecraft:music.nether.crimson_forest")),
        "minecraft:deep_dark" => Some(single_track("minecraft:music.overworld.deep_dark")),
        "minecraft:desert" => Some(single_track("minecraft:music.overworld.desert")),
        "minecraft:dripstone_caves" => {
            Some(single_track("minecraft:music.overworld.dripstone_caves"))
        }
        "minecraft:flower_forest" => Some(single_track("minecraft:music.overworld.flower_forest")),
        "minecraft:frozen_peaks" => Some(single_track("minecraft:music.overworld.frozen_peaks")),
        "minecraft:grove" => Some(single_track("minecraft:music.overworld.grove")),
        "minecraft:jagged_peaks" => Some(single_track("minecraft:music.overworld.jagged_peaks")),
        "minecraft:jungle" => Some(single_track("minecraft:music.overworld.jungle")),
        "minecraft:lush_caves" => Some(single_track("minecraft:music.overworld.lush_caves")),
        "minecraft:mangrove_swamp" | "minecraft:swamp" => {
            Some(single_track("minecraft:music.overworld.swamp"))
        }
        "minecraft:meadow" => Some(single_track("minecraft:music.overworld.meadow")),
        "minecraft:nether_wastes" => Some(single_track("minecraft:music.nether.nether_wastes")),
        "minecraft:old_growth_pine_taiga" | "minecraft:old_growth_spruce_taiga" => {
            Some(single_track("minecraft:music.overworld.old_growth_taiga"))
        }
        "minecraft:pale_garden" => Some(BiomeMusic {
            background: BackgroundMusic {
                default_music: None,
                creative_music: None,
                underwater_music: None,
            },
            music_volume: Some(0.0),
        }),
        "minecraft:snowy_slopes" => Some(single_track("minecraft:music.overworld.snowy_slopes")),
        "minecraft:soul_sand_valley" => {
            Some(single_track("minecraft:music.nether.soul_sand_valley"))
        }
        "minecraft:sparse_jungle" => Some(single_track("minecraft:music.overworld.sparse_jungle")),
        "minecraft:stony_peaks" => Some(single_track("minecraft:music.overworld.stony_peaks")),
        "minecraft:sulfur_caves" => Some(single_track("minecraft:music.overworld.sulfur_caves")),
        "minecraft:warped_forest" => Some(single_track("minecraft:music.nether.warped_forest")),
        _ => None,
    }
}
