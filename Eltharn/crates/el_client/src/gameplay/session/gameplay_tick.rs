use super::ClientSession;
use super::interaction::{AttackClickDecision, AttackClickGuards, classify_attack_click};
use super::screen_tick::{ClientTickGate, ScreenTickHost};
use super::use_item_decision::{UseClickDecision, UseClickGuards, classify_use_tick};
use crate::client::state::{consumable_starts_active_use, get_firework_flight_duration};
use crate::gameplay::entity::remote::{
    RemoteBoatInputLatch, is_remote_abstract_boat, is_remote_abstract_chest_boat, is_remote_raft,
    remote_abstract_boat_is_locally_controlled, remote_entity_type_key,
};
use crate::gameplay::player::local_player::LocalPlayer;
use crate::gameplay::player::types::UnderwaterSoundCue;
use crate::input::KeyAction;
use crate::network::protocol_backend::RemotePacketSender;
use crate::render::item_in_hand_renderer;
use crate::render::model::player::player_model::HandSide;
use el_math::Direction;
use el_math::block_pos::BlockPos;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locomotion {
    Flying,
    Walking,
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GameplayEffect {
    SpawnBlockCrack {
        pos: BlockPos,
        dir: Direction,
    },
    SpawnCriticalTrackingEmitter {
        entity_id: i32,
        position: glam::Vec3,
        width: f32,
        height: f32,
    },
}
#[derive(Clone, Debug, Default)]
pub struct GameplayTickOutcome {
    pub effects: Vec<GameplayEffect>,
    pub underwater_cue: Option<UnderwaterSoundCue>,
    pub movement_held_debug: bool,
}
pub trait GameplayTickHost: ScreenTickHost {
    fn attempt_block_use(&mut self, hand: usize) -> bool;
    fn open_chat_screen(&mut self, initial: Option<String>);
    fn open_inventory_screen(&mut self);
    fn open_advancements_screen(&mut self);
}
pub struct GameplayTickInput {
    pub gate: ClientTickGate,
    pub mouse_grabbed: bool,
    pub screen_active: bool,
    pub block_hit: Option<(BlockPos, Direction)>,
    pub target_entity: Option<i32>,
    pub interaction_target: Option<crate::client::state::RemoteEntityRaycastHit>,
    pub crosshair_dir: Direction,
}
pub fn tick_gameplay_player_while_active<H: GameplayTickHost>(
    host: &mut H,
    input: &GameplayTickInput,
    locomotion: Locomotion,
) -> GameplayTickOutcome {
    let mut outcome = GameplayTickOutcome::default();
    let gate = input.gate;
    let held_vehicle_id = match locomotion {
        Locomotion::Flying => None,
        Locomotion::Walking => host.session().remote_vehicle_entity_id,
    };
    host.session_mut().remote_movement_was_held = !gate.local_player_loaded;
    if !gate.local_player_loaded {
        outcome.movement_held_debug = host.session().remote_connection.is_some();
        host.session_mut()
            .finish_handle_keybinds_miss_time(input.screen_active);
        host.tick_camera();
        super::screen_tick::tick_remote_entities(host, gate.runs_normally, held_vehicle_id);
        if locomotion != Locomotion::Flying {
            if let Some(vehicle_id) = held_vehicle_id {
                if let (Some((left, right)), Some(packet_tx)) = (
                    host.tick_controlled_vehicle(vehicle_id),
                    host.session()
                        .remote_connection
                        .as_ref()
                        .map(|connection| connection.packet_tx.clone()),
                ) {
                    packet_tx.send_paddle_boat(left, right);
                }
                attach_local_player_to_remote_vehicle(host, vehicle_id);
                latch_remote_boat_input_after_ride_tick(host, vehicle_id);
            }
        }
        return outcome;
    }
    let can_destroy = !host
        .session()
        .game_mode
        .local_player_mode()
        .is_block_placing_restricted();
    let active_destroy = host.session().is_left_mouse_pressed && input.mouse_grabbed && can_destroy;
    let target_entity = input.target_entity;
    let block_target = input.block_hit.map(|(pos, _)| pos);
    let (block_attack_clicks, click_instant_attack) = dispatch_attack_clicks(
        host,
        target_entity,
        block_target.is_some(),
        &mut outcome.effects,
    );
    let (click_swing, started_instant_attack) = {
        let session = host.session_mut();
        match session.level.as_mut() {
            Some(level) => {
                let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
                let mut local_conn = session.connection.as_mut();
                ClientSession::start_block_attack_clicks_for_tick(
                    &mut session.game_mode,
                    level,
                    block_target,
                    block_attack_clicks,
                    input.crosshair_dir,
                    &mut session.player,
                    session.remote_player_entity_id.unwrap_or(0),
                    session.active_hotbar_slot,
                    remote_conn,
                    &mut local_conn,
                    &mut session.remote_action_sequence,
                    &mut session.miss_time,
                )
            }
            None => (false, false),
        }
    };
    let instant_attack = click_instant_attack || started_instant_attack;
    dispatch_use_tick(host, input.interaction_target, input.screen_active);
    host.tick_camera();
    host.tick_active_item_use();
    let mut block_swing = click_swing;
    let continued = {
        let session = host.session_mut();
        match session.level.as_mut() {
            Some(level) => {
                let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
                let mut local_conn = session.connection.as_mut();
                ClientSession::continue_block_action_for_tick(
                    &mut session.game_mode,
                    level,
                    block_target,
                    active_destroy,
                    instant_attack,
                    input.crosshair_dir,
                    &mut session.player,
                    session.remote_player_entity_id.unwrap_or(0),
                    session.active_hotbar_slot,
                    remote_conn,
                    &mut local_conn,
                    &mut session.remote_action_sequence,
                    &mut session.miss_time,
                )
            }
            None => false,
        }
    };
    if continued {
        block_swing = true;
        if let Some(pos) = block_target {
            outcome.effects.push(GameplayEffect::SpawnBlockCrack {
                pos,
                dir: input.crosshair_dir,
            });
        }
    }
    host.session_mut()
        .finish_handle_keybinds_miss_time(input.screen_active);
    let passenger_vehicle_id = match locomotion {
        Locomotion::Flying => None,
        Locomotion::Walking => host.session().remote_vehicle_entity_id,
    };
    let vehicle_for_entities = passenger_vehicle_id;
    super::screen_tick::tick_remote_entities(host, gate.runs_normally, vehicle_for_entities);
    host.session_mut().tick_creative_flight_toggle_like_java();
    host.session_mut().enforce_spectator_flying_like_java();
    let controlled = host.controlled_camera();
    host.session_mut()
        .player
        .set_controlled_camera_like_java(controlled);
    if host.session().level.is_some() {
        if block_swing {
            let main_hand = host.main_hand();
            host.session_mut()
                .start_swing_for_hand(main_hand, main_hand);
        }
        let main_hand = host.main_hand();
        host.session_mut().tick_attack_animation_once(main_hand);
    }
    let travelled = match locomotion {
        Locomotion::Flying => {
            let flying_speed = host.session().game_mode.abilities().flying_speed;
            host.session_mut().tick_flying_player_travel(flying_speed)
        }
        Locomotion::Walking => {
            if passenger_vehicle_id.is_some() {
                let boat_paddle = passenger_vehicle_id
                    .and_then(|vehicle_id| host.tick_controlled_vehicle(vehicle_id));
                if let (Some((left, right)), Some(packet_tx)) = (
                    boat_paddle,
                    host.session()
                        .remote_connection
                        .as_ref()
                        .map(|connection| connection.packet_tx.clone()),
                ) {
                    packet_tx.send_paddle_boat(left, right);
                }
            }
            host.session_mut()
                .tick_walking_player_travel(passenger_vehicle_id)
        }
    };
    if travelled {
        outcome.underwater_cue = host.session_mut().player.pending_underwater_sound.take();
    }
    if matches!(locomotion, Locomotion::Flying)
        && host.session().player.on_ground
        && host.session().player_flying
        && !host.session().game_mode.is_spectator()
    {
        host.session_mut().game_mode.set_flying(false);
        host.session_mut()
            .reconcile_player_flying_from_game_mode(true);
    }
    match locomotion {
        Locomotion::Flying => {
            let controlled = host.controlled_camera();
            host.session_mut().send_local_move_if_needed(controlled);
            if host.session().remote_connection.is_none() {
                let enabled = host.auto_jump_enabled();
                sync_auto_jump_after_send(host, controlled, enabled);
            }
            if let Some(packet_tx) = host
                .session()
                .remote_connection
                .as_ref()
                .map(|conn| conn.packet_tx.clone())
            {
                host.session_mut()
                    .send_remote_fall_flying_before_player_input_if_needed(&packet_tx);
                let sent_input = host.session().player.input.key_presses.clone();
                let session = host.session_mut();
                ClientSession::send_remote_player_input_if_changed(
                    &packet_tx,
                    &sent_input,
                    &mut session.remote_last_sent_input,
                );
                host.session_mut()
                    .send_remote_move_if_needed(&packet_tx, "flying", controlled);
                let enabled = host.auto_jump_enabled();
                sync_auto_jump_after_send(host, controlled, enabled);
            }
        }
        Locomotion::Walking => {
            if let Some(vehicle_id) = passenger_vehicle_id {
                let remote_packet_tx = host
                    .session()
                    .remote_connection
                    .as_ref()
                    .map(|connection| connection.packet_tx.clone());
                let controlled = host.controlled_camera();
                send_passenger_tick_packets_before_rider_attachment(
                    host,
                    remote_packet_tx.as_ref(),
                    "walking",
                    controlled,
                );
                attach_local_player_to_remote_vehicle(host, vehicle_id);
                latch_remote_boat_input_after_ride_tick(host, vehicle_id);
            }
            if passenger_vehicle_id.is_none() {
                let controlled = host.controlled_camera();
                host.session_mut().send_local_move_if_needed(controlled);
                if host.session().remote_connection.is_none() {
                    let enabled = host.auto_jump_enabled();
                    sync_auto_jump_after_send(host, controlled, enabled);
                }
                if let Some(packet_tx) = host
                    .session()
                    .remote_connection
                    .as_ref()
                    .map(|conn| conn.packet_tx.clone())
                {
                    host.session_mut()
                        .send_remote_fall_flying_before_player_input_if_needed(&packet_tx);
                    let sent_input = host.session().player.input.key_presses.clone();
                    let session = host.session_mut();
                    ClientSession::send_remote_player_input_if_changed(
                        &packet_tx,
                        &sent_input,
                        &mut session.remote_last_sent_input,
                    );
                    host.session_mut()
                        .send_remote_move_if_needed(&packet_tx, "walking", controlled);
                    let enabled = host.auto_jump_enabled();
                    sync_auto_jump_after_send(host, controlled, enabled);
                }
            }
        }
    }
    outcome
}
pub fn finish_gameplay_tick_with_outcome<H: GameplayTickHost>(
    host: &mut H,
    outcome: GameplayTickOutcome,
    runs_normally: bool,
    apply_outcome: impl FnOnce(&mut H, GameplayTickOutcome),
) {
    apply_outcome(host, outcome);
    super::screen_tick::finish_client_tick(host, runs_normally);
}
fn sync_auto_jump_after_send<H: GameplayTickHost>(
    host: &mut H,
    is_controlled_camera: bool,
    enabled: bool,
) {
    if is_controlled_camera && host.session().remote_vehicle_entity_id.is_none() {
        host.session_mut().player.auto_jump_enabled = enabled;
    }
}
pub fn dispatch_menu_keybinds<H: GameplayTickHost>(host: &mut H, control_down: bool) {
    while host.session_mut().controls.consume_click(KeyAction::Chat) {
        host.open_chat_screen(None);
    }
    while host
        .session_mut()
        .controls
        .consume_click(KeyAction::Command)
    {
        host.open_chat_screen(Some("/".to_string()));
    }
    for slot in 0..9 {
        while host
            .session_mut()
            .controls
            .consume_click(KeyAction::HotbarSlot(slot))
        {
            host.session_mut().set_active_hotbar_slot(slot);
        }
    }
    while host
        .session_mut()
        .controls
        .consume_click(KeyAction::Inventory)
    {
        host.open_inventory_screen();
    }
    while host
        .session_mut()
        .controls
        .consume_click(KeyAction::SwapOffhand)
    {
        host.session_mut().swap_selected_with_offhand();
    }
    while host.session_mut().controls.consume_click(KeyAction::Drop) {
        let session = host.session_mut();
        session.drop_selected_item(control_down);
    }
    let options = host.input_options();
    host.session_mut().apply_movement_toggle_keybinds(
        options.toggle_crouch,
        options.toggle_sprint,
        options.toggle_attack,
        options.toggle_use,
    );
    while host
        .session_mut()
        .controls
        .consume_click(KeyAction::PickItem)
    {
        let session = host.session_mut();
        pick_block_at_crosshair(session);
    }
    while host
        .session_mut()
        .controls
        .consume_click(KeyAction::Advancements)
    {
        host.open_advancements_screen();
    }
    host.session_mut().consume_unimplemented_keybinds();
}
pub(crate) fn pick_block_at_crosshair(session: &mut ClientSession) {
    let origin = session.player.position + glam::Vec3::new(0.0, session.pose_eye_height(), 0.0);
    let direction = session.player_view_forward();
    let Some(ref level) = session.level else {
        return;
    };
    let holding_item = session
        .player
        .inventory
        .get(session.active_hotbar_slot)
        .filter(|stack| !stack.is_empty())
        .map(|stack| stack.item.location.to_string());
    let outline_context = session
        .player
        .camera_collision_context()
        .with_held_item_path(holding_item);
    let Some(hit) = crate::gameplay::raycast::raycast_block_with_collision_context(
        level,
        origin,
        direction,
        session.player.block_interaction_range(),
        &outline_context,
    ) else {
        return;
    };
    let block_state = level.get_block_state(hit.pos);
    if block_state.block.path == "air" {
        return;
    }
    let item = el_core::registry::BuiltInRegistries::items()
        .get_by_location(&block_state.block)
        .clone();
    if session.game_mode.is_creative() {
        let mut found_hotbar_slot = None;
        for i in 0..9 {
            if session.player.inventory[i].item.location == item.location {
                found_hotbar_slot = Some(i);
                break;
            }
        }
        if let Some(slot) = found_hotbar_slot {
            session.active_hotbar_slot = slot;
        } else {
            let mut target_slot = session.active_hotbar_slot;
            for i in 0..9 {
                if session.player.inventory[i].is_empty() {
                    target_slot = i;
                    break;
                }
            }
            session.active_hotbar_slot = target_slot;
            let new_stack = el_core::item::ItemStack::new(item.clone(), 1);
            session.player.inventory[target_slot] = new_stack.clone();
            let slot_num = (target_slot + 36) as i16;
            if let Some(conn) = session.connection.as_mut() {
                let _ = conn.send_packet(
                    &el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket {
                        slot_num,
                        item_stack: new_stack.clone(),
                    },
                );
            } else if let Some(conn) = session.remote_connection.as_ref() {
                conn.packet_tx.send_set_creative_mode_slot(
                    el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket {
                        slot_num,
                        item_stack: new_stack.clone(),
                    },
                );
            }
        }
    } else {
        let mut found_slot = None;
        for i in 0..36 {
            if session.player.inventory[i].item.location == item.location {
                found_slot = Some(i);
                break;
            }
        }
        if let Some(slot) = found_slot {
            if slot < 9 {
                session.active_hotbar_slot = slot;
            } else {
                let h = session.active_hotbar_slot;
                let m = slot;
                let item_at_m = session.player.inventory[m].clone();
                let item_at_h = session.player.inventory[h].clone();
                session.player.inventory.swap(m, h);
                let mut changed_slots = std::collections::BTreeMap::new();
                changed_slots.insert(m as i16, item_at_h);
                changed_slots.insert((h + 36) as i16, item_at_m);
                let packet = el_protocol::packets::game::ServerboundContainerClickPacket {
                    container_id: 0,
                    state_id: session.player_inventory_state_id,
                    slot_num: m as i16,
                    button_num: h as i8,
                    container_input: el_protocol::packets::game::ContainerInput::Swap,
                    changed_slots,
                    carried_item: el_core::item::ItemStack::empty(),
                };
                if let Some(conn) = session.connection.as_mut() {
                    let _ = conn.send_packet(&packet);
                } else if let Some(conn) = session.remote_connection.as_ref() {
                    conn.packet_tx.send_container_click(packet);
                }
            }
        }
    }
}
pub(crate) fn crossbow_charge_ticks(quick_charge: i32) -> f32 {
    (25 - quick_charge.clamp(0, 3) * 5).max(1) as f32
}
pub(crate) fn consumable_feedback_should_emit_like_java(
    consume_ticks: i32,
    remaining_ticks: f32,
) -> bool {
    if consume_ticks <= 0 {
        return false;
    }
    let remaining = remaining_ticks as i32;
    let item_used_for_ticks = consume_ticks - remaining;
    let wait_ticks = (consume_ticks as f32 * 0.21875) as i32;
    item_used_for_ticks > wait_ticks && remaining.rem_euclid(4) == 0
}
pub(crate) fn advance_active_use_state(
    session: &mut ClientSession,
    main_hand: HandSide,
) -> Option<(el_core::item::ItemStack, f32)> {
    if session.item_use.hand.is_none() {
        return None;
    }
    if !active_use_item_matches_current_hand(session, main_hand) {
        if LocalPlayer::movement_diagnostics_enabled() {
            LocalPlayer::slide_probe_log("[EatDebug] clear reason=item_mismatch".to_string());
        }
        el_protocol_backend::diagnostics::write_line(&format!(
            "[EatDebug] clear reason=item_mismatch remaining={}",
            session.item_use.remaining
        ));
        session.item_use.stop();
        session.sync_local_using_item_flag();
        return None;
    }
    let stack = match session.item_use.hand {
        Some(hand) if hand == main_hand => session.player.inventory.get(session.active_hotbar_slot),
        Some(_) => session.player.inventory.get(40),
        None => None,
    }
    .cloned();
    let pre_tick_remaining = session.item_use.remaining;
    if let Some(stack) = stack.as_ref() {
        if session.remote_connection.is_none() {
            tick_local_crossbow_charge(session, stack, pre_tick_remaining, main_hand);
        }
    }
    session.item_use.tick();
    session.sync_local_using_item_flag();
    stack.map(|stack| (stack, pre_tick_remaining))
}
fn active_use_item_matches_current_hand(session: &ClientSession, main_hand: HandSide) -> bool {
    let Some(expected) = session.item_use.item_id.as_ref() else {
        return false;
    };
    let current_stack = match session.item_use.hand {
        Some(hand) if hand == main_hand => session.player.inventory.get(session.active_hotbar_slot),
        Some(_) => session.player.inventory.get(40),
        None => return false,
    };
    current_stack.is_some_and(|stack| !stack.is_empty() && stack.item.location == *expected)
}
fn tick_local_crossbow_charge(
    session: &mut ClientSession,
    stack: &el_core::item::ItemStack,
    ticks_remaining: f32,
    main_hand: HandSide,
) {
    if stack.item.location.path != "crossbow" {
        return;
    }
    if crate::render::item_in_hand_renderer::is_charged_crossbow(stack) {
        return;
    }
    let quick_charge = session
        .player
        .get_enchantment_level(stack, "minecraft:quick_charge");
    let charge_duration = crossbow_charge_ticks(quick_charge);
    if charge_duration <= 0.0 {
        return;
    }
    let time_held = (session.item_use.duration - ticks_remaining).max(0.0);
    if time_held < charge_duration {
        return;
    }
    let Some(projectile) = local_crossbow_projectile_like_java(session) else {
        return;
    };
    let Some(slot) = local_using_item_slot_like_java(session, main_hand) else {
        return;
    };
    if LocalPlayer::slide_probe_enabled() {
        println!(
            "[CrossbowCharge] local_charge_complete slot={} item={} projectile={} time_held={:.1} charge_duration={:.1}",
            slot,
            session.player.inventory[slot].item.location,
            projectile.item.location,
            time_held,
            charge_duration,
        );
    }
    session.player.inventory[slot].components.set(
        "minecraft:charged_projectiles",
        el_core::item::DataComponent::Generic {
            type_name: "minecraft:charged_projectiles".to_string(),
            value: el_nbt::Tag::List(vec![projectile.to_nbt()]),
        },
    );
}
fn local_crossbow_projectile_like_java(
    session: &ClientSession,
) -> Option<el_core::item::ItemStack> {
    use crate::network::multiplayer::multiplayer_game_mode::{
        is_vanilla_bow_arrow, is_vanilla_crossbow_held_projectile,
    };
    if session
        .player
        .inventory
        .get(40)
        .is_some_and(is_vanilla_crossbow_held_projectile)
    {
        return session.player.inventory.get(40).cloned();
    }
    if session
        .player
        .inventory
        .get(session.active_hotbar_slot)
        .is_some_and(is_vanilla_crossbow_held_projectile)
    {
        return session
            .player
            .inventory
            .get(session.active_hotbar_slot)
            .cloned();
    }
    if let Some(found) = session
        .player
        .inventory
        .iter()
        .take(36)
        .find(|stack| is_vanilla_bow_arrow(stack))
    {
        return Some(found.clone());
    }
    if session.game_mode.abilities.instabuild {
        let arrow = el_core::registry::BuiltInRegistries::items()
            .get_by_location(&el_core::resource_location::ResourceLocation::new(
                "minecraft",
                "arrow",
            ))
            .clone();
        return Some(el_core::item::ItemStack::new(arrow, 1));
    }
    None
}
fn local_using_item_slot_like_java(session: &ClientSession, main_hand: HandSide) -> Option<usize> {
    match session.item_use.hand {
        Some(hand) if hand == main_hand => Some(session.active_hotbar_slot),
        Some(_) => Some(40),
        None => None,
    }
}
fn predict_client_attack_target_knockback(
    target: &mut crate::gameplay::entity::remote::RemoteEntityRenderState,
    knockback_amount: f32,
    yaw: f32,
) {
    use crate::gameplay::attributes::KNOCKBACK_RESISTANCE;
    let radians = yaw * (std::f64::consts::PI / 180.0) as f32;
    let sin = el_math::mth::sin(radians);
    let cos = el_math::mth::cos(radians);
    if !crate::gameplay::entity::remote::is_remote_player_entity(&target.kind) {
        let impulse = glam::DVec3::new(
            f64::from(-sin * knockback_amount),
            0.1,
            f64::from(cos * knockback_amount),
        );
        if is_remote_abstract_boat(&target.kind) {
            target.set_boat_physics_velocity(target.boat_physics_velocity() + impulse);
        } else if crate::gameplay::entity::remote::is_remote_minecart(&target.kind)
            || remote_entity_type_key(&target.kind) == "shulker_bullet"
        {
            target.set_packet_velocity(target.velocity.as_dvec3() + impulse);
        }
        return;
    }
    let resistance = target
        .received_attribute_snapshots
        .as_ref()
        .and_then(|snapshots| snapshots.get(KNOCKBACK_RESISTANCE.id))
        .map_or(KNOCKBACK_RESISTANCE.default_value, |snapshot| {
            snapshot.value(KNOCKBACK_RESISTANCE)
        });
    let power = f64::from(knockback_amount) * (1.0 - resistance);
    if power <= 0.0 {
        return;
    }
    let direction = glam::DVec3::new(f64::from(sin), 0.0, -f64::from(cos)).normalize() * power;
    let old = target.player_velocity_d;
    target.set_packet_velocity(glam::DVec3::new(
        old.x / 2.0 - direction.x,
        if target.on_ground {
            (old.y / 2.0 + power).min(0.4)
        } else {
            old.y
        },
        old.z / 2.0 - direction.z,
    ));
}
fn dispatch_attack_clicks<H: GameplayTickHost>(
    host: &mut H,
    target_entity: Option<i32>,
    has_block_target: bool,
    effects: &mut Vec<GameplayEffect>,
) -> (u32, bool) {
    let mut block_attack_clicks = 0_u32;
    let mut instant_attack = false;
    loop {
        if !host.session_mut().controls.consume_click(KeyAction::Attack) {
            break;
        }
        let held_item = {
            let session = host.session();
            session.player.inventory[session.active_hotbar_slot].clone()
        };
        let decision = {
            let session = host.session_mut();
            classify_attack_click(&AttackClickGuards {
                miss_time: session.miss_time,
                is_using_item: session.player.is_using_item,
                hands_busy: session.local_player_hands_busy_like_java(),
                is_spectator: session.game_mode.is_spectator(),
                charge_rejected: session.player.cannot_attack_with_item(&held_item, 0),
                has_piercing_weapon: held_item
                    .components
                    .get("minecraft:piercing_weapon")
                    .is_some(),
                has_block_target,
                target_entity,
            })
        };
        match decision {
            AttackClickDecision::SuppressedByMissTime
            | AttackClickDecision::HandsBusy
            | AttackClickDecision::RejectedCharge => {
                continue;
            }
            AttackClickDecision::DrainQueueWhileUsing => {
                while host.session_mut().controls.consume_click(KeyAction::Attack) {}
                break;
            }
            AttackClickDecision::Spectator => {
                let session = host.session_mut();
                let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
                let mut local_conn = session.connection.as_mut();
                if let Some(entity_id) = target_entity {
                    session
                        .game_mode
                        .spectate(entity_id, remote_conn, &mut local_conn);
                } else {
                    session
                        .game_mode
                        .spectator_no_action(remote_conn, &mut local_conn);
                }
                instant_attack = true;
                continue;
            }
            AttackClickDecision::Piercing
            | AttackClickDecision::Block
            | AttackClickDecision::Entity
            | AttackClickDecision::Miss => {}
        }
        if matches!(decision, AttackClickDecision::Piercing) {
            let main_hand = host.main_hand();
            let session = host.session_mut();
            let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
            let mut local_conn = session.connection.as_mut();
            session.game_mode.piercing_attack(
                &held_item,
                remote_conn,
                &mut local_conn,
                session.active_hotbar_slot,
            );
            session.player.reset_only_attack_strength_ticker();
            if let Some(remote) = remote_conn {
                remote.send_swing_main_hand();
            }
            session.start_swing_for_hand(main_hand, main_hand);
            instant_attack = true;
            continue;
        }
        if matches!(decision, AttackClickDecision::Block) {
            block_attack_clicks = block_attack_clicks.saturating_add(1);
            continue;
        }
        if let Some(entity_id) = target_entity {
            let (predicted_knockback, critical_effect) = {
                let session = host.session_mut();
                let full_strength = session.player.attack_strength_scale(0.5) > 0.9;
                let sprint_hit = session.player.is_sprinting && full_strength;
                let positive_base_damage = session
                    .player
                    .attr_f32(crate::gameplay::attributes::ATTACK_DAMAGE)
                    > 0.0;
                let target_is_remote_player =
                    session
                        .remote_entities
                        .get(&entity_id)
                        .is_some_and(|target| {
                            crate::gameplay::entity::remote::is_remote_player_entity(&target.kind)
                        });
                let knockback_amount = session
                    .player
                    .attr_f32(crate::gameplay::attributes::ATTACK_KNOCKBACK)
                    / 2.0
                    + if sprint_hit { 0.5 } else { 0.0 };
                let predict_extra_knockback = positive_base_damage
                    && knockback_amount > 0.0
                    && session
                        .remote_entities
                        .get(&entity_id)
                        .is_some_and(|target| {
                            crate::gameplay::entity::remote::is_remote_player_entity(&target.kind)
                                || is_remote_abstract_boat(&target.kind)
                                || crate::gameplay::entity::remote::is_remote_minecart(&target.kind)
                                || remote_entity_type_key(&target.kind) == "shulker_bullet"
                        });
                let predict_critical_hit = positive_base_damage
                    && target_is_remote_player
                    && full_strength
                    && session.player.fall_distance > 0.0
                    && !session.player.on_ground
                    && session
                        .level
                        .as_ref()
                        .is_some_and(|level| !session.player.on_climbable(level))
                    && !session.player.is_in_water
                    && !session
                        .player
                        .has_effect(crate::gameplay::player::types::MOB_EFFECT_BLINDNESS)
                    && !session.player.is_passenger
                    && !session.player.is_sprinting;
                let critical_effect = predict_critical_hit
                    .then(|| {
                        crate::client::state::State::tracking_emitter_target(
                            entity_id,
                            session.remote_player_entity_id,
                            None,
                            &session.remote_entities,
                        )
                    })
                    .flatten()
                    .map(|(position, width, height)| {
                        GameplayEffect::SpawnCriticalTrackingEmitter {
                            entity_id,
                            position,
                            width,
                            height,
                        }
                    });
                (
                    predict_extra_knockback.then_some(knockback_amount),
                    critical_effect,
                )
            };
            {
                let main_hand = host.main_hand();
                let session = host.session_mut();
                let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
                let mut local_conn = session.connection.as_mut();
                session.game_mode.attack(
                    &mut session.player,
                    entity_id,
                    remote_conn,
                    &mut local_conn,
                    session.active_hotbar_slot,
                );
                if let Some(knockback_amount) = predicted_knockback {
                    let yaw = session.player.y_rot_degrees();
                    if let Some(target) = session.remote_entities.get_mut(&entity_id) {
                        predict_client_attack_target_knockback(target, knockback_amount, yaw);
                    }
                    let before = session.player.exact_velocity();
                    session.player.cause_extra_knockback_like_java();
                    let after = session.player.exact_velocity();
                    el_protocol_backend::diagnostics::write_line(&format!(
                        "[CombatTrace] attack_slowdown tick={} target={} before=[{:.12},{:.12},{:.12}] after=[{:.12},{:.12},{:.12}] sprinting={}",
                        session.logical_client_tick_sequence,
                        entity_id,
                        before.x,
                        before.y,
                        before.z,
                        after.x,
                        after.y,
                        after.z,
                        session.player.is_sprinting,
                    ));
                }
                if let Some(effect) = critical_effect {
                    effects.push(effect);
                }
                session.start_swing_for_hand(main_hand, main_hand);
            }
            continue;
        }
        {
            let main_hand = host.main_hand();
            let session = host.session_mut();
            session.player.reset_attack_strength_ticker();
            if session.game_mode.has_miss_time() {
                session.miss_time = 10;
            }
            if let Some(remote) = session.remote_connection.as_ref() {
                remote.packet_tx.send_swing_main_hand();
            }
            session.start_swing_for_hand(main_hand, main_hand);
        }
    }
    (block_attack_clicks, instant_attack)
}
fn dispatch_use_tick<H: GameplayTickHost>(
    host: &mut H,
    interaction_target: Option<crate::client::state::RemoteEntityRaycastHit>,
    screen_active: bool,
) {
    let release = {
        let session = host.session_mut();
        classify_use_tick(&UseClickGuards {
            has_click: false,
            is_using_item: session.item_use.hand.is_some(),
            right_click_delay: session.right_click_delay_ticks,
            is_use_pressed: session.is_use_pressed && !screen_active,
            hands_busy: session.local_player_hands_busy_like_java(),
            has_target: interaction_target.is_some(),
        }) == UseClickDecision::ReleaseActive
    };
    if host.session().item_use.hand.is_some() {
        if release {
            let duration = host.session().item_use.duration;
            let remaining = host.session().item_use.remaining;
            let time_held = duration - remaining;
            if time_held >= 10.0 {
                let main_hand = host.main_hand();
                let session = host.session_mut();
                let used_item_slot = if session.item_use.hand == Some(main_hand) {
                    session.active_hotbar_slot
                } else {
                    40
                };
                let main_item = &session.player.inventory[used_item_slot];
                if main_item.item.location.to_string() == "minecraft:trident" {
                    let riptide_level = session
                        .player
                        .get_enchantment_level(main_item, "minecraft:riptide");
                    if riptide_level > 0
                        && session
                            .level
                            .as_ref()
                            .map(|lvl| session.player.is_in_water_or_rain(lvl))
                            .unwrap_or(false)
                    {
                        let riptide_strength = 1.5 + 0.75 * (riptide_level - 1) as f32;
                        let y_rot = session.player.yaw;
                        let x_rot = session.player.pitch;
                        let xd = -y_rot.sin() * x_rot.cos();
                        let yd = -x_rot.sin();
                        let zd = y_rot.cos() * x_rot.cos();
                        let dist = (xd * xd + yd * yd + zd * zd).sqrt();
                        let xd = xd * (riptide_strength / dist);
                        let yd = yd * (riptide_strength / dist);
                        let zd = zd * (riptide_strength / dist);
                        let push_vel = glam::DVec3::new(xd as f64, yd as f64, zd as f64);
                        session
                            .player
                            .set_velocity_d(session.player.exact_velocity() + push_vel);
                        session.player.auto_spin_attack_ticks = 20;
                        session.player.is_auto_spin_attack = true;
                        if session.player.on_ground {
                            if let Some(ref mut lvl) = session.level {
                                let d_pos = glam::DVec3::new(0.0, 1.1999999f64, 0.0);
                                let collision = session.player.move_and_resolve_collisions(
                                    d_pos,
                                    lvl,
                                    false,
                                    &[],
                                );
                                session.player.set_position_d(collision.position);
                                session.player.set_velocity_d(collision.velocity);
                                session.player.sync_public_from_exact();
                            }
                        }
                    }
                }
            }
            let release_time_held =
                host.session().item_use.duration - host.session().item_use.remaining;
            let release_use_pressed = host.session().is_use_pressed;
            if LocalPlayer::movement_diagnostics_enabled() {
                LocalPlayer::slide_probe_log(format!(
                    "[EatDebug] clear reason=release time_held={release_time_held} use_pressed={release_use_pressed}",
                ));
            }
            el_protocol_backend::diagnostics::write_line(&format!(
                "[EatDebug] clear reason=release time_held={release_time_held} use_pressed={release_use_pressed}",
            ));
            {
                let session = host.session_mut();
                let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
                let mut local_conn = session.connection.as_mut();
                session.game_mode.release_using_item(
                    &mut session.player,
                    remote_conn,
                    &mut local_conn,
                    session.active_hotbar_slot,
                );
                session.item_use.stop();
                session.sync_local_using_item_flag();
            }
        }
        while host.session_mut().controls.consume_click(KeyAction::Use) {}
    } else if !screen_active {
        while host.session_mut().controls.consume_click(KeyAction::Use) {
            dispatch_use_start(host, interaction_target, "click");
        }
        let held = {
            let session = host.session_mut();
            classify_use_tick(&UseClickGuards {
                has_click: false,
                is_using_item: false,
                right_click_delay: session.right_click_delay_ticks,
                is_use_pressed: session.is_use_pressed,
                hands_busy: session.local_player_hands_busy_like_java(),
                has_target: interaction_target.is_some(),
            }) == UseClickDecision::HeldUse
        };
        if held {
            dispatch_use_start(host, interaction_target, "held");
        }
    }
    host.session_mut().sync_local_using_item_flag();
}
fn dispatch_use_start<H: GameplayTickHost>(
    host: &mut H,
    interaction_target: Option<crate::client::state::RemoteEntityRaycastHit>,
    source: &'static str,
) {
    if host.session().game_mode.is_destroying() {
        if LocalPlayer::slide_probe_enabled() {
            println!(
                "[InputDebug] phase=use_suppressed source={} reason=destroying target_entity={}",
                source,
                interaction_target.map_or(-1, |hit| hit.entity_id),
            );
        }
        return;
    }
    host.session_mut().right_click_delay_ticks = 4;
    if host.session().item_use.hand.is_some() {
        if LocalPlayer::slide_probe_enabled() {
            println!(
                "[InputDebug] phase=use_suppressed source={} reason=hands_busy target_entity={}",
                source,
                interaction_target.map_or(-1, |hit| hit.entity_id),
            );
        }
        return;
    }
    if LocalPlayer::slide_probe_enabled() {
        let session = host.session();
        println!(
            "[InputDebug] phase=use_start source={} target_entity={} target_distance={:.3} use_down={} screen_active={} delay=4",
            source,
            interaction_target.map_or(-1, |hit| hit.entity_id),
            interaction_target.map_or(-1.0, |hit| hit.distance),
            session.is_use_pressed,
            false,
        );
    }
    for hand in [0usize, 1usize] {
        if use_interaction_entity_at_crosshair(host, interaction_target, hand) {
            return;
        }
        if host.attempt_block_use(hand) {
            return;
        }
    }
}
fn use_interaction_entity_at_crosshair<H: GameplayTickHost>(
    host: &mut H,
    hit: Option<crate::client::state::RemoteEntityRaycastHit>,
    hand: usize,
) -> bool {
    let Some(hit) = hit else {
        return false;
    };
    let Some((entity_id, entity_position, response, width, height)) = host
        .session()
        .remote_entities
        .get(&hit.entity_id)
        .and_then(|entity| {
            (remote_entity_type_key(&entity.kind) == "interaction").then_some((
                entity.id,
                entity.position,
                entity.interaction_response,
                entity.interaction_width,
                entity.interaction_height,
            ))
        })
    else {
        return false;
    };
    let relative_hit = hit.location - entity_position;
    let remote_packet_tx = host
        .session()
        .remote_connection
        .as_ref()
        .map(|connection| connection.packet_tx.clone());
    let remote_conn = remote_packet_tx.as_ref();
    let session = host.session_mut();
    let mut local_conn = session.connection.as_mut();
    let sent_interact = session.game_mode.interact(
        &mut session.player,
        entity_id,
        hand,
        relative_hit,
        remote_conn,
        &mut local_conn,
        session.active_hotbar_slot,
    );
    drop(local_conn);
    let sent_any = sent_interact;
    let spectator = session.game_mode.is_spectator();
    if LocalPlayer::slide_probe_enabled() {
        println!(
            "[InteractionDebug] phase=use id={} entity_pos=[{:.3},{:.3},{:.3}] width={:.3} height={:.3} hit=[{:.3},{:.3},{:.3}] relative=[{:.3},{:.3},{:.3}] response={} spectator={} sent_interact={}",
            entity_id,
            entity_position.x,
            entity_position.y,
            entity_position.z,
            width,
            height,
            hit.location.x,
            hit.location.y,
            hit.location.z,
            relative_hit.x,
            relative_hit.y,
            relative_hit.z,
            response,
            spectator,
            sent_interact,
        );
    }
    if sent_any && response && !spectator {
        if let Some(packet_tx) = remote_packet_tx.as_ref() {
            if hand == 1 {
                packet_tx.send_swing_off_hand();
            } else {
                packet_tx.send_swing_main_hand();
            }
        }
        let main_hand = host.main_hand();
        let physical = if hand == 1 {
            match main_hand {
                HandSide::Right => HandSide::Left,
                HandSide::Left => HandSide::Right,
            }
        } else {
            main_hand
        };
        host.session_mut().start_swing_for_hand(physical, main_hand);
    }
    sent_any
}
fn send_passenger_tick_packets_before_rider_attachment<H: GameplayTickHost>(
    host: &mut H,
    packet_tx: Option<&RemotePacketSender>,
    reason: &str,
    is_controlled_camera: bool,
) {
    let session = host.session_mut();
    debug_assert!(session.remote_vehicle_entity_id.is_some());
    session.send_local_player_input_if_changed();
    if let Some(connection) = session.connection.as_mut() {
        let packet = el_protocol::packets::game::ServerboundMovePlayerRotPacket {
            y_rot: session.player.y_rot_degrees(),
            x_rot: session.player.x_rot_degrees(),
            on_ground: session.player.on_ground,
            horizontal_collision: session.player.horizontal_collision,
        };
        let _ = connection.send_packet(&packet);
    }
    if let Some(packet_tx) = packet_tx {
        session.send_remote_fall_flying_before_player_input_if_needed(packet_tx);
        let sent_input = session.player.input.key_presses.clone();
        ClientSession::send_remote_player_input_if_changed(
            packet_tx,
            &sent_input,
            &mut session.remote_last_sent_input,
        );
        session.send_remote_move_if_needed(packet_tx, reason, is_controlled_camera);
        let enabled = host.auto_jump_enabled();
        sync_auto_jump_after_send(host, is_controlled_camera, enabled);
    }
}
fn attach_local_player_to_remote_vehicle<H: GameplayTickHost>(host: &mut H, vehicle_id: i32) {
    let session = host.session_mut();
    let passenger_vehicle_attachment_y = session.player.vehicle_attachment_y(session.player_flying);
    let Some(local_player_id) = session.remote_player_entity_id else {
        return;
    };
    let Some((
        kind,
        position,
        yaw,
        yaw_degrees,
        boat_delta_rotation_degrees,
        passenger_index,
        passenger_count,
    )) = session
        .remote_entities
        .get(&vehicle_id)
        .and_then(|vehicle| {
            vehicle
                .passengers
                .iter()
                .position(|passenger_id| *passenger_id == local_player_id)
                .map(|passenger_index| {
                    (
                        vehicle.kind.clone(),
                        if is_remote_abstract_boat(&vehicle.kind) {
                            vehicle.boat_physics_position()
                        } else {
                            vehicle.position.as_dvec3()
                        },
                        vehicle.yaw,
                        vehicle.y_rot_degrees(),
                        vehicle.boat_delta_rotation_degrees,
                        passenger_index,
                        vehicle.passengers.len(),
                    )
                })
        })
    else {
        return;
    };
    let offset = vehicle_passenger_offset(
        &kind,
        yaw,
        passenger_vehicle_attachment_y,
        passenger_index,
        passenger_count,
    );
    session
        .player
        .set_position_from_vehicle_attachment_d(position + offset);
    if is_remote_abstract_boat(&kind) {
        session
            .player
            .apply_abstract_boat_rider_rotation_degrees(yaw_degrees, boat_delta_rotation_degrees);
    }
    session.player.finish_passenger_ride_tick();
}
fn latch_remote_boat_input_after_ride_tick<H: GameplayTickHost>(host: &mut H, vehicle_id: i32) {
    let session = host.session_mut();
    let local_controls_boat = session
        .remote_entities
        .get(&vehicle_id)
        .is_some_and(|vehicle| {
            remote_abstract_boat_is_locally_controlled(vehicle, session.remote_player_entity_id)
        });
    if !local_controls_boat {
        if session
            .remote_boat_input_latch
            .is_some_and(|latch| latch.vehicle_id == vehicle_id)
        {
            session.remote_boat_input_latch = None;
        }
        return;
    }
    session.remote_boat_input_latch = Some(RemoteBoatInputLatch {
        vehicle_id,
        left: session.player_input.left,
        right: session.player_input.right,
        forward: session.player_input.forward,
        backward: session.player_input.backward,
    });
}
fn vehicle_passenger_offset(
    kind: &str,
    yaw_radians: f32,
    passenger_vehicle_attachment_y: f32,
    passenger_index: usize,
    passenger_count: usize,
) -> glam::DVec3 {
    let is_raft = is_remote_raft(kind);
    let is_abstract_boat = is_remote_abstract_boat(kind);
    let is_minecart = kind.contains("minecart");
    let is_horse = kind.contains("horse")
        || kind.contains("donkey")
        || kind.contains("mule")
        || kind.contains("skeleton_horse")
        || kind.contains("zombie_horse");
    let is_llama = kind.contains("llama");
    let is_camel = kind.contains("camel");
    if is_abstract_boat {
        let local_z = if passenger_count > 1 {
            if passenger_index == 0 {
                0.2_f32
            } else {
                -0.6_f32
            }
        } else if is_remote_abstract_chest_boat(kind) {
            0.15_f32
        } else {
            0.0_f32
        };
        let ride_height = if is_raft {
            0.5625_f32 * 0.888_888_9_f32
        } else {
            0.5625_f32 / 3.0_f32
        };
        let angle = -yaw_radians;
        let rotated_x = local_z * angle.sin();
        let rotated_z = local_z * angle.cos();
        glam::DVec3::new(
            f64::from(rotated_x),
            f64::from(ride_height - passenger_vehicle_attachment_y),
            f64::from(rotated_z),
        )
    } else if is_minecart {
        glam::DVec3::new(0.0, 0.1875 - passenger_vehicle_attachment_y as f64, 0.0)
    } else if is_horse {
        let height = if kind.contains("donkey") || kind.contains("mule") {
            1.2125
        } else {
            1.44375
        };
        glam::DVec3::new(0.0, height - passenger_vehicle_attachment_y as f64, 0.0)
    } else if is_llama {
        let local_offset =
            glam::DVec3::new(0.0, 1.37 - passenger_vehicle_attachment_y as f64, -0.3);
        let cos = (-yaw_radians).cos() as f64;
        let sin = (-yaw_radians).sin() as f64;
        glam::DVec3::new(
            local_offset.x * cos + local_offset.z * sin,
            local_offset.y,
            -local_offset.x * sin + local_offset.z * cos,
        )
    } else if is_camel {
        glam::DVec3::new(0.0, 1.8 - passenger_vehicle_attachment_y as f64, 0.0)
    } else {
        glam::DVec3::new(0.0, 0.0, 0.0)
    }
}
fn begin_predicted_item_use(
    session: &mut ClientSession,
    hand: HandSide,
    stack: &el_core::item::ItemStack,
    animation: Option<crate::render::item_in_hand_renderer::ItemUseAnimation>,
    duration: f32,
) {
    if !session.item_use.start(hand, stack, animation, duration) {
        return;
    }
    el_protocol_backend::diagnostics::write_line(&format!(
        "[EatDebug] begin item={} anim={animation:?} dur={duration}",
        stack.item.location
    ));
    session.sync_local_using_item_flag();
}
fn physical_hand_for_main(main_hand: HandSide, hand: usize) -> HandSide {
    let logical = if hand == 1 {
        HandSide::Left
    } else {
        HandSide::Right
    };
    if logical == HandSide::Right {
        main_hand
    } else {
        match main_hand {
            HandSide::Right => HandSide::Left,
            HandSide::Left => HandSide::Right,
        }
    }
}
pub struct BlockUsePrediction {
    pub consumed: bool,
    pub is_failure: bool,
    pub place_sound: Option<(BlockPos, el_core::block_state::BlockState)>,
    pub interaction_sounds:
        Vec<crate::network::multiplayer::multiplayer_game_mode::BlockInteractionSound>,
    pub chest_updates: Vec<(BlockPos, el_core::block_state::BlockState)>,
    pub local_updates: Vec<(BlockPos, el_core::block_state::BlockState)>,
}
impl BlockUsePrediction {
    pub fn stops_hand_iteration(&self) -> bool {
        self.consumed || self.is_failure
    }
}
pub fn predict_block_use(
    session: &mut ClientSession,
    main_hand: HandSide,
    screen_active: bool,
    hand: usize,
) -> BlockUsePrediction {
    let mut prediction = BlockUsePrediction {
        consumed: false,
        is_failure: false,
        place_sound: None,
        interaction_sounds: Vec::new(),
        chest_updates: Vec::new(),
        local_updates: Vec::new(),
    };
    let item_slot = if hand == 1 {
        40
    } else {
        session.active_hotbar_slot
    };
    let physical_hand = if hand == 1 {
        match main_hand {
            HandSide::Right => HandSide::Left,
            HandSide::Left => HandSide::Right,
        }
    } else {
        main_hand
    };
    let on_cooldown = session
        .player
        .inventory
        .get(item_slot)
        .map(|stack| session.player.cooldowns.is_on_cooldown(stack))
        .unwrap_or(false);
    if on_cooldown {
        return prediction;
    }
    let origin = session.player.position + glam::Vec3::new(0.0, session.pose_eye_height(), 0.0);
    let direction = session.player_view_forward();
    let active_item_is_empty = session
        .player
        .inventory
        .get(item_slot)
        .map(|stack| stack.is_empty())
        .unwrap_or(true);
    let active_item_name = session
        .player
        .inventory
        .get(item_slot)
        .map(|stack| format!("{}x{}", stack.item.location.to_string(), stack.count))
        .unwrap_or_else(|| "<missing-slot>".to_string());
    let holding_item = session
        .player
        .inventory
        .get(session.active_hotbar_slot)
        .filter(|stack| !stack.is_empty())
        .map(|stack| stack.item.location.to_string());
    let outline_context = session
        .player
        .camera_collision_context()
        .with_held_item_path(holding_item);
    if LocalPlayer::slide_probe_enabled() {
        println!(
            "[MenuDebug] LOCAL_USE_INPUT active_slot={} item={} empty={} pos=[{:.2},{:.2},{:.2}] yaw={:.2} pitch={:.2} remote={} screen_active={}",
            session.active_hotbar_slot,
            active_item_name,
            active_item_is_empty,
            session.player.position.x,
            session.player.position.y,
            session.player.position.z,
            session.player.y_rot_degrees(),
            session.player.x_rot_degrees(),
            session.remote_connection.is_some(),
            screen_active,
        );
    }
    let Some(hit_result) = session.level.as_ref().and_then(|level| {
        crate::gameplay::raycast::raycast_block_with_collision_context(
            level,
            origin,
            direction,
            session.player.block_interaction_range(),
            &outline_context,
        )
    }) else {
        if session.level.is_some() && !active_item_is_empty {
            let seq = session.next_remote_action_sequence();
            if LocalPlayer::slide_probe_enabled() {
                println!(
                    "[MenuDebug] LOCAL_USE_ITEM_EMPTY_SPACE seq={} active_slot={} item={}",
                    seq, session.active_hotbar_slot, active_item_name
                );
            }
            let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
            let mut local_conn = session.connection.as_mut();
            let used = session.game_mode.use_item(
                &mut session.player,
                session.level.as_mut().unwrap(),
                hand,
                remote_conn,
                &mut local_conn,
                seq,
                session.active_hotbar_slot,
            );
            if used.consumes_action {
                if used.swing_source_client {
                    session.start_swing_for_hand(physical_hand, main_hand);
                }
                let main_item = &session.player.inventory[item_slot];
                if main_item.item.location.to_string() == "minecraft:firework_rocket"
                    && session.player.is_fall_flying
                {
                    let flight_dur = get_firework_flight_duration(main_item);
                    session.player.firework_boost_ticks = 10 * (flight_dur + 1) + 5;
                }
                let use_stack = main_item.clone();
                if used.starts_using_item {
                    let resolved =
                        item_in_hand_renderer::get_item_use_animation_and_duration(&use_stack);
                    if resolved.is_none_or(|(_, dur)| consumable_starts_active_use(dur)) {
                        let (anim, dur) = resolved.unwrap_or((
                            crate::render::item_in_hand_renderer::ItemUseAnimation::None,
                            0.0,
                        ));
                        begin_predicted_item_use(
                            session,
                            physical_hand_for_main(main_hand, hand),
                            &use_stack,
                            Some(anim),
                            dur,
                        );
                    }
                }
                prediction.consumed = true;
                return prediction;
            }
        }
        return prediction;
    };
    let item_use_on_outcome = {
        let seq = session.next_remote_action_sequence();
        if LocalPlayer::slide_probe_enabled() {
            println!(
                "[MenuDebug] LOCAL_USE_ITEM_ON seq={} active_slot={} item={} target=[{},{},{}] face={:?} hit=[{:.3},{:.3},{:.3}]",
                seq,
                session.active_hotbar_slot,
                active_item_name,
                hit_result.pos.x,
                hit_result.pos.y,
                hit_result.pos.z,
                hit_result.face,
                hit_result.hit_pos.x,
                hit_result.hit_pos.y,
                hit_result.hit_pos.z
            );
        }
        let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
        let mut local_conn = session.connection.as_mut();
        session.game_mode.use_item_on(
            &mut session.player,
            session.level.as_mut().unwrap(),
            hand,
            &hit_result,
            remote_conn,
            &mut local_conn,
            seq,
            session.active_hotbar_slot,
        )
    };
    let item_use_on_consumed = item_use_on_outcome.consumes_action;
    prediction.consumed = item_use_on_consumed;
    prediction.is_failure = item_use_on_outcome.is_failure;
    prediction.place_sound = session.game_mode.pending_place_sound.take();
    prediction.interaction_sounds = session.game_mode.take_block_interaction_sounds();
    prediction.chest_updates = session
        .level
        .as_mut()
        .map(|level| level.take_chest_neighbor_shape_updates())
        .unwrap_or_default();
    if prediction.is_failure {
        return prediction;
    }
    if !item_use_on_consumed && !active_item_is_empty {
        let seq = session.next_remote_action_sequence();
        if LocalPlayer::slide_probe_enabled() {
            println!(
                "[MenuDebug] LOCAL_USE_ITEM_AFTER_BLOCK seq={} active_slot={} item={} item_use_on_consumed={}",
                seq, session.active_hotbar_slot, active_item_name, item_use_on_consumed
            );
        }
        let remote_conn = session.remote_connection.as_ref().map(|c| &c.packet_tx);
        let mut local_conn = session.connection.as_mut();
        let used = session.game_mode.use_item(
            &mut session.player,
            session.level.as_mut().unwrap(),
            hand,
            remote_conn,
            &mut local_conn,
            seq,
            session.active_hotbar_slot,
        );
        if used.consumes_action {
            if used.swing_source_client {
                session.start_swing_for_hand(physical_hand, main_hand);
            }
            let main_item = &session.player.inventory[item_slot];
            let use_stack = main_item.clone();
            if used.starts_using_item {
                let resolved =
                    item_in_hand_renderer::get_item_use_animation_and_duration(&use_stack);
                if resolved.is_none_or(|(_, dur)| consumable_starts_active_use(dur)) {
                    let (anim, dur) = resolved.unwrap_or((
                        crate::render::item_in_hand_renderer::ItemUseAnimation::None,
                        0.0,
                    ));
                    begin_predicted_item_use(
                        session,
                        physical_hand_for_main(main_hand, hand),
                        &use_stack,
                        Some(anim),
                        dur,
                    );
                }
            }
            prediction.consumed = true;
            return prediction;
        }
    }
    if item_use_on_consumed {
        if item_use_on_outcome.swing_source_client {
            session.start_swing_for_hand(physical_hand, main_hand);
        }
        prediction.consumed = true;
        return prediction;
    }
    let held_item_path = session
        .player
        .inventory
        .get(item_slot)
        .filter(|stack| !stack.is_empty())
        .map(|stack| stack.item.location.path.clone());
    if matches!(held_item_path.as_deref(), Some("bucket" | "water_bucket")) {
        let mut handled_bucket = false;
        if let Some(ref mut level) = session.level {
            if held_item_path.as_deref() == Some("bucket") {
                let hit_fluid = level.get_block_state(hit_result.pos).get_fluid_state();
                if hit_fluid.is_water() && hit_fluid.is_source() {
                    let air = level.air.clone();
                    prediction.local_updates.push((hit_result.pos, air));
                    let water_bucket = el_core::registry::BuiltInRegistries::items()
                        .get_by_location(&el_core::resource_location::ResourceLocation::new(
                            "minecraft",
                            "water_bucket",
                        ))
                        .clone();
                    session.player.inventory[item_slot] =
                        el_core::item::ItemStack::new(water_bucket, 1);
                    handled_bucket = true;
                }
            } else {
                let place_pos = hit_result.pos.relative(hit_result.face);
                let water = el_core::registry::BuiltInRegistries::blocks()
                    .get_by_location(&el_core::resource_location::ResourceLocation::new(
                        "minecraft",
                        "water",
                    ))
                    .default_state()
                    .clone();
                if level
                    .get_block_state(place_pos)
                    .get_collision_shape()
                    .is_empty()
                {
                    prediction.local_updates.push((place_pos, water));
                    if !session.game_mode.local_player_mode().is_creative() {
                        let bucket = el_core::registry::BuiltInRegistries::items()
                            .get_by_location(&el_core::resource_location::ResourceLocation::new(
                                "minecraft",
                                "bucket",
                            ))
                            .clone();
                        session.player.inventory[item_slot] =
                            el_core::item::ItemStack::new(bucket, 1);
                    }
                    handled_bucket = true;
                }
            }
        }
        if handled_bucket {
            if let Some(remote) = session.remote_connection.as_ref() {
                if hand == 1 {
                    remote.packet_tx.send_swing_off_hand();
                } else {
                    remote.packet_tx.send_swing_main_hand();
                }
            }
            session.start_swing_for_hand(physical_hand, main_hand);
            prediction.consumed = true;
            return prediction;
        }
    }
    prediction
}
