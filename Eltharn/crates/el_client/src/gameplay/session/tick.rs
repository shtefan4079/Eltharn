use super::ClientSession;
use crate::gameplay::entity::remote::remote_entity_is_player_rideable_jumping;
use crate::input::KeyAction;
use crate::network::protocol_backend::RemotePacketSender as PacketSender;
use el_core::resource_location::ResourceLocation;

impl ClientSession {
    pub fn begin_client_tick(&mut self) {
        self.logical_client_tick_sequence = self.logical_client_tick_sequence.wrapping_add(1);
        self.tick_right_click_delay_once();
        if self.level.is_some() {
            let remote = self
                .remote_connection
                .as_ref()
                .map(|connection| &connection.packet_tx);
            let mut local = self.connection.as_mut();
            self.game_mode
                .tick(remote, &mut local, self.active_hotbar_slot);
        }
    }

    pub fn finish_client_tick(&mut self) {
        if self.level.is_none() {
            return;
        }
        if let Some(remote) = self.remote_connection.as_ref() {
            remote.packet_tx.send_client_tick_end();
        } else if let Some(local) = self.connection.as_mut() {
            let _ = local.send_packet(&el_protocol::packets::game::ServerboundClientTickEndPacket);
        }
    }

    pub(crate) fn client_level_game_time(&self) -> i64 {
        self.level.as_ref().map_or(0, |level| level.game_time())
    }
    pub(crate) fn reconcile_player_flying_from_game_mode(&mut self, send_serverbound_update: bool) {
        self.player.no_physics = self.game_mode.is_spectator();
        let prev_flying = self.player_flying;
        let new_flying = self.game_mode.abilities().flying;
        if prev_flying != new_flying {
            self.player_flying = new_flying;
            if send_serverbound_update {
                if let Some(conn) = self.remote_connection.as_ref() {
                    conn.packet_tx.send_player_abilities(new_flying);
                }
                if let Some(local) = self.connection.as_mut() {
                    let pkt = el_protocol::packets::game::ServerboundPlayerAbilitiesPacket {
                        is_flying: new_flying,
                    };
                    let _ = local.send_packet(&pkt);
                }
            }
        }
    }
    pub(crate) fn enforce_spectator_flying_like_java(&mut self) {
        let abilities = self.game_mode.abilities();
        if self.game_mode.is_spectator() && abilities.mayfly && !abilities.flying {
            self.game_mode.set_flying(true);
            self.reconcile_player_flying_from_game_mode(true);
        }
    }
    pub(crate) fn set_active_hotbar_slot(&mut self, slot: usize) {
        let slot = slot.min(8);
        if self.active_hotbar_slot == slot {
            return;
        }
        self.active_hotbar_slot = slot;
    }
    pub(crate) fn remote_vehicle_allows_creative_flight_toggle_like_java(&self) -> bool {
        let Some(vehicle_id) = self.remote_vehicle_entity_id else {
            return true;
        };
        self.remote_entities
            .get(&vehicle_id)
            .is_some_and(|vehicle| remote_entity_is_player_rideable_jumping(&vehicle.kind))
    }
    pub(crate) fn tick_creative_flight_toggle_like_java(&mut self) {
        let abilities = self.game_mode.abilities();
        if !abilities.mayfly {
            return;
        }

        let was_jumping = self.player.previous_jump_pressed_like_java();
        let was_auto_jump = self.player.auto_jump_active_like_java();
        let jump_pressed = self.player_input.jump;

        if self.game_mode.is_spectator() {
            if !abilities.flying {
                self.game_mode.set_flying(true);
                self.player
                    .suppress_fall_flying_start_for_creative_toggle_like_java();
                self.reconcile_player_flying_from_game_mode(true);
            }
            return;
        }

        if was_jumping || !jump_pressed || was_auto_jump {
            return;
        }

        if self.jump_trigger_time == 0 {
            self.jump_trigger_time = 7;
            return;
        }

        if self
            .player
            .is_swimming_for_creative_flight_toggle_like_java(abilities.flying)
            || !self.remote_vehicle_allows_creative_flight_toggle_like_java()
        {
            return;
        }

        let next_flying = !abilities.flying;
        self.game_mode.set_flying(next_flying);
        if next_flying && self.player.on_ground {
            if let Some(level) = self.level.as_ref() {
                self.player.jump_from_ground_like_java(level);
            }
        }
        self.jump_trigger_time = 0;
        self.player
            .suppress_fall_flying_start_for_creative_toggle_like_java();
        self.reconcile_player_flying_from_game_mode(true);
    }
    pub(crate) fn tick_jump_trigger_time_after_player_ai_step_like_java(&mut self) {
        if self.jump_trigger_time > 0 {
            self.jump_trigger_time -= 1;
        }
    }
    pub(crate) fn pose_eye_height(&self) -> f32 {
        self.player.pose_eye_height(self.player_flying)
    }
    pub(crate) fn render_partial_tick(&self) -> f32 {
        self.delta_tracker
            .game_time_delta_partial_tick(false)
            .clamp(0.0, 1.0)
    }
    pub(crate) fn player_view_forward(&self) -> glam::Vec3 {
        java_view_vector(self.player.yaw, self.player.pitch)
    }
    pub(crate) fn next_remote_action_sequence(&mut self) -> u32 {
        self.remote_action_sequence = self.remote_action_sequence.wrapping_add(1);
        self.remote_action_sequence
    }
    pub(crate) fn send_remote_fall_flying_before_player_input_if_needed(
        &mut self,
        packet_tx: &PacketSender,
    ) {
        if !self.player.pending_start_fall_flying {
            return;
        }

        let Some(entity_id) = self.remote_player_entity_id else {
            return;
        };

        packet_tx.send_player_command(entity_id, "start_fall_flying", 0);
        self.player.pending_start_fall_flying = false;
    }
    pub(crate) fn drop_selected_item(&mut self, drop_all: bool) {
        if self.game_mode.is_spectator() {
            return;
        }
        let Some(stack) = self.player.inventory.get_mut(self.active_hotbar_slot) else {
            return;
        };
        if stack.is_empty() {
            return;
        }

        let action = if drop_all { 3 } else { 4 };
        if let Some(remote) = self.remote_connection.as_ref() {
            if drop_all {
                remote.packet_tx.send_drop_all_items();
            } else {
                remote.packet_tx.send_drop_item();
            }
        }
        if let Some(local) = self.connection.as_mut() {
            let pkt = el_protocol::packets::game::ServerboundPlayerActionPacket {
                action,
                pos: el_protocol::packets::game::ProtoBlockPos { x: 0, y: 0, z: 0 },
                direction: 0,
                sequence: 0,
            };
            let _ = local.send_packet(&pkt);
        }

        if drop_all || stack.count <= 1 {
            *stack = el_core::item::ItemStack::empty();
        } else {
            stack.count -= 1;
        }
    }
    pub(crate) fn swap_selected_with_offhand(&mut self) {
        if self.game_mode.is_spectator() {
            return;
        }
        if let Some(remote) = self.remote_connection.as_ref() {
            remote.packet_tx.send_swap_item_with_offhand();
        }
        if let Some(local) = self.connection.as_mut() {
            let packet = el_protocol::packets::game::ServerboundPlayerActionPacket {
                action: 6,
                pos: el_protocol::packets::game::ProtoBlockPos { x: 0, y: 0, z: 0 },
                direction: 0,
                sequence: 0,
            };
            let _ = local.send_packet(&packet);
        }
    }

