pub mod legacy;
pub mod mobs;
pub mod objects;
pub mod tree;
pub mod types;
pub use mobs::guardian::*;
pub use mobs::sheep::*;
pub use objects::end_crystal::*;
pub use types::*;
pub fn vanilla_entity_model_kind(entity_type_path: &str) -> Option<VanillaEntityModelKind> {
    Some(match entity_type_path {
        "acacia_boat" | "birch_boat" | "cherry_boat" | "dark_oak_boat" | "jungle_boat"
        | "mangrove_boat" | "oak_boat" | "pale_oak_boat" | "spruce_boat" => {
            VanillaEntityModelKind::Boat
        }
        "acacia_chest_boat"
        | "birch_chest_boat"
        | "cherry_chest_boat"
        | "dark_oak_chest_boat"
        | "jungle_chest_boat"
        | "mangrove_chest_boat"
        | "oak_chest_boat"
        | "pale_oak_chest_boat"
        | "spruce_chest_boat" => VanillaEntityModelKind::ChestBoat,
        "bamboo_raft" => VanillaEntityModelKind::Raft,
        "bamboo_chest_raft" => VanillaEntityModelKind::ChestRaft,
        "allay" => VanillaEntityModelKind::Allay,
        "armadillo" => VanillaEntityModelKind::Armadillo,
        "armor_stand" => VanillaEntityModelKind::ArmorStand,
        "arrow" | "spectral_arrow" => VanillaEntityModelKind::Arrow,
        "bat" => VanillaEntityModelKind::Bat,
        "bogged" => VanillaEntityModelKind::Bogged,
        "cave_spider" => VanillaEntityModelKind::CaveSpider,
        "leash_knot" => VanillaEntityModelKind::LeashKnot,
        "llama_spit" => VanillaEntityModelKind::LlamaSpit,
        "chest_minecart"
        | "command_block_minecart"
        | "furnace_minecart"
        | "hopper_minecart"
        | "minecart"
        | "spawner_minecart"
        | "tnt_minecart" => VanillaEntityModelKind::Minecart,
        "chicken" => VanillaEntityModelKind::Chicken,
        "cod" => VanillaEntityModelKind::Cod,
        "cow" | "mooshroom" => VanillaEntityModelKind::Cow,
        "creaking" => VanillaEntityModelKind::Creaking,
        "creeper" => VanillaEntityModelKind::Creeper,
        "drowned" => VanillaEntityModelKind::Drowned,
        "end_crystal" => VanillaEntityModelKind::EndCrystal,
        "glow_squid" => VanillaEntityModelKind::Squid,
        "iron_golem" => VanillaEntityModelKind::IronGolem,
        "magma_cube" => VanillaEntityModelKind::MagmaCube,
        "pig" => VanillaEntityModelKind::Pig,
        "piglin" | "piglin_brute" => VanillaEntityModelKind::Piglin,
        "zombified_piglin" => VanillaEntityModelKind::ZombifiedPiglin,
        "pufferfish" => VanillaEntityModelKind::Pufferfish,
        "rabbit" => VanillaEntityModelKind::Rabbit,
        "salmon" => VanillaEntityModelKind::Salmon,
        "sheep" => VanillaEntityModelKind::Sheep,
        "skeleton" | "stray" | "parched" => VanillaEntityModelKind::Skeleton,
        "slime" => VanillaEntityModelKind::Slime,
        "snow_golem" => VanillaEntityModelKind::SnowGolem,
        "spider" => VanillaEntityModelKind::Spider,
        "squid" => VanillaEntityModelKind::Squid,
        "sulfur_cube" => VanillaEntityModelKind::SulfurCube,
        "tadpole" => VanillaEntityModelKind::Tadpole,
        "trident" => VanillaEntityModelKind::Trident,
        "zombie" | "giant" | "husk" => VanillaEntityModelKind::Zombie,
        "zombie_villager" => VanillaEntityModelKind::ZombieVillager,
        "fox" => VanillaEntityModelKind::Fox,
        "strider" => VanillaEntityModelKind::Strider,
        "camel" | "camel_husk" => VanillaEntityModelKind::Camel,
        "nautilus" | "zombie_nautilus" => VanillaEntityModelKind::Nautilus,
        "wither_skeleton" => VanillaEntityModelKind::WitherSkeleton,
        "blaze" => VanillaEntityModelKind::Blaze,
        "breeze_wind_charge" => VanillaEntityModelKind::BreezeWindCharge,
        "endermite" => VanillaEntityModelKind::Endermite,
        "ghast" => VanillaEntityModelKind::Ghast,
        "happy_ghast" => VanillaEntityModelKind::HappyGhast,
        "shulker_bullet" => VanillaEntityModelKind::ShulkerBullet,
        "silverfish" => VanillaEntityModelKind::Silverfish,
        "wind_charge" => VanillaEntityModelKind::WindCharge,
        "wither_skull" => VanillaEntityModelKind::WitherSkull,
        "bee" => VanillaEntityModelKind::Bee,
        "wolf" => VanillaEntityModelKind::Wolf,
        "axolotl" => VanillaEntityModelKind::Axolotl,
        "cat" => VanillaEntityModelKind::Cat,
        "dolphin" => VanillaEntityModelKind::Dolphin,
        "evoker" => VanillaEntityModelKind::Evoker,
        "evoker_fangs" => VanillaEntityModelKind::EvokerFangs,
        "frog" => VanillaEntityModelKind::Frog,
        "goat" => VanillaEntityModelKind::Goat,
        "guardian" | "elder_guardian" => VanillaEntityModelKind::Guardian,
        "hoglin" => VanillaEntityModelKind::Hoglin,
        "illusioner" => VanillaEntityModelKind::Illusioner,
        "llama" => VanillaEntityModelKind::Llama,
        "ocelot" => VanillaEntityModelKind::Ocelot,
        "panda" => VanillaEntityModelKind::Panda,
        "parrot" => VanillaEntityModelKind::Parrot,
        "phantom" => VanillaEntityModelKind::Phantom,
        "pillager" => VanillaEntityModelKind::Pillager,
        "polar_bear" => VanillaEntityModelKind::PolarBear,
        "shulker" => VanillaEntityModelKind::Shulker,
        "sniffer" => VanillaEntityModelKind::Sniffer,
        "trader_llama" => VanillaEntityModelKind::TraderLlama,
        "tropical_fish" => VanillaEntityModelKind::TropicalFish,
        "villager" => VanillaEntityModelKind::Villager,
        "vindicator" => VanillaEntityModelKind::Vindicator,
        "wandering_trader" => VanillaEntityModelKind::WanderingTrader,
        "witch" => VanillaEntityModelKind::Witch,
        "zoglin" => VanillaEntityModelKind::Zoglin,
        "ravager" => VanillaEntityModelKind::Ravager,
        "turtle" => VanillaEntityModelKind::Turtle,
        "donkey" | "mule" => VanillaEntityModelKind::Donkey,
        "horse" => VanillaEntityModelKind::Horse,
        "skeleton_horse" => VanillaEntityModelKind::SkeletonHorse,
        "zombie_horse" => VanillaEntityModelKind::ZombieHorse,
        "warden" => VanillaEntityModelKind::Warden,
        "breeze" => VanillaEntityModelKind::Breeze,
        "enderman" => VanillaEntityModelKind::Enderman,
        "vex" => VanillaEntityModelKind::Vex,
        "wither" => VanillaEntityModelKind::Wither,
        "ender_dragon" => VanillaEntityModelKind::EnderDragon,
        _ => return None,
    })
}
