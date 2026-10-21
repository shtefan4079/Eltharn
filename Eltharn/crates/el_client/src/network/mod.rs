pub mod backend_mappers;
pub(crate) mod inbound;
pub mod local_connection;
pub mod multiplayer;
#[cfg(not(target_arch = "wasm32"))]
pub mod protocol_backend;
pub mod ui_adapter;
#[cfg(target_arch = "wasm32")]
pub mod protocol_backend {
    use crate::app::main_entry::ProtocolBackendData;
    use crossbeam_channel::{Receiver, Sender};
    pub use el_protocol_api::ClientConnectRequest;
    pub use el_protocol_backend::BackendNetworkEvent as ClientNetworkEvent;
    use el_protocol_backend::{
        BackendConnectArgs, BackendOutboundPacket, BackendSeenAdvancementsAction,
        BackendToClientMessage, ClientToBackendMessage, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
        decode_json_line, encode_json_line,
    };
    use wasm_bindgen::{JsCast, JsValue, closure::Closure};
    use web_sys::{BinaryType, MessageEvent, WebSocket};
    const WASM_QUEUE_BOUND: usize = 8192;
    fn queue_wasm_event(tx: &Sender<ClientNetworkEvent>, event: ClientNetworkEvent) {
        if let Err(crossbeam_channel::TrySendError::Full(_)) = tx.try_send(event) {
            log::warn!(
                "WASM inbound backend queue full ({WASM_QUEUE_BOUND} messages); dropping event"
            );
        }
    }
    #[derive(Clone)]
    pub struct RemotePacketSender {
        tx: Sender<ClientToBackendMessage>,
        socket: Option<WebSocket>,
        hello_ready: std::rc::Rc<std::cell::Cell<bool>>,
    }
    impl RemotePacketSender {
        fn send_packet(&self, packet: BackendOutboundPacket) {
            let msg = ClientToBackendMessage::Packet(packet);
            if let Some(socket) = &self.socket {
                if self.hello_ready.get() && socket.ready_state() == WebSocket::OPEN {
                    if let Ok(line) = encode_json_line(&msg) {
                        if let Ok(text) = std::str::from_utf8(&line) {
                            let _ = socket.send_with_str(text);
                            return;
                        }
                    }
                }
            }
            if let Err(crossbeam_channel::TrySendError::Full(_)) = self.tx.try_send(msg) {
                log::warn!(
                    "WASM outbound command queue full ({WASM_QUEUE_BOUND} messages); dropping packet"
                );
            }
        }
        fn queue_mod_payload(&self, msg: ClientToBackendMessage) -> Result<(), String> {
            match self.tx.try_send(msg) {
                Ok(()) => Ok(()),
                Err(crossbeam_channel::TrySendError::Full(_)) => {
                    log::warn!(
                        "WASM outbound command queue full ({WASM_QUEUE_BOUND} messages); dropping mod payload"
                    );
                    Err("Mod payload queue is full".to_string())
                }
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    Err("Mod payload transport closed".to_string())
                }
            }
        }
        pub fn send_resource_pack_status(&self, id: uuid::Uuid, action: u8) {
            self.send_packet(BackendOutboundPacket::ResourcePackStatus { id, action });
        }
        pub fn send_chat(&self, message: impl Into<String>) {
            self.send_packet(BackendOutboundPacket::Chat {
                message: message.into(),
            });
        }
        pub fn send_command(&self, command: impl Into<String>) {
            self.send_packet(BackendOutboundPacket::Command {
                command: command.into(),
            });
        }
        pub fn send_custom_click_action(&self, id: String, payload: Option<Vec<u8>>) {
            self.send_packet(BackendOutboundPacket::CustomClickAction { id, payload });
        }
        pub fn send_custom_payload(&self, channel: String, data: Vec<u8>) -> Result<(), String> {
            el_protocol_backend::payload::validate_custom_payload(&channel, &data, true)?;
            let msg = ClientToBackendMessage::Packet(BackendOutboundPacket::CustomPayload {
                channel,
                data,
            });
            if let Some(socket) = &self.socket {
                if !self.hello_ready.get() {
                    return self.queue_mod_payload(msg);
                }
                if socket.ready_state() != WebSocket::OPEN {
                    return Err("Mod payload socket closed".into());
                }
                let line = encode_json_line(&msg).map_err(|e| e.to_string())?;
                let text = std::str::from_utf8(&line).map_err(|e| e.to_string())?;
                socket
                    .send_with_str(text)
                    .map_err(|_| "Mod payload send failed".to_string())
            } else {
                self.queue_mod_payload(msg)
            }
        }
        pub fn send_command_suggestion(&self, id: u32, command: impl Into<String>) {
            self.send_packet(BackendOutboundPacket::CommandSuggestion {
                id,
                command: command.into(),
            });
        }
        pub fn send_player_loaded(&self) {
            self.send_packet(BackendOutboundPacket::PlayerLoaded);
        }
        pub fn send_client_tick_end(&self) {
            self.send_packet(BackendOutboundPacket::ClientTickEnd);
        }
        pub fn send_request_stats(&self) {
            self.send_packet(BackendOutboundPacket::RequestStats);
        }
        pub fn send_request_game_rule_values(&self) {
            self.send_packet(BackendOutboundPacket::RequestGameRuleValues);
        }
        pub fn send_set_game_rules(&self, entries: Vec<(String, String)>) {
            for (key, value) in entries {
                self.send_packet(BackendOutboundPacket::SetGameRule { key, value });
            }
        }
        pub fn send_container_slot_state_changed(
            &self,
            slot_id: u32,
            container_id: i32,
            new_state: bool,
        ) {
            self.send_packet(BackendOutboundPacket::ContainerSlotStateChanged {
                slot_id,
                container_id,
                new_state,
            });
        }
        pub fn send_seen_advancements_action(
            &self,
            action: BackendSeenAdvancementsAction,
            tab: Option<String>,
        ) {
            self.send_packet(BackendOutboundPacket::SeenAdvancements { action, tab });
        }
        pub fn send_pong(&self, id: u32) {
            self.send_packet(BackendOutboundPacket::Pong { id });
        }
        pub fn send_perform_respawn(&self) {
            self.send_packet(BackendOutboundPacket::PerformRespawn);
        }
        pub fn send_move_pos_rot(
            &self,
            x: f64,
            y: f64,
            z: f64,
            yaw: f32,
            pitch: f32,
            on_ground: bool,
            horizontal_collision: bool,
        ) {
            self.send_packet(BackendOutboundPacket::MovePosRot {
                x,
                y,
                z,
                yaw_degrees: yaw,
                pitch_degrees: pitch,
                on_ground,
                horizontal_collision,
            });
        }
        pub fn send_move_pos(
            &self,
            x: f64,
            y: f64,
            z: f64,
            on_ground: bool,
            horizontal_collision: bool,
        ) {
            self.send_packet(BackendOutboundPacket::MovePos {
                x,
                y,
                z,
                on_ground,
                horizontal_collision,
            });
        }
        pub fn send_move_rot(
            &self,
            yaw: f32,
            pitch: f32,
            on_ground: bool,
            horizontal_collision: bool,
        ) {
            self.send_packet(BackendOutboundPacket::MoveRot {
                yaw_degrees: yaw,
                pitch_degrees: pitch,
                on_ground,
                horizontal_collision,
            });
        }
        pub fn send_move_status_only(&self, on_ground: bool, horizontal_collision: bool) {
            self.send_packet(BackendOutboundPacket::MoveStatusOnly {
                on_ground,
                horizontal_collision,
            });
        }
        pub fn send_move_vehicle(
            &self,
            x: f64,
            y: f64,
            z: f64,
            yaw: f32,
            pitch: f32,
            on_ground: bool,
        ) {
            self.send_packet(BackendOutboundPacket::MoveVehicle {
                x,
                y,
                z,
                yaw_degrees: yaw,
                pitch_degrees: pitch,
                on_ground,
            });
        }
        pub fn send_paddle_boat(&self, _left: bool, _right: bool) {}
        pub fn send_player_input(
            &self,
            forward: bool,
            backward: bool,
            left: bool,
            right: bool,
            jump: bool,
            shift: bool,
            sprint: bool,
        ) {
            self.send_packet(BackendOutboundPacket::PlayerInput {
                forward,
                backward,
                left,
                right,
                jump,
                shift,
                sprint,
            });
        }
        pub fn send_player_command(&self, entity_id: i32, action: &str, data: i32) {
            self.send_packet(BackendOutboundPacket::PlayerCommand {
                entity_id,
                action: action.to_string(),
                data,
            });
        }
        pub fn send_accept_teleportation(&self, id: u32) {
            self.send_packet(BackendOutboundPacket::AcceptTeleport { teleport_id: id });
        }
        pub fn send_teleport_confirm(
            &self,
            teleport_id: u32,
            x: f64,
            y: f64,
            z: f64,
            yaw_degrees: f32,
            pitch_degrees: f32,
        ) {
            self.send_packet(BackendOutboundPacket::TeleportConfirm {
                teleport_id,
                x,
                y,
                z,
                yaw_degrees,
                pitch_degrees,
            });
        }
        pub fn send_set_carried_item(&self, slot: u16) {
            self.send_packet(BackendOutboundPacket::SetCarriedItem { slot });
        }
        pub fn send_start_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::StartDestroyBlock,
                x,
                y,
                z,
                direction,
                seq,
            });
        }
        pub fn send_stop_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::StopDestroyBlock,
                x,
                y,
                z,
                direction,
                seq,
            });
        }
        pub fn send_abort_destroy_block(&self, x: i32, y: i32, z: i32, direction: u8, seq: u32) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::AbortDestroyBlock,
                x,
                y,
                z,
                direction,
                seq,
            });
        }
        pub fn send_drop_item(&self) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::DropItem,
                x: 0,
                y: 0,
                z: 0,
                direction: 0,
                seq: 0,
            });
        }
        pub fn send_drop_all_items(&self) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::DropAllItems,
                x: 0,
                y: 0,
                z: 0,
                direction: 0,
                seq: 0,
            });
        }
        pub fn send_release_using_item(&self) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::ReleaseUseItem,
                x: 0,
                y: 0,
                z: 0,
                direction: 0,
                seq: 0,
            });
        }
        pub fn send_swap_item_with_offhand(&self) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::SwapItemWithOffhand,
                x: 0,
                y: 0,
                z: 0,
                direction: 0,
                seq: 0,
            });
        }
        pub fn send_stab(&self) {
            self.send_packet(BackendOutboundPacket::PlayerAction {
                action: el_protocol_backend::BackendPlayerActionKind::Stab,
                x: 0,
                y: 0,
                z: 0,
                direction: 0,
                seq: 0,
            });
        }
        pub fn send_use_item_on(
            &self,
            hand: u8,
            x: i32,
            y: i32,
            z: i32,
            direction: u8,
            cursor_x: f32,
            cursor_y: f32,
            cursor_z: f32,
            inside: bool,
            seq: u32,
        ) {
            self.send_packet(BackendOutboundPacket::UseItemOn {
                hand,
                x,
                y,
                z,
                direction,
                cursor_x,
                cursor_y,
                cursor_z,
                inside,
                seq,
            });
        }
        pub fn send_use_item(&self, hand: u8, seq: u32, yaw_degrees: f32, pitch_degrees: f32) {
            self.send_packet(BackendOutboundPacket::UseItem {
                hand,
                seq,
                yaw_degrees,
                pitch_degrees,
            });
        }
        pub fn send_swing_main_hand(&self) {
            self.send_packet(BackendOutboundPacket::SwingMainHand);
        }
        pub fn send_swing_off_hand(&self) {
            self.send_packet(BackendOutboundPacket::SwingOffHand);
        }
        pub fn send_attack(&self, entity_id: i32, using_secondary_action: bool) {
            self.send_packet(BackendOutboundPacket::Attack {
                entity_id,
                using_secondary_action,
            });
        }
        pub fn send_interact_entity(
            &self,
            entity_id: i32,
            hand: u8,
            relative_x: f32,
            relative_y: f32,
            relative_z: f32,
            using_secondary_action: bool,
        ) -> bool {
            self.send_packet(BackendOutboundPacket::InteractAt {
                entity_id,
                hand,
                relative_x,
                relative_y,
                relative_z,
                using_secondary_action,
            });
            true
        }
        pub fn send_container_click(
            &self,
            packet: el_protocol::packets::game::ServerboundContainerClickPacket,
        ) {
            use super::backend_mappers::map_item_stack_to_backend;
            self.send_packet(BackendOutboundPacket::ContainerClick {
                container_id: packet.container_id,
                state_id: packet.state_id,
                slot_num: packet.slot_num,
                button_num: packet.button_num,
                click_type: packet.container_input.id(),
                changed_slots: packet
                    .changed_slots
                    .iter()
                    .map(|(slot, stack)| (*slot, map_item_stack_to_backend(stack)))
                    .collect(),
                carried_item: map_item_stack_to_backend(&packet.carried_item),
            });
        }
        pub fn send_set_creative_mode_slot(
            &self,
            packet: el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket,
        ) {
            use super::backend_mappers::map_item_stack_to_backend;
            self.send_packet(BackendOutboundPacket::SetCreativeModeSlot {
                slot_num: packet.slot_num,
                item_stack: map_item_stack_to_backend(&packet.item_stack),
            });
        }
        pub fn send_container_close(&self, container_id: i32) {
            self.send_packet(BackendOutboundPacket::ContainerClose { container_id });
        }
        pub fn send_player_abilities(&self, is_flying: bool) {
            self.send_packet(BackendOutboundPacket::PlayerAbilities { is_flying });
        }
        pub fn send_change_game_mode(&self, mode: crate::gameplay::game_mode::GameType) {
            self.send_packet(BackendOutboundPacket::ChangeGameMode {
                mode: mode.name().to_string(),
            });
        }
        pub fn send_sign_update(
            &self,
            x: i32,
            y: i32,
            z: i32,
            is_front_text: bool,
            lines: [String; 4],
        ) {
            self.send_packet(BackendOutboundPacket::SignUpdate {
                x,
                y,
                z,
                is_front_text,
                lines,
            });
        }
        pub fn send_edit_book(&self, slot: u32, pages: Vec<String>, title: Option<String>) {
            self.send_packet(BackendOutboundPacket::EditBook { slot, pages, title });
        }
        pub fn send_recipe_book_seen_recipe(&self, recipe: String) {
            self.send_packet(BackendOutboundPacket::RecipeBookSeenRecipe { recipe });
        }
        pub fn send_select_bundle_item(&self, _slot_index: i32, _selected_item: i32) {}
        pub fn send_bundle_item_selected(&self, _slot_id: i32, _selected_item_index: u32) {}
        pub fn send_cookie_response(&self, _key: String, _payload: Option<Vec<u8>>) {}
        pub fn send_chat_ack(&self, _offset: u32) {}
    }
    pub struct RemoteConnectionHandle {
        pub event_rx: Receiver<ClientNetworkEvent>,
        pub packet_tx: RemotePacketSender,
        socket: Option<WebSocket>,
        game_rule_support: std::rc::Rc<std::cell::Cell<bool>>,
    }
    #[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
    pub struct IpcTelemetrySnapshot {
        pub route: &'static str,
        pub available: bool,
        pub stdout_read_calls: u64,
        pub stdout_read_eofs: u64,
        pub stdout_read_errors: u64,
        pub stdout_bytes: u64,
        pub stdout_read_wait_ns: u64,
        pub stdout_read_wait_max_ns: u64,
        pub inbound_decode_calls: u64,
        pub inbound_decode_errors: u64,
        pub inbound_decode_ns: u64,
        pub inbound_decode_max_ns: u64,
        pub inbound_map_enqueue_calls: u64,
        pub inbound_map_enqueue_errors: u64,
        pub inbound_map_enqueue_ns: u64,
        pub inbound_map_enqueue_max_ns: u64,
        pub stdin_encode_calls: u64,
        pub stdin_encode_errors: u64,
        pub stdin_encoded_bytes: u64,
        pub stdin_encode_ns: u64,
        pub stdin_encode_max_ns: u64,
        pub stdin_write_flush_calls: u64,
        pub stdin_write_flush_errors: u64,
        pub stdin_write_attempted_bytes: u64,
        pub stdin_write_flush_ns: u64,
        pub stdin_write_flush_max_ns: u64,
    }
    impl RemoteConnectionHandle {
        pub fn supports_game_rule_editor(&self) -> bool {
            self.game_rule_support.get()
        }
        pub fn performance_route(&self) -> &'static str {
            "wasm_websocket"
        }
        pub fn ipc_telemetry_snapshot(&self) -> IpcTelemetrySnapshot {
            IpcTelemetrySnapshot {
                route: "wasm_websocket",
                available: false,
                ..IpcTelemetrySnapshot::default()
            }
        }
        pub fn supports_bundle_item_selection(&self) -> bool {
            true
        }
    }
    impl Drop for RemoteConnectionHandle {
        fn drop(&mut self) {
            let Some(socket) = self.socket.take() else {
                return;
            };
            socket.set_onopen(None);
            socket.set_onmessage(None);
            socket.set_onerror(None);
            socket.set_onclose(None);
            if socket.ready_state() != WebSocket::CLOSED {
                let _ = socket.close();
            }
            log::info!("WASM WebSocket closed with remote connection handle");
        }
    }
    const WASM_DEV_GATEWAY_URL: &str = "ws://127.0.0.1:9001";
    fn wasm_dev_gateway_allowed() -> bool {
        cfg!(debug_assertions)
            || std::env::var("ELTHARN_DEV_GATEWAY")
                .map(|value| value.trim() == "1")
                .unwrap_or(false)
    }
    fn get_wasm_gateway_url() -> Result<String, String> {
        let dev_gateway = || -> Result<String, String> {
            if wasm_dev_gateway_allowed() {
                Ok(WASM_DEV_GATEWAY_URL.to_string())
            } else {
                Err(
                    "No WASM gateway URL configured. Set ELTHARN_GATE_CONFIG.gateway_url or serve \
                     the page over http(s); the ws://127.0.0.1:9001 fallback is development-only \
                     (set ELTHARN_DEV_GATEWAY=1 to force it)."
                        .to_string(),
                )
            }
        };
        let Some(window) = web_sys::window() else {
            return dev_gateway();
        };
        let location = window.location();
        let protocol = location.protocol().unwrap_or_default();
        let host = location.host().unwrap_or_default();
        let config =
            js_sys::Reflect::get(window.as_ref(), &JsValue::from_str("ELTHARN_GATE_CONFIG"))
                .unwrap_or(JsValue::UNDEFINED);
        let configured = js_sys::Reflect::get(&config, &JsValue::from_str("gateway_url"))
            .ok()
            .and_then(|value| value.as_string())
            .unwrap_or_default();
        let configured = configured.trim();
        if configured.starts_with("wss://")
            || (protocol == "http:" && configured.starts_with("ws://"))
        {
            return Ok(configured.to_string());
        }
        if protocol == "http:"
            && (host == "localhost"
                || host.starts_with("localhost:")
                || host == "127.0.0.1"
                || host.starts_with("127.0.0.1:")
                || host == "[::1]"
                || host.starts_with("[::1]:"))
        {
            dev_gateway()
        } else if protocol == "https:" && !host.is_empty() {
            Ok(format!("wss://{host}/game"))
        } else {
            dev_gateway()
        }
    }
    pub fn spawn_connection(
        args: ClientConnectRequest,
        config: &ProtocolBackendData,
    ) -> RemoteConnectionHandle {
        let (event_tx, event_rx) =
            crossbeam_channel::bounded::<ClientNetworkEvent>(WASM_QUEUE_BOUND);
        let (packet_tx, packet_rx) =
            crossbeam_channel::bounded::<ClientToBackendMessage>(WASM_QUEUE_BOUND);
        let hello_ready = std::rc::Rc::new(std::cell::Cell::new(false));
        let game_rule_support = std::rc::Rc::new(std::cell::Cell::new(false));
        let gateway_url = match get_wasm_gateway_url() {
            Ok(url) => url,
            Err(reason) => {
                log::error!("{reason}");
                queue_wasm_event(
                    &event_tx,
                    el_protocol_backend::BackendNetworkEvent::Disconnected { reason },
                );
                return RemoteConnectionHandle {
                    event_rx,
                    packet_tx: RemotePacketSender {
                        tx: packet_tx,
                        socket: None,
                        hello_ready,
                    },
                    socket: None,
                    game_rule_support,
                };
            }
        };
        log::info!(
            "WASM: connecting to gateway {gateway_url} for server {}",
            args.server
        );
        let requested_protocol = if config.requested_version.is_empty() {
            el_protocol_backend::VANILLA_VERSION_NAME.to_string()
        } else {
            config.requested_version.clone()
        };
        let backend_connect_args = BackendConnectArgs {
            server: args.server,
            username: args.username,
            language: args.language,
            uuid: args.uuid,
            view_distance: args.view_distance,
            main_hand_left: args.main_hand_left,
            allows_listing: args.allows_listing,
            server_pack_status: args.server_pack_status,
            model_customisation: args.model_customisation,
            server_cookies: args.server_cookies,
        };
        let mut handle_socket = None;
        if let Ok(socket) = WebSocket::new(&gateway_url) {
            socket.set_binary_type(BinaryType::Arraybuffer);
            handle_socket = Some(socket.clone());
            let event_tx_cloned = event_tx.clone();
            let inbound_version = requested_protocol.clone();
            let inbound_socket = socket.clone();
            let inbound_ready = hello_ready.clone();
            let inbound_packets = packet_rx.clone();
            let inbound_game_rules = game_rule_support.clone();
            let mut inbound_closed = false;
            let on_message = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
                if inbound_closed {
                    return;
                }
                let data = event.data();
                let parse_result = if let Some(txt) = data.as_string() {
                    decode_json_line::<BackendToClientMessage>(txt.as_bytes())
                } else if data.is_instance_of::<js_sys::ArrayBuffer>() {
                    let buffer = data.unchecked_into::<js_sys::ArrayBuffer>();
                    let bytes = js_sys::Uint8Array::new(&buffer).to_vec();
                    decode_json_line::<BackendToClientMessage>(&bytes)
                } else {
                    return;
                };
                match parse_result
                    .map_err(|error| format!("protocol backend decoded invalid message: {error}"))
                    .and_then(|message| {
                        let is_hello = matches!(&message, BackendToClientMessage::Hello { .. });
                        let game_rules = match &message {
                            BackendToClientMessage::Hello { manifest } => {
                                Some(manifest.supports_game_rule_editor)
                            }
                            _ => None,
                        };
                        if !inbound_ready.get()
                            && matches!(&message, BackendToClientMessage::Event(_))
                        {
                            return Err(
                                "Protocol backend sent gameplay before compatible IPC Hello"
                                    .to_owned(),
                            );
                        }
                        if let BackendToClientMessage::Status { level, message } = &message {
                            log::info!("Gateway status [{level:?}]: {message}");
                        }
                        let event = super::inbound::into_event(message, &inbound_version)?;
                        if let Some(supported) = game_rules {
                            inbound_game_rules.set(supported);
                        }
                        if is_hello && !inbound_ready.replace(true) {
                            let connect =
                                ClientToBackendMessage::Connect(backend_connect_args.clone());
                            let line = encode_json_line(&connect).map_err(|e| e.to_string())?;
                            inbound_socket
                                .send_with_str(
                                    std::str::from_utf8(&line).map_err(|e| e.to_string())?,
                                )
                                .map_err(|_| "Failed to send backend Connect".to_owned())?;
                            while let Ok(packet) = inbound_packets.try_recv() {
                                let line = encode_json_line(&packet).map_err(|e| e.to_string())?;
                                inbound_socket
                                    .send_with_str(
                                        std::str::from_utf8(&line).map_err(|e| e.to_string())?,
                                    )
                                    .map_err(|_| {
                                        "Failed to send queued backend command".to_owned()
                                    })?;
                            }
                        }
                        Ok(event)
                    }) {
                    Ok(Some(ev)) => {
                        queue_wasm_event(&event_tx_cloned, ev);
                    }
                    Err(reason) => {
                        inbound_closed = true;
                        queue_wasm_event(
                            &event_tx_cloned,
                            el_protocol_backend::BackendNetworkEvent::Disconnected { reason },
                        );
                        let _ = inbound_socket.close();
                    }
                    Ok(None) => {}
                }
            });
            socket.set_onmessage(Some(on_message.as_ref().unchecked_ref()));
            on_message.forget();
            let socket_clone = socket.clone();
            let on_open = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                log::info!("WASM WebSocket opened to gateway");
                let hello = ClientToBackendMessage::Hello {
                    ipc_version: ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
                    requested_minecraft_version: requested_protocol.clone(),
                };
                if let Ok(line) = encode_json_line(&hello) {
                    if let Ok(text) = std::str::from_utf8(&line) {
                        let _ = socket_clone.send_with_str(text);
                    }
                }
            });
            socket.set_onopen(Some(on_open.as_ref().unchecked_ref()));
            on_open.forget();
            let event_tx_onerror = event_tx.clone();
            let on_error = Closure::<dyn FnMut(web_sys::Event)>::new(move |_| {
                log::error!("WASM WebSocket connection error");
                queue_wasm_event(
                    &event_tx_onerror,
                    el_protocol_backend::BackendNetworkEvent::Disconnected {
                        reason: "Failed to connect to gateway (WebSocket error)".to_string(),
                    },
                );
            });
            socket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
            on_error.forget();
            let event_tx_onclose = event_tx.clone();
            let on_close =
                Closure::<dyn FnMut(web_sys::CloseEvent)>::new(move |e: web_sys::CloseEvent| {
                    let reason = if e.reason().is_empty() {
                        format!("Gateway connection closed (code {})", e.code())
                    } else {
                        e.reason()
                    };
                    log::warn!("WASM WebSocket closed: {reason}");
                    queue_wasm_event(
                        &event_tx_onclose,
                        el_protocol_backend::BackendNetworkEvent::Disconnected { reason },
                    );
                });
            socket.set_onclose(Some(on_close.as_ref().unchecked_ref()));
            on_close.forget();
        } else {
            log::error!("Failed to construct WebSocket to {gateway_url}");
            queue_wasm_event(
                &event_tx,
                el_protocol_backend::BackendNetworkEvent::Disconnected {
                    reason: format!("Failed to open WebSocket to {gateway_url}"),
                },
            );
        }
        RemoteConnectionHandle {
            event_rx,
            packet_tx: RemotePacketSender {
                tx: packet_tx,
                socket: handle_socket.clone(),
                hello_ready,
            },
            socket: handle_socket,
            game_rule_support,
        }
    }
}