    pub(crate) fn apply_movement_toggle_keybinds(
        &mut self,
        toggle_crouch: bool,
        toggle_sprint: bool,
        toggle_attack: bool,
        toggle_use: bool,
    ) {
        if toggle_crouch {
            while self.controls.consume_click(KeyAction::Sneak) {
                self.toggled_crouch_down = !self.toggled_crouch_down;
            }
        } else {
            self.toggled_crouch_down = false;
        }

        if toggle_sprint {
            while self.controls.consume_click(KeyAction::Sprint) {
                self.toggled_sprint_down = !self.toggled_sprint_down;
            }
        } else {
            self.toggled_sprint_down = false;
        }

        if toggle_attack {
            while self.controls.consume_click(KeyAction::Attack) {
                self.toggled_attack_down = !self.toggled_attack_down;
            }
        } else {
            self.toggled_attack_down = false;
        }

        if toggle_use {
            while self.controls.consume_click(KeyAction::Use) {
                self.toggled_use_down = !self.toggled_use_down;
            }
        } else {
            self.toggled_use_down = false;
        }
        self.is_use_pressed = if toggle_use {
            self.toggled_use_down
        } else {
            self.controls.is_down(KeyAction::Use)
        };
        self.is_left_mouse_pressed = if toggle_attack {
            self.toggled_attack_down
        } else {
            self.controls.is_down(KeyAction::Attack)
        };
    }
    pub(crate) fn consume_unimplemented_keybinds(&mut self) {
        for action in [
            KeyAction::QuickActions,
            KeyAction::Friends,
            KeyAction::SocialInteractions,
            KeyAction::PlayerList,
            KeyAction::ToggleSpectatorShaderEffects,
            KeyAction::SpectatorOutlines,
            KeyAction::SpectatorHotbar,
            KeyAction::SaveHotbarActivator,
            KeyAction::LoadHotbarActivator,
            KeyAction::DebugModifier,
            KeyAction::DebugCrash,
            KeyAction::DebugReloadChunk,
            KeyAction::DebugShowHitboxes,
            KeyAction::DebugClearChat,
            KeyAction::DebugShowChunkBorders,
            KeyAction::DebugShowAdvancedTooltips,
            KeyAction::DebugCopyRecreateCommand,
            KeyAction::DebugSpectate,
            KeyAction::DebugSwitchGameMode,
            KeyAction::DebugDebugOptions,
            KeyAction::DebugFocusPause,
            KeyAction::DebugDumpDynamicTextures,
            KeyAction::DebugReloadResourcePacks,
            KeyAction::DebugProfiling,
            KeyAction::DebugCopyLocation,
            KeyAction::DebugDumpVersion,
            KeyAction::DebugProfilingChart,
            KeyAction::DebugFpsCharts,
            KeyAction::DebugNetworkCharts,
            KeyAction::DebugLightmapTexture,
        ] {
            while self.controls.consume_click(action) {}
        }
    }
    pub(crate) fn active_held_item_location(&self) -> Option<String> {
        let active_item = &self.player.inventory[self.active_hotbar_slot];
        if active_item.is_empty() {
            None
        } else {
            Some(active_item.item.location.to_string())
        }
    }
    pub(crate) fn active_using_item_location(&self) -> Option<&ResourceLocation> {
        self.item_use.item_id.as_ref()
    }
    pub(crate) fn active_offhand_item_location(&self) -> Option<String> {
        if self.player.inventory.len() > 40 {
            let item = &self.player.inventory[40];
            if !item.is_empty() {
                return Some(item.item.location.to_string());
            }
        }
        None
    }
    pub(crate) fn consume_client_ticks(&mut self, dt: f32) -> u32 {
        self.delta_tracker.advance_real_time_seconds(dt);
        let ticks = self
            .delta_tracker
            .advance_game_time_seconds(dt, self.level.is_some());
        if cfg!(target_arch = "wasm32") {
            ticks.min(2)
        } else {
            ticks.min(10)
        }
    }
    pub(crate) fn network_player_pose(&self) -> (glam::Vec3, f32, f32) {
        (
            self.player.position,
            self.player.y_rot_degrees(),
            self.player.x_rot_degrees(),
        )
    }
    pub(crate) fn tick_right_click_delay_once(&mut self) {
        if self.right_click_delay_ticks > 0 {
            self.right_click_delay_ticks -= 1;
        }
    }
    pub(crate) fn sync_local_using_item_flag(&mut self) {
        self.player.is_using_item = self.item_use.hand.is_some();
    }
}

pub(crate) fn java_view_vector(yaw: f32, pitch: f32) -> glam::Vec3 {
    let (sin_pitch, cos_pitch) = pitch.sin_cos();
    let (sin_yaw, cos_yaw) = yaw.sin_cos();
    glam::Vec3::new(-cos_pitch * sin_yaw, -sin_pitch, cos_pitch * cos_yaw).normalize()
}
