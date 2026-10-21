use crate::render::texture::{SpriteUvRect, TextureAtlas};
use crate::resources::manager::ClientResourceManager;
use el_core::block_behaviour::java_26_2_vanilla_block_lighting_properties;
use el_core::block_state::BlockState;
use el_core::collision_context::CollisionContext;
use el_core::resource_location::ResourceLocation;
use el_math::Direction;
use el_math::block_pos::BlockPos;
use el_math::random::{LegacyRandomSource, RandomSource};
use std::sync::atomic::{AtomicI64, Ordering};
const JAVA_RANDOM_SEED_UNIQUIFIER_MULTIPLIER: i64 = 1_181_783_497_276_652_981;
static JAVA_PARTICLE_SEED_UNIQUIFIER: AtomicI64 = AtomicI64::new(8_682_522_807_148_012);
pub(crate) fn java_unique_particle_seed() -> i64 {
    let mut current = JAVA_PARTICLE_SEED_UNIQUIFIER.load(Ordering::SeqCst);
    loop {
        let next = current.wrapping_mul(JAVA_RANDOM_SEED_UNIQUIFIER_MULTIPLIER);
        match JAVA_PARTICLE_SEED_UNIQUIFIER.compare_exchange_weak(
            current,
            next,
            Ordering::SeqCst,
            Ordering::SeqCst,
        ) {
            Ok(_) => {
                let nanos = web_time::SystemTime::now()
                    .duration_since(web_time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos() as i64)
                    .unwrap_or(0);
                return next ^ nanos;
            }
            Err(actual) => current = actual,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TorchFlameKind {
    Normal,
    Soul,
    Copper,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TorchParticleKind {
    Smoke,
    Flame(TorchFlameKind),
    RedstoneDust,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ParticleBehavior {
    Generic,
    Rain,
    TorchSmoke,
    TorchFlame,
    RedstoneDust,
    Portal,
    ReversePortal,
    CurrentDown,
    Bubble,
    SuspendedTown,
    CampfireSmoke,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NetworkParticleKind {
    Poof,
    Flame,
    SoulFlame,
    SmallFlame,
    Smoke,
    LargeSmoke,
    WhiteSmoke,
    Crit,
    EnchantedHit,
    DamageIndicator,
    EndRod,
    Heart,
    AngryVillager,
    Note,
    Splash,
    Dust,
    Block,
    Lava,
    Portal,
    ReversePortal,
    Bubble,
    BubbleColumnUp,
    CurrentDown,
    BubblePop,
    Soul,
    SculkSoul,
    Snowflake,
    Spit,
    Cloud,
    Sneeze,
    Mycelium,
    Composter,
    Dolphin,
    EggCrack,
    HappyVillager,
    Underwater,
    CrimsonSpore,
    WarpedSpore,
    SporeBlossomAir,
    Ash,
    WhiteAsh,
    CampfireCosySmoke,
    CampfireSignalSmoke,
    Flash,
    Effect,
    EntityEffect,
    DragonBreath,
    SculkCharge,
    Trail,
    BlockMarker,
    FallingDust,
    DustPillar,
    BlockCrumble,
    DustColorTransition,
    Witch,
    Shriek,
    Vibration,
    GeyserBase,
    GeyserPlume,
    Glow,
    Totem,
    SquidInk,
    CopperFlame,
    DustPlume,
    Enchant,
    Nautilus,
    VaultConnection,
    CherryLeaves,
    PaleOakLeaves,
    TintedLeaves,
    SweepAttack,
    SculkChargePop,
    Infested,
    RaidOmen,
    TrialOmen,
    PauseMobGrowth,
    ResetMobGrowth,
    Fishing,
    FireworkSpark,
    Explosion,
    SonicBoom,
    Gust,
    SmallGust,
    SulfurBubbles,
    NoxiousGas,
    OminousSpawning,
    TrialSpawnerDetection,
    TrialSpawnerDetectionOminous,
    Firefly,
    Rain,
    DrippingWater,
    DrippingDripstoneWater,
    DrippingHoney,
    DrippingObsidianTear,
    DrippingLava,
    DrippingDripstoneLava,
    FallingWater,
    FallingDripstoneWater,
    FallingHoney,
    FallingObsidianTear,
    FallingLava,
    FallingDripstoneLava,
    FallingNectar,
    FallingSporeBlossom,
    LandingLava,
    LandingHoney,
    LandingObsidianTear,
    SulfurCubeGoo,
    ExplosionEmitter,
    GustEmitterLarge,
    GustEmitterSmall,
    NoxiousGasCloud,
    GeyserEruption,
    ElderGuardian,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct TorchParticleSpawn {
    kind: TorchParticleKind,
    position: glam::Vec3,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ParticleTextureType {
    ParticleAtlas,
    BlockAtlas,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct FallingLeavesState {
    wind_big: f32,
    swirl: bool,
    flow_away: bool,
    xa_flow_scale: f32,
    za_flow_scale: f32,
    swirl_period: f32,
    rot_speed: f32,
    spin_acceleration: f32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct SulfurBubbleState {
    y_start: f32,
    y_end: f32,
    size_start: f32,
    y_prev: f32,
}
pub struct Particle {
    pub position: glam::Vec3,
    previous_position: glam::Vec3,
    pub velocity: glam::Vec3,
    pub color: [f32; 4],
    pub light: [f32; 2],
    pub scale: f32,
    pub age: i32,
    pub max_age: i32,
    pub gravity: f32,
    pub u0: f32,
    pub v0: f32,
    pub u1: f32,
    pub v1: f32,
    pub texture_type: ParticleTextureType,
    friction: f32,
    behavior: ParticleBehavior,
    network_kind: Option<NetworkParticleKind>,
    alpha: f32,
    fade_color: Option<[f32; 3]>,
    has_physics: bool,
    speed_up_when_y_motion_is_blocked: bool,
    counts_down_lifetime: bool,
    origin: Option<glam::Vec3>,
    angle: f32,
    target: Option<glam::Vec3>,
    delay: i32,
    translucent: bool,
    geyser_plume_max_y: f32,
    geyser_plume_propulsion: f32,
    geyser_plume_spray: [f32; 2],
    geyser_plume_size: [f32; 2],
    leaves: Option<FallingLeavesState>,
    sulfur_bubble: Option<SulfurBubbleState>,
    geyser_eruption_water_blocks: i32,
}
#[derive(Clone, Copy, Debug, PartialEq)]
struct TrackingEmitterTarget {
    position: glam::Vec3,
    width: f32,
    height: f32,
}
struct EntityTrackingEmitter {
    entity_id: i32,
    particle: el_protocol_backend::BackendParticle,
    target: TrackingEmitterTarget,
    additional_ticks: usize,
    random: LegacyRandomSource,
}
impl EntityTrackingEmitter {
    fn new(
        entity_id: i32,
        particle: el_protocol_backend::BackendParticle,
        target: TrackingEmitterTarget,
        seed: i64,
        lifetime: usize,
    ) -> Self {
        Self {
            entity_id,
            particle,
            target,
            additional_ticks: lifetime.saturating_sub(1),
            random: LegacyRandomSource::new(seed),
        }
    }
    fn emit_tick(&mut self) -> Vec<(glam::Vec3, glam::Vec3)> {
        let mut samples = Vec::with_capacity(16);
        for _ in 0..16 {
            let xa = self.random.next_float() * 2.0 - 1.0;
            let ya = self.random.next_float() * 2.0 - 1.0;
            let za = self.random.next_float() * 2.0 - 1.0;
            if xa * xa + ya * ya + za * za <= 1.0 {
                let target = self.target;
                let position = glam::Vec3::new(
                    target.position.x + target.width * (xa / 4.0),
                    target.position.y + target.height * (0.5 + ya / 4.0),
                    target.position.z + target.width * (za / 4.0),
                );
                samples.push((position, glam::Vec3::new(xa, ya + 0.2, za)));
            }
        }
        samples
    }
    fn advance_tick(
        &mut self,
        target: Option<TrackingEmitterTarget>,
    ) -> Option<Vec<(glam::Vec3, glam::Vec3)>> {
        if self.additional_ticks == 0 {
            return None;
        }
        if let Some(target) = target {
            self.target = target;
        }
        let samples = self.emit_tick();
        self.additional_ticks -= 1;
        Some(samples)
    }
}
fn java_animate_tick_position<R: RandomSource + ?Sized>(
    center: BlockPos,
    radius: i32,
    random: &mut R,
) -> BlockPos {
    BlockPos::new(
        center.x + random.next_int_bound(radius) - random.next_int_bound(radius),
        center.y + random.next_int_bound(radius) - random.next_int_bound(radius),
        center.z + random.next_int_bound(radius) - random.next_int_bound(radius),
    )
}
fn for_each_java_torch_animate_tick_sample<R, F>(center: BlockPos, random: &mut R, mut visitor: F)
where
    R: RandomSource + ?Sized,
    F: FnMut(BlockPos, &mut R),
{
    for _ in 0..667 {
        visitor(java_animate_tick_position(center, 16, random), random);
        visitor(java_animate_tick_position(center, 32, random), random);
    }
}
fn torch_particle_spawns(state: &BlockState, pos: BlockPos) -> Option<[TorchParticleSpawn; 2]> {
    if state.block.namespace != "minecraft" {
        return None;
    }
    let (flame, is_wall) = match state.block.path.as_str() {
        "torch" => (TorchFlameKind::Normal, false),
        "wall_torch" => (TorchFlameKind::Normal, true),
        "soul_torch" => (TorchFlameKind::Soul, false),
        "soul_wall_torch" => (TorchFlameKind::Soul, true),
        "copper_torch" => (TorchFlameKind::Copper, false),
        "copper_wall_torch" => (TorchFlameKind::Copper, true),
        _ => return None,
    };
    let mut particle_pos =
        glam::Vec3::new(pos.x as f32 + 0.5, pos.y as f32 + 0.7, pos.z as f32 + 0.5);
    if is_wall {
        let facing = match state.get_property("facing") {
            Some(value) => Direction::from_name(value)?,
            None => Direction::North,
        };
        if matches!(facing, Direction::Down | Direction::Up) {
            return None;
        }
        let opposite = facing.opposite();
        particle_pos.x += 0.27 * opposite.step_x() as f32;
        particle_pos.y += 0.22;
        particle_pos.z += 0.27 * opposite.step_z() as f32;
    }
    Some([
        TorchParticleSpawn {
            kind: TorchParticleKind::Smoke,
            position: particle_pos,
        },
        TorchParticleSpawn {
            kind: TorchParticleKind::Flame(flame),
            position: particle_pos,
        },
    ])
}
fn redstone_torch_particle_spawn<R: RandomSource + ?Sized>(
    state: &BlockState,
    pos: BlockPos,
    random: &mut R,
) -> Option<TorchParticleSpawn> {
    if state.block.namespace != "minecraft"
        || !matches!(
            state.block.path.as_str(),
            "redstone_torch" | "redstone_wall_torch"
        )
        || state
            .get_property("lit")
            .is_some_and(|value| value.eq_ignore_ascii_case("false"))
    {
        return None;
    }
    Some(TorchParticleSpawn {
        kind: TorchParticleKind::RedstoneDust,
        position: glam::Vec3::new(
            pos.x as f32 + 0.5 + (random.next_double() as f32 - 0.5) * 0.2,
            pos.y as f32 + 0.7 + (random.next_double() as f32 - 0.5) * 0.2,
            pos.z as f32 + 0.5 + (random.next_double() as f32 - 0.5) * 0.2,
        ),
    })
}
fn particle_sprite_id(kind: TorchParticleKind, age: i32, max_age: i32) -> ResourceLocation {
    match kind {
        TorchParticleKind::Smoke => {
            let lifetime = max_age.max(1);
            let source_index = (age * 7 / lifetime).clamp(0, 7);
            ResourceLocation::new(
                "minecraft",
                &format!("particle/generic_{}", 7 - source_index),
            )
        }
        TorchParticleKind::Flame(TorchFlameKind::Normal) => {
            ResourceLocation::new("minecraft", "particle/flame")
        }
        TorchParticleKind::Flame(TorchFlameKind::Soul) => {
            ResourceLocation::new("minecraft", "particle/soul_fire_flame")
        }
        TorchParticleKind::Flame(TorchFlameKind::Copper) => {
            ResourceLocation::new("minecraft", "particle/copper_fire_flame")
        }
        TorchParticleKind::RedstoneDust => ResourceLocation::new("minecraft", "particle/generic_7"),
    }
}
fn java_base_single_quad_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let base_max_age = (4.0 / (random.next_float() * 0.9 + 0.1)) as i32;
    let mut velocity = glam::Vec3::new(
        requested_velocity.x + (random.next_float() * 2.0 - 1.0) * 0.4,
        requested_velocity.y + (random.next_float() * 2.0 - 1.0) * 0.4,
        requested_velocity.z + (random.next_float() * 2.0 - 1.0) * 0.4,
    );
    let speed = (random.next_float() + random.next_float() + 1.0) * 0.15;
    velocity = velocity / velocity.length() * speed * 0.4;
    velocity.y += 0.1;
    Particle {
        position,
        previous_position: position,
        velocity,
        color: [1.0, 1.0, 1.0, 1.0],
        light: [1.0, 1.0],
        scale: 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0,
        age: 0,
        max_age: base_max_age,
        gravity: 0.0,
        u0: rect.u0,
        v0: rect.v0,
        u1: rect.u1,
        v1: rect.v1,
        texture_type: ParticleTextureType::ParticleAtlas,
        friction: 0.98,
        behavior: ParticleBehavior::Generic,
        network_kind: None,
        alpha: 1.0,
        fade_color: None,
        has_physics: true,
        speed_up_when_y_motion_is_blocked: false,
        counts_down_lifetime: false,
        origin: None,
        angle: 0.0,
        target: None,
        delay: 0,
        translucent: false,
        geyser_plume_max_y: 0.0,
        geyser_plume_propulsion: 0.0,
        geyser_plume_spray: [0.0, 0.0],
        geyser_plume_size: [0.0, 0.0],
        leaves: None,
        sulfur_bubble: None,
        geyser_eruption_water_blocks: 0,
    }
}
fn java_single_quad_4arg_base<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let particle = Particle {
        position,
        previous_position: position,
        velocity: glam::Vec3::ZERO,
        color: [1.0, 1.0, 1.0, 1.0],
        light: [1.0, 1.0],
        scale: 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0,
        age: 0,
        max_age: (4.0 / (random.next_float() * 0.9 + 0.1)) as i32,
        gravity: 0.0,
        u0: rect.u0,
        v0: rect.v0,
        u1: rect.u1,
        v1: rect.v1,
        texture_type: ParticleTextureType::ParticleAtlas,
        friction: 0.98,
        behavior: ParticleBehavior::Generic,
        network_kind: None,
        alpha: 1.0,
        fade_color: None,
        has_physics: true,
        speed_up_when_y_motion_is_blocked: false,
        counts_down_lifetime: false,
        origin: None,
        angle: 0.0,
        target: None,
        delay: 0,
        translucent: false,
        geyser_plume_max_y: 0.0,
        geyser_plume_propulsion: 0.0,
        geyser_plume_spray: [0.0, 0.0],
        geyser_plume_size: [0.0, 0.0],
        leaves: None,
        sulfur_bubble: None,
        geyser_eruption_water_blocks: 0,
    };
    particle
}
fn java_torch_flame_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.friction = 0.96;
    particle.velocity = particle.velocity * 0.01 + requested_velocity;
    particle.position.x += (random.next_float() - random.next_float()) * 0.05;
    particle.position.y += (random.next_float() - random.next_float()) * 0.05;
    particle.position.z += (random.next_float() - random.next_float()) * 0.05;
    particle.max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32 + 4;
    particle.behavior = ParticleBehavior::TorchFlame;
    particle
}
fn java_torch_smoke_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.gravity = -0.1;
    particle.velocity *= 0.1;
    let color = random.next_float() * 0.3;
    particle.color = [color, color, color, 1.0];
    particle.scale *= 0.75;
    particle.max_age = ((8.0 / (random.next_float() * 0.8 + 0.2)) as i32).max(1);
    particle.behavior = ParticleBehavior::TorchSmoke;
    particle
}
fn java_redstone_dust_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.velocity *= 0.1;
    particle.scale *= 0.75;
    particle.max_age = (8.0 / (random.next_double() * 0.8 + 0.2)) as i32;
    particle.max_age = particle.max_age.max(1);
    particle.behavior = ParticleBehavior::RedstoneDust;
    let base_factor = random.next_float() * 0.4 + 0.6;
    let randomize =
        |channel: f32, random: &mut R| (random.next_float() * 0.2 + 0.8) * channel * base_factor;
    particle.color = [
        randomize(1.0, random),
        randomize(0.0, random),
        randomize(0.0, random),
        1.0,
    ];
    particle
}
fn particle_render_scale(
    behavior: ParticleBehavior,
    network_kind: Option<NetworkParticleKind>,
    scale: f32,
    age: i32,
    max_age: i32,
    partial_tick: f32,
) -> f32 {
    let render_age = age as f32 + partial_tick;
    let progress = render_age / max_age.max(1) as f32;
    match network_kind {
        Some(
            NetworkParticleKind::Flame
            | NetworkParticleKind::SoulFlame
            | NetworkParticleKind::SmallFlame
            | NetworkParticleKind::Soul
            | NetworkParticleKind::SculkSoul
            | NetworkParticleKind::CopperFlame,
        ) => scale * (1.0 - progress * progress * 0.5),
        Some(
            NetworkParticleKind::Smoke
            | NetworkParticleKind::LargeSmoke
            | NetworkParticleKind::Crit
            | NetworkParticleKind::EnchantedHit
            | NetworkParticleKind::DamageIndicator
            | NetworkParticleKind::Heart
            | NetworkParticleKind::AngryVillager
            | NetworkParticleKind::Note
            | NetworkParticleKind::Dust
            | NetworkParticleKind::DustColorTransition
            | NetworkParticleKind::Cloud
            | NetworkParticleKind::Sneeze
            | NetworkParticleKind::FallingDust
            | NetworkParticleKind::Ash
            | NetworkParticleKind::WhiteAsh
            | NetworkParticleKind::WhiteSmoke
            | NetworkParticleKind::DragonBreath
            | NetworkParticleKind::GeyserBase
            | NetworkParticleKind::DustPlume
            | NetworkParticleKind::NoxiousGas
            | NetworkParticleKind::TrialSpawnerDetection
            | NetworkParticleKind::TrialSpawnerDetectionOminous,
        ) => scale * (progress * 32.0).clamp(0.0, 1.0),
        Some(NetworkParticleKind::Lava) => scale * (1.0 - progress * progress),
        Some(NetworkParticleKind::Portal) => {
            let s = 1.0 - progress;
            scale * (1.0 - s * s)
        }
        Some(NetworkParticleKind::ReversePortal) => scale * (1.0 - progress / 1.5),
        Some(NetworkParticleKind::Shriek) => scale * (progress * 0.75).clamp(0.0, 1.0),
        Some(NetworkParticleKind::Flash) => {
            let s = ((render_age - 1.0) * 0.25 * std::f32::consts::PI).sin();
            scale * s.max(0.0)
        }
        Some(
            NetworkParticleKind::Poof
            | NetworkParticleKind::EndRod
            | NetworkParticleKind::Splash
            | NetworkParticleKind::Block
            | NetworkParticleKind::BlockMarker
            | NetworkParticleKind::Trail
            | NetworkParticleKind::Spit
            | NetworkParticleKind::Bubble
            | NetworkParticleKind::BubbleColumnUp
            | NetworkParticleKind::CurrentDown
            | NetworkParticleKind::BubblePop
            | NetworkParticleKind::Mycelium
            | NetworkParticleKind::Composter
            | NetworkParticleKind::Dolphin
            | NetworkParticleKind::EggCrack
            | NetworkParticleKind::HappyVillager
            | NetworkParticleKind::Underwater
            | NetworkParticleKind::CrimsonSpore
            | NetworkParticleKind::WarpedSpore
            | NetworkParticleKind::SporeBlossomAir
            | NetworkParticleKind::CampfireCosySmoke
            | NetworkParticleKind::CampfireSignalSmoke
            | NetworkParticleKind::Effect
            | NetworkParticleKind::EntityEffect
            | NetworkParticleKind::Witch
            | NetworkParticleKind::SculkCharge
            | NetworkParticleKind::Snowflake
            | NetworkParticleKind::Vibration
            | NetworkParticleKind::GeyserPlume
            | NetworkParticleKind::Glow
            | NetworkParticleKind::Totem
            | NetworkParticleKind::SquidInk
            | NetworkParticleKind::DustPillar
            | NetworkParticleKind::BlockCrumble
            | NetworkParticleKind::Enchant
            | NetworkParticleKind::Nautilus
            | NetworkParticleKind::VaultConnection
            | NetworkParticleKind::CherryLeaves
            | NetworkParticleKind::PaleOakLeaves
            | NetworkParticleKind::TintedLeaves
            | NetworkParticleKind::SweepAttack
            | NetworkParticleKind::SculkChargePop
            | NetworkParticleKind::Infested
            | NetworkParticleKind::RaidOmen
            | NetworkParticleKind::TrialOmen
            | NetworkParticleKind::PauseMobGrowth
            | NetworkParticleKind::ResetMobGrowth
            | NetworkParticleKind::Fishing
            | NetworkParticleKind::FireworkSpark
            | NetworkParticleKind::Explosion
            | NetworkParticleKind::SonicBoom
            | NetworkParticleKind::Gust
            | NetworkParticleKind::SmallGust
            | NetworkParticleKind::SulfurBubbles
            | NetworkParticleKind::OminousSpawning
            | NetworkParticleKind::Firefly
            | NetworkParticleKind::Rain
            | NetworkParticleKind::DrippingWater
            | NetworkParticleKind::DrippingDripstoneWater
            | NetworkParticleKind::DrippingHoney
            | NetworkParticleKind::DrippingObsidianTear
            | NetworkParticleKind::DrippingLava
            | NetworkParticleKind::DrippingDripstoneLava
            | NetworkParticleKind::FallingWater
            | NetworkParticleKind::FallingDripstoneWater
            | NetworkParticleKind::FallingHoney
            | NetworkParticleKind::FallingObsidianTear
            | NetworkParticleKind::FallingLava
            | NetworkParticleKind::FallingDripstoneLava
            | NetworkParticleKind::FallingNectar
            | NetworkParticleKind::FallingSporeBlossom
            | NetworkParticleKind::LandingLava
            | NetworkParticleKind::LandingHoney
            | NetworkParticleKind::LandingObsidianTear
            | NetworkParticleKind::SulfurCubeGoo
            | NetworkParticleKind::ExplosionEmitter
            | NetworkParticleKind::GustEmitterLarge
            | NetworkParticleKind::GustEmitterSmall
            | NetworkParticleKind::NoxiousGasCloud
            | NetworkParticleKind::GeyserEruption
            | NetworkParticleKind::ElderGuardian,
        ) => scale,
        None => match behavior {
            ParticleBehavior::TorchFlame => scale * (1.0 - progress * progress * 0.5),
            ParticleBehavior::TorchSmoke | ParticleBehavior::RedstoneDust => {
                scale * (progress * 32.0).clamp(0.0, 1.0)
            }
            ParticleBehavior::Generic
            | ParticleBehavior::Rain
            | ParticleBehavior::Portal
            | ParticleBehavior::ReversePortal
            | ParticleBehavior::CurrentDown
            | ParticleBehavior::Bubble
            | ParticleBehavior::SuspendedTown
            | ParticleBehavior::CampfireSmoke => scale,
        },
    }
}
fn java_particle_light_coords(
    level: &crate::world::client_level::ClientLevel,
    position: glam::Vec3,
) -> [u8; 2] {
    let pos = BlockPos::new(
        position.x.floor() as i32,
        position.y.floor() as i32,
        position.z.floor() as i32,
    );
    if !level.has_chunk_at(pos) {
        return [0, 240];
    }
    let state = level.get_block_state(pos);
    if let Some(properties) = java_26_2_vanilla_block_lighting_properties(&state) {
        if properties.emissive_rendering {
            return [240, 240];
        }
        return [
            level
                .brightness(false, pos)
                .max(properties.light_emission)
                .saturating_mul(16),
            level.brightness(true, pos).saturating_mul(16),
        ];
    }
    [
        level.brightness(false, pos).saturating_mul(16),
        level.brightness(true, pos).saturating_mul(16),
    ]
}
fn torch_particle_light(
    base_light: [u8; 2],
    behavior: ParticleBehavior,
    network_kind: Option<NetworkParticleKind>,
    age: i32,
    max_age: i32,
) -> [f32; 2] {
    let progress = (age as f32 / max_age.max(1) as f32).clamp(0.0, 1.0);
    let smooth_emission = |emission: f32| {
        let emitted = (emission.clamp(0.0, 1.0) * 240.0) as u8;
        base_light[0].saturating_add(emitted).min(240)
    };
    let block = match (behavior, network_kind) {
        (
            _,
            Some(
                NetworkParticleKind::Flame
                | NetworkParticleKind::SoulFlame
                | NetworkParticleKind::SmallFlame,
            ),
        ) => smooth_emission(progress),
        (
            _,
            Some(
                NetworkParticleKind::SculkSoul
                | NetworkParticleKind::SculkCharge
                | NetworkParticleKind::Shriek
                | NetworkParticleKind::Vibration
                | NetworkParticleKind::SculkChargePop
                | NetworkParticleKind::TrialSpawnerDetection
                | NetworkParticleKind::TrialSpawnerDetectionOminous
                | NetworkParticleKind::OminousSpawning
                | NetworkParticleKind::VaultConnection,
            ),
        ) => 240,
        (_, Some(NetworkParticleKind::Lava)) => 240,
        (_, Some(NetworkParticleKind::Portal)) => {
            smooth_emission(progress * progress * progress * progress)
        }
        (_, Some(NetworkParticleKind::Glow)) => smooth_emission(progress),
        (_, Some(NetworkParticleKind::CopperFlame)) => smooth_emission(progress),
        (_, Some(NetworkParticleKind::Enchant | NetworkParticleKind::Nautilus)) => {
            smooth_emission(progress * progress * progress * progress)
        }
        (_, Some(NetworkParticleKind::Firefly)) => {
            let fade = java_firefly_fade_amount(progress, 0.1, 0.3);
            ((((fade * 255.0) as i32 >> 4) & 0xF) * 16) as u8
        }
        (ParticleBehavior::TorchFlame, _) => smooth_emission(progress),
        _ => base_light[0],
    };
    [block as f32 / 256.0, base_light[1] as f32 / 256.0]
}
impl Particle {
    fn render_position(&self, partial_tick: f32) -> glam::Vec3 {
        self.previous_position.lerp(self.position, partial_tick)
    }
    fn render_scale(&self, partial_tick: f32) -> f32 {
        particle_render_scale(
            self.behavior,
            self.network_kind,
            self.scale,
            self.age,
            self.max_age,
            partial_tick,
        )
    }
    fn update_torch_light(&mut self, level: &crate::world::client_level::ClientLevel) {
        if self.behavior == ParticleBehavior::Generic && self.network_kind.is_none() {
            return;
        }
        if matches!(
            self.network_kind,
            Some(
                NetworkParticleKind::EndRod
                    | NetworkParticleKind::Trail
                    | NetworkParticleKind::SweepAttack
                    | NetworkParticleKind::Explosion
                    | NetworkParticleKind::SonicBoom
                    | NetworkParticleKind::Gust
                    | NetworkParticleKind::SmallGust
                    | NetworkParticleKind::FireworkSpark
            )
        ) {
            self.light = [240.0 / 256.0, 240.0 / 256.0];
            return;
        }
        self.light = torch_particle_light(
            java_particle_light_coords(level, self.position),
            self.behavior,
            self.network_kind,
            self.age,
            self.max_age,
        );
        if matches!(
            self.network_kind,
            Some(
                NetworkParticleKind::DrippingObsidianTear
                    | NetworkParticleKind::FallingObsidianTear
                    | NetworkParticleKind::LandingObsidianTear
            )
        ) {
            self.light[1] = 240.0 / 256.0;
        }
    }
}
fn java_water_drop_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let _base_lifetime = 4.0 / (random.next_float() * 0.9 + 0.1);
    let mut velocity = glam::Vec3::new(
        random.next_float() * 0.8 - 0.4,
        random.next_float() * 0.8 - 0.4,
        random.next_float() * 0.8 - 0.4,
    );
    let speed = (random.next_float() + random.next_float() + 1.0) * 0.15;
    let length = velocity.length();
    if length > f32::EPSILON {
        velocity = velocity / length * speed * 0.4;
    }
    velocity.y += 0.1;
    let scale = 0.1 * (random.next_float() * 0.5 + 0.5) * 2.0;
    velocity.x *= 0.3;
    velocity.y = random.next_float() * 0.2 + 0.1;
    velocity.z *= 0.3;
    let max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    Particle {
        position,
        previous_position: position,
        velocity,
        color: [1.0, 1.0, 1.0, 1.0],
        light: [1.0, 1.0],
        scale,
        age: 0,
        max_age,
        gravity: 0.06,
        u0: rect.u0,
        v0: rect.v0,
        u1: rect.u1,
        v1: rect.v1,
        texture_type: ParticleTextureType::ParticleAtlas,
        friction: 0.98,
        behavior: ParticleBehavior::Rain,
        network_kind: None,
        alpha: 1.0,
        fade_color: None,
        has_physics: true,
        speed_up_when_y_motion_is_blocked: false,
        counts_down_lifetime: false,
        origin: None,
        angle: 0.0,
        target: None,
        delay: 0,
        translucent: false,
        geyser_plume_max_y: 0.0,
        geyser_plume_propulsion: 0.0,
        geyser_plume_spray: [0.0, 0.0],
        geyser_plume_size: [0.0, 0.0],
        leaves: None,
        sulfur_bubble: None,
        geyser_eruption_water_blocks: 0,
    }
}
fn java_explode_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.gravity = -0.1;
    particle.friction = 0.9;
    particle.velocity = glam::Vec3::new(
        requested_velocity.x + (random.next_float() * 2.0 - 1.0) * 0.05,
        requested_velocity.y + (random.next_float() * 2.0 - 1.0) * 0.05,
        requested_velocity.z + (random.next_float() * 2.0 - 1.0) * 0.05,
    );
    let col = random.next_float() * 0.3 + 0.7;
    particle.color = [col, col, col, 1.0];
    particle.scale = 0.1 * (random.next_float() * random.next_float() * 6.0 + 1.0);
    particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32 + 2;
    particle.network_kind = Some(NetworkParticleKind::Poof);
    particle
}
fn java_flame_network_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.friction = 0.96;
    particle.velocity = particle.velocity * 0.01 + requested_velocity;
    particle.position.x += (random.next_float() - random.next_float()) * 0.05;
    particle.position.y += (random.next_float() - random.next_float()) * 0.05;
    particle.position.z += (random.next_float() - random.next_float()) * 0.05;
    particle.max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32 + 4;
    particle.network_kind = Some(kind);
    particle
}
fn java_ash_smoke_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
    scale: f32,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.gravity = -0.1;
    particle.velocity *= 0.1;
    particle.velocity += requested_velocity;
    let col = random.next_float() * 0.3;
    particle.color = [col, col, col, 1.0];
    particle.scale *= 0.75 * scale;
    particle.max_age = ((8.0 / (random.next_float() * 0.8 + 0.2)) * scale) as i32;
    particle.max_age = particle.max_age.max(1);
    particle.network_kind = Some(kind);
    particle
}
fn java_crit_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.7;
    particle.gravity = 0.5;
    particle.has_physics = false;
    particle.velocity *= 0.1;
    particle.velocity += requested_velocity * 0.4;
    let col = random.next_float() * 0.3 + 0.6;
    particle.color = [col, col, col, 1.0];
    particle.scale *= 0.75;
    particle.max_age = (6.0 / (random.next_float() * 0.8 + 0.6)).max(1.0) as i32;
    particle.age = 1;
    particle.velocity.y -= 0.04 * particle.gravity;
    particle.position += particle.velocity;
    particle.velocity *= particle.friction;
    particle.color[1] *= 0.96;
    particle.color[2] *= 0.9;
    match kind {
        NetworkParticleKind::EnchantedHit => {
            particle.color[0] *= 0.3;
            particle.color[1] *= 0.8;
        }
        NetworkParticleKind::DamageIndicator => {
            particle.max_age = 20;
        }
        _ => {}
    }
    particle.network_kind = Some(kind);
    particle
}
fn java_end_rod_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.91;
    particle.gravity = 0.0125;
    particle.velocity = requested_velocity;
    particle.scale *= 0.75;
    particle.max_age = 60 + random.next_int_bound(12);
    particle.fade_color = Some([
        0xF2 as f32 / 255.0,
        0xDE as f32 / 255.0,
        0xC9 as f32 / 255.0,
    ]);
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::EndRod);
    particle
}
fn java_heart_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.86;
    particle.velocity *= 0.01;
    particle.velocity.y += 0.1;
    particle.scale *= 1.5;
    particle.max_age = 16;
    particle.network_kind = Some(NetworkParticleKind::Heart);
    particle
}
fn java_note_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    color_parameter: f32,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.66;
    particle.velocity *= 0.01;
    particle.velocity.y += 0.2;
    let sin_color = |phase: f32| {
        let v = (color_parameter + phase) * std::f32::consts::TAU;
        (v.sin() * 0.65 + 0.35).max(0.0)
    };
    particle.color = [
        sin_color(0.0),
        sin_color(1.0 / 3.0),
        sin_color(2.0 / 3.0),
        1.0,
    ];
    particle.scale *= 1.5;
    particle.max_age = 6;
    particle.speed_up_when_y_motion_is_blocked = true;
    particle.network_kind = Some(NetworkParticleKind::Note);
    particle
}
fn java_splash_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_water_drop_particle(position, rect, random);
    particle.gravity = 0.04;
    if requested_velocity.y == 0.0 && (requested_velocity.x != 0.0 || requested_velocity.z != 0.0) {
        particle.velocity.x = requested_velocity.x;
        particle.velocity.y = 0.1;
        particle.velocity.z = requested_velocity.z;
    }
    particle.network_kind = Some(NetworkParticleKind::Splash);
    particle
}
fn java_dust_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    color: [f32; 3],
    scale: f32,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.friction = 0.96;
    particle.velocity *= 0.1;
    particle.scale *= 0.75 * scale;
    let base_lifetime = (8.0 / (random.next_double() * 0.8 + 0.2)) as f64;
    particle.max_age = (base_lifetime * scale as f64).max(1.0) as i32;
    let base_factor = random.next_float() * 0.4 + 0.6;
    let mut randomize = |channel: f32| (random.next_float() * 0.2 + 0.8) * channel * base_factor;
    particle.color = [
        randomize(color[0]),
        randomize(color[1]),
        randomize(color[2]),
        1.0,
    ];
    particle.network_kind = Some(NetworkParticleKind::Dust);
    particle
}
fn java_terrain_network_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    tint: [f32; 4],
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.gravity = 1.0;
    particle.color = [0.6 * tint[0], 0.6 * tint[1], 0.6 * tint[2], tint[3]];
    particle.scale *= 0.5;
    let u_size = rect.u1 - rect.u0;
    let v_size = rect.v1 - rect.v0;
    let uo = random.next_float() * 3.0;
    let vo = random.next_float() * 3.0;
    particle.u0 = rect.u0 + u_size * ((uo + 1.0) / 4.0);
    particle.v0 = rect.v0 + v_size * (vo / 4.0);
    particle.u1 = rect.u0 + u_size * (uo / 4.0);
    particle.v1 = rect.v0 + v_size * ((vo + 1.0) / 4.0);
    particle.texture_type = ParticleTextureType::BlockAtlas;
    particle.network_kind = Some(NetworkParticleKind::Block);
    particle
}
fn java_lava_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.gravity = 0.75;
    particle.friction = 0.999;
    particle.velocity *= 0.8;
    particle.velocity.y = random.next_float() * 0.4 + 0.05;
    particle.scale *= random.next_float() * 2.0 + 0.2;
    particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.network_kind = Some(NetworkParticleKind::Lava);
    particle
}
fn java_portal_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.velocity = requested_velocity;
    particle.origin = Some(position);
    particle.scale = 0.1 * (random.next_float() * 0.2 + 0.5);
    let brightness = random.next_float() * 0.6 + 0.4;
    particle.color = [brightness * 0.9, brightness * 0.3, brightness, 1.0];
    particle.max_age = (random.next_float() * 10.0) as i32 + 40;
    particle.behavior = ParticleBehavior::Portal;
    particle.network_kind = Some(NetworkParticleKind::Portal);
    particle
}
fn java_reverse_portal_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_portal_particle(position, requested_velocity, rect, random);
    particle.scale *= 1.5;
    particle.max_age = (random.next_float() * 2.0) as i32 + 60;
    particle.behavior = ParticleBehavior::ReversePortal;
    particle.network_kind = Some(NetworkParticleKind::ReversePortal);
    particle
}
fn java_bubble_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.scale *= random.next_float() * 0.6 + 0.2;
    particle.velocity = glam::Vec3::new(
        requested_velocity.x * 0.2 + (random.next_float() * 2.0 - 1.0) * 0.02,
        requested_velocity.y * 0.2 + (random.next_float() * 2.0 - 1.0) * 0.02,
        requested_velocity.z * 0.2 + (random.next_float() * 2.0 - 1.0) * 0.02,
    );
    particle.max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.counts_down_lifetime = true;
    particle.behavior = ParticleBehavior::Bubble;
    particle.network_kind = Some(NetworkParticleKind::Bubble);
    particle
}
fn java_bubble_column_up_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_bubble_particle(position, requested_velocity, rect, random);
    particle.gravity = -0.125;
    particle.friction = 0.85;
    particle.max_age = (40.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.counts_down_lifetime = false;
    particle.behavior = ParticleBehavior::Generic;
    particle.network_kind = Some(NetworkParticleKind::BubbleColumnUp);
    particle
}
fn java_current_down_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.max_age = (random.next_float() * 60.0) as i32 + 30;
    particle.has_physics = false;
    particle.velocity = glam::Vec3::new(0.0, -0.05, 0.0);
    particle.scale *= random.next_float() * 0.6 + 0.2;
    particle.gravity = 0.002;
    particle.behavior = ParticleBehavior::CurrentDown;
    particle.network_kind = Some(NetworkParticleKind::CurrentDown);
    particle
}
fn java_bubble_pop_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.max_age = 4;
    particle.gravity = 0.008;
    particle.velocity = requested_velocity;
    particle.network_kind = Some(NetworkParticleKind::BubblePop);
    particle
}
fn java_rising_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.friction = 0.96;
    particle.velocity = particle.velocity * 0.01 + requested_velocity;
    particle.position.x += (random.next_float() - random.next_float()) * 0.05;
    particle.position.y += (random.next_float() - random.next_float()) * 0.05;
    particle.position.z += (random.next_float() - random.next_float()) * 0.05;
    particle.max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32 + 4;
    particle
}
fn java_soul_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    glowing: bool,
) -> Particle {
    let mut particle = java_rising_particle(position, requested_velocity, rect, random);
    particle.scale *= 1.5;
    particle.alpha = 1.0;
    particle.translucent = true;
    particle.network_kind = Some(if glowing {
        NetworkParticleKind::SculkSoul
    } else {
        NetworkParticleKind::Soul
    });
    particle
}
fn java_snowflake_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.gravity = 0.225;
    particle.friction = 1.0;
    particle.velocity = glam::Vec3::new(
        requested_velocity.x + (random.next_float() * 2.0 - 1.0) * 0.05,
        requested_velocity.y + (random.next_float() * 2.0 - 1.0) * 0.05,
        requested_velocity.z + (random.next_float() * 2.0 - 1.0) * 0.05,
    );
    particle.scale = 0.1 * (random.next_float() * random.next_float() + 1.0);
    particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32 + 2;
    particle.color = [0.923, 0.964, 0.999, 1.0];
    particle.network_kind = Some(NetworkParticleKind::Snowflake);
    particle
}
fn java_spit_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_explode_particle(position, requested_velocity, rect, random);
    particle.gravity = 0.5;
    particle.network_kind = Some(NetworkParticleKind::Spit);
    particle
}
fn java_player_cloud_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    sneeze: bool,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.velocity *= 0.1;
    particle.velocity += requested_velocity;
    let col = 1.0 - random.next_float() * 0.3;
    particle.color = [col, col, col, 1.0];
    particle.scale *= 1.875;
    let base_lifetime = (8.0 / (random.next_float() * 0.8 + 0.3)) as i32;
    particle.max_age = ((base_lifetime as f32 * 2.5).max(1.0)) as i32;
    particle.has_physics = false;
    particle.translucent = true;
    if sneeze {
        particle.color = [0.22, 1.0, 0.53, 1.0];
        particle.alpha = 0.4;
    }
    particle.network_kind = Some(if sneeze {
        NetworkParticleKind::Sneeze
    } else {
        NetworkParticleKind::Cloud
    });
    particle
}
fn java_suspended_town_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    let br = random.next_float() * 0.1 + 0.2;
    particle.color = [br, br, br, 1.0];
    particle.scale *= random.next_float() * 0.6 + 0.5;
    particle.velocity *= 0.02;
    particle.max_age = (20.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.friction = 0.99;
    particle.counts_down_lifetime = true;
    particle.behavior = ParticleBehavior::SuspendedTown;
    particle.network_kind = Some(NetworkParticleKind::Mycelium);
    particle
}
fn java_suspended_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(
        glam::Vec3::new(position.x, position.y - 0.125, position.z),
        rect,
        random,
    );
    particle.scale *= random.next_float() * 0.6 + 0.2;
    particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.has_physics = false;
    particle.friction = 1.0;
    particle.gravity = 0.0;
    particle.network_kind = Some(NetworkParticleKind::Underwater);
    particle
}
fn java_suspended_particle_7arg<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let mut particle = java_base_single_quad_particle(
        glam::Vec3::new(position.x, position.y - 0.125, position.z),
        requested_velocity,
        rect,
        random,
    );
    particle.scale *= random.next_float() * 0.6 + 0.6;
    particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.has_physics = false;
    particle.friction = 1.0;
    particle.gravity = 0.0;
    particle
}
fn java_base_ash_smoke_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    dir: glam::Vec3,
    color_random: f32,
    max_lifetime: f32,
    gravity: f32,
    has_physics: bool,
    fixed_color: Option<[f32; 3]>,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.gravity = gravity;
    particle.velocity *= dir;
    particle.velocity += requested_velocity;
    let col = random.next_float() * color_random;
    particle.color = match fixed_color {
        Some(fixed) => [fixed[0], fixed[1], fixed[2], 1.0],
        None => [col, col, col, 1.0],
    };
    particle.scale *= 0.75;
    particle.max_age = (max_lifetime / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.max_age = particle.max_age.max(1);
    particle.has_physics = has_physics;
    particle
}
fn java_campfire_smoke_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    signal: bool,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.scale *= 3.0;
    particle.max_age = if signal {
        random.next_int_bound(50) + 280
    } else {
        random.next_int_bound(50) + 80
    };
    particle.gravity = 3.0e-6;
    particle.velocity = glam::Vec3::new(
        requested_velocity.x,
        requested_velocity.y + random.next_float() / 500.0,
        requested_velocity.z,
    );
    particle.alpha = if signal { 0.95 } else { 0.9 };
    particle.translucent = true;
    particle.behavior = ParticleBehavior::CampfireSmoke;
    particle.network_kind = Some(if signal {
        NetworkParticleKind::CampfireSignalSmoke
    } else {
        NetworkParticleKind::CampfireCosySmoke
    });
    particle
}
fn java_flash_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    color: [f32; 3],
    alpha: f32,
) -> Particle {
    let particle = Particle {
        position,
        previous_position: position,
        velocity: glam::Vec3::ZERO,
        color: [color[0], color[1], color[2], alpha],
        light: [1.0, 1.0],
        scale: 7.1,
        age: 0,
        max_age: 4,
        gravity: 0.0,
        u0: rect.u0,
        v0: rect.v0,
        u1: rect.u1,
        v1: rect.v1,
        texture_type: ParticleTextureType::ParticleAtlas,
        friction: 0.98,
        behavior: ParticleBehavior::Generic,
        network_kind: Some(NetworkParticleKind::Flash),
        alpha,
        fade_color: None,
        has_physics: false,
        speed_up_when_y_motion_is_blocked: false,
        counts_down_lifetime: false,
        origin: None,
        angle: 0.0,
        target: None,
        delay: 0,
        translucent: true,
        geyser_plume_max_y: 0.0,
        geyser_plume_propulsion: 0.0,
        geyser_plume_spray: [0.0, 0.0],
        geyser_plume_size: [0.0, 0.0],
        leaves: None,
        sulfur_bubble: None,
        geyser_eruption_water_blocks: 0,
    };
    particle
}
fn java_spell_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.friction = 0.96;
    particle.gravity = -0.1;
    particle.velocity = glam::Vec3::new(
        0.5 - random.next_float(),
        requested_velocity.y,
        0.5 - random.next_float(),
    );
    particle.velocity.y *= 0.2;
    if requested_velocity.x == 0.0 && requested_velocity.z == 0.0 {
        particle.velocity.x *= 0.1;
        particle.velocity.z *= 0.1;
    }
    particle.scale *= 0.75;
    particle.max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.has_physics = false;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::Effect);
    particle
}
fn java_dragon_breath_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    power: f32,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.friction = 0.96;
    particle.velocity = requested_velocity;
    particle.velocity *= power;
    particle.velocity.y = (particle.velocity.y - 0.1) * power + 0.1;
    particle.color = [
        0.7176471 + random.next_float() * (0.8745098 - 0.7176471),
        0.0,
        0.8235294 + random.next_float() * (0.9764706 - 0.8235294),
        1.0,
    ];
    particle.scale *= 0.75;
    particle.max_age = (20.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.has_physics = false;
    particle.network_kind = Some(NetworkParticleKind::DragonBreath);
    particle
}
fn java_sculk_charge_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    roll: f32,
) -> Particle {
    let mut particle = java_rising_particle(position, requested_velocity, rect, random);
    particle.scale *= 1.5;
    particle.velocity = requested_velocity;
    particle.has_physics = false;
    particle.angle = roll;
    particle.translucent = true;
    particle.max_age = random.next_int_bound(12) + 8;
    particle.network_kind = Some(NetworkParticleKind::SculkCharge);
    particle
}
fn java_trail_scale_r<R: RandomSource + ?Sized>(random: &mut R) -> f32 {
    (random.next_float() * 0.25 + 0.875).min(1.0)
}
fn java_dust_color_randomize<R: RandomSource + ?Sized>(
    random: &mut R,
    channel: f32,
    base_factor: f32,
) -> f32 {
    (random.next_float() * 0.2 + 0.8) * channel * base_factor
}
fn java_trail_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    target: [f64; 3],
    color: [f32; 3],
    duration: i32,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.color = [
        (color[0] * java_trail_scale_r(random)).min(1.0),
        (color[1] * java_trail_scale_r(random)).min(1.0),
        (color[2] * java_trail_scale_r(random)).min(1.0),
        1.0,
    ];
    particle.scale = 0.26;
    particle.target = Some(glam::Vec3::new(
        target[0] as f32,
        target[1] as f32,
        target[2] as f32,
    ));
    particle.max_age = duration;
    particle.network_kind = Some(NetworkParticleKind::Trail);
    particle
}
fn java_block_marker_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.max_age = 80;
    particle.has_physics = false;
    particle.scale = 0.5;
    particle.texture_type = ParticleTextureType::BlockAtlas;
    particle.network_kind = Some(NetworkParticleKind::BlockMarker);
    particle
}
fn java_falling_dust_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    tint: [f32; 4],
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.scale *= 0.67499995;
    let base_lifetime = (32.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.max_age = ((base_lifetime as f32 * 0.9).max(1.0)) as i32;
    particle.color = [tint[0], tint[1], tint[2], tint[3]];
    particle.texture_type = ParticleTextureType::BlockAtlas;
    particle.network_kind = Some(NetworkParticleKind::FallingDust);
    particle
}
fn java_dust_pillar_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    tint: [f32; 4],
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle =
        java_terrain_network_particle(position, requested_velocity, rect, tint, random);
    particle.velocity = glam::Vec3::new(
        random.next_gaussian() as f32 / 30.0,
        requested_velocity.y + random.next_gaussian() as f32 / 2.0,
        random.next_gaussian() as f32 / 30.0,
    );
    particle.max_age = random.next_int_bound(20) + 20;
    particle.network_kind = Some(NetworkParticleKind::DustPillar);
    particle
}
fn java_block_crumble_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    tint: [f32; 4],
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle =
        java_terrain_network_particle(position, requested_velocity, rect, tint, random);
    particle.velocity = glam::Vec3::ZERO;
    particle.max_age = random.next_int_bound(10) + 1;
    particle.network_kind = Some(NetworkParticleKind::BlockCrumble);
    particle
}
fn java_dust_color_transition_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    from: [f32; 3],
    to: [f32; 3],
    scale: f32,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_dust_particle(position, requested_velocity, rect, from, scale, random);
    let base_factor = random.next_float() * 0.4 + 0.6;
    let from_color = [
        java_dust_color_randomize(random, from[0], base_factor),
        java_dust_color_randomize(random, from[1], base_factor),
        java_dust_color_randomize(random, from[2], base_factor),
    ];
    let to_color = [
        java_dust_color_randomize(random, to[0], base_factor),
        java_dust_color_randomize(random, to[1], base_factor),
        java_dust_color_randomize(random, to[2], base_factor),
    ];
    particle.network_kind = Some(NetworkParticleKind::DustColorTransition);
    particle.fade_color = Some([to_color[0], to_color[1], to_color[2]]);
    particle.color = [from_color[0], from_color[1], from_color[2], 1.0];
    particle
}
fn java_glow_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    name: &str,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.has_physics = false;
    particle.scale *= 0.75;
    match name {
        "electric_spark" => {
            particle.color = [1.0, 0.9, 1.0, 1.0];
            particle.velocity = requested_velocity * 0.25;
            particle.max_age = random.next_int_bound(2) + 2;
        }
        "glow" => {
            particle.velocity = glam::Vec3::new(
                0.5 - random.next_float(),
                requested_velocity.y,
                0.5 - random.next_float(),
            );
            particle.color = if random.next_boolean() {
                [0.6, 1.0, 0.8, 1.0]
            } else {
                [0.08, 0.4, 0.4, 1.0]
            };
            particle.velocity.y *= 0.2;
            if requested_velocity.x == 0.0 && requested_velocity.z == 0.0 {
                particle.velocity.x *= 0.1;
                particle.velocity.z *= 0.1;
            }
            particle.max_age = (8.0 / (random.next_double() * 0.8 + 0.2)) as i32;
        }
        "scrape" => {
            particle.color = if random.next_boolean() {
                [0.29, 0.58, 0.51, 1.0]
            } else {
                [0.43, 0.77, 0.62, 1.0]
            };
            particle.velocity = requested_velocity * 0.01;
            particle.max_age = random.next_int_bound(30) + 10;
        }
        "wax_on" => {
            particle.color = [0.91, 0.55, 0.08, 1.0];
            particle.velocity = requested_velocity * glam::Vec3::new(0.005, 0.01, 0.005);
            particle.max_age = random.next_int_bound(30) + 10;
        }
        "wax_off" => {
            particle.color = [1.0, 0.9, 1.0, 1.0];
            particle.velocity = requested_velocity * glam::Vec3::new(0.005, 0.01, 0.005);
            particle.max_age = random.next_int_bound(30) + 10;
        }
        _ => {}
    }
    particle.network_kind = Some(NetworkParticleKind::Glow);
    particle
}
fn java_totem_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.6;
    particle.gravity = 1.25;
    particle.velocity = requested_velocity;
    particle.scale *= 0.75;
    particle.max_age = 60 + random.next_int_bound(12);
    if random.next_int_bound(4) == 0 {
        particle.color = [
            0.6 + random.next_float() * 0.2,
            0.6 + random.next_float() * 0.3,
            random.next_float() * 0.2,
            1.0,
        ];
    } else {
        particle.color = [
            0.1 + random.next_float() * 0.2,
            0.4 + random.next_float() * 0.3,
            random.next_float() * 0.2,
            1.0,
        ];
    }
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::Totem);
    particle
}
fn java_squid_ink_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    glowing: bool,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.92;
    particle.scale = 0.5;
    particle.color = if glowing {
        [0.2, 0.8, 0.6, 1.0]
    } else {
        [0.0, 0.0, 0.0, 1.0]
    };
    particle.max_age = (6.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.has_physics = false;
    particle.velocity = requested_velocity;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::SquidInk);
    particle
}
fn java_shriek_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    delay: i32,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.scale = 0.85;
    particle.delay = delay;
    particle.max_age = 30;
    particle.gravity = 0.0;
    particle.velocity = glam::Vec3::new(0.0, 0.1, 0.0);
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::Shriek);
    particle
}
fn java_vibration_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    target: glam::Vec3,
    arrival_in_ticks: i32,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.scale = 0.3;
    particle.target = Some(target);
    particle.max_age = arrival_in_ticks.max(1);
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::Vibration);
    particle
}
fn java_geyser_base_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    water_blocks: i32,
    burst_impulse_base: f32,
) -> Particle {
    let burst_impulse = burst_impulse_base + 0.25 * water_blocks as f32;
    let size = 3.0 + 0.125 * water_blocks as f32;
    let mut particle = java_base_ash_smoke_particle(
        position,
        requested_velocity,
        rect,
        random,
        glam::Vec3::splat(burst_impulse),
        0.0,
        0.0,
        0.0,
        true,
        Some([1.0, 1.0, 1.0]),
    );
    particle.friction = 0.725;
    particle.velocity.y = particle.velocity.y.abs();
    particle.max_age = (25.0 * (0.8 + 0.2 * random.next_float())) as i32;
    particle.scale *= size;
    particle.network_kind = Some(NetworkParticleKind::GeyserBase);
    particle
}
fn java_geyser_plume_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    water_blocks: i32,
) -> Particle {
    let plume_height = 5 * water_blocks.max(1) as i32;
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.has_physics = true;
    particle.max_age = plume_height * 5;
    particle.velocity.y = 0.0;
    particle.geyser_plume_max_y = position.y + plume_height as f32 - 1.0;
    particle.geyser_plume_spray = [
        (random.next_float() - 0.5) * 0.2,
        (random.next_float() - 0.5) * 0.2,
    ];
    particle.friction = 1.0;
    let propulsion = (if water_blocks == 1 { 1.5 } else { 1.0 }) * plume_height as f32 * 1.45;
    particle.geyser_plume_propulsion = propulsion;
    particle.gravity = -propulsion;
    let initial_size = particle.scale * 0.75;
    particle.geyser_plume_size = [
        initial_size * (2.0 + plume_height as f32 / 8.0),
        initial_size * (3.0 + plume_height as f32 / 8.0),
    ];
    particle.scale = particle.geyser_plume_size[0];
    particle.origin = Some(position);
    particle.translucent = false;
    particle.network_kind = Some(NetworkParticleKind::GeyserPlume);
    particle
}
fn java_firefly_fade_amount(lifetime_progress: f32, fade_in_time: f32, fade_out_time: f32) -> f32 {
    let progress = lifetime_progress.clamp(0.0, 1.0);
    if progress >= 1.0 - fade_in_time {
        (1.0 - progress) / fade_in_time
    } else if progress <= fade_out_time {
        progress / fade_out_time
    } else {
        1.0
    }
}
fn java_dust_plume_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_ash_smoke_particle(
        position,
        glam::Vec3::new(
            requested_velocity.x,
            requested_velocity.y + 0.15,
            requested_velocity.z,
        ),
        rect,
        random,
        glam::Vec3::new(0.7, 0.6, 0.7),
        0.5,
        7.0,
        0.5,
        false,
        None,
    );
    let shift = random.next_float() * 0.2;
    particle.color = [0.7294118 - shift, 0.6941176 - shift, 0.7607843 - shift, 1.0];
    particle.network_kind = Some(NetworkParticleKind::DustPlume);
    particle
}
fn java_fly_towards_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.velocity = requested_velocity;
    particle.origin = Some(position);
    particle.position = position + particle.velocity;
    particle.scale = 0.1 * (random.next_float() * 0.5 + 0.2);
    let br = random.next_float() * 0.6 + 0.4;
    particle.color = [0.9 * br, 0.9 * br, br, 1.0];
    particle.has_physics = false;
    particle.max_age = (random.next_float() * 10.0) as i32 + 30;
    if kind == NetworkParticleKind::VaultConnection {
        particle.scale *= 1.5;
        particle.alpha = 0.0;
    }
    particle.network_kind = Some(kind);
    particle
}
fn java_fly_straight_towards_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.velocity = requested_velocity;
    particle.origin = Some(position);
    particle.position = position + particle.velocity;
    particle.scale = 0.1 * (random.next_float() * 0.5 + 0.2);
    particle.has_physics = false;
    particle.max_age = (random.next_float() * 5.0) as i32 + 25;
    particle.scale *= random.next_float() * 2.0 + 3.0;
    particle.fade_color = Some([0.2705882, 0.6823529, 0.9960784]);
    particle.network_kind = Some(NetworkParticleKind::OminousSpawning);
    particle
}
fn java_falling_leaves_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
) -> Particle {
    let (fall_accel, side_accel, swirl, flow_away, scale, start_velocity) = match kind {
        NetworkParticleKind::CherryLeaves => (0.25, 2.0, false, true, 1.0, 0.0),
        _ => (0.07, 10.0, true, false, 2.0, 0.021),
    };
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.max_age = 300;
    particle.gravity = fall_accel * 1.2 * 0.0025;
    let rot_speed = if random.next_boolean() {
        -30.0_f32.to_radians()
    } else {
        30.0_f32.to_radians()
    };
    let spin_acceleration = if random.next_boolean() {
        -5.0_f32.to_radians()
    } else {
        5.0_f32.to_radians()
    };
    let size = scale * if random.next_boolean() { 0.05 } else { 0.075 };
    particle.scale = size;
    particle.friction = 1.0;
    particle.velocity.y = -start_velocity;
    let particle_random = random.next_float();
    let xa_flow_scale = (particle_random * 60.0_f32).to_radians().cos() * side_accel;
    let za_flow_scale = (particle_random * 60.0_f32).to_radians().sin() * side_accel;
    particle.leaves = Some(FallingLeavesState {
        wind_big: side_accel,
        swirl,
        flow_away,
        xa_flow_scale,
        za_flow_scale,
        swirl_period: (1000.0 + particle_random * 3000.0).to_radians(),
        rot_speed,
        spin_acceleration,
    });
    particle.network_kind = Some(kind);
    particle
}
fn java_simple_vertical_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    upwards: bool,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.velocity = requested_velocity;
    particle.velocity.y += if upwards { 0.03 } else { -0.03 };
    particle.gravity = 0.0;
    particle.scale *= random.next_float() * 0.6 + 0.5;
    particle.max_age = 8;
    particle.network_kind = Some(kind);
    particle
}
fn java_wake_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.velocity = requested_velocity;
    particle.gravity = 0.0;
    let _yd = random.next_float() * 0.2 + 0.1;
    particle.max_age = (8.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    particle.network_kind = Some(NetworkParticleKind::Fishing);
    particle
}
fn java_attack_sweep_particle(
    position: glam::Vec3,
    size: f32,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.max_age = 4;
    let col = random.next_float() * 0.6 + 0.4;
    particle.color = [col, col, col, 1.0];
    particle.scale = 1.0 - size * 0.5;
    particle.network_kind = Some(NetworkParticleKind::SweepAttack);
    particle
}
fn java_huge_explosion_particle(
    position: glam::Vec3,
    size: f32,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.max_age = 6 + random.next_int_bound(4);
    let col = random.next_float() * 0.6 + 0.4;
    particle.color = [col, col, col, 1.0];
    particle.scale = 2.0 * (1.0 - size * 0.5);
    if kind == NetworkParticleKind::SonicBoom {
        particle.max_age = 16;
        particle.scale = 1.5;
    }
    particle.network_kind = Some(kind);
    particle
}
fn java_gust_particle(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    small: bool,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.max_age = 12 + random.next_int_bound(4);
    particle.scale = 1.0;
    if small {
        particle.scale *= 0.15;
    }
    particle.network_kind = Some(if small {
        NetworkParticleKind::SmallGust
    } else {
        NetworkParticleKind::Gust
    });
    particle
}
fn java_sculk_charge_pop_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, requested_velocity, rect, random);
    particle.friction = 0.96;
    particle.has_physics = false;
    particle.velocity = requested_velocity;
    particle.max_age = random.next_int_bound(4) + 6;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::SculkChargePop);
    particle
}
fn java_noxious_gas_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_ash_smoke_particle(
        position,
        requested_velocity,
        rect,
        random,
        glam::Vec3::splat(0.1),
        0.3,
        5.0,
        -0.02,
        true,
        Some([1.0, 1.0, 1.0]),
    );
    particle.scale *= 3.0;
    particle.max_age = (6.0 / (random.next_float() * 0.5 + 0.5) * 3.0) as i32;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::NoxiousGas);
    particle
}
fn java_particle_4arg_base<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    random: &mut R,
) -> Particle {
    let base_max_age = (4.0 / (random.next_float() * 0.9 + 0.1)) as i32;
    let mut velocity = glam::Vec3::new(
        (random.next_float() * 2.0 - 1.0) * 0.4,
        (random.next_float() * 2.0 - 1.0) * 0.4,
        (random.next_float() * 2.0 - 1.0) * 0.4,
    );
    let speed = (random.next_float() + random.next_float() + 1.0) * 0.15;
    velocity = velocity / velocity.length().max(f32::EPSILON) * speed * 0.4;
    velocity.y += 0.1;
    Particle {
        position,
        previous_position: position,
        velocity,
        color: [1.0, 1.0, 1.0, 1.0],
        light: [1.0, 1.0],
        scale: 0.0,
        age: 0,
        max_age: base_max_age,
        gravity: 0.0,
        u0: 0.0,
        v0: 0.0,
        u1: 0.0,
        v1: 0.0,
        texture_type: ParticleTextureType::ParticleAtlas,
        friction: 0.98,
        behavior: ParticleBehavior::Generic,
        network_kind: None,
        alpha: 1.0,
        fade_color: None,
        has_physics: true,
        speed_up_when_y_motion_is_blocked: false,
        counts_down_lifetime: false,
        origin: None,
        angle: 0.0,
        target: None,
        delay: 0,
        translucent: false,
        geyser_plume_max_y: 0.0,
        geyser_plume_propulsion: 0.0,
        geyser_plume_spray: [0.0, 0.0],
        geyser_plume_size: [0.0, 0.0],
        leaves: None,
        sulfur_bubble: None,
        geyser_eruption_water_blocks: 0,
    }
}
fn java_drip_particle_base<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.gravity = 0.06;
    particle.network_kind = Some(kind);
    particle
}
fn java_drip_hang_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_drip_particle_base(position, rect, random, kind);
    particle.gravity *= 0.02;
    particle.max_age = 40;
    match kind {
        NetworkParticleKind::DrippingHoney => {
            particle.gravity *= 0.01;
            particle.max_age = 100;
            particle.color = [0.622, 0.508, 0.082, 1.0];
        }
        NetworkParticleKind::DrippingObsidianTear => {
            particle.gravity *= 0.01;
            particle.max_age = 100;
            particle.color = [0.51171875, 0.03125, 0.890625, 1.0];
        }
        NetworkParticleKind::DrippingWater | NetworkParticleKind::DrippingDripstoneWater => {
            particle.color = [0.2, 0.3, 1.0, 1.0];
        }
        NetworkParticleKind::DrippingLava | NetworkParticleKind::DrippingDripstoneLava => {}
        _ => {}
    }
    particle
}
fn java_drip_fall_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_drip_particle_base(position, rect, random, kind);
    particle.max_age = (64.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    match kind {
        NetworkParticleKind::FallingNectar => {
            particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32;
            particle.gravity = 0.007;
            particle.color = [0.92, 0.782, 0.72, 1.0];
        }
        NetworkParticleKind::FallingSporeBlossom => {
            let between = 0.1 + random.next_float() * (0.9 - 0.1);
            particle.max_age = (64.0 / between) as i32;
            particle.gravity = 0.005;
            particle.color = [0.32, 0.5, 0.22, 1.0];
        }
        NetworkParticleKind::FallingHoney => {
            particle.gravity = 0.01;
            particle.color = [0.582, 0.448, 0.082, 1.0];
        }
        NetworkParticleKind::FallingObsidianTear => {
            particle.gravity = 0.01;
            particle.color = [0.51171875, 0.03125, 0.890625, 1.0];
        }
        NetworkParticleKind::FallingWater | NetworkParticleKind::FallingDripstoneWater => {
            particle.color = [0.2, 0.3, 1.0, 1.0];
        }
        NetworkParticleKind::FallingLava | NetworkParticleKind::FallingDripstoneLava => {
            particle.color = [1.0, 0.2857143, 0.083333336, 1.0];
        }
        _ => {}
    }
    particle
}
fn java_drip_land_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_drip_particle_base(position, rect, random, kind);
    particle.max_age = (16.0 / (random.next_float() * 0.8 + 0.2)) as i32;
    match kind {
        NetworkParticleKind::LandingHoney => {
            particle.max_age = (128.0 / (random.next_float() * 0.8 + 0.2)) as i32;
            particle.color = [0.522, 0.408, 0.082, 1.0];
        }
        NetworkParticleKind::LandingObsidianTear => {
            particle.max_age = (28.0 / (random.next_float() * 0.8 + 0.2)) as i32;
            particle.color = [0.51171875, 0.03125, 0.890625, 1.0];
        }
        NetworkParticleKind::LandingLava => {
            particle.color = [1.0, 0.2857143, 0.083333336, 1.0];
        }
        _ => {}
    }
    particle
}
fn drip_hang_cascade_kind(kind: NetworkParticleKind) -> Option<NetworkParticleKind> {
    Some(match kind {
        NetworkParticleKind::DrippingWater => NetworkParticleKind::FallingWater,
        NetworkParticleKind::DrippingDripstoneWater => NetworkParticleKind::FallingDripstoneWater,
        NetworkParticleKind::DrippingHoney => NetworkParticleKind::FallingHoney,
        NetworkParticleKind::DrippingObsidianTear => NetworkParticleKind::FallingObsidianTear,
        NetworkParticleKind::DrippingLava => NetworkParticleKind::FallingLava,
        NetworkParticleKind::DrippingDripstoneLava => NetworkParticleKind::FallingDripstoneLava,
        _ => return None,
    })
}
fn java_sulfur_cube_goo_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut R,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.gravity = 1.0;
    particle.scale *= 0.5;
    let uo = random.next_float() * 3.0;
    let vo = random.next_float() * 3.0;
    particle.u0 = rect.u0 + (rect.u1 - rect.u0) * ((uo + 1.0) / 4.0);
    particle.u1 = rect.u0 + (rect.u1 - rect.u0) * (uo / 4.0);
    particle.v0 = rect.v0 + (rect.v1 - rect.v0) * (vo / 4.0);
    particle.v1 = rect.v0 + (rect.v1 - rect.v0) * ((vo + 1.0) / 4.0);
    particle.network_kind = Some(NetworkParticleKind::SulfurCubeGoo);
    particle
}
fn java_huge_explosion_seed_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    random: &mut R,
) -> Particle {
    let mut particle = java_particle_4arg_base(position, random);
    particle.max_age = 8;
    particle.network_kind = Some(NetworkParticleKind::ExplosionEmitter);
    particle
}
fn java_gust_seed_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    random: &mut R,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_particle_4arg_base(position, random);
    particle.max_age = match kind {
        NetworkParticleKind::GustEmitterLarge => 7,
        NetworkParticleKind::GustEmitterSmall => 3,
        _ => 0,
    };
    particle.network_kind = Some(kind);
    particle
}
fn java_noxious_gas_cloud_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    random: &mut R,
) -> Particle {
    let mut particle = java_particle_4arg_base(position, random);
    particle.max_age = 20;
    particle.network_kind = Some(NetworkParticleKind::NoxiousGasCloud);
    particle
}
fn java_geyser_eruption_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    water_blocks: i32,
    random: &mut R,
) -> Particle {
    let mut particle = java_particle_4arg_base(position, random);
    particle.velocity = requested_velocity;
    particle.max_age = 20;
    particle.geyser_eruption_water_blocks = water_blocks;
    particle.network_kind = Some(NetworkParticleKind::GeyserEruption);
    particle
}
fn java_elder_guardian_particle<R: RandomSource + ?Sized>(
    position: glam::Vec3,
    random: &mut R,
) -> Particle {
    let mut particle = java_particle_4arg_base(position, random);
    particle.velocity = glam::Vec3::ZERO;
    particle.gravity = 0.0;
    particle.max_age = 30;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::ElderGuardian);
    particle
}
#[derive(Clone, Copy, Debug)]
pub(crate) struct ElderGuardianRenderInstance {
    pub(crate) age_scale: f32,
    pub(crate) alpha: f32,
}
impl ParticleEngine {
    pub(crate) fn elder_guardian_render_instances(
        &self,
        partial_tick: f32,
    ) -> Vec<ElderGuardianRenderInstance> {
        self.particles
            .iter()
            .filter_map(|p| {
                if p.network_kind == Some(NetworkParticleKind::ElderGuardian) && p.max_age > 0 {
                    let age_scale = (p.age as f32 + partial_tick) / p.max_age as f32;
                    let alpha = 0.05 + 0.5 * (age_scale * std::f32::consts::PI).sin();
                    Some(ElderGuardianRenderInstance { age_scale, alpha })
                } else {
                    None
                }
            })
            .collect()
    }
}
fn noxious_gas_can_reach(
    level: &crate::world::client_level::ClientLevel,
    source_pos: glam::Vec3,
    pos: glam::Vec3,
) -> bool {
    let block_pos = BlockPos::new(
        pos.x.floor() as i32,
        pos.y.floor() as i32,
        pos.z.floor() as i32,
    );
    let source_block = BlockPos::new(
        source_pos.x.floor() as i32,
        source_pos.y.floor() as i32,
        source_pos.z.floor() as i32,
    );
    let state = level.get_block_state(block_pos);
    let geyser_passable = if state.is_known_vanilla_air() || state.get_fluid_state().is_water() {
        true
    } else {
        state.get_collision_shape().is_empty()
    };
    if !geyser_passable {
        return false;
    }
    let source_center = glam::Vec3::new(
        source_block.x as f32 + 0.5,
        source_block.y as f32 + 0.5,
        source_block.z as f32 + 0.5,
    );
    let delta = pos - source_center;
    if delta.x * delta.x + delta.y * delta.y + delta.z * delta.z > 9.0 {
        return false;
    }
    let below = level
        .get_block_state(BlockPos::new(block_pos.x, block_pos.y - 1, block_pos.z))
        .get_fluid_state();
    below.is_water() && below.is_source()
}
fn java_sulfur_bubble_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.gravity = -0.04;
    particle.friction = 0.85;
    particle.velocity = glam::Vec3::new(
        requested_velocity.x * 0.2 + (random.next_float() * 2.0 - 1.0) * 0.02,
        0.0,
        requested_velocity.z * 0.2 + (random.next_float() * 2.0 - 1.0) * 0.02,
    );
    let size_start = 0.02 + 0.02 * random.next_float();
    particle.scale = size_start;
    particle.max_age = i32::MAX;
    let y_start = position.y;
    particle.sulfur_bubble = Some(SulfurBubbleState {
        y_start,
        y_end: y_start + 3.0,
        size_start,
        y_prev: position.y,
    });
    particle.network_kind = Some(NetworkParticleKind::SulfurBubbles);
    particle
}
fn java_firework_spark_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_single_quad_4arg_base(position, rect, random);
    particle.friction = 0.91;
    particle.gravity = 0.1;
    particle.velocity = requested_velocity;
    particle.scale *= 0.75;
    particle.max_age = 48 + random.next_int_bound(12);
    particle.alpha = 0.99;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::FireworkSpark);
    particle
}
fn java_firefly_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.velocity = glam::Vec3::new(
        0.5 - random.next_float(),
        if random.next_boolean() {
            requested_velocity.y
        } else {
            -requested_velocity.y
        },
        0.5 - random.next_float(),
    );
    particle.friction = 0.96;
    particle.speed_up_when_y_motion_is_blocked = true;
    particle.scale *= 0.75;
    particle.velocity *= 0.8;
    particle.max_age = 200 + random.next_int_bound(101);
    particle.scale *= 1.5;
    particle.alpha = 0.0;
    particle.translucent = true;
    particle.network_kind = Some(NetworkParticleKind::Firefly);
    particle
}
fn java_trial_spawner_detection_particle(
    position: glam::Vec3,
    requested_velocity: glam::Vec3,
    rect: SpriteUvRect,
    random: &mut impl RandomSource,
    kind: NetworkParticleKind,
) -> Particle {
    let mut particle = java_base_single_quad_particle(position, glam::Vec3::ZERO, rect, random);
    particle.friction = 0.96;
    particle.gravity = -0.1;
    particle.speed_up_when_y_motion_is_blocked = true;
    particle.velocity.x = 0.0;
    particle.velocity.y *= 0.9;
    particle.velocity.z = 0.0;
    particle.velocity += requested_velocity;
    particle.scale *= 0.75 * 1.5;
    let rand_between = 0.5 + random.next_float() * 0.5;
    particle.max_age = ((8.0 / rand_between) * 1.5) as i32;
    particle.max_age = particle.max_age.max(1);
    particle.network_kind = Some(kind);
    particle
}
pub struct ParticleEngine {
    pub particle_atlas: TextureAtlas,
    pub particles: Vec<Particle>,
    pending_torch_particles: Vec<Particle>,
    pending_entity_emitter_particles: Vec<Particle>,
    entity_tracking_emitters: Vec<EntityTrackingEmitter>,
    torch_animation_position_random: LegacyRandomSource,
    torch_provider_random: LegacyRandomSource,
    vertex_buffer: Option<(wgpu::Buffer, usize)>,
    index_buffer: Option<(wgpu::Buffer, usize)>,
    block_index_buffer: Option<(wgpu::Buffer, usize)>,
    vertex_scratch: Vec<crate::render::baked_model::Vertex>,
    particle_index_scratch: Vec<u32>,
    block_index_scratch: Vec<u32>,
    translucent_particle_index_scratch: Vec<u32>,
    translucent_block_index_scratch: Vec<u32>,
}
impl ParticleEngine {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_bind_group_layout: &wgpu::BindGroupLayout,
        resource_manager: &ClientResourceManager,
    ) -> Result<Self, crate::resources::manager::ResourceError> {
        log::info!("Scanning and loading Particle sprites...");
        let sprite_ids =
            resource_manager.list_texture_ids_in_directory("minecraft", "particle", "particle/")?;
        let particle_atlas = TextureAtlas::from_resource_ids(
            device,
            queue,
            texture_bind_group_layout,
            resource_manager,
            &sprite_ids,
        )?;
        log::info!("Packed particle atlas with {} sprites", sprite_ids.len());
        Ok(Self {
            particle_atlas,
            particles: Vec::new(),
            pending_torch_particles: Vec::new(),
            pending_entity_emitter_particles: Vec::new(),
            entity_tracking_emitters: Vec::new(),
            torch_animation_position_random: LegacyRandomSource::new(java_unique_particle_seed()),
            torch_provider_random: LegacyRandomSource::new(java_unique_particle_seed()),
            vertex_buffer: None,
            index_buffer: None,
            block_index_buffer: None,
            vertex_scratch: Vec::new(),
            particle_index_scratch: Vec::new(),
            block_index_scratch: Vec::new(),
            translucent_particle_index_scratch: Vec::new(),
            translucent_block_index_scratch: Vec::new(),
        })
    }
    pub fn spawn_particle(
        &mut self,
        sprite_id: &ResourceLocation,
        pos: glam::Vec3,
        vel: glam::Vec3,
        scale: f32,
        max_age: i32,
        gravity: f32,
        color: [f32; 4],
    ) {
        let Some(rect) = self.particle_atlas.sprites.get(sprite_id) else {
            log::warn!("Missing particle sprite: {}", sprite_id);
            return;
        };
        self.particles.push(Particle {
            position: pos,
            previous_position: pos,
            velocity: vel,
            color,
            light: [1.0, 1.0],
            scale,
            age: 0,
            max_age,
            gravity,
            u0: rect.u0,
            v0: rect.v0,
            u1: rect.u1,
            v1: rect.v1,
            texture_type: ParticleTextureType::ParticleAtlas,
            friction: 0.98,
            behavior: ParticleBehavior::Generic,
            network_kind: None,
            alpha: 1.0,
            fade_color: None,
            has_physics: true,
            speed_up_when_y_motion_is_blocked: false,
            counts_down_lifetime: false,
            origin: None,
            angle: 0.0,
            target: None,
            delay: 0,
            translucent: false,
            geyser_plume_max_y: 0.0,
            geyser_plume_propulsion: 0.0,
            geyser_plume_spray: [0.0, 0.0],
            geyser_plume_size: [0.0, 0.0],
            leaves: None,
            sulfur_bubble: None,
            geyser_eruption_water_blocks: 0,
        });
    }
    fn particle_sprite_rect(&self, sprite_id: &str) -> Option<SpriteUvRect> {
        let id = ResourceLocation::new("minecraft", &format!("particle/{}", sprite_id));
        self.particle_atlas.sprites.get(&id).copied()
    }
    pub fn spawn_network_particle(
        &mut self,
        particle: &el_protocol_backend::BackendParticle,
        pos: glam::Vec3,
        vel: glam::Vec3,
        block_particle: Option<(&TextureAtlas, SpriteUvRect, [f32; 4])>,
    ) -> bool {
        self.spawn_network_particle_queued(particle, pos, vel, block_particle, false)
    }
    fn spawn_network_particle_deferred(
        &mut self,
        particle: &el_protocol_backend::BackendParticle,
        pos: glam::Vec3,
        vel: glam::Vec3,
    ) -> bool {
        self.spawn_network_particle_queued(particle, pos, vel, None, true)
    }
    fn spawn_network_particle_queued(
        &mut self,
        particle: &el_protocol_backend::BackendParticle,
        pos: glam::Vec3,
        vel: glam::Vec3,
        block_particle: Option<(&TextureAtlas, SpriteUvRect, [f32; 4])>,
        defer_until_after_tick: bool,
    ) -> bool {
        let mut random = LegacyRandomSource::new(java_unique_particle_seed());
        let spawn = |engine: &mut Self, p: Particle| {
            if defer_until_after_tick {
                engine.pending_entity_emitter_particles.push(p);
            } else {
                engine.particles.push(p);
            }
            true
        };
        match particle {
            el_protocol_backend::BackendParticle::Simple { name: id } => {
                let name = id.strip_prefix("minecraft:").unwrap_or(id);
                if matches!(name, "item_cobweb" | "item_slime" | "item_snowball") {
                    return false;
                }
                let build = (|| -> Option<Particle> {
                    let rect_of = |engine: &Self, sprite: &str| -> Option<SpriteUvRect> {
                        engine.particle_sprite_rect(sprite)
                    };
                    let particle = match name {
                        "poof" => java_explode_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                        ),
                        "flame" => java_flame_network_particle(
                            pos,
                            vel,
                            rect_of(self, "flame")?,
                            &mut random,
                            NetworkParticleKind::Flame,
                        ),
                        "soul_fire_flame" => java_flame_network_particle(
                            pos,
                            vel,
                            rect_of(self, "soul_fire_flame")?,
                            &mut random,
                            NetworkParticleKind::SoulFlame,
                        ),
                        "small_flame" => {
                            let mut p = java_flame_network_particle(
                                pos,
                                vel,
                                rect_of(self, "flame")?,
                                &mut random,
                                NetworkParticleKind::SmallFlame,
                            );
                            p.scale *= 0.5;
                            p
                        }
                        "smoke" => java_ash_smoke_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                            NetworkParticleKind::Smoke,
                            1.0,
                        ),
                        "large_smoke" => java_ash_smoke_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                            NetworkParticleKind::LargeSmoke,
                            2.5,
                        ),
                        "white_smoke" => {
                            let mut p = java_base_ash_smoke_particle(
                                pos,
                                vel,
                                rect_of(self, "generic_7")?,
                                &mut random,
                                glam::Vec3::splat(0.1),
                                0.3,
                                8.0,
                                -0.1,
                                true,
                                Some([0.7294118, 0.69411767, 0.7607843]),
                            );
                            p.network_kind = Some(NetworkParticleKind::WhiteSmoke);
                            p
                        }
                        "crit" => java_crit_particle(
                            pos,
                            vel,
                            rect_of(self, "critical_hit")?,
                            &mut random,
                            NetworkParticleKind::Crit,
                        ),
                        "enchanted_hit" => java_crit_particle(
                            pos,
                            vel,
                            rect_of(self, "enchanted_hit")?,
                            &mut random,
                            NetworkParticleKind::EnchantedHit,
                        ),
                        "damage_indicator" => java_crit_particle(
                            pos,
                            vel + glam::Vec3::new(0.0, 1.0, 0.0),
                            rect_of(self, "damage")?,
                            &mut random,
                            NetworkParticleKind::DamageIndicator,
                        ),
                        "end_rod" => java_end_rod_particle(
                            pos,
                            vel,
                            rect_of(self, "glitter_7")?,
                            &mut random,
                        ),
                        "heart" => java_heart_particle(pos, rect_of(self, "heart")?, &mut random),
                        "angry_villager" => {
                            let mut p = java_heart_particle(
                                glam::Vec3::new(pos.x, pos.y + 0.5, pos.z),
                                rect_of(self, "angry")?,
                                &mut random,
                            );
                            p.network_kind = Some(NetworkParticleKind::AngryVillager);
                            p
                        }
                        "note" => {
                            java_note_particle(pos, rect_of(self, "note")?, vel.x, &mut random)
                        }
                        "splash" => {
                            java_splash_particle(pos, vel, rect_of(self, "splash_0")?, &mut random)
                        }
                        "lava" => java_lava_particle(pos, rect_of(self, "lava")?, &mut random),
                        "portal" | "reverse_portal" => {
                            let index = random.next_int_bound(8);
                            let rect = rect_of(self, &format!("generic_{}", index))?;
                            if name == "portal" {
                                java_portal_particle(pos, vel, rect, &mut random)
                            } else {
                                java_reverse_portal_particle(pos, vel, rect, &mut random)
                            }
                        }
                        "bubble" => {
                            java_bubble_particle(pos, vel, rect_of(self, "bubble")?, &mut random)
                        }
                        "bubble_column_up" => java_bubble_column_up_particle(
                            pos,
                            vel,
                            rect_of(self, "bubble")?,
                            &mut random,
                        ),
                        "current_down" => {
                            java_current_down_particle(pos, rect_of(self, "bubble")?, &mut random)
                        }
                        "bubble_pop" => java_bubble_pop_particle(
                            pos,
                            vel,
                            rect_of(self, "bubble_pop_0")?,
                            &mut random,
                        ),
                        "soul" => java_soul_particle(
                            pos,
                            vel,
                            rect_of(self, "soul_0")?,
                            &mut random,
                            false,
                        ),
                        "sculk_soul" => java_soul_particle(
                            pos,
                            vel,
                            rect_of(self, "sculk_soul_0")?,
                            &mut random,
                            true,
                        ),
                        "snowflake" => java_snowflake_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                        ),
                        "spit" => {
                            java_spit_particle(pos, vel, rect_of(self, "generic_7")?, &mut random)
                        }
                        "sneeze" => java_player_cloud_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                            true,
                        ),
                        "cloud" => java_player_cloud_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                            false,
                        ),
                        "mycelium" => java_suspended_town_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_0")?,
                            &mut random,
                        ),
                        "composter" => {
                            let mut p = java_suspended_town_particle(
                                pos,
                                vel,
                                rect_of(self, "glint")?,
                                &mut random,
                            );
                            p.color = [1.0, 1.0, 1.0, 1.0];
                            p.max_age = 3 + random.next_int_bound(5);
                            p.network_kind = Some(NetworkParticleKind::Composter);
                            p
                        }
                        "dolphin" => {
                            let mut p = java_suspended_town_particle(
                                pos,
                                vel,
                                rect_of(self, "generic_0")?,
                                &mut random,
                            );
                            p.color = [0.3, 0.5, 1.0, 1.0];
                            p.alpha = 1.0 - random.next_float() * 0.7;
                            p.max_age /= 2;
                            p.network_kind = Some(NetworkParticleKind::Dolphin);
                            p
                        }
                        "egg_crack" => {
                            let mut p = java_suspended_town_particle(
                                pos,
                                vel,
                                rect_of(self, "glint")?,
                                &mut random,
                            );
                            p.color = [1.0, 1.0, 1.0, 1.0];
                            p.network_kind = Some(NetworkParticleKind::EggCrack);
                            p
                        }
                        "happy_villager" => {
                            let mut p = java_suspended_town_particle(
                                pos,
                                vel,
                                rect_of(self, "glint")?,
                                &mut random,
                            );
                            p.color = [1.0, 1.0, 1.0, 1.0];
                            p.network_kind = Some(NetworkParticleKind::HappyVillager);
                            p
                        }
                        "underwater" => {
                            let mut p = java_suspended_particle(
                                pos,
                                rect_of(self, "generic_0")?,
                                &mut random,
                            );
                            p.color = [0.4, 0.4, 0.7, 1.0];
                            p
                        }
                        "crimson_spore" => {
                            let mut p = java_suspended_particle_7arg(
                                pos,
                                glam::Vec3::new(
                                    random.next_gaussian() as f32 * 1.0e-6,
                                    random.next_gaussian() as f32 * 1.0e-4,
                                    random.next_gaussian() as f32 * 1.0e-6,
                                ),
                                rect_of(self, "generic_0")?,
                                &mut random,
                            );
                            p.color = [0.9, 0.4, 0.5, 1.0];
                            p.network_kind = Some(NetworkParticleKind::CrimsonSpore);
                            p
                        }
                        "warped_spore" => {
                            let ya = random.next_float() * -1.9 * random.next_float() * 0.1;
                            let mut p = java_suspended_particle_7arg(
                                pos,
                                glam::Vec3::new(0.0, ya, 0.0),
                                rect_of(self, "generic_0")?,
                                &mut random,
                            );
                            p.color = [0.1, 0.1, 0.3, 1.0];
                            p.scale *= 0.005;
                            p.network_kind = Some(NetworkParticleKind::WarpedSpore);
                            p
                        }
                        "spore_blossom_air" => {
                            let mut p = java_suspended_particle_7arg(
                                pos,
                                glam::Vec3::new(0.0, -0.8, 0.0),
                                rect_of(self, "drip_fall")?,
                                &mut random,
                            );
                            p.max_age = 500 + random.next_int_bound(501);
                            p.gravity = 0.01;
                            p.color = [0.32, 0.5, 0.22, 1.0];
                            p.network_kind = Some(NetworkParticleKind::SporeBlossomAir);
                            p
                        }
                        "ash" => {
                            let mut p = java_base_ash_smoke_particle(
                                pos,
                                glam::Vec3::ZERO,
                                rect_of(self, "generic_0")?,
                                &mut random,
                                glam::Vec3::new(0.1, -0.1, 0.1),
                                0.5,
                                20.0,
                                0.1,
                                false,
                                None,
                            );
                            p.network_kind = Some(NetworkParticleKind::Ash);
                            p
                        }
                        "white_ash" => {
                            let xa = random.next_float() * -1.9 * random.next_float() * 0.1;
                            let ya = random.next_float() * -0.5 * random.next_float() * 0.1 * 5.0;
                            let za = random.next_float() * -1.9 * random.next_float() * 0.1;
                            let mut p = java_base_ash_smoke_particle(
                                pos,
                                glam::Vec3::new(xa, ya, za),
                                rect_of(self, "generic_0")?,
                                &mut random,
                                glam::Vec3::new(0.1, -0.1, 0.1),
                                0.0,
                                20.0,
                                0.0125,
                                false,
                                Some([0.7294, 0.6235, 0.8863]),
                            );
                            p.network_kind = Some(NetworkParticleKind::WhiteAsh);
                            p
                        }
                        "campfire_cosy_smoke" => java_campfire_smoke_particle(
                            pos,
                            vel,
                            rect_of(self, "big_smoke_0")?,
                            &mut random,
                            false,
                        ),
                        "campfire_signal_smoke" => java_campfire_smoke_particle(
                            pos,
                            vel,
                            rect_of(self, "big_smoke_0")?,
                            &mut random,
                            true,
                        ),
                        "witch" => {
                            let mut p = java_spell_particle(
                                pos,
                                vel,
                                rect_of(self, "spell_7")?,
                                &mut random,
                            );
                            let b = random.next_float() * 0.5 + 0.35;
                            p.color = [b, 0.0, b, 1.0];
                            p.network_kind = Some(NetworkParticleKind::Witch);
                            p
                        }
                        "totem_of_undying" => {
                            java_totem_particle(pos, vel, rect_of(self, "glitter_7")?, &mut random)
                        }
                        "electric_spark" | "glow" | "wax_on" | "wax_off" | "scrape" => {
                            java_glow_particle(pos, vel, rect_of(self, "glow")?, &mut random, name)
                        }
                        "squid_ink" => java_squid_ink_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                            false,
                        ),
                        "glow_squid_ink" => java_squid_ink_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                            true,
                        ),
                        "copper_fire_flame" => java_flame_network_particle(
                            pos,
                            vel,
                            rect_of(self, "copper_fire_flame")?,
                            &mut random,
                            NetworkParticleKind::CopperFlame,
                        ),
                        "dust_plume" => java_dust_plume_particle(
                            pos,
                            vel,
                            rect_of(self, "generic_7")?,
                            &mut random,
                        ),
                        "enchant" => {
                            let index = random.next_int_bound(26);
                            let rect =
                                rect_of(self, &format!("sga_{}", (b'a' + index as u8) as char))?;
                            java_fly_towards_particle(
                                pos,
                                vel,
                                rect,
                                &mut random,
                                NetworkParticleKind::Enchant,
                            )
                        }
                        "nautilus" => java_fly_towards_particle(
                            pos,
                            vel,
                            rect_of(self, "nautilus")?,
                            &mut random,
                            NetworkParticleKind::Nautilus,
                        ),
                        "vault_connection" => java_fly_towards_particle(
                            pos,
                            vel,
                            rect_of(self, "vault_connection")?,
                            &mut random,
                            NetworkParticleKind::VaultConnection,
                        ),
                        "cherry_leaves" | "pale_oak_leaves" | "tinted_leaves" => {
                            let index = random.next_int_bound(12);
                            let (prefix, kind) = match name {
                                "cherry_leaves" => ("cherry", NetworkParticleKind::CherryLeaves),
                                "pale_oak_leaves" => {
                                    ("pale_oak", NetworkParticleKind::PaleOakLeaves)
                                }
                                _ => ("leaf", NetworkParticleKind::TintedLeaves),
                            };
                            let rect = rect_of(self, &format!("{prefix}_{index}"))?;
                            java_falling_leaves_particle(pos, rect, &mut random, kind)
                        }
                        "sweep_attack" => java_attack_sweep_particle(
                            pos,
                            vel.x,
                            rect_of(self, "sweep_0")?,
                            &mut random,
                        ),
                        "sculk_charge_pop" => java_sculk_charge_pop_particle(
                            pos,
                            vel,
                            rect_of(self, "sculk_charge_pop_0")?,
                            &mut random,
                        ),
                        "infested" => {
                            let mut p = java_spell_particle(
                                pos,
                                vel,
                                rect_of(self, "infested")?,
                                &mut random,
                            );
                            p.network_kind = Some(NetworkParticleKind::Infested);
                            p
                        }
                        "raid_omen" => {
                            let mut p = java_spell_particle(
                                pos,
                                vel,
                                rect_of(self, "raid_omen")?,
                                &mut random,
                            );
                            p.network_kind = Some(NetworkParticleKind::RaidOmen);
                            p
                        }
                        "trial_omen" => {
                            let mut p = java_spell_particle(
                                pos,
                                vel,
                                rect_of(self, "trial_omen")?,
                                &mut random,
                            );
                            p.network_kind = Some(NetworkParticleKind::TrialOmen);
                            p
                        }
                        "pause_mob_growth" => java_simple_vertical_particle(
                            pos,
                            vel,
                            rect_of(self, "glint")?,
                            &mut random,
                            false,
                            NetworkParticleKind::PauseMobGrowth,
                        ),
                        "reset_mob_growth" => java_simple_vertical_particle(
                            pos,
                            vel,
                            rect_of(self, "glint")?,
                            &mut random,
                            true,
                            NetworkParticleKind::ResetMobGrowth,
                        ),
                        "fishing" => {
                            java_wake_particle(pos, vel, rect_of(self, "splash_0")?, &mut random)
                        }
                        "firework" => java_firework_spark_particle(
                            pos,
                            vel,
                            rect_of(self, "spark_7")?,
                            &mut random,
                        ),
                        "explosion" => java_huge_explosion_particle(
                            pos,
                            vel.x,
                            rect_of(self, "explosion_0")?,
                            &mut random,
                            NetworkParticleKind::Explosion,
                        ),
                        "sonic_boom" => java_huge_explosion_particle(
                            pos,
                            vel.x,
                            rect_of(self, "sonic_boom_0")?,
                            &mut random,
                            NetworkParticleKind::SonicBoom,
                        ),
                        "gust" => {
                            java_gust_particle(pos, rect_of(self, "gust_0")?, &mut random, false)
                        }
                        "small_gust" => java_gust_particle(
                            pos,
                            rect_of(self, "small_gust_0")?,
                            &mut random,
                            true,
                        ),
                        "sulfur_bubbles" => java_sulfur_bubble_particle(
                            pos,
                            vel,
                            rect_of(self, "bubble_white")?,
                            &mut random,
                        ),
                        "noxious_gas" => java_noxious_gas_particle(
                            pos,
                            vel,
                            rect_of(self, "noxious_gas_01")?,
                            &mut random,
                        ),
                        "ominous_spawning" => java_fly_straight_towards_particle(
                            pos,
                            vel,
                            rect_of(self, "ominous_spawning")?,
                            &mut random,
                        ),
                        "trial_spawner_detection" => java_trial_spawner_detection_particle(
                            pos,
                            vel,
                            rect_of(self, "trial_spawner_detection_0")?,
                            &mut random,
                            NetworkParticleKind::TrialSpawnerDetection,
                        ),
                        "trial_spawner_detection_ominous" => java_trial_spawner_detection_particle(
                            pos,
                            vel,
                            rect_of(self, "trial_spawner_detection_ominous_0")?,
                            &mut random,
                            NetworkParticleKind::TrialSpawnerDetectionOminous,
                        ),
                        "firefly" => {
                            java_firefly_particle(pos, vel, rect_of(self, "firefly")?, &mut random)
                        }
                        "rain" => {
                            let index = random.next_int_bound(4);
                            let mut p = java_water_drop_particle(
                                pos,
                                rect_of(self, &format!("splash_{}", index))?,
                                &mut random,
                            );
                            p.network_kind = Some(NetworkParticleKind::Rain);
                            p
                        }
                        "dripping_water" => java_drip_hang_particle(
                            pos,
                            rect_of(self, "drip_hang")?,
                            &mut random,
                            NetworkParticleKind::DrippingWater,
                        ),
                        "dripping_dripstone_water" => java_drip_hang_particle(
                            pos,
                            rect_of(self, "drip_hang")?,
                            &mut random,
                            NetworkParticleKind::DrippingDripstoneWater,
                        ),
                        "dripping_honey" => java_drip_hang_particle(
                            pos,
                            rect_of(self, "drip_hang")?,
                            &mut random,
                            NetworkParticleKind::DrippingHoney,
                        ),
                        "dripping_obsidian_tear" => java_drip_hang_particle(
                            pos,
                            rect_of(self, "drip_hang")?,
                            &mut random,
                            NetworkParticleKind::DrippingObsidianTear,
                        ),
                        "dripping_lava" => java_drip_hang_particle(
                            pos,
                            rect_of(self, "drip_hang")?,
                            &mut random,
                            NetworkParticleKind::DrippingLava,
                        ),
                        "dripping_dripstone_lava" => java_drip_hang_particle(
                            pos,
                            rect_of(self, "drip_hang")?,
                            &mut random,
                            NetworkParticleKind::DrippingDripstoneLava,
                        ),
                        "falling_water" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingWater,
                        ),
                        "falling_dripstone_water" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingDripstoneWater,
                        ),
                        "falling_honey" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingHoney,
                        ),
                        "falling_obsidian_tear" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingObsidianTear,
                        ),
                        "falling_lava" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingLava,
                        ),
                        "falling_dripstone_lava" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingDripstoneLava,
                        ),
                        "falling_nectar" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingNectar,
                        ),
                        "falling_spore_blossom" => java_drip_fall_particle(
                            pos,
                            rect_of(self, "drip_fall")?,
                            &mut random,
                            NetworkParticleKind::FallingSporeBlossom,
                        ),
                        "landing_lava" => java_drip_land_particle(
                            pos,
                            rect_of(self, "drip_land")?,
                            &mut random,
                            NetworkParticleKind::LandingLava,
                        ),
                        "landing_honey" => java_drip_land_particle(
                            pos,
                            rect_of(self, "drip_land")?,
                            &mut random,
                            NetworkParticleKind::LandingHoney,
                        ),
                        "landing_obsidian_tear" => java_drip_land_particle(
                            pos,
                            rect_of(self, "drip_land")?,
                            &mut random,
                            NetworkParticleKind::LandingObsidianTear,
                        ),
                        "sulfur_cube_goo" => java_sulfur_cube_goo_particle(
                            pos,
                            rect_of(self, "sulfur_cube_goo")?,
                            &mut random,
                        ),
                        "explosion_emitter" => java_huge_explosion_seed_particle(pos, &mut random),
                        "gust_emitter_large" => java_gust_seed_particle(
                            pos,
                            &mut random,
                            NetworkParticleKind::GustEmitterLarge,
                        ),
                        "gust_emitter_small" => java_gust_seed_particle(
                            pos,
                            &mut random,
                            NetworkParticleKind::GustEmitterSmall,
                        ),
                        "noxious_gas_cloud" => java_noxious_gas_cloud_particle(pos, &mut random),
                        "elder_guardian" => java_elder_guardian_particle(pos, &mut random),
                        other => {
                            let direct = format!("particle/{}", other);
                            let indexed = format!("particle/{}_0", other);
                            let sprite_id = ResourceLocation::new("minecraft", &direct);
                            let sprite_id_0 = ResourceLocation::new("minecraft", &indexed);
                            if let Some(rect) = self.particle_atlas.sprites.get(&sprite_id).copied()
                            {
                                java_explode_particle(pos, vel, rect, &mut random)
                            } else if let Some(rect) =
                                self.particle_atlas.sprites.get(&sprite_id_0).copied()
                            {
                                java_explode_particle(pos, vel, rect, &mut random)
                            } else {
                                return None;
                            }
                        }
                    };
                    Some(particle)
                })();
                let Some(build) = build else {
                    return false;
                };
                spawn(self, build)
            }
            el_protocol_backend::BackendParticle::Dust { color, scale } => {
                let Some(rect) = self.particle_sprite_rect("generic_7") else {
                    return false;
                };
                let p = java_dust_particle(pos, vel, rect, *color, *scale, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Block { block_state_id: _ } => {
                let Some((_block_atlas, rect, tint)) = block_particle else {
                    return false;
                };
                let p = java_terrain_network_particle(pos, vel, rect, tint, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::BlockMarker { block_state_id: _ } => {
                let Some((_block_atlas, rect, _tint)) = block_particle else {
                    return false;
                };
                let p = java_block_marker_particle(pos, rect, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::FallingDust { block_state_id: _ } => {
                let Some((_block_atlas, rect, tint)) = block_particle else {
                    return false;
                };
                let p = java_falling_dust_particle(pos, rect, tint, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::DustPillar { block_state_id: _ } => {
                let Some((_block_atlas, rect, tint)) = block_particle else {
                    return false;
                };
                let p = java_dust_pillar_particle(pos, vel, rect, tint, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::BlockCrumble { block_state_id: _ } => {
                let Some((_block_atlas, rect, tint)) = block_particle else {
                    return false;
                };
                let p = java_block_crumble_particle(pos, vel, rect, tint, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::DustColorTransition { from, to, scale } => {
                let Some(rect) = self.particle_sprite_rect("generic_7") else {
                    return false;
                };
                let p = java_dust_color_transition_particle(
                    pos,
                    vel,
                    rect,
                    *from,
                    *to,
                    *scale,
                    &mut random,
                );
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::EntityEffect { color } => {
                let Some(rect) = self.particle_sprite_rect("effect_7") else {
                    return false;
                };
                let mut p = java_spell_particle(pos, vel, rect, &mut random);
                p.color = [color[0], color[1], color[2], 1.0];
                p.network_kind = Some(NetworkParticleKind::EntityEffect);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Effect { color, power } => {
                let Some(rect) = self.particle_sprite_rect("effect_7") else {
                    return false;
                };
                let mut p = java_spell_particle(pos, vel, rect, &mut random);
                p.color = [color[0], color[1], color[2], 1.0];
                p.velocity *= *power;
                p.velocity.y = (p.velocity.y - 0.1) * power + 0.1;
                p.network_kind = Some(NetworkParticleKind::Effect);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::InstantEffect { color, power } => {
                let Some(rect) = self.particle_sprite_rect("spell_7") else {
                    return false;
                };
                let mut p = java_spell_particle(pos, vel, rect, &mut random);
                p.color = [color[0], color[1], color[2], 1.0];
                p.velocity *= *power;
                p.velocity.y = (p.velocity.y - 0.1) * power + 0.1;
                p.network_kind = Some(NetworkParticleKind::Effect);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Item { .. } => false,
            el_protocol_backend::BackendParticle::SculkCharge { roll } => {
                let Some(rect) = self.particle_sprite_rect("sculk_charge_0") else {
                    return false;
                };
                let p = java_sculk_charge_particle(pos, vel, rect, &mut random, *roll);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Shriek { delay } => {
                let Some(rect) = self.particle_sprite_rect("shriek") else {
                    return false;
                };
                let p = java_shriek_particle(pos, rect, &mut random, *delay);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Vibration {
                x,
                y,
                z,
                entity_id: _,
                y_offset: _,
                ticks,
            } => {
                let Some(rect) = self.particle_sprite_rect("vibration") else {
                    return false;
                };
                let p = java_vibration_particle(
                    pos,
                    rect,
                    &mut random,
                    glam::Vec3::new(*x as f32, *y as f32, *z as f32),
                    *ticks as i32,
                );
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Geyser { water_blocks } => {
                let p = java_geyser_eruption_particle(pos, vel, *water_blocks, &mut random);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::GeyserBase {
                water_blocks,
                burst_impulse_base,
            } => {
                let Some(rect) = self.particle_sprite_rect("geyser_base_01") else {
                    return false;
                };
                let p = java_geyser_base_particle(
                    pos,
                    vel,
                    rect,
                    &mut random,
                    *water_blocks,
                    *burst_impulse_base,
                );
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::DragonBreath { power } => {
                let Some(rect) = self.particle_sprite_rect("generic_5") else {
                    return false;
                };
                let p = java_dragon_breath_particle(pos, vel, rect, &mut random, *power);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Flash { color } => {
                let Some(rect) = self.particle_sprite_rect("flash") else {
                    return false;
                };
                let p = java_flash_particle(pos, rect, *color, 1.0);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::Trail {
                target,
                color,
                duration,
            } => {
                let Some(rect) = self.particle_sprite_rect("generic_0") else {
                    return false;
                };
                let p =
                    java_trail_particle(pos, vel, rect, &mut random, *target, *color, *duration);
                spawn(self, p)
            }
            el_protocol_backend::BackendParticle::TintedLeaves { color } => {
                let index = random.next_int_bound(12);
                let Some(rect) = self.particle_sprite_rect(&format!("leaf_{index}")) else {
                    return false;
                };
                let mut p = java_falling_leaves_particle(
                    pos,
                    rect,
                    &mut random,
                    NetworkParticleKind::TintedLeaves,
                );
                p.color = [color[0], color[1], color[2], 1.0];
                spawn(self, p)
            }
        }
    }
    pub fn spawn_tracking_emitter_burst(
        &mut self,
        pos: glam::Vec3,
        width: f32,
        height: f32,
        particle: &el_protocol_backend::BackendParticle,
        ticks: usize,
    ) {
        use el_math::random::{LegacyRandomSource, RandomSource};
        let mut random = LegacyRandomSource::new(java_unique_particle_seed());
        for _ in 0..ticks {
            for _ in 0..16 {
                let xa = random.next_float() * 2.0 - 1.0;
                let ya = random.next_float() * 2.0 - 1.0;
                let za = random.next_float() * 2.0 - 1.0;
                if xa * xa + ya * ya + za * za <= 1.0 {
                    let p = glam::Vec3::new(
                        pos.x + width * (xa / 4.0),
                        pos.y + height * (0.5 + ya / 4.0),
                        pos.z + width * (za / 4.0),
                    );
                    let vel = glam::Vec3::new(xa, ya + 0.2, za);
                    self.spawn_network_particle(particle, p, vel, None);
                }
            }
        }
    }
    pub fn spawn_entity_tracking_emitter(
        &mut self,
        entity_id: i32,
        pos: glam::Vec3,
        width: f32,
        height: f32,
        particle: &el_protocol_backend::BackendParticle,
    ) {
        self.spawn_entity_tracking_emitter_with_lifetime(
            entity_id, pos, width, height, particle, 3,
        );
    }
    pub fn spawn_entity_tracking_emitter_with_lifetime(
        &mut self,
        entity_id: i32,
        pos: glam::Vec3,
        width: f32,
        height: f32,
        particle: &el_protocol_backend::BackendParticle,
        lifetime: usize,
    ) {
        let target = TrackingEmitterTarget {
            position: pos,
            width,
            height,
        };
        let mut emitter = EntityTrackingEmitter::new(
            entity_id,
            particle.clone(),
            target,
            java_unique_particle_seed(),
            lifetime,
        );
        for (position, velocity) in emitter.emit_tick() {
            self.spawn_network_particle_deferred(particle, position, velocity);
        }
        if emitter.additional_ticks > 0 {
            self.entity_tracking_emitters.push(emitter);
        }
    }
    pub fn tick_entity_tracking_emitters(
        &mut self,
        targets: &std::collections::HashMap<i32, (glam::Vec3, f32, f32)>,
    ) {
        let mut to_spawn = Vec::new();
        for emitter in &mut self.entity_tracking_emitters {
            let target = targets
                .get(&emitter.entity_id)
                .map(|&(position, width, height)| TrackingEmitterTarget {
                    position,
                    width,
                    height,
                });
            let Some(samples) = emitter.advance_tick(target) else {
                continue;
            };
            let particle = emitter.particle.clone();
            to_spawn.extend(
                samples
                    .into_iter()
                    .map(|(position, velocity)| (particle.clone(), position, velocity)),
            );
        }
        self.entity_tracking_emitters
            .retain(|emitter| emitter.additional_ticks > 0);
        for (particle, position, velocity) in to_spawn {
            self.spawn_network_particle_deferred(&particle, position, velocity);
        }
    }
    pub fn spawn_block_crack_particle(
        &mut self,
        block_pos: el_math::block_pos::BlockPos,
        direction: el_math::Direction,
        block_state: &el_core::block_state::BlockState,
        block_atlas: &TextureAtlas,
        baked_particle_rect: Option<SpriteUvRect>,
        tint: [f32; 4],
    ) {
        let block_name = &block_state.block.path;
        if block_state.is_known_vanilla_air()
            || matches!(
                block_name.as_str(),
                "moving_piston" | "barrier" | "structure_void"
            )
        {
            return;
        }
        let tex_id = ResourceLocation::new("minecraft", &format!("block/{}", block_name));
        let rect = match baked_particle_rect.or_else(|| block_atlas.sprites.get(&tex_id).copied()) {
            Some(rect) => rect,
            None => return,
        };
        use el_math::random::RandomSource;
        let mut rng = el_math::random::WorldgenRandom::new(Box::new(
            el_math::random::LegacyRandomSource::new(java_unique_particle_seed()),
        ));
        let shape = block_state.get_outline_shape();
        if shape.is_empty() {
            return;
        }
        let bounds = shape.bounds();
        let x = block_pos.x as f64;
        let y = block_pos.y as f64;
        let z = block_pos.z as f64;
        let inset = 0.1_f32 as f64;
        let diff_x = bounds.max_x - bounds.min_x - 0.2_f32 as f64;
        let diff_y = bounds.max_y - bounds.min_y - 0.2_f32 as f64;
        let diff_z = bounds.max_z - bounds.min_z - 0.2_f32 as f64;
        let mut xp = x + rng.next_double() * diff_x + inset + bounds.min_x;
        let mut yp = y + rng.next_double() * diff_y + inset + bounds.min_y;
        let mut zp = z + rng.next_double() * diff_z + inset + bounds.min_z;
        match direction {
            el_math::Direction::Down => yp = y + bounds.min_y - inset,
            el_math::Direction::Up => yp = y + bounds.max_y + inset,
            el_math::Direction::North => zp = z + bounds.min_z - inset,
            el_math::Direction::South => zp = z + bounds.max_z + inset,
            el_math::Direction::West => xp = x + bounds.min_x - inset,
            el_math::Direction::East => xp = x + bounds.max_x + inset,
        }
        let pos = glam::Vec3::new(xp as f32, yp as f32, zp as f32);
        let particle_color = [0.6 * tint[0], 0.6 * tint[1], 0.6 * tint[2], tint[3]];
        let mut xd = (rng.next_float() * 2.0 - 1.0) * 0.4;
        let mut yd = (rng.next_float() * 2.0 - 1.0) * 0.4;
        let mut zd = (rng.next_float() * 2.0 - 1.0) * 0.4;
        let speed = (rng.next_float() + rng.next_float() + 1.0) * 0.15;
        let length = (xd * xd + yd * yd + zd * zd).sqrt();
        xd = xd / length * speed * 0.4;
        yd = yd / length * speed * 0.4 + 0.1;
        zd = zd / length * speed * 0.4;
        xd *= 0.2;
        yd = (yd - 0.1) * 0.2 + 0.1;
        zd *= 0.2;
        let vel = glam::Vec3::new(xd, yd, zd);
        let u_size = rect.u1 - rect.u0;
        let v_size = rect.v1 - rect.v0;
        let uo = rng.next_float() * 3.0;
        let vo = rng.next_float() * 3.0;
        let sub_u0 = rect.u0 + u_size * ((uo + 1.0) / 4.0);
        let sub_v0 = rect.v0 + v_size * (vo / 4.0);
        let sub_u1 = rect.u0 + u_size * (uo / 4.0);
        let sub_v1 = rect.v0 + v_size * ((vo + 1.0) / 4.0);
        let scale = (0.05 + rng.next_float() * 0.05) * 0.6;
        let max_age = (4.0 / (rng.next_float() * 0.9 + 0.1)) as i32;
        self.particles.push(Particle {
            position: pos,
            previous_position: pos,
            velocity: vel,
            color: particle_color,
            light: [1.0, 1.0],
            scale,
            age: 0,
            max_age,
            gravity: 1.0,
            u0: sub_u0,
            v0: sub_v0,
            u1: sub_u1,
            v1: sub_v1,
            texture_type: ParticleTextureType::BlockAtlas,
            friction: 0.98,
            behavior: ParticleBehavior::Generic,
            network_kind: None,
            alpha: 1.0,
            fade_color: None,
            has_physics: true,
            speed_up_when_y_motion_is_blocked: false,
            counts_down_lifetime: false,
            origin: None,
            angle: 0.0,
            target: None,
            delay: 0,
            translucent: false,
            geyser_plume_max_y: 0.0,
            geyser_plume_propulsion: 0.0,
            geyser_plume_spray: [0.0, 0.0],
            geyser_plume_size: [0.0, 0.0],
            leaves: None,
            sulfur_bubble: None,
            geyser_eruption_water_blocks: 0,
        });
    }
    pub fn spawn_block_break_particles(
        &mut self,
        block_pos: el_math::block_pos::BlockPos,
        block_state: &el_core::block_state::BlockState,
        block_atlas: &TextureAtlas,
        baked_particle_rect: Option<SpriteUvRect>,
        tint: [f32; 4],
    ) {
        let block_name = &block_state.block.path;
        if block_state.is_known_vanilla_air()
            || matches!(
                block_name.as_str(),
                "moving_piston" | "barrier" | "structure_void"
            )
        {
            return;
        }
        let tex_id = ResourceLocation::new("minecraft", &format!("block/{}", block_name));
        let rect = match baked_particle_rect.or_else(|| block_atlas.sprites.get(&tex_id).copied()) {
            Some(rect) => rect,
            None => return,
        };
        let mut rng = el_math::random::WorldgenRandom::new(Box::new(
            el_math::random::LegacyRandomSource::new(java_unique_particle_seed()),
        ));
        let particle_color = [0.6 * tint[0], 0.6 * tint[1], 0.6 * tint[2], tint[3]];
        let shape = block_state.get_outline_shape();
        for bounds in shape.to_aabbs() {
            let width_x = (bounds.max_x - bounds.min_x).min(1.0);
            let width_y = (bounds.max_y - bounds.min_y).min(1.0);
            let width_z = (bounds.max_z - bounds.min_z).min(1.0);
            let count_x = 2.max((width_x / 0.25).ceil() as usize);
            let count_y = 2.max((width_y / 0.25).ceil() as usize);
            let count_z = 2.max((width_z / 0.25).ceil() as usize);
            for xx in 0..count_x {
                for yy in 0..count_y {
                    for zz in 0..count_z {
                        let rel_x = (xx as f64 + 0.5) / count_x as f64;
                        let rel_y = (yy as f64 + 0.5) / count_y as f64;
                        let rel_z = (zz as f64 + 0.5) / count_z as f64;
                        let pos = glam::Vec3::new(
                            block_pos.x as f32 + (rel_x * width_x + bounds.min_x) as f32,
                            block_pos.y as f32 + (rel_y * width_y + bounds.min_y) as f32,
                            block_pos.z as f32 + (rel_z * width_z + bounds.min_z) as f32,
                        );
                        let vx = (rel_x as f32 - 0.5) + (rng.next_float() * 2.0 - 1.0) * 0.4;
                        let vy = (rel_y as f32 - 0.5) + (rng.next_float() * 2.0 - 1.0) * 0.4;
                        let vz = (rel_z as f32 - 0.5) + (rng.next_float() * 2.0 - 1.0) * 0.4;
                        let speed = (rng.next_float() + rng.next_float() + 1.0) * 0.15;
                        let length = (vx * vx + vy * vy + vz * vz).sqrt().max(f32::EPSILON);
                        let vel = glam::Vec3::new(
                            vx / length * speed * 0.4,
                            vy / length * speed * 0.4 + 0.1,
                            vz / length * speed * 0.4,
                        );
                        let u_size = rect.u1 - rect.u0;
                        let v_size = rect.v1 - rect.v0;
                        let uo = rng.next_float() * 3.0;
                        let vo = rng.next_float() * 3.0;
                        let sub_u0 = rect.u0 + u_size * ((uo + 1.0) / 4.0);
                        let sub_v0 = rect.v0 + v_size * (vo / 4.0);
                        let sub_u1 = rect.u0 + u_size * (uo / 4.0);
                        let sub_v1 = rect.v0 + v_size * ((vo + 1.0) / 4.0);
                        self.particles.push(Particle {
                            position: pos,
                            previous_position: pos,
                            velocity: vel,
                            color: particle_color,
                            light: [1.0, 1.0],
                            scale: 0.05 + rng.next_float() * 0.05,
                            age: 0,
                            max_age: (4.0 / (rng.next_float() * 0.9 + 0.1)) as i32,
                            gravity: 1.0,
                            u0: sub_u0,
                            v0: sub_v0,
                            u1: sub_u1,
                            v1: sub_v1,
                            texture_type: ParticleTextureType::BlockAtlas,
                            friction: 0.98,
                            behavior: ParticleBehavior::Generic,
                            network_kind: None,
                            alpha: 1.0,
                            fade_color: None,
                            has_physics: true,
                            speed_up_when_y_motion_is_blocked: false,
                            counts_down_lifetime: false,
                            origin: None,
                            angle: 0.0,
                            target: None,
                            delay: 0,
                            translucent: false,
                            geyser_plume_max_y: 0.0,
                            geyser_plume_propulsion: 0.0,
                            geyser_plume_spray: [0.0, 0.0],
                            geyser_plume_size: [0.0, 0.0],
                            leaves: None,
                            sulfur_bubble: None,
                            geyser_eruption_water_blocks: 0,
                        });
                    }
                }
            }
        }
    }
    pub fn spawn_weather_particles(
        &mut self,
        level: &crate::world::client_level::ClientLevel,
        camera_pos: glam::Vec3,
        radius: i32,
        rain_level: f32,
        min_y: i32,
        sea_level: i32,
        particle_status: i32,
    ) {
        if rain_level <= 0.0 || radius < 1 {
            return;
        }
        let diameter = radius * 2 + 1;
        let area = diameter * diameter;
        let divisor = if particle_status == 1 { 2 } else { 1 };
        let particle_count = (0.225 * area as f32 * rain_level * rain_level) as i32 / divisor;
        if particle_count <= 0 {
            return;
        }
        let mut random = LegacyRandomSource::new(level.game_time().wrapping_mul(312_987_231));
        let camera_block = BlockPos::new(
            camera_pos.x.floor() as i32,
            camera_pos.y.floor() as i32,
            camera_pos.z.floor() as i32,
        );
        for _ in 0..particle_count {
            let x = camera_block.x + random.next_int_bound(diameter) - radius;
            let z = camera_block.z + random.next_int_bound(diameter) - radius;
            let Some(terrain_height) = level.motion_blocking_height(x, z) else {
                continue;
            };
            if terrain_height <= min_y
                || terrain_height > camera_block.y + 10
                || terrain_height < camera_block.y - 10
                || level.precipitation_at(x, terrain_height, z, sea_level)
                    != crate::render::weather_renderer::WeatherPrecipitation::Rain
            {
                continue;
            }
            let rain_position = BlockPos::new(x, terrain_height - 1, z);
            if particle_status >= 2 {
                break;
            }
            let block_x = random.next_double();
            let block_z = random.next_double();
            let block_state = level.get_block_state(rain_position);
            let block_shape = block_state
                .get_collision_shape_with_context(rain_position, &CollisionContext::empty());
            let block_top = block_shape
                .to_aabbs()
                .iter()
                .filter(|bounds| {
                    block_x >= bounds.min_x
                        && block_x <= bounds.max_x
                        && block_z >= bounds.min_z
                        && block_z <= bounds.max_z
                })
                .map(|bounds| bounds.max_y)
                .fold(0.0_f64, f64::max);
            let fluid_top = f64::from(level.fluid_height(rain_position));
            let particle_position = glam::Vec3::new(
                rain_position.x as f32 + block_x as f32,
                rain_position.y as f32 + block_top.max(fluid_top) as f32,
                rain_position.z as f32 + block_z as f32,
            );
            let is_smoke_surface = block_state.get_fluid_state().is_lava()
                || block_state.block.path == "magma_block"
                || matches!(
                    block_state.block.path.as_str(),
                    "campfire" | "soul_campfire"
                ) && block_state
                    .get_property("lit")
                    .is_some_and(|value| value.eq_ignore_ascii_case("true"));
            let sprite_path = if is_smoke_surface {
                "particle/generic_7".to_string()
            } else {
                let index = self.torch_provider_random.next_int_bound(4);
                format!("particle/splash_{index}")
            };
            let sprite_id = ResourceLocation::new("minecraft", &sprite_path);
            let Some(rect) = self.particle_atlas.sprites.get(&sprite_id).copied() else {
                continue;
            };
            let mut particle_random = LegacyRandomSource::new(java_unique_particle_seed());
            let mut particle = if is_smoke_surface {
                java_base_single_quad_particle(
                    particle_position,
                    glam::Vec3::ZERO,
                    rect,
                    &mut particle_random,
                )
            } else {
                java_water_drop_particle(particle_position, rect, &mut particle_random)
            };
            particle.update_torch_light(level);
            self.pending_torch_particles.push(particle);
        }
    }
    pub fn animate_torch_blocks(
        &mut self,
        level: &crate::world::client_level::ClientLevel,
        player_block_pos: BlockPos,
    ) {
        let mut spawns = Vec::new();
        for_each_java_torch_animate_tick_sample(
            player_block_pos,
            &mut self.torch_animation_position_random,
            |pos, random| {
                let state = level.get_block_state(pos);
                if let Some(pair) = torch_particle_spawns(&state, pos) {
                    spawns.extend(pair);
                }
                if let Some(redstone) = redstone_torch_particle_spawn(&state, pos, random) {
                    spawns.push(redstone);
                }
            },
        );
        for spawn in spawns {
            self.queue_torch_particle(level, spawn);
        }
    }
    fn queue_torch_particle(
        &mut self,
        level: &crate::world::client_level::ClientLevel,
        spawn: TorchParticleSpawn,
    ) {
        let sprite_id = particle_sprite_id(spawn.kind, 0, 1);
        if matches!(spawn.kind, TorchParticleKind::Flame(_)) {
            let _ = self.torch_provider_random.next_int_bound(1);
        }
        let Some(rect) = self.particle_atlas.sprites.get(&sprite_id).copied() else {
            log::warn!("Missing torch particle sprite: {}", sprite_id);
            return;
        };
        let mut random = LegacyRandomSource::new(java_unique_particle_seed());
        let mut particle = match spawn.kind {
            TorchParticleKind::Smoke => {
                java_torch_smoke_particle(spawn.position, rect, &mut random)
            }
            TorchParticleKind::Flame(_) => {
                java_torch_flame_particle(spawn.position, glam::Vec3::ZERO, rect, &mut random)
            }
            TorchParticleKind::RedstoneDust => {
                java_redstone_dust_particle(spawn.position, rect, &mut random)
            }
        };
        particle.update_torch_light(level);
        self.pending_torch_particles.push(particle);
    }
    pub fn tick(&mut self, level: &crate::world::client_level::ClientLevel) {
        let particle_sprites = &self.particle_atlas.sprites;
        let mut to_add: Vec<Particle> = Vec::new();
        for p in &mut self.particles {
            p.previous_position = p.position;
            p.age += 1;
            if p.age > p.max_age {
                continue;
            }
            match p.network_kind {
                Some(
                    NetworkParticleKind::Enchant
                    | NetworkParticleKind::Nautilus
                    | NetworkParticleKind::VaultConnection,
                ) => {
                    let progress = p.age as f32 / p.max_age.max(1) as f32;
                    let pos = 1.0 - progress;
                    let droop = progress * progress * progress * progress;
                    if let Some(origin) = p.origin {
                        p.position = origin + p.velocity * pos;
                        p.position.y -= droop * 1.2;
                    }
                    if p.network_kind == Some(NetworkParticleKind::VaultConnection) {
                        let normalized = (progress - 0.25) / 0.75;
                        p.alpha = normalized.clamp(0.0, 1.0) * 0.6;
                    }
                }
                Some(NetworkParticleKind::OminousSpawning) => {
                    let progress = p.age as f32 / p.max_age.max(1) as f32;
                    if let Some(origin) = p.origin {
                        p.position = origin + p.velocity * (1.0 - progress);
                    }
                    let r = (69.0 + (186.0 * progress).floor()) / 255.0;
                    let g = (174.0 + (81.0 * progress).floor()) / 255.0;
                    let b = (254.0 + progress.floor()) / 255.0;
                    p.color = [r, g, b, 1.0];
                }
                Some(
                    NetworkParticleKind::CherryLeaves
                    | NetworkParticleKind::PaleOakLeaves
                    | NetworkParticleKind::TintedLeaves,
                ) => {
                    if let Some(leaves) = p.leaves.as_mut() {
                        let relative_age = (p.age as f32 / 300.0).min(1.0);
                        let mut xa = 0.0_f64;
                        let mut za = 0.0_f64;
                        if leaves.flow_away {
                            xa += (leaves.xa_flow_scale as f64) * (relative_age as f64).powf(1.25);
                            za += (leaves.za_flow_scale as f64) * (relative_age as f64).powf(1.25);
                        }
                        if leaves.swirl {
                            xa += relative_age as f64
                                * ((relative_age * leaves.swirl_period) as f64).cos()
                                * leaves.wind_big as f64;
                            za += relative_age as f64
                                * ((relative_age * leaves.swirl_period) as f64).sin()
                                * leaves.wind_big as f64;
                        }
                        p.velocity.x += (xa * 0.0025) as f32;
                        p.velocity.z += (za * 0.0025) as f32;
                        p.velocity.y -= p.gravity;
                        leaves.rot_speed += leaves.spin_acceleration / 20.0;
                        p.angle += leaves.rot_speed / 20.0;
                        p.position += p.velocity;
                    }
                }
                Some(NetworkParticleKind::DustPlume) => {
                    p.gravity *= 0.88;
                    p.friction *= 0.92;
                    p.velocity.y -= 0.04 * p.gravity;
                    p.position += p.velocity;
                    p.velocity *= p.friction;
                }
                Some(
                    NetworkParticleKind::Explosion
                    | NetworkParticleKind::SonicBoom
                    | NetworkParticleKind::Gust
                    | NetworkParticleKind::SmallGust
                    | NetworkParticleKind::SweepAttack,
                ) => {}
                Some(
                    NetworkParticleKind::DrippingWater
                    | NetworkParticleKind::DrippingDripstoneWater
                    | NetworkParticleKind::DrippingHoney
                    | NetworkParticleKind::DrippingObsidianTear
                    | NetworkParticleKind::DrippingLava
                    | NetworkParticleKind::DrippingDripstoneLava
                    | NetworkParticleKind::FallingWater
                    | NetworkParticleKind::FallingDripstoneWater
                    | NetworkParticleKind::FallingHoney
                    | NetworkParticleKind::FallingObsidianTear
                    | NetworkParticleKind::FallingLava
                    | NetworkParticleKind::FallingDripstoneLava
                    | NetworkParticleKind::FallingNectar
                    | NetworkParticleKind::FallingSporeBlossom
                    | NetworkParticleKind::LandingLava
                    | NetworkParticleKind::LandingHoney
                    | NetworkParticleKind::LandingObsidianTear,
                ) => {
                    let kind = p.network_kind.unwrap();
                    if matches!(
                        kind,
                        NetworkParticleKind::DrippingLava
                            | NetworkParticleKind::DrippingDripstoneLava
                    ) {
                        p.color[1] = 16.0 / (p.age + 15) as f32;
                        p.color[2] = 4.0 / (p.age + 7) as f32;
                    }
                    if let Some(cascade) = drip_hang_cascade_kind(kind) {
                        if p.age >= p.max_age {
                            if let Some(rect) = particle_sprites
                                .get(&ResourceLocation::new("minecraft", "particle/drip_fall"))
                                .copied()
                            {
                                let mut child_random =
                                    LegacyRandomSource::new(java_unique_particle_seed());
                                let child = java_drip_fall_particle(
                                    p.position,
                                    rect,
                                    &mut child_random,
                                    cascade,
                                );
                                to_add.push(child);
                            }
                            p.age = p.max_age + 1;
                        } else {
                            p.velocity.y -= p.gravity;
                            p.position += p.velocity;
                            p.velocity *= 0.02;
                            p.velocity *= 0.98;
                        }
                    } else {
                        p.velocity.y -= p.gravity;
                        p.position += p.velocity;
                        p.velocity *= 0.98;
                        let fluid_is_water = matches!(
                            kind,
                            NetworkParticleKind::FallingWater
                                | NetworkParticleKind::FallingDripstoneWater
                        );
                        let fluid_is_lava = matches!(
                            kind,
                            NetworkParticleKind::FallingLava
                                | NetworkParticleKind::FallingDripstoneLava
                                | NetworkParticleKind::LandingLava
                        );
                        if fluid_is_water || fluid_is_lava {
                            let pos = BlockPos::new(
                                p.position.x.floor() as i32,
                                p.position.y.floor() as i32,
                                p.position.z.floor() as i32,
                            );
                            let fluid = level.get_block_state(pos).get_fluid_state();
                            let matches_fluid = if fluid_is_water {
                                fluid.is_water()
                            } else {
                                fluid.is_lava()
                            };
                            let height = if fluid.is_source() {
                                1.0
                            } else {
                                fluid.own_height()
                            };
                            if matches_fluid && p.position.y < pos.y as f32 + height {
                                p.age = p.max_age + 1;
                            }
                        }
                    }
                }
                Some(NetworkParticleKind::ExplosionEmitter) => {
                    if let Some(rect) = particle_sprites
                        .get(&ResourceLocation::new("minecraft", "particle/explosion_0"))
                        .copied()
                    {
                        let mut random = LegacyRandomSource::new(java_unique_particle_seed());
                        for _ in 0..6 {
                            let xx =
                                p.position.x + (random.next_float() - random.next_float()) * 4.0;
                            let yy =
                                p.position.y + (random.next_float() - random.next_float()) * 4.0;
                            let zz =
                                p.position.z + (random.next_float() - random.next_float()) * 4.0;
                            let mut child_random =
                                LegacyRandomSource::new(java_unique_particle_seed());
                            let child = java_huge_explosion_particle(
                                glam::Vec3::new(xx, yy, zz),
                                (p.age - 1) as f32 / p.max_age.max(1) as f32,
                                rect,
                                &mut child_random,
                                NetworkParticleKind::Explosion,
                            );
                            to_add.push(child);
                        }
                    }
                    if p.age == p.max_age {
                        p.age = p.max_age + 1;
                    }
                }
                Some(
                    NetworkParticleKind::GustEmitterLarge | NetworkParticleKind::GustEmitterSmall,
                ) => {
                    let kind = p.network_kind.unwrap();
                    let (scale, delay, lifetime) = match kind {
                        NetworkParticleKind::GustEmitterLarge => (3.0_f32, 0_i32, 7_i32),
                        NetworkParticleKind::GustEmitterSmall => (1.0_f32, 2_i32, 3_i32),
                        _ => unreachable!(),
                    };
                    if (p.age - 1) % (delay + 1) == 0 {
                        if let Some(rect) = particle_sprites
                            .get(&ResourceLocation::new("minecraft", "particle/gust_0"))
                            .copied()
                        {
                            let mut random = LegacyRandomSource::new(java_unique_particle_seed());
                            for _ in 0..3 {
                                let x = p.position.x
                                    + (random.next_float() - random.next_float()) * scale;
                                let y = p.position.y
                                    + (random.next_float() - random.next_float()) * scale;
                                let z = p.position.z
                                    + (random.next_float() - random.next_float()) * scale;
                                let mut child_random =
                                    LegacyRandomSource::new(java_unique_particle_seed());
                                let child = java_gust_particle(
                                    glam::Vec3::new(x, y, z),
                                    rect,
                                    &mut child_random,
                                    false,
                                );
                                to_add.push(child);
                            }
                        }
                    }
                    if p.age >= lifetime {
                        p.age = p.max_age + 1;
                    }
                }
                Some(NetworkParticleKind::NoxiousGasCloud) => {
                    p.velocity.y -= 0.04 * p.gravity;
                    p.position += p.velocity;
                    p.velocity *= p.friction;
                    if p.age % 2 == 0 {
                        let mut random = LegacyRandomSource::new(java_unique_particle_seed());
                        let dir_x = random.next_float() - 0.5;
                        let dir_z = random.next_float() - 0.5;
                        let distance = random.next_float() * 3.0;
                        let length = (dir_x * dir_x + dir_z * dir_z).sqrt().max(f32::EPSILON);
                        let spawn_pos = glam::Vec3::new(
                            p.position.x.floor() + 0.5 + dir_x / length * distance,
                            p.position.y.floor() + 0.5 - 0.25,
                            p.position.z.floor() + 0.5 + dir_z / length * distance,
                        );
                        if noxious_gas_can_reach(level, p.position, spawn_pos) {
                            if let Some(rect) = particle_sprites
                                .get(&ResourceLocation::new(
                                    "minecraft",
                                    "particle/noxious_gas_01",
                                ))
                                .copied()
                            {
                                let mut child_random =
                                    LegacyRandomSource::new(java_unique_particle_seed());
                                let child = java_noxious_gas_particle(
                                    spawn_pos,
                                    glam::Vec3::ZERO,
                                    rect,
                                    &mut child_random,
                                );
                                to_add.push(child);
                            }
                        }
                    }
                }
                Some(NetworkParticleKind::GeyserEruption) => {
                    if p.age % 2 == 0 {
                        for _ in 0..2 {
                            let mut child_random =
                                LegacyRandomSource::new(java_unique_particle_seed());
                            if let Some(rect) = particle_sprites
                                .get(&ResourceLocation::new(
                                    "minecraft",
                                    "particle/geyser_base_01",
                                ))
                                .copied()
                            {
                                let child_pos = p.position
                                    + glam::Vec3::new(
                                        (child_random.next_float() - 0.5) * 0.5,
                                        (child_random.next_float() - 0.5) * 0.5 + 0.2,
                                        (child_random.next_float() - 0.5) * 0.5,
                                    );
                                let child = java_geyser_base_particle(
                                    child_pos,
                                    p.velocity,
                                    rect,
                                    &mut child_random,
                                    p.geyser_eruption_water_blocks,
                                    1.5,
                                );
                                to_add.push(child);
                            }
                        }
                    }
                    for _ in 0..p.geyser_eruption_water_blocks + 2 {
                        let mut child_random = LegacyRandomSource::new(java_unique_particle_seed());
                        if let Some(rect) = particle_sprites
                            .get(&ResourceLocation::new(
                                "minecraft",
                                "particle/geyser_plume_01",
                            ))
                            .copied()
                        {
                            let child_pos = p.position
                                + glam::Vec3::new(
                                    (child_random.next_float() - 0.5) * 0.2,
                                    child_random.next_float(),
                                    (child_random.next_float() - 0.5) * 0.2,
                                );
                            let child = java_geyser_plume_particle(
                                child_pos,
                                p.velocity,
                                rect,
                                &mut child_random,
                                p.geyser_eruption_water_blocks,
                            );
                            to_add.push(child);
                        }
                    }
                    if p.age % 10 == 0 {
                        for _ in 0..20 {
                            let mut child_random =
                                LegacyRandomSource::new(java_unique_particle_seed());
                            if let Some(rect) = particle_sprites
                                .get(&ResourceLocation::new(
                                    "minecraft",
                                    "particle/geyser_base_01",
                                ))
                                .copied()
                            {
                                let child_pos = p.position
                                    + glam::Vec3::new(
                                        (child_random.next_float() - 0.5) * 0.5,
                                        (child_random.next_float() - 0.5) * 0.5 + 0.2,
                                        (child_random.next_float() - 0.5) * 0.5,
                                    );
                                let child = java_geyser_base_particle(
                                    child_pos,
                                    p.velocity,
                                    rect,
                                    &mut child_random,
                                    p.geyser_eruption_water_blocks,
                                    2.0,
                                );
                                to_add.push(child);
                            }
                        }
                    }
                }
                _ => {
                    p.velocity.y -= 0.04 * p.gravity;
                    p.position += p.velocity;
                    p.velocity *= p.friction;
                }
            }
            if matches!(
                p.behavior,
                ParticleBehavior::TorchSmoke | ParticleBehavior::RedstoneDust
            ) {
                let sprite_id = particle_sprite_id(TorchParticleKind::Smoke, p.age, p.max_age);
                if let Some(rect) = particle_sprites.get(&sprite_id) {
                    p.u0 = rect.u0;
                    p.v0 = rect.v0;
                    p.u1 = rect.u1;
                    p.v1 = rect.v1;
                }
            }
            if let Some(kind) = p.network_kind {
                let animated_sprite = match kind {
                    NetworkParticleKind::Poof
                    | NetworkParticleKind::Smoke
                    | NetworkParticleKind::LargeSmoke
                    | NetworkParticleKind::Dust
                    | NetworkParticleKind::DustPlume => {
                        let index = p.age * 7 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/generic_{}", (7 - index).clamp(0, 7)),
                        ))
                    }
                    NetworkParticleKind::EndRod => {
                        let index = p.age * 7 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/glitter_{}", (7 - index).clamp(0, 7)),
                        ))
                    }
                    NetworkParticleKind::NoxiousGas => {
                        let index = p.age * 7 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/noxious_gas_0{}", index.clamp(0, 7) + 1),
                        ))
                    }
                    NetworkParticleKind::SculkChargePop => {
                        let index = p.age * 3 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/sculk_charge_pop_{}", index.clamp(0, 3)),
                        ))
                    }
                    NetworkParticleKind::TrialSpawnerDetection
                    | NetworkParticleKind::TrialSpawnerDetectionOminous => {
                        let index = p.age * 4 / p.max_age.max(1);
                        let prefix = if kind == NetworkParticleKind::TrialSpawnerDetection {
                            "trial_spawner_detection"
                        } else {
                            "trial_spawner_detection_ominous"
                        };
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/{}_{}", prefix, index.clamp(0, 4)),
                        ))
                    }
                    NetworkParticleKind::Explosion => {
                        let index = p.age * 15 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/explosion_{}", index.clamp(0, 15)),
                        ))
                    }
                    NetworkParticleKind::SonicBoom => {
                        let index = p.age * 15 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/sonic_boom_{}", index.clamp(0, 15)),
                        ))
                    }
                    NetworkParticleKind::Gust => {
                        let index = p.age * 11 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/gust_{}", index.clamp(0, 11)),
                        ))
                    }
                    NetworkParticleKind::SmallGust => {
                        let index = p.age * 6 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/small_gust_{}", index.clamp(0, 6)),
                        ))
                    }
                    NetworkParticleKind::SweepAttack => {
                        let index = p.age * 7 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/sweep_{}", index.clamp(0, 7)),
                        ))
                    }
                    NetworkParticleKind::FireworkSpark => {
                        let index = p.age * 7 / p.max_age.max(1);
                        Some(ResourceLocation::new(
                            "minecraft",
                            &format!("particle/spark_{}", (7 - index).clamp(0, 7)),
                        ))
                    }
                    _ => None,
                };
                if let Some(sprite_id) = animated_sprite {
                    if let Some(rect) = particle_sprites.get(&sprite_id) {
                        p.u0 = rect.u0;
                        p.v0 = rect.v0;
                        p.u1 = rect.u1;
                        p.v1 = rect.v1;
                    }
                }
                if matches!(
                    kind,
                    NetworkParticleKind::Crit
                        | NetworkParticleKind::EnchantedHit
                        | NetworkParticleKind::DamageIndicator
                ) {
                    p.color[1] *= 0.96;
                    p.color[2] *= 0.9;
                }
                if kind == NetworkParticleKind::DustColorTransition {
                    if let Some(fade) = p.fade_color {
                        let progress = (p.age as f32) / (p.max_age as f32 + 1.0);
                        p.color[0] = p.color[0] + (fade[0] - p.color[0]) * progress;
                        p.color[1] = p.color[1] + (fade[1] - p.color[1]) * progress;
                        p.color[2] = p.color[2] + (fade[2] - p.color[2]) * progress;
                    }
                }
                if matches!(
                    kind,
                    NetworkParticleKind::CampfireCosySmoke
                        | NetworkParticleKind::CampfireSignalSmoke
                ) {
                    if p.age >= p.max_age - 60 && p.alpha > 0.01 {
                        p.alpha = (p.alpha - 0.015).max(0.0);
                    }
                }
                if matches!(
                    kind,
                    NetworkParticleKind::EndRod | NetworkParticleKind::FireworkSpark
                ) {
                    if p.age > p.max_age / 2 {
                        p.alpha = 1.0 - (p.age - p.max_age / 2) as f32 / p.max_age as f32;
                        if let Some(fade) = p.fade_color {
                            p.color[0] += (fade[0] - p.color[0]) * 0.2;
                            p.color[1] += (fade[1] - p.color[1]) * 0.2;
                            p.color[2] += (fade[2] - p.color[2]) * 0.2;
                        }
                    }
                }
                if kind == NetworkParticleKind::NoxiousGas {
                    let fade_start = p.max_age as f32 / 2.0;
                    if p.age as f32 > fade_start {
                        let since = p.age as f32 - fade_start;
                        p.alpha = (p.max_age as f32 - since) / p.max_age.max(1) as f32;
                    }
                }
                if kind == NetworkParticleKind::Firefly {
                    let pos = BlockPos::new(
                        p.position.x.floor() as i32,
                        p.position.y.floor() as i32,
                        p.position.z.floor() as i32,
                    );
                    if !level.get_block_state(pos).is_known_vanilla_air() {
                        p.max_age = p.age - 1;
                    } else {
                        let progress = (p.age as f32 / p.max_age.max(1) as f32).clamp(0.0, 1.0);
                        p.alpha = java_firefly_fade_amount(progress, 0.3, 0.5);
                    }
                }
                if kind == NetworkParticleKind::Fishing {
                    let life = 60 - p.max_age + (p.age - 1);
                    let frame = (life % 4) * 3 / 4;
                    if let Some(rect) = particle_sprites.get(&ResourceLocation::new(
                        "minecraft",
                        &format!("particle/splash_{}", frame),
                    )) {
                        p.u0 = rect.u0;
                        p.v0 = rect.v0;
                        p.u1 = rect.u1;
                        p.v1 = rect.v1;
                    }
                }
                if kind == NetworkParticleKind::SulfurBubbles {
                    if let Some(sulfur) = p.sulfur_bubble.as_mut() {
                        let pos = BlockPos::new(
                            p.position.x.floor() as i32,
                            p.position.y.floor() as i32,
                            p.position.z.floor() as i32,
                        );
                        let fluid = level.get_block_state(pos).get_fluid_state();
                        let in_source_water = fluid.is_water() && fluid.is_source();
                        if !in_source_water
                            || p.position.y >= sulfur.y_end
                            || p.position.y <= sulfur.y_prev
                        {
                            p.max_age = p.age - 1;
                        } else {
                            let travel =
                                (p.position.y - sulfur.y_start) / (sulfur.y_end - sulfur.y_start);
                            p.scale = sulfur.size_start + travel * (0.15 - sulfur.size_start);
                        }
                        sulfur.y_prev = p.position.y;
                    }
                }
            }
            p.update_torch_light(level);
        }
        self.particles.retain(|p| p.age <= p.max_age);
        self.particles.append(&mut to_add);
        self.particles.append(&mut self.pending_torch_particles);
        self.particles
            .append(&mut self.pending_entity_emitter_particles);
    }
    pub fn render<'rp>(
        &'rp mut self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        render_pass: &mut wgpu::RenderPass<'rp>,
        camera_bind_group: &'rp wgpu::BindGroup,
        block_bind_group: &'rp wgpu::BindGroup,
        opaque_pipeline: &'rp wgpu::RenderPipeline,
        pipeline: &'rp wgpu::RenderPipeline,
        camera_right: glam::Vec3,
        camera_up: glam::Vec3,
        partial_tick: f32,
    ) {
        if self.particles.is_empty() {
            return;
        }
        self.vertex_scratch.clear();
        self.particle_index_scratch.clear();
        self.block_index_scratch.clear();
        self.translucent_particle_index_scratch.clear();
        self.translucent_block_index_scratch.clear();
        let vertices = &mut self.vertex_scratch;
        let particle_batch = &mut self.particle_index_scratch;
        let block_batch = &mut self.block_index_scratch;
        let translucent_particle_batch = &mut self.translucent_particle_index_scratch;
        let translucent_block_batch = &mut self.translucent_block_index_scratch;
        let mut index_offset = 0;
        for p in &self.particles {
            if matches!(
                p.network_kind,
                Some(
                    NetworkParticleKind::ExplosionEmitter
                        | NetworkParticleKind::GustEmitterLarge
                        | NetworkParticleKind::GustEmitterSmall
                        | NetworkParticleKind::NoxiousGasCloud
                        | NetworkParticleKind::ElderGuardian
                )
            ) {
                continue;
            }
            let start_idx = index_offset;
            let scale = p.render_scale(partial_tick);
            let position = p.render_position(partial_tick);
            let render_color = [p.color[0], p.color[1], p.color[2], p.color[3] * p.alpha];
            let p0 = position - camera_right * scale - camera_up * scale;
            let p1 = position + camera_right * scale - camera_up * scale;
            let p2 = position + camera_right * scale + camera_up * scale;
            let p3 = position - camera_right * scale + camera_up * scale;
            let normal = [0.0, 1.0, 0.0];
            let v0 = crate::render::baked_model::Vertex {
                position: p0.into(),
                tex_coords: [p.u0, p.v1],
                normal,
                color: render_color,
                light: p.light,
            };
            let v1 = crate::render::baked_model::Vertex {
                position: p1.into(),
                tex_coords: [p.u1, p.v1],
                normal,
                color: render_color,
                light: p.light,
            };
            let v2 = crate::render::baked_model::Vertex {
                position: p2.into(),
                tex_coords: [p.u1, p.v0],
                normal,
                color: render_color,
                light: p.light,
            };
            let v3 = crate::render::baked_model::Vertex {
                position: p3.into(),
                tex_coords: [p.u0, p.v0],
                normal,
                color: render_color,
                light: p.light,
            };
            vertices.push(v0);
            vertices.push(v1);
            vertices.push(v2);
            vertices.push(v3);
            let quad_indices = [
                start_idx,
                start_idx + 1,
                start_idx + 2,
                start_idx,
                start_idx + 2,
                start_idx + 3,
            ];
            match (p.texture_type, p.translucent) {
                (ParticleTextureType::ParticleAtlas, false) => {
                    particle_batch.extend(quad_indices);
                }
                (ParticleTextureType::BlockAtlas, false) => {
                    block_batch.extend(quad_indices);
                }
                (ParticleTextureType::ParticleAtlas, true) => {
                    translucent_particle_batch.extend(quad_indices);
                }
                (ParticleTextureType::BlockAtlas, true) => {
                    translucent_block_batch.extend(quad_indices);
                }
            }
            index_offset += 4;
        }
        if vertices.is_empty() {
            return;
        }
        let opaque_particle_count = particle_batch.len() as u32;
        let opaque_block_count = block_batch.len() as u32;
        particle_batch.append(translucent_particle_batch);
        block_batch.append(translucent_block_batch);
        let vertex_bytes = bytemuck::cast_slice(&vertices);
        if let Some((buffer, capacity)) = &mut self.vertex_buffer {
            if *capacity >= vertex_bytes.len() {
                _queue.write_buffer(buffer, 0, vertex_bytes);
            } else {
                let new_cap = (vertex_bytes.len() * 2).max(65536);
                let new_buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Particle Vertices"),
                    size: new_cap as u64,
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                _queue.write_buffer(&new_buf, 0, vertex_bytes);
                *buffer = new_buf;
                *capacity = new_cap;
            }
        } else {
            let cap = (vertex_bytes.len() * 2).max(65536);
            let buf = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Particle Vertices"),
                size: cap as u64,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });
            _queue.write_buffer(&buf, 0, vertex_bytes);
            self.vertex_buffer = Some((buf, cap));
        }
        render_pass.set_bind_group(0, camera_bind_group, &[]);
        render_pass.set_pipeline(opaque_pipeline);
        render_pass.set_vertex_buffer(
            0,
            self.vertex_buffer
                .as_ref()
                .unwrap()
                .0
                .slice(..vertex_bytes.len() as u64),
        );
        if !particle_batch.is_empty() {
            let particle_index_bytes = bytemuck::cast_slice(&particle_batch);
            if let Some((buffer, capacity)) = &mut self.index_buffer {
                if *capacity >= particle_index_bytes.len() {
                    _queue.write_buffer(buffer, 0, particle_index_bytes);
                } else {
                    let new_cap = (particle_index_bytes.len() * 2).max(32768);
                    let new_buf = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Particle Index Buffer"),
                        size: new_cap as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    _queue.write_buffer(&new_buf, 0, particle_index_bytes);
                    *buffer = new_buf;
                    *capacity = new_cap;
                }
            } else {
                let cap = (particle_index_bytes.len() * 2).max(32768);
                let buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Particle Index Buffer"),
                    size: cap as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                _queue.write_buffer(&buf, 0, particle_index_bytes);
                self.index_buffer = Some((buf, cap));
            }
            render_pass.set_bind_group(1, &self.particle_atlas.bind_group, &[]);
            render_pass.set_index_buffer(
                self.index_buffer
                    .as_ref()
                    .unwrap()
                    .0
                    .slice(..particle_index_bytes.len() as u64),
                wgpu::IndexFormat::Uint32,
            );
            if opaque_particle_count > 0 {
                render_pass.draw_indexed(0..opaque_particle_count, 0, 0..1);
            }
        }
        if !block_batch.is_empty() {
            let block_index_bytes = bytemuck::cast_slice(&block_batch);
            if let Some((buffer, capacity)) = &mut self.block_index_buffer {
                if *capacity >= block_index_bytes.len() {
                    _queue.write_buffer(buffer, 0, block_index_bytes);
                } else {
                    let new_cap = (block_index_bytes.len() * 2).max(32768);
                    let new_buf = device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Block Index Buffer"),
                        size: new_cap as u64,
                        usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                    _queue.write_buffer(&new_buf, 0, block_index_bytes);
                    *buffer = new_buf;
                    *capacity = new_cap;
                }
            } else {
                let cap = (block_index_bytes.len() * 2).max(32768);
                let buf = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Block Index Buffer"),
                    size: cap as u64,
                    usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                    mapped_at_creation: false,
                });
                _queue.write_buffer(&buf, 0, block_index_bytes);
                self.block_index_buffer = Some((buf, cap));
            }
            render_pass.set_bind_group(1, block_bind_group, &[]);
            render_pass.set_index_buffer(
                self.block_index_buffer
                    .as_ref()
                    .unwrap()
                    .0
                    .slice(..block_index_bytes.len() as u64),
                wgpu::IndexFormat::Uint32,
            );
            if opaque_block_count > 0 {
                render_pass.draw_indexed(0..opaque_block_count, 0, 0..1);
            }
        }
        render_pass.set_pipeline(pipeline);
        if particle_batch.len() as u32 > opaque_particle_count {
            render_pass.set_bind_group(1, &self.particle_atlas.bind_group, &[]);
            render_pass.set_index_buffer(
                self.index_buffer.as_ref().unwrap().0.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(opaque_particle_count..particle_batch.len() as u32, 0, 0..1);
        }
        if block_batch.len() as u32 > opaque_block_count {
            render_pass.set_bind_group(1, block_bind_group, &[]);
            render_pass.set_index_buffer(
                self.block_index_buffer.as_ref().unwrap().0.slice(..),
                wgpu::IndexFormat::Uint32,
            );
            render_pass.draw_indexed(opaque_block_count..block_batch.len() as u32, 0, 0..1);
        }
    }
}
