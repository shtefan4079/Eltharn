use crate::{
    BiomeRenderData, BlockUpdateEvent, ChatCompletionAction, ChatTrust, ConnectArgs,
    ConnectionHandle, GameMode, NetworkChatSuggestion, NetworkEvent, OutboundPacket, PacketSender,
    RemoteEntityEvent,
};
use el_protocol_backend::{
    BackendBiomeRenderData, BackendBlockPos, BackendBlockUpdateEvent, BackendChatCompletionAction,
    BackendChatTrust, BackendConnectArgs, BackendLogLevel, BackendMinecartStep,
    BackendNetworkChatSuggestion, BackendNetworkEvent, BackendOutboundPacket,
    BackendPlayerActionKind, BackendRelativeMovementFlags, BackendRemoteEntityEvent,
    BackendToClientMessage, ClientToBackendMessage, ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
    decode_json_line, diagnostics, encode_json_line,
};
use std::{
    collections::HashSet,
    io::{self, BufRead, Read, Write},
    sync::{OnceLock, mpsc},
    thread,
    time::{Duration, Instant},
};
const ELTHARN_NOISY_DEBUG_ENV: &str = "ELTHARN_NOISY_DEBUG";
const MAX_IPC_LINE_BYTES: usize = 64 * 1024 * 1024;
fn noisy_debug_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var(ELTHARN_NOISY_DEBUG_ENV)
            .map(|value| matches!(value.as_str(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
    })
}
fn should_print_console_line(line: &str) -> bool {
    if noisy_debug_enabled()
        || std::env::var("ELTHARN_VERBOSE_LOGS")
            .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
            .unwrap_or(false)
        || matches!(
            std::env::var("ELTHARN_LOGS").as_deref(),
            Ok("all" | "trace" | "debug" | "info")
        )
    {
        return true;
    }
    if std::env::var("ELTHARN_DIAGNOSTICS")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
        && (line.starts_with("[MovementDebug]")
            || line.starts_with("[MoveDebug]")
            || line.starts_with("[SlideDebug]")
            || line.starts_with("[NetDebug]"))
    {
        return true;
    }
    if !matches!(
        std::env::var("ELTHARN_LOGS").as_deref(),
        Ok("error" | "warn")
    ) {
        return false;
    }
    let lower = line.to_ascii_lowercase();
    line.starts_with("[ERROR]")
        || lower.contains("error")
        || lower.contains("failed")
        || lower.contains("fatal")
        || lower.contains("disconnect")
        || lower.contains("kicked")
        || lower.contains("kick reason")
}
macro_rules! eprintln {
    ($($arg:tt)*) => {
        { let line = format!($($arg)*); if should_print_console_line(& line) {
        std::eprintln!("{}", line); } }
    };
}
pub fn run_stdio_backend() {
    if let Err(err) = run() {
        let _ = write_message(&BackendToClientMessage::Disconnected {
            reason: err.to_string(),
        });
        eprintln!("[EltharnProtocolBackend] fatal: {err}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), Box<dyn std::error::Error>> {
    let registry_bootstrap_started = Instant::now();
    el_core::registry::BuiltInRegistries::init();
    diagnostics::write_diagnostic_line(&format!(
        "[ParityProbe] phase=backend_startup stage=builtin_registries elapsed_ms={:.3}",
        registry_bootstrap_started.elapsed().as_secs_f64() * 1000.0
    ));
    let (input_tx, input_rx) = mpsc::channel::<ClientToBackendMessage>();
    thread::Builder::new()
        .name("eltharn-protocol-stdin".to_string())
        .spawn(move || read_stdin_loop(input_tx))?;
    let connection = loop {
        let Ok(message) = input_rx.recv() else {
            return Ok(());
        };
        match message {
            ClientToBackendMessage::Hello {
                ipc_version,
                requested_minecraft_version,
            } => {
                if ipc_version != ELTHARN_PROTOCOL_BACKEND_IPC_VERSION
                    || requested_minecraft_version != crate::VANILLA_VERSION_NAME
                {
                    write_message(&BackendToClientMessage::Disconnected {
                        reason: format!(
                            "Backend contract mismatch: expected IPC {} / Minecraft {}",
                            ELTHARN_PROTOCOL_BACKEND_IPC_VERSION,
                            crate::VANILLA_VERSION_NAME
                        ),
                    })?;
                    return Ok(());
                }
                let manifest = crate::backend_manifest();
                write_message(&BackendToClientMessage::Hello { manifest })?;
            }
            ClientToBackendMessage::Connect(args) => {
                let connect_args = map_connect_args(args);
                write_message(&BackendToClientMessage::Status {
                    level: BackendLogLevel::Info,
                    message: format!(
                        "Connecting to {} as {} with protocol {} ({})",
                        connect_args.server,
                        connect_args.username,
                        crate::VANILLA_PROTOCOL_VERSION,
                        crate::VANILLA_VERSION_NAME
                    ),
                })?;
                break crate::spawn_connection(connect_args);
            }
            ClientToBackendMessage::Packet(_) => {
                write_message(&BackendToClientMessage::Status {
                    level: BackendLogLevel::Warn,
                    message: "Dropping packet before Connect".to_string(),
                })?;
            }
            ClientToBackendMessage::Disconnect { reason } => {
                write_message(&BackendToClientMessage::Disconnected { reason })?;
                return Ok(());
            }
        }
    };
    let (notice_tx, notice_rx) = mpsc::channel();
    let sender = connection.packet_tx.clone();
    thread::Builder::new()
        .name("eltharn-protocol-commands".into())
        .spawn(move || {
            dispatch_play_commands(input_rx, notice_tx, |packet| {
                forward_outbound_packet(&sender, packet);
            });
        })?;
    pump_play_events(&connection, notice_rx, &mut io::stdout().lock())
}
fn dispatch_play_commands(
    input_rx: mpsc::Receiver<ClientToBackendMessage>,
    notice_tx: mpsc::Sender<BackendToClientMessage>,
    mut forward: impl FnMut(BackendOutboundPacket),
) {
    while let Ok(message) = input_rx.recv() {
        match message {
            ClientToBackendMessage::Packet(packet) => {
                log_movement_ipc_packet(&packet);
                forward(packet);
            }
            ClientToBackendMessage::Disconnect { reason } => {
                let _ = notice_tx.send(BackendToClientMessage::Disconnected { reason });
                return;
            }
            ClientToBackendMessage::Hello { .. } | ClientToBackendMessage::Connect(_) => {
                let _ = notice_tx.send(BackendToClientMessage::Disconnected {
                    reason: "Repeated Hello/Connect during an active backend connection".into(),
                });
                return;
            }
        }
    }
    let _ = notice_tx.send(BackendToClientMessage::Disconnected {
        reason: "Client IPC command stream closed".into(),
    });
}
fn pump_play_events(
    connection: &ConnectionHandle,
    notice_rx: mpsc::Receiver<BackendToClientMessage>,
    output: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    pump_event_stream(&connection.event_rx, notice_rx, output)
}
fn pump_event_stream(
    event_rx: &crossbeam_channel::Receiver<NetworkEvent>,
    notice_rx: mpsc::Receiver<BackendToClientMessage>,
    output: &mut impl Write,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut unsupported_events = HashSet::<&'static str>::new();
    loop {
        if let Ok(notice) = notice_rx.try_recv() {
            write_message_to(output, &notice)?;
            return Ok(());
        }
        match event_rx.recv_timeout(Duration::from_millis(2)) {
            Ok(event) => {
                if let Some(event) = map_network_event(event, &mut unsupported_events)? {
                    let disconnected = matches!(event, BackendNetworkEvent::Disconnected { .. });
                    write_message_to(output, &BackendToClientMessage::Event(event))?;
                    if disconnected {
                        return Ok(());
                    }
                }
            }
            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
}
fn read_capped_ipc_line(
    reader: &mut impl BufRead,
    line: &mut String,
    max_bytes: usize,
) -> io::Result<usize> {
    line.clear();
    let mut limited = Read::take(&mut *reader, max_bytes.saturating_add(1) as u64);
    limited.read_line(line)
}
fn read_stdin_loop(tx: mpsc::Sender<ClientToBackendMessage>) {
    let stdin = io::stdin();
    let mut reader = stdin.lock();
    let mut line = String::new();
    loop {
        match read_capped_ipc_line(&mut reader, &mut line, MAX_IPC_LINE_BYTES) {
            Ok(0) => return,
            Ok(_) => {
                if line.len() > MAX_IPC_LINE_BYTES && !line.ends_with('\n') {
                    let _ = tx.send(ClientToBackendMessage::Disconnect {
                        reason: format!(
                            "client IPC line exceeds the {MAX_IPC_LINE_BYTES}-byte limit"
                        ),
                    });
                    return;
                }
                if line.trim().is_empty() {
                    continue;
                }
                match decode_json_line::<ClientToBackendMessage>(line.as_bytes()) {
                    Ok(message) => {
                        if tx.send(message).is_err() {
                            return;
                        }
                    }
                    Err(err) => {
                        let _ = tx.send(ClientToBackendMessage::Disconnect {
                            reason: format!("Failed to decode client IPC line: {err}"),
                        });
                        return;
                    }
                }
            }
            Err(err) => {
                let _ = tx.send(ClientToBackendMessage::Disconnect {
                    reason: format!("stdin read error: {err}"),
                });
                return;
            }
        }
    }
}
fn write_message(message: &BackendToClientMessage) -> Result<(), Box<dyn std::error::Error>> {
    let stdout = io::stdout();
    write_message_to(&mut stdout.lock(), message)
}
fn write_message_to(
    output: &mut impl Write,
    message: &BackendToClientMessage,
) -> Result<(), Box<dyn std::error::Error>> {
    let line = encode_json_line(message)?;
    output.write_all(&line)?;
    output.flush()?;
    Ok(())
}
fn map_connect_args(args: BackendConnectArgs) -> ConnectArgs {
    ConnectArgs {
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
    }
}
fn log_movement_ipc_packet(packet: &BackendOutboundPacket) {
    if !std::env::var("ELTHARN_DIAGNOSTICS")
        .map(|value| matches!(value.trim(), "1" | "true" | "TRUE" | "yes" | "on"))
        .unwrap_or(false)
    {
        return;
    }
    match packet {
        BackendOutboundPacket::MovePosRot {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        } => {
            eprintln!(
                "[MovementDebug][Backend] ipc_outbound kind=pos_rot pos=[{x:.6},{y:.6},{z:.6}] look=[{yaw_degrees:.3},{pitch_degrees:.3}] on_ground={on_ground} horiz_collision={horizontal_collision}"
            )
        }
        BackendOutboundPacket::MovePos {
            x,
            y,
            z,
            on_ground,
            horizontal_collision,
        } => {
            eprintln!(
                "[MovementDebug][Backend] ipc_outbound kind=pos pos=[{x:.6},{y:.6},{z:.6}] on_ground={on_ground} horiz_collision={horizontal_collision}"
            )
        }
        BackendOutboundPacket::MoveRot {
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        } => {
            eprintln!(
                "[MovementDebug][Backend] ipc_outbound kind=rot look=[{yaw_degrees:.3},{pitch_degrees:.3}] on_ground={on_ground} horiz_collision={horizontal_collision}"
            )
        }
        BackendOutboundPacket::MoveStatusOnly {
            on_ground,
            horizontal_collision,
        } => {
            eprintln!(
                "[MovementDebug][Backend] ipc_outbound kind=status on_ground={on_ground} horiz_collision={horizontal_collision}"
            )
        }
        BackendOutboundPacket::TeleportConfirm {
            teleport_id,
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
        } => {
            eprintln!(
                "[MovementDebug][Backend] ipc_outbound kind=teleport_confirm id={teleport_id} pos=[{x:.6},{y:.6},{z:.6}] look=[{yaw_degrees:.3},{pitch_degrees:.3}]"
            )
        }
        BackendOutboundPacket::AcceptTeleport { teleport_id } => {
            eprintln!("[MovementDebug][Backend] ipc_outbound kind=accept_teleport id={teleport_id}")
        }
        _ => {}
    }
}
fn forward_outbound_packet(sender: &PacketSender, packet: BackendOutboundPacket) {
    match packet {
        BackendOutboundPacket::ResourcePackStatus { id, action } => {
            sender.send_resource_pack_status(id, action)
        }
        BackendOutboundPacket::Chat { message } => sender.send_chat(message),
        BackendOutboundPacket::Command { command } => sender.send_command(command),
        BackendOutboundPacket::CustomClickAction { id, payload } => {
            let (payload_kind, payload_bytes) = match payload.as_ref() {
                Some(payload) => ("opaque_nbt", payload.len()),
                None => ("absent_tag_end", 0),
            };
            diagnostics::write_line(&format!(
                "[ParityProbe] custom_click phase=backend_ipc_receive payload_kind={payload_kind} payload_bytes={payload_bytes}"
            ));
            sender.send_custom_click_action(id, payload)
        }
        BackendOutboundPacket::CustomPayload { channel, data } => {
            if let Err(error) = sender.send_custom_payload(channel, data) {
                eprintln!("[CustomPayload] send rejected: {error}");
            }
        }
        BackendOutboundPacket::CommandSuggestion { id, command } => {
            sender.send_command_suggestion(id, command)
        }
        BackendOutboundPacket::PlayerLoaded => sender.send_player_loaded(),
        BackendOutboundPacket::ClientTickEnd => sender.send_client_tick_end(),
        BackendOutboundPacket::RequestStats => {
            sender.send_raw_outbound(OutboundPacket::RequestStats)
        }
        BackendOutboundPacket::PerformRespawn => sender.send_perform_respawn(),
        BackendOutboundPacket::MovePosRot {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        } => sender.send_move_pos_rot(
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        ),
        BackendOutboundPacket::MovePos {
            x,
            y,
            z,
            on_ground,
            horizontal_collision,
        } => sender.send_move_pos(x, y, z, on_ground, horizontal_collision),
        BackendOutboundPacket::MoveRot {
            yaw_degrees,
            pitch_degrees,
            on_ground,
            horizontal_collision,
        } => sender.send_move_rot(yaw_degrees, pitch_degrees, on_ground, horizontal_collision),
        BackendOutboundPacket::MoveStatusOnly {
            on_ground,
            horizontal_collision,
        } => sender.send_move_status_only(on_ground, horizontal_collision),
        BackendOutboundPacket::MoveVehicle {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        } => sender.send_move_vehicle(x, y, z, yaw_degrees, pitch_degrees, on_ground),
        BackendOutboundPacket::PaddleBoat { left, right } => sender.send_paddle_boat(left, right),
        BackendOutboundPacket::PlayerInput {
            forward,
            backward,
            left,
            right,
            jump,
            shift,
            sprint,
        } => sender.send_player_input(forward, backward, left, right, jump, shift, sprint),
        BackendOutboundPacket::PlayerCommand {
            entity_id,
            action,
            data,
        } => sender.send_player_command(entity_id, &action, data),
        BackendOutboundPacket::AcceptTeleport { teleport_id } => {
            sender.send_accept_teleportation(teleport_id)
        }
        BackendOutboundPacket::TeleportConfirm {
            teleport_id,
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
        } => sender.send_teleport_confirm(teleport_id, x, y, z, yaw_degrees, pitch_degrees),
        BackendOutboundPacket::SetCarriedItem { slot } => sender.send_set_carried_item(slot),
        BackendOutboundPacket::PlayerAction {
            action,
            x,
            y,
            z,
            direction,
            seq,
        } => match action {
            BackendPlayerActionKind::StartDestroyBlock => {
                sender.send_start_destroy_block(x, y, z, direction, seq)
            }
            BackendPlayerActionKind::AbortDestroyBlock => {
                sender.send_abort_destroy_block(x, y, z, direction, seq)
            }
            BackendPlayerActionKind::StopDestroyBlock => {
                sender.send_stop_destroy_block(x, y, z, direction, seq)
            }
            BackendPlayerActionKind::DropAllItems => sender.send_drop_all_items(),
            BackendPlayerActionKind::DropItem => sender.send_drop_item(),
            BackendPlayerActionKind::ReleaseUseItem => sender.send_release_using_item(),
            BackendPlayerActionKind::SwapItemWithOffhand => sender.send_swap_item_with_offhand(),
            BackendPlayerActionKind::Stab => sender.send_stab(),
        },
        BackendOutboundPacket::UseItemOn {
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
        } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_RECV_USE_ITEM_ON hand={} seq={} pos=[{},{},{}] face={} cursor=[{:.3},{:.3},{:.3}] inside={}",
                hand, seq, x, y, z, direction, cursor_x, cursor_y, cursor_z, inside
            );
            sender.send_use_item_on(
                hand, x, y, z, direction, cursor_x, cursor_y, cursor_z, inside, seq,
            )
        }
        BackendOutboundPacket::UseItem {
            hand,
            seq,
            yaw_degrees,
            pitch_degrees,
        } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_RECV_USE_ITEM hand={} seq={} yaw={:.2} pitch={:.2}",
                hand, seq, yaw_degrees, pitch_degrees
            );
            sender.send_use_item(hand, seq, yaw_degrees, pitch_degrees)
        }
        BackendOutboundPacket::SwingMainHand => sender.send_swing_main_hand(),
        BackendOutboundPacket::SwingOffHand => sender.send_swing_off_hand(),
        BackendOutboundPacket::Attack {
            entity_id,
            using_secondary_action,
        } => sender.send_attack(entity_id, using_secondary_action),
        BackendOutboundPacket::Interact { .. } => diagnostics::write_line(
            "[ProtocolParity] outbound=interact_legacy result=dropped_missing_relative_hit",
        ),
        BackendOutboundPacket::InteractAt {
            entity_id,
            hand,
            relative_x,
            relative_y,
            relative_z,
            using_secondary_action,
        } => sender.send_interact_entity(
            entity_id,
            hand,
            relative_x,
            relative_y,
            relative_z,
            using_secondary_action,
        ),
        BackendOutboundPacket::ContainerClick {
            container_id,
            state_id,
            slot_num,
            button_num,
            click_type,
            changed_slots,
            carried_item,
        } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_RECV_CONTAINER_CLICK container={} state={} slot={} button={} click_type={} changed={} carried={:?}",
                container_id,
                state_id,
                slot_num,
                button_num,
                click_type,
                changed_slots.len(),
                carried_item
            );
            if let Ok(container_input) =
                el_protocol::packets::game::ContainerInput::from_id(click_type)
            {
                let packet = el_protocol::packets::game::ServerboundContainerClickPacket {
                    container_id,
                    state_id,
                    slot_num,
                    button_num,
                    container_input,
                    changed_slots: changed_slots
                        .into_iter()
                        .map(|(slot, stack)| (slot, map_backend_item_stack(stack)))
                        .collect(),
                    carried_item: map_backend_item_stack(carried_item),
                };
                sender.send_container_click(packet);
            } else {
                eprintln!(
                    "[MenuDebug] BACKEND_IPC_DROP_CONTAINER_CLICK container={} state={} slot={} invalid_click_type={}",
                    container_id, state_id, slot_num, click_type
                );
            }
        }
        BackendOutboundPacket::SetCreativeModeSlot {
            slot_num,
            item_stack,
        } => {
            let packet = el_protocol::packets::game::ServerboundSetCreativeModeSlotPacket {
                slot_num,
                item_stack: map_backend_item_stack(item_stack),
            };
            sender.send_set_creative_mode_slot(packet);
        }
        BackendOutboundPacket::ContainerClose { container_id } => {
            sender.send_container_close(container_id)
        }
        BackendOutboundPacket::PlayerAbilities { is_flying } => {
            sender.send_player_abilities(is_flying)
        }
        BackendOutboundPacket::ChangeGameMode { mode } => {
            if let Some(mode) = parse_game_mode(&mode) {
                sender.send_change_game_mode(mode);
            }
        }
        BackendOutboundPacket::Raw { packet, payload } => {
            eprintln!("[EltharnProtocolBackend] unsupported raw packet {packet}: {payload}");
        }
        BackendOutboundPacket::Pong { id } => sender.send_pong(id),
        BackendOutboundPacket::SignUpdate {
            x,
            y,
            z,
            is_front_text,
            lines,
        } => sender.send_raw_outbound(OutboundPacket::SignUpdate {
            x,
            y,
            z,
            is_front_text,
            lines,
        }),
        BackendOutboundPacket::EditBook { slot, pages, title } => {
            sender.send_raw_outbound(OutboundPacket::EditBook { slot, pages, title })
        }
        BackendOutboundPacket::RecipeBookSeenRecipe { recipe } => {
            sender.send_raw_outbound(OutboundPacket::RecipeBookSeenRecipe { recipe })
        }
        BackendOutboundPacket::RecipeBookChangeSettings {
            book_type,
            is_open,
            is_filtering,
        } => sender.send_raw_outbound(OutboundPacket::RecipeBookChangeSettings {
            book_type,
            is_open,
            is_filtering,
        }),
        BackendOutboundPacket::PlaceRecipe {
            container_id,
            recipe,
            shift_down,
        } => sender.send_raw_outbound(OutboundPacket::PlaceRecipe {
            container_id,
            recipe,
            shift_down,
        }),
        BackendOutboundPacket::SetStructureBlock {
            x,
            y,
            z,
            update_type,
            mode,
            name,
            offset_x,
            offset_y,
            offset_z,
            size_x,
            size_y,
            size_z,
            mirror,
            rotation,
            data,
            integrity,
            seed,
            ignore_entities,
            show_air,
            show_bounding_box,
        } => sender.send_raw_outbound(OutboundPacket::SetStructureBlock {
            x,
            y,
            z,
            update_type,
            mode,
            name,
            offset_x,
            offset_y,
            offset_z,
            size_x,
            size_y,
            size_z,
            mirror,
            rotation,
            data,
            integrity,
            seed,
            ignore_entities,
            show_air,
            show_bounding_box,
        }),
        BackendOutboundPacket::SetJigsawBlock {
            x,
            y,
            z,
            name,
            target,
            pool,
            final_state,
            joint,
            selection_priority,
            placement_priority,
        } => sender.send_raw_outbound(OutboundPacket::SetJigsawBlock {
            x,
            y,
            z,
            name,
            target,
            pool,
            final_state,
            joint,
            selection_priority,
            placement_priority,
        }),
        BackendOutboundPacket::SetCommandBlock {
            x,
            y,
            z,
            command,
            mode,
            track_output,
            conditional,
            automatic,
        } => sender.send_raw_outbound(OutboundPacket::SetCommandBlock {
            x,
            y,
            z,
            command,
            mode,
            track_output,
            conditional,
            automatic,
        }),
        BackendOutboundPacket::SetCommandMinecart {
            entity_id,
            command,
            track_output,
        } => sender.send_raw_outbound(OutboundPacket::SetCommandMinecart {
            entity_id,
            command,
            track_output,
        }),
        BackendOutboundPacket::SetTestBlock {
            x,
            y,
            z,
            mode,
            message,
        } => sender.send_raw_outbound(OutboundPacket::SetTestBlock {
            x,
            y,
            z,
            mode,
            message,
        }),
        BackendOutboundPacket::SpectatorAction { spectate_entity_id } => {
            sender.send_raw_outbound(OutboundPacket::SpectatorAction { spectate_entity_id })
        }
        BackendOutboundPacket::PickItem { slot } => {
            sender.send_raw_outbound(OutboundPacket::PickItem { slot })
        }
        BackendOutboundPacket::PickItemFromEntity { id, include_data } => {
            sender.send_raw_outbound(OutboundPacket::PickItemFromEntity { id, include_data })
        }
        BackendOutboundPacket::SelectTrade { item } => {
            sender.send_raw_outbound(OutboundPacket::SelectTrade { item })
        }
        BackendOutboundPacket::SetBeacon { primary, secondary } => {
            sender.send_raw_outbound(OutboundPacket::SetBeacon { primary, secondary })
        }
        BackendOutboundPacket::RenameItem { name } => {
            sender.send_raw_outbound(OutboundPacket::RenameItem { name })
        }
        BackendOutboundPacket::CookieResponse { key, payload } => {
            sender.send_raw_outbound(OutboundPacket::CookieResponse { key, payload })
        }
        BackendOutboundPacket::AcceptCodeOfConduct => {
            diagnostics::write_line(
                "[ProtocolDebug][backend] direction=serverbound phase=ipc disposition=dropped_configuration_only packet=accept_code_of_conduct",
            );
        }
        BackendOutboundPacket::SetGameRule { key, value } => {
            sender.send_raw_outbound(OutboundPacket::SetGameRule { key, value })
        }
        BackendOutboundPacket::RequestGameRuleValues => {
            sender.send_raw_outbound(OutboundPacket::RequestGameRuleValues)
        }
        BackendOutboundPacket::LockDifficulty { locked } => {
            sender.send_raw_outbound(OutboundPacket::LockDifficulty { locked })
        }
        BackendOutboundPacket::SeenAdvancements { action, tab } => {
            sender.send_raw_outbound(OutboundPacket::SeenAdvancements { action, tab })
        }
        BackendOutboundPacket::ChatSessionUpdate {
            session_id,
            expires_at,
            key,
            key_signature,
        } => sender.send_raw_outbound(OutboundPacket::ChatSessionUpdate {
            session_id,
            expires_at,
            key,
            key_signature,
        }),
        BackendOutboundPacket::ChatAck { offset } => {
            sender.send_raw_outbound(OutboundPacket::ChatAck { offset })
        }
        BackendOutboundPacket::BundleItemSelected {
            slot_id,
            selected_item_index,
        } => sender.send_raw_outbound(OutboundPacket::BundleItemSelected {
            slot_id,
            selected_item_index,
        }),
        BackendOutboundPacket::ContainerButtonClick {
            container_id,
            button_id,
        } => sender.send_raw_outbound(OutboundPacket::ContainerButtonClick {
            container_id,
            button_id,
        }),
        BackendOutboundPacket::ContainerSlotStateChanged {
            slot_id,
            container_id,
            new_state,
        } => sender.send_raw_outbound(OutboundPacket::ContainerSlotStateChanged {
            slot_id,
            container_id,
            new_state,
        }),
        BackendOutboundPacket::BlockEntityTagQuery {
            transaction_id,
            x,
            y,
            z,
        } => sender.send_raw_outbound(OutboundPacket::BlockEntityTagQuery {
            transaction_id,
            x,
            y,
            z,
        }),
        BackendOutboundPacket::EntityTagQuery {
            transaction_id,
            entity_id,
        } => sender.send_raw_outbound(OutboundPacket::EntityTagQuery {
            transaction_id,
            entity_id,
        }),
        BackendOutboundPacket::DebugSubscription { subscriptions } => {
            sender.send_raw_outbound(OutboundPacket::DebugSubscription { subscriptions })
        }
        BackendOutboundPacket::TestInstanceBlockAction {
            x,
            y,
            z,
            action,
            test,
            size_x,
            size_y,
            size_z,
            rotation,
            ignore_entities,
            status,
            error_message,
        } => sender.send_raw_outbound(OutboundPacket::TestInstanceBlockAction {
            x,
            y,
            z,
            action,
            test,
            size_x,
            size_y,
            size_z,
            rotation,
            ignore_entities,
            status,
            error_message,
        }),
    }
}
fn parse_game_mode(mode: &str) -> Option<GameMode> {
    match mode.to_ascii_lowercase().as_str() {
        "survival" => Some(GameMode::Survival),
        "creative" => Some(GameMode::Creative),
        "adventure" => Some(GameMode::Adventure),
        "spectator" => Some(GameMode::Spectator),
        other => {
            eprintln!("[EltharnProtocolBackend] unsupported game mode {other}");
            None
        }
    }
}
fn map_network_event(
    event: NetworkEvent,
    _unsupported_events: &mut HashSet<&'static str>,
) -> Result<Option<BackendNetworkEvent>, Box<dyn std::error::Error>> {
    let mapped = match event {
        NetworkEvent::Connected {
            protocol_version,
            version_name,
            transport_peer,
        } => Some(BackendNetworkEvent::Connected {
            protocol_version,
            version_name: version_name.to_string(),
            transport_peer,
        }),
        NetworkEvent::Disconnected { reason } => Some(BackendNetworkEvent::Disconnected { reason }),
        NetworkEvent::Login {
            entity_id,
            player_uuid,
            game_mode,
            show_death_screen,
            online_mode,
            hardcore,
            dimension,
            sea_level,
            min_y,
            height,
            biome_zoom_seed,
            biomes,
        } => Some(BackendNetworkEvent::Login {
            entity_id,
            player_uuid,
            game_mode,
            show_death_screen,
            online_mode,
            hardcore,
            dimension,
            sea_level,
            min_y,
            height,
            biome_zoom_seed,
            biomes: biomes.into_iter().map(map_biome).collect(),
        }),
        NetworkEvent::Respawn {
            game_mode,
            dimension,
            sea_level,
            min_y,
            height,
            biome_zoom_seed,
            biomes,
            data_to_keep,
        } => Some(BackendNetworkEvent::Respawn {
            game_mode,
            dimension,
            sea_level,
            min_y,
            height,
            biome_zoom_seed,
            biomes: biomes.into_iter().map(map_biome).collect(),
            data_to_keep,
        }),
        NetworkEvent::PlayerPosition {
            teleport_id,
            raw_x,
            raw_y,
            raw_z,
            raw_yaw_degrees,
            raw_pitch_degrees,
            raw_delta_x,
            raw_delta_y,
            raw_delta_z,
            relative,
        } => {
            eprintln!(
                "[MovementDebug][Backend] server_position id={teleport_id} pos=[{raw_x:.6},{raw_y:.6},{raw_z:.6}] look=[{raw_yaw_degrees:.3},{raw_pitch_degrees:.3}] delta=[{raw_delta_x:.5},{raw_delta_y:.5},{raw_delta_z:.5}] rel_flags=[x:{} y:{} z:{} y_rot:{} x_rot:{} dx:{} dy:{} dz:{} rotate_delta:{}]",
                relative.x,
                relative.y,
                relative.z,
                relative.y_rot,
                relative.x_rot,
                relative.delta_x,
                relative.delta_y,
                relative.delta_z,
                relative.rotate_delta,
            );
            Some(BackendNetworkEvent::PlayerPosition {
                teleport_id,
                raw_x,
                raw_y,
                raw_z,
                raw_yaw_degrees,
                raw_pitch_degrees,
                raw_delta_x,
                raw_delta_y,
                raw_delta_z,
                relative: BackendRelativeMovementFlags {
                    x: relative.x,
                    y: relative.y,
                    z: relative.z,
                    y_rot: relative.y_rot,
                    x_rot: relative.x_rot,
                    delta_x: relative.delta_x,
                    delta_y: relative.delta_y,
                    delta_z: relative.delta_z,
                    rotate_delta: relative.rotate_delta,
                },
            })
        }
        NetworkEvent::PlayerRotation {
            raw_yaw_degrees,
            raw_pitch_degrees,
            relative_y,
            relative_x,
        } => Some(BackendNetworkEvent::PlayerRotation {
            raw_yaw_degrees,
            raw_pitch_degrees,
            relative_y,
            relative_x,
        }),
        NetworkEvent::ChunkLoaded {
            x,
            z,
            non_air_blocks,
            chunk,
        } => Some(BackendNetworkEvent::ChunkLoaded {
            x,
            z,
            non_air_blocks,
            chunk: map_chunk_to_backend(chunk),
        }),
        NetworkEvent::ChunkUnloaded { x, z } => Some(BackendNetworkEvent::ChunkUnloaded { x, z }),
        NetworkEvent::ChunkCacheCenter { x, z } => {
            Some(BackendNetworkEvent::ChunkCacheCenter { x, z })
        }
        NetworkEvent::ChunkCacheRadius { radius } => {
            Some(BackendNetworkEvent::ChunkCacheRadius { radius })
        }
        NetworkEvent::SimulationDistance { distance } => {
            Some(BackendNetworkEvent::SimulationDistance { distance })
        }
        NetworkEvent::BlockUpdate(update) => Some(BackendNetworkEvent::BlockUpdate {
            update: map_block_update(update),
        }),
        NetworkEvent::Sound {
            sound,
            source,
            x,
            y,
            z,
            volume,
            pitch,
            seed,
            fixed_range,
        } => Some(BackendNetworkEvent::Sound {
            sound,
            source,
            x,
            y,
            z,
            volume,
            pitch,
            seed,
            fixed_range,
        }),
        NetworkEvent::LevelEvent {
            event_type,
            pos,
            data,
            global_event,
        } => Some(BackendNetworkEvent::LevelEvent {
            event_type,
            pos: BackendBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            data,
            global_event,
        }),
        NetworkEvent::SoundEntity {
            sound,
            source,
            entity_id,
            volume,
            pitch,
            seed,
            fixed_range,
        } => Some(BackendNetworkEvent::SoundEntity {
            sound,
            source,
            entity_id,
            volume,
            pitch,
            seed,
            fixed_range,
        }),
        NetworkEvent::StopSound { sound, source } => {
            Some(BackendNetworkEvent::StopSound { sound, source })
        }
        NetworkEvent::BlockEvent {
            pos,
            action_id,
            action_parameter,
        } => Some(BackendNetworkEvent::BlockEvent {
            pos: BackendBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            action_id,
            action_parameter,
        }),
        NetworkEvent::BlockEntityUpdate { pos, data } => {
            Some(BackendNetworkEvent::BlockEntityUpdate {
                pos: BackendBlockPos {
                    x: pos.x,
                    y: pos.y,
                    z: pos.z,
                },
                data: el_protocol_backend::BackendBlockEntityData {
                    kind: data.kind,
                    nbt: data.nbt,
                },
            })
        }
        NetworkEvent::SectionBlocksUpdate { updates } => {
            Some(BackendNetworkEvent::SectionBlocksUpdate {
                updates: updates.into_iter().map(map_block_update).collect(),
            })
        }
        NetworkEvent::LightUpdate(update) => Some(BackendNetworkEvent::LightUpdate {
            update: map_light_update_to_backend(update),
        }),
        NetworkEvent::ChunkBiomesUpdated(updates) => {
            Some(BackendNetworkEvent::ChunkBiomesUpdated {
                updates: updates
                    .into_iter()
                    .map(map_biome_update_to_backend)
                    .collect(),
            })
        }
        NetworkEvent::BlockChangedAck { sequence } => {
            Some(BackendNetworkEvent::BlockChangedAck { sequence })
        }
        NetworkEvent::InventoryContent { items } => Some(BackendNetworkEvent::InventoryContent {
            items: items.into_iter().map(map_item_stack_to_backend).collect(),
        }),
        NetworkEvent::InventorySlot {
            slot,
            item,
            direct_player_slot,
        } => Some(BackendNetworkEvent::InventorySlot {
            slot,
            item: map_item_stack_to_backend(item),
            direct_player_slot,
        }),
        NetworkEvent::SetCursorItem { item } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_SEND_SET_CURSOR_ITEM item={}x{}",
                item.item.location, item.count
            );
            Some(BackendNetworkEvent::SetCursorItem {
                item: map_item_stack_to_backend(item),
            })
        }
        NetworkEvent::OpenScreen {
            container_id,
            menu_type,
            title,
        } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_SEND_OPEN_SCREEN container={} menu_type={} title={:?}",
                container_id, menu_type, title
            );
            Some(BackendNetworkEvent::OpenScreen {
                container_id,
                menu_type,
                title,
            })
        }
        NetworkEvent::ContainerContent {
            container_id,
            state_id,
            items,
            carried_item,
        } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_SEND_CONTAINER_CONTENT container={} state_id={} items={} carried={}x{}",
                container_id,
                state_id,
                items.len(),
                carried_item.item.location,
                carried_item.count
            );
            Some(BackendNetworkEvent::ContainerContent {
                container_id,
                state_id,
                items: items.into_iter().map(map_item_stack_to_backend).collect(),
                carried_item: map_item_stack_to_backend(carried_item),
            })
        }
        NetworkEvent::ContainerSlot {
            container_id,
            state_id,
            slot,
            item,
        } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_SEND_CONTAINER_SLOT container={} state_id={} slot={} item={}x{}",
                container_id, state_id, slot, item.item.location, item.count
            );
            Some(BackendNetworkEvent::ContainerSlot {
                container_id,
                state_id,
                slot,
                item: map_item_stack_to_backend(item),
            })
        }
        NetworkEvent::ContainerClose { container_id } => {
            eprintln!(
                "[MenuDebug] BACKEND_IPC_SEND_CONTAINER_CLOSE container={}",
                container_id
            );
            Some(BackendNetworkEvent::ContainerClose { container_id })
        }
        NetworkEvent::TransferRequested { host, port } => {
            eprintln!(
                "[TransferDebug] BACKEND_IPC_SEND_TRANSFER host={} port={}",
                host, port
            );
            Some(BackendNetworkEvent::TransferRequested { host, port })
        }
        NetworkEvent::EntityAdded(entity) => Some(BackendNetworkEvent::EntityAdded {
            entity: map_remote_entity(entity),
        }),
        NetworkEvent::EntityEvent { id, event_id } => {
            Some(BackendNetworkEvent::EntityEvent { id, event_id })
        }
        NetworkEvent::EntityAnimation { id, action } => {
            Some(BackendNetworkEvent::EntityAnimation { id, action })
        }
        NetworkEvent::EntityMetadata {
            id,
            item_stack,
            no_gravity,
            item_frame_rotation,
            item_frame_direction,
            shared_flags,
            living_flags,
            living_health,
            custom_name,
            custom_name_visible,
            interaction_width,
            interaction_height,
            interaction_response,
            pose,
            sleeping_pos,
            player_main_arm_right,
            player_model_customisation,
            player_absorption,
            ticks_frozen,
            mob_size,
            primed_tnt_fuse,
            primed_tnt_block_state,
            end_crystal_beam_target,
            end_crystal_show_bottom,
            abstract_arrow_flags,
            abstract_arrow_pierce_level,
            abstract_arrow_in_ground,
            arrow_effect_color,
            experience_orb_value,
            armor_stand_client_flags,
            armor_stand_head_pose,
            armor_stand_body_pose,
            armor_stand_left_arm_pose,
            armor_stand_right_arm_pose,
            armor_stand_left_leg_pose,
            armor_stand_right_leg_pose,
            is_baby,
            is_aggressive,
            mob_flags,
            camel_dashing,
            camel_last_pose_change_tick,
            mob_boolean_metadata,
            piglin_is_baby,
            piglin_is_charging_crossbow,
            piglin_is_dancing,
            tamable_flags,
            creaking_can_move,
            creaking_is_active,
            creaking_is_tearing_down,
            display_interpolation_start_delta_ticks,
            display_interpolation_duration,
            display_teleport_duration,
            display_translation,
            display_scale,
            display_left_rotation,
            display_right_rotation,
            display_billboard,
            display_brightness_override,
            display_view_range,
            display_shadow_radius,
            display_shadow_strength,
            display_width,
            display_height,
            display_glow_color_override,
            text_display_text,
            text_display_line_width,
            text_display_background,
            text_display_text_opacity,
            text_display_flags,
            block_display_block_state: _,
            item_display_item: _,
            item_display_context: _,
        } => Some(BackendNetworkEvent::EntityMetadata {
            id,
            no_gravity,
            item_stack: item_stack.map(map_item_stack_to_backend),
            item_frame_rotation,
            item_frame_direction,
            shared_flags,
            living_flags,
            living_health,
            custom_name,
            custom_name_visible,
            interaction_width,
            interaction_height,
            interaction_response,
            pose,
            sleeping_pos,
            player_main_arm_right,
            player_model_customisation,
            player_absorption,
            ticks_frozen,
            mob_size,
            primed_tnt_fuse,
            primed_tnt_block_state_id: primed_tnt_block_state.map(|state| state.id),
            end_crystal_beam_target,
            end_crystal_show_bottom,
            abstract_arrow_flags,
            abstract_arrow_pierce_level,
            abstract_arrow_in_ground,
            arrow_effect_color,
            experience_orb_value,
            armor_stand_client_flags,
            armor_stand_head_pose,
            armor_stand_body_pose,
            armor_stand_left_arm_pose,
            armor_stand_right_arm_pose,
            armor_stand_left_leg_pose,
            armor_stand_right_leg_pose,
            is_baby,
            is_aggressive,
            mob_flags,
            camel_dashing,
            camel_last_pose_change_tick,
            mob_boolean_metadata,
            piglin_is_baby,
            piglin_is_charging_crossbow,
            piglin_is_dancing,
            tamable_flags,
            creaking_can_move,
            creaking_is_active,
            creaking_is_tearing_down,
            display_interpolation_start_delta_ticks,
            display_interpolation_duration,
            display_teleport_duration,
            display_translation,
            display_scale,
            display_left_rotation,
            display_right_rotation,
            display_billboard,
            display_brightness_override,
            display_view_range,
            display_shadow_radius,
            display_shadow_strength,
            display_width,
            display_height,
            display_glow_color_override,
            text_display_text,
            text_display_line_width,
            text_display_background,
            text_display_text_opacity,
            text_display_flags,
            block_display_block_state_id: None,
            item_display_item: None,
            item_display_context: None,
        }),
        NetworkEvent::EntityVelocity { id, x, y, z } => {
            Some(BackendNetworkEvent::EntityVelocity { id, x, y, z })
        }
        NetworkEvent::Explosion {
            center_x,
            center_y,
            center_z,
            radius,
            block_count,
            player_knockback,
            explosion_particle,
            explosion_sound,
            block_particles,
        } => Some(BackendNetworkEvent::Explosion {
            center_x,
            center_y,
            center_z,
            radius,
            block_count,
            player_knockback,
            explosion_particle,
            explosion_sound,
            block_particles,
        }),
        NetworkEvent::UpdateAttributes {
            entity_id,
            attributes,
        } => Some(BackendNetworkEvent::UpdateAttributes {
            entity_id,
            attributes: attributes
                .into_iter()
                .map(|a| el_protocol_backend::BackendAttributeSnapshot {
                    attribute_id: a.attribute_id,
                    base: a.base,
                    modifiers: a.modifiers,
                })
                .collect(),
        }),
        NetworkEvent::WorldBorder {
            center_x,
            center_z,
            size,
            absolute_max_size,
        } => Some(BackendNetworkEvent::WorldBorder {
            center_x,
            center_z,
            size,
            absolute_max_size,
        }),
        NetworkEvent::WorldBorderInitialize {
            center_x,
            center_z,
            old_size,
            new_size,
            lerp_time,
            absolute_max_size,
            warning_blocks,
            warning_time,
        } => Some(BackendNetworkEvent::WorldBorderInitialize {
            center_x,
            center_z,
            old_size,
            new_size,
            lerp_time,
            absolute_max_size,
            warning_blocks,
            warning_time,
        }),
        NetworkEvent::WorldBorderCenter { center_x, center_z } => {
            Some(BackendNetworkEvent::WorldBorderCenter { center_x, center_z })
        }
        NetworkEvent::WorldBorderLerpSize {
            old_size,
            new_size,
            lerp_time,
        } => Some(BackendNetworkEvent::WorldBorderLerpSize {
            old_size,
            new_size,
            lerp_time,
        }),
        NetworkEvent::WorldBorderSize { size } => {
            Some(BackendNetworkEvent::WorldBorderSize { size })
        }
        NetworkEvent::WorldBorderWarningDistance { warning_blocks } => {
            Some(BackendNetworkEvent::WorldBorderWarningDistance { warning_blocks })
        }
        NetworkEvent::WorldBorderWarningDelay { warning_time } => {
            Some(BackendNetworkEvent::WorldBorderWarningDelay { warning_time })
        }
        NetworkEvent::ChangeDifficulty { difficulty, locked } => {
            Some(BackendNetworkEvent::ChangeDifficulty { difficulty, locked })
        }
        NetworkEvent::TabList { header, footer } => {
            Some(BackendNetworkEvent::TabList { header, footer })
        }
        NetworkEvent::SetCamera { camera_id } => Some(BackendNetworkEvent::SetCamera { camera_id }),
        NetworkEvent::BossEvent { id, operation } => {
            let mapped_op = map_to_backend_boss_event_operation(operation);
            Some(BackendNetworkEvent::BossEvent {
                id,
                operation: mapped_op,
            })
        }
        NetworkEvent::SetPassengers {
            vehicle_id,
            passenger_ids,
        } => Some(BackendNetworkEvent::SetPassengers {
            vehicle_id,
            passenger_ids,
        }),
        NetworkEvent::MoveVehicle {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
        } => Some(BackendNetworkEvent::MoveVehicle {
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
        }),
        NetworkEvent::MoveMinecartAlongTrack { entity_id, steps } => {
            Some(BackendNetworkEvent::MoveMinecartAlongTrack {
                entity_id,
                steps: steps
                    .iter()
                    .map(|step| BackendMinecartStep {
                        x: step.position.x,
                        y: step.position.y,
                        z: step.position.z,
                        dx: step.movement.x,
                        dy: step.movement.y,
                        dz: step.movement.z,
                        yaw_degrees: step.y_rot as f32 * 360.0 / 256.0,
                        pitch_degrees: step.x_rot as f32 * 360.0 / 256.0,
                        weight: step.weight,
                    })
                    .collect(),
            })
        }
        NetworkEvent::EntityPositionSync {
            id,
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        } => Some(BackendNetworkEvent::EntityPositionSync {
            id,
            x,
            y,
            z,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        }),
        NetworkEvent::EntityTeleport {
            id,
            raw_x,
            raw_y,
            raw_z,
            raw_delta_x,
            raw_delta_y,
            raw_delta_z,
            raw_yaw_degrees,
            raw_pitch_degrees,
            relative,
            on_ground,
        } => Some(BackendNetworkEvent::EntityTeleport {
            id,
            raw_x,
            raw_y,
            raw_z,
            raw_delta_x,
            raw_delta_y,
            raw_delta_z,
            raw_yaw_degrees,
            raw_pitch_degrees,
            relative: BackendRelativeMovementFlags {
                x: relative.x,
                y: relative.y,
                z: relative.z,
                y_rot: relative.y_rot,
                x_rot: relative.x_rot,
                delta_x: relative.delta_x,
                delta_y: relative.delta_y,
                delta_z: relative.delta_z,
                rotate_delta: relative.rotate_delta,
            },
            on_ground,
        }),
        NetworkEvent::EntityDeltaMoved {
            id,
            xa,
            ya,
            za,
            has_position,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        } => Some(BackendNetworkEvent::EntityDeltaMoved {
            id,
            xa,
            ya,
            za,
            has_position,
            yaw_degrees,
            pitch_degrees,
            on_ground,
        }),
        NetworkEvent::EntityHeadLook {
            id,
            y_head_rot_degrees,
        } => Some(BackendNetworkEvent::EntityHeadLook {
            id,
            y_head_rot_degrees,
        }),
        NetworkEvent::EntityEquipment {
            id,
            main_hand,
            off_hand,
            head,
            chest,
            legs,
            feet,
            body,
            saddle,
        } => Some(BackendNetworkEvent::EntityEquipment {
            id,
            main_hand: main_hand.map(map_item_stack_to_backend),
            off_hand: off_hand.map(map_item_stack_to_backend),
            head: head.map(map_item_stack_to_backend),
            chest: chest.map(map_item_stack_to_backend),
            legs: legs.map(map_item_stack_to_backend),
            feet: feet.map(map_item_stack_to_backend),
            body: body.map(map_item_stack_to_backend),
            saddle: saddle.map(map_item_stack_to_backend),
        }),
        NetworkEvent::EntitiesRemoved { ids } => Some(BackendNetworkEvent::EntitiesRemoved { ids }),
        NetworkEvent::SystemChat {
            message,
            overlay,
            click_spans,
        } => Some(BackendNetworkEvent::SystemChat {
            message,
            overlay,
            click_spans,
        }),
        NetworkEvent::ActionBar { message } => Some(BackendNetworkEvent::ActionBar { message }),
        NetworkEvent::TitleText { message } => Some(BackendNetworkEvent::TitleText { message }),
        NetworkEvent::SubtitleText { message } => {
            Some(BackendNetworkEvent::SubtitleText { message })
        }
        NetworkEvent::TitlesAnimation {
            fade_in,
            stay,
            fade_out,
        } => Some(BackendNetworkEvent::TitlesAnimation {
            fade_in,
            stay,
            fade_out,
        }),
        NetworkEvent::ClearTitles { reset_times } => {
            Some(BackendNetworkEvent::ClearTitles { reset_times })
        }
        NetworkEvent::CommandTree {
            root_commands,
            root_index,
            nodes,
        } => Some(BackendNetworkEvent::CommandTree {
            root_commands,
            root_index,
            nodes,
        }),
        NetworkEvent::CommandSuggestions {
            id,
            range_start,
            range_end,
            suggestions,
        } => Some(BackendNetworkEvent::CommandSuggestions {
            id,
            range_start,
            range_end,
            suggestions: suggestions.into_iter().map(map_chat_suggestion).collect(),
        }),
        NetworkEvent::OnlinePlayers { names } => Some(BackendNetworkEvent::OnlinePlayers { names }),
        NetworkEvent::CustomChatCompletions { action, entries } => {
            Some(BackendNetworkEvent::CustomChatCompletions {
                action: map_chat_completion_action(action),
                entries,
            })
        }
        NetworkEvent::PlayerChat {
            message,
            signed_content,
            signature,
            trust,
            click_spans,
        } => Some(BackendNetworkEvent::PlayerChat {
            message,
            signed_content,
            signature,
            trust: map_chat_trust(trust),
            click_spans,
        }),
        NetworkEvent::DisguisedChat {
            message,
            click_spans,
        } => Some(BackendNetworkEvent::DisguisedChat {
            message,
            click_spans,
        }),
        NetworkEvent::DeleteChat { signature } => {
            Some(BackendNetworkEvent::DeleteChat { signature })
        }
        NetworkEvent::Health {
            health,
            food,
            saturation,
        } => Some(BackendNetworkEvent::Health {
            health,
            food,
            saturation,
        }),
        NetworkEvent::Experience {
            progress,
            level,
            total,
        } => Some(BackendNetworkEvent::Experience {
            progress,
            level,
            total,
        }),
        NetworkEvent::Abilities {
            invulnerable,
            flying,
            mayfly,
            instabuild,
            flying_speed,
            walking_speed,
        } => Some(BackendNetworkEvent::Abilities {
            invulnerable,
            flying,
            mayfly,
            instabuild,
            flying_speed,
            walking_speed,
        }),
        NetworkEvent::Time {
            game_time,
            day_time,
            day_time_rate,
            day_time_partial_tick,
        } => Some(BackendNetworkEvent::Time {
            game_time,
            day_time,
            day_time_rate,
            day_time_partial_tick,
        }),
        NetworkEvent::HeldSlot { slot } => Some(BackendNetworkEvent::HeldSlot { slot }),
        NetworkEvent::GameModeChanged { game_mode } => {
            Some(BackendNetworkEvent::GameModeChanged { game_mode })
        }
        NetworkEvent::GameEvent { event_id, value } => {
            Some(BackendNetworkEvent::GameEvent { event_id, value })
        }
        NetworkEvent::ChunkBatchStart => Some(BackendNetworkEvent::ChunkBatchStart),
        NetworkEvent::ChunkBatchFinished { batch_size } => {
            Some(BackendNetworkEvent::ChunkBatchFinished { batch_size })
        }
        NetworkEvent::TickingState {
            tick_rate,
            is_frozen,
        } => Some(BackendNetworkEvent::TickingState {
            tick_rate,
            is_frozen,
        }),
        NetworkEvent::TickingStep { tick_steps } => {
            Some(BackendNetworkEvent::TickingStep { tick_steps })
        }
        NetworkEvent::Ping { id } => Some(BackendNetworkEvent::Ping { id }),
        NetworkEvent::PongResponse { time } => Some(BackendNetworkEvent::PongResponse { time }),
        NetworkEvent::ProjectilePower {
            entity_id,
            acceleration_power,
        } => Some(BackendNetworkEvent::ProjectilePower {
            entity_id,
            acceleration_power,
        }),
        NetworkEvent::PlayerLookAt {
            from_anchor,
            x,
            y,
            z,
            at_entity,
            entity_id,
            to_anchor,
        } => Some(BackendNetworkEvent::PlayerLookAt {
            from_anchor,
            x,
            y,
            z,
            at_entity,
            entity_id,
            to_anchor,
        }),
        NetworkEvent::BlockDestruction { id, pos, progress } => {
            Some(BackendNetworkEvent::BlockDestruction { id, pos, progress })
        }
        NetworkEvent::SetEntityLink { source_id, dest_id } => {
            Some(BackendNetworkEvent::SetEntityLink { source_id, dest_id })
        }
        NetworkEvent::LevelParticles(spawn) => {
            Some(BackendNetworkEvent::LevelParticles(Box::new(*spawn)))
        }
        NetworkEvent::DebugBlockValue { pos, update } => {
            Some(BackendNetworkEvent::DebugBlockValue {
                pos: BackendBlockPos {
                    x: pos[0],
                    y: pos[1],
                    z: pos[2],
                },
                update,
            })
        }
        NetworkEvent::DebugChunkValue {
            chunk_x,
            chunk_z,
            update,
        } => Some(BackendNetworkEvent::DebugChunkValue {
            chunk_x,
            chunk_z,
            update,
        }),
        NetworkEvent::DebugEntityValue { entity_id, update } => {
            Some(BackendNetworkEvent::DebugEntityValue { entity_id, update })
        }
        NetworkEvent::DebugEvent { event } => Some(BackendNetworkEvent::DebugEvent { event }),
        NetworkEvent::GameTestHighlightPos {
            absolute_pos,
            relative_pos,
        } => Some(BackendNetworkEvent::GameTestHighlightPos {
            absolute_pos: BackendBlockPos {
                x: absolute_pos[0],
                y: absolute_pos[1],
                z: absolute_pos[2],
            },
            relative_pos: BackendBlockPos {
                x: relative_pos[0],
                y: relative_pos[1],
                z: relative_pos[2],
            },
        }),
        NetworkEvent::MountScreenOpen {
            container_id,
            inventory_columns,
            entity_id,
        } => Some(BackendNetworkEvent::MountScreenOpen {
            container_id,
            inventory_columns,
            entity_id,
        }),
        NetworkEvent::SelectAdvancementsTab { tab } => {
            Some(BackendNetworkEvent::SelectAdvancementsTab { tab })
        }
        NetworkEvent::TestInstanceBlockStatus { status, size } => {
            Some(BackendNetworkEvent::TestInstanceBlockStatus { status, size })
        }
        NetworkEvent::UpdateMobEffect {
            entity_id,
            effect_id,
            amplifier,
            duration,
            ambient,
            show_particles,
            show_icon,
            blend,
        } => Some(BackendNetworkEvent::UpdateMobEffect {
            entity_id,
            effect_id,
            amplifier,
            duration,
            ambient,
            show_particles,
            show_icon,
            blend,
        }),
        NetworkEvent::RemoveMobEffect {
            entity_id,
            effect_id,
        } => Some(BackendNetworkEvent::RemoveMobEffect {
            entity_id,
            effect_id,
        }),
        NetworkEvent::HurtAnimation { entity_id, yaw } => {
            Some(BackendNetworkEvent::HurtAnimation { entity_id, yaw })
        }
        NetworkEvent::DamageEvent {
            entity_id,
            source_type_id,
            source_cause_id,
            source_direct_id,
            source_pos_x,
            source_pos_y,
            source_pos_z,
        } => Some(BackendNetworkEvent::DamageEvent {
            entity_id,
            source_type_id,
            source_cause_id,
            source_direct_id,
            source_pos_x,
            source_pos_y,
            source_pos_z,
        }),
        NetworkEvent::TakeItem {
            item_id,
            player_id,
            amount,
        } => Some(BackendNetworkEvent::TakeItem {
            item_id,
            player_id,
            amount,
        }),
        NetworkEvent::SetDefaultSpawnPosition {
            x,
            y,
            z,
            yaw,
            pitch,
        } => Some(BackendNetworkEvent::SetDefaultSpawnPosition {
            x,
            y,
            z,
            yaw,
            pitch,
        }),
        NetworkEvent::Cooldown {
            cooldown_group,
            duration_ticks,
        } => Some(BackendNetworkEvent::Cooldown {
            cooldown_group,
            duration_ticks,
        }),
        NetworkEvent::ContainerSetData {
            container_id,
            id,
            value,
        } => Some(BackendNetworkEvent::ContainerSetData {
            container_id,
            id,
            value,
        }),
        NetworkEvent::AddPlayerInfo {
            uuid,
            name,
            texture_url,
            model,
            cape_url,
        } => Some(BackendNetworkEvent::AddPlayerInfo {
            uuid,
            name,
            texture_url,
            model,
            cape_url,
        }),
        NetworkEvent::RemovePlayerInfo { uuid } => {
            Some(BackendNetworkEvent::RemovePlayerInfo { uuid })
        }
        NetworkEvent::PlayerInfoUpdate { entries } => {
            Some(BackendNetworkEvent::PlayerInfoUpdate { entries })
        }
        NetworkEvent::SetPlayerTeam {
            name,
            op,
            collision_rule,
            members,
            display_name,
            prefix,
            suffix,
            team_color,
            nametag_visibility,
            team_options,
        } => Some(BackendNetworkEvent::SetPlayerTeam {
            name,
            op,
            collision_rule,
            members,
            display_name,
            prefix,
            suffix,
            team_color,
            nametag_visibility,
            team_options,
        }),
        NetworkEvent::RecipeBookSettings {
            gui_open,
            filtering_craftable,
            furnace_gui_open,
            furnace_filtering_craftable,
            blast_furnace_gui_open,
            blast_furnace_filtering_craftable,
            smoker_gui_open,
            smoker_filtering_craftable,
        } => Some(BackendNetworkEvent::RecipeBookSettings {
            gui_open,
            filtering_craftable,
            furnace_gui_open,
            furnace_filtering_craftable,
            blast_furnace_gui_open,
            blast_furnace_filtering_craftable,
            smoker_gui_open,
            smoker_filtering_craftable,
        }),
        NetworkEvent::RecipeBookAdd {
            recipe_ids,
            replace,
        } => Some(BackendNetworkEvent::RecipeBookAdd {
            recipe_ids,
            replace,
        }),
        NetworkEvent::RecipeBookRemove { recipe_ids } => {
            Some(BackendNetworkEvent::RecipeBookRemove { recipe_ids })
        }
        NetworkEvent::UpdateRecipes => Some(BackendNetworkEvent::UpdateRecipes),
        NetworkEvent::UpdateTags => Some(BackendNetworkEvent::UpdateTags),
        NetworkEvent::PlaceGhostRecipe { container_id } => {
            Some(BackendNetworkEvent::PlaceGhostRecipe { container_id })
        }
        NetworkEvent::PlayerCombatKill { player_id, message } => {
            Some(BackendNetworkEvent::PlayerCombatKill { player_id, message })
        }
        NetworkEvent::PlayerCombatEnter => Some(BackendNetworkEvent::PlayerCombatEnter),
        NetworkEvent::PlayerCombatEnd { duration } => {
            Some(BackendNetworkEvent::PlayerCombatEnd { duration })
        }
        NetworkEvent::SetObjective {
            objective_name,
            title,
            display_format,
            method,
            number_format,
        } => Some(BackendNetworkEvent::SetObjective {
            objective_name,
            title,
            display_format,
            method,
            number_format,
        }),
        NetworkEvent::SetScore {
            owner,
            objective_name,
            score,
            display_name,
            display_format,
            number_format,
        } => Some(BackendNetworkEvent::SetScore {
            owner,
            objective_name,
            score,
            display_name,
            display_format,
            number_format,
        }),
        NetworkEvent::ResetScore {
            owner,
            objective_name,
        } => Some(BackendNetworkEvent::ResetScore {
            owner,
            objective_name,
        }),
        NetworkEvent::SetDisplayObjective {
            slot,
            objective_name,
        } => Some(BackendNetworkEvent::SetDisplayObjective {
            slot,
            objective_name,
        }),
        NetworkEvent::CustomPayload { channel, data } => {
            Some(BackendNetworkEvent::CustomPayload { channel, data })
        }
        NetworkEvent::ResourcePackPush {
            id,
            url,
            hash,
            required,
            prompt,
            server_pack_status,
            initial_status_sent,
        } => Some(BackendNetworkEvent::ResourcePackPush {
            id,
            url,
            hash,
            required,
            prompt,
            server_pack_status,
            initial_status_sent,
        }),
        NetworkEvent::ResourcePackPop { id } => Some(BackendNetworkEvent::ResourcePackPop { id }),
        NetworkEvent::ResetChat => Some(BackendNetworkEvent::ResetChat),
        NetworkEvent::ShowDialog {
            dialog_kind,
            dialog_nbt,
        } => Some(BackendNetworkEvent::ShowDialog {
            dialog_kind,
            dialog_nbt,
        }),
        NetworkEvent::ClearDialog => Some(BackendNetworkEvent::ClearDialog),
        NetworkEvent::MerchantOffers {
            container_id,
            offers,
            level,
            xp,
            show_progress,
            can_restock,
        } => Some(BackendNetworkEvent::MerchantOffers {
            container_id,
            offers,
            level,
            xp,
            show_progress,
            can_restock,
        }),
        NetworkEvent::MapItemData {
            map_id,
            scale,
            locked,
            tracking_position,
            color_patch,
            decorations,
        } => Some(BackendNetworkEvent::MapItemData {
            map_id,
            scale,
            locked,
            tracking_position,
            color_patch,
            decorations,
        }),
        NetworkEvent::OpenBook { hand } => Some(BackendNetworkEvent::OpenBook { hand }),
        NetworkEvent::OpenSignEditor { pos, is_front_text } => {
            Some(BackendNetworkEvent::OpenSignEditor { pos, is_front_text })
        }
        NetworkEvent::AwardStats { payload } => Some(BackendNetworkEvent::AwardStats { payload }),
        NetworkEvent::UpdateAdvancements { payload } => {
            Some(BackendNetworkEvent::UpdateAdvancements { payload })
        }
        NetworkEvent::ServerData { motd, icon_bytes } => {
            Some(BackendNetworkEvent::ServerData { motd, icon_bytes })
        }
        NetworkEvent::ServerLinks { links } => Some(BackendNetworkEvent::ServerLinks { links }),
        NetworkEvent::PauseScreenDialog {
            dialog_kind,
            dialog_nbt,
            requires_server_links,
        } => Some(BackendNetworkEvent::PauseScreenDialog {
            dialog_kind,
            dialog_nbt,
            requires_server_links,
        }),
        NetworkEvent::StoreCookie { key, payload } => {
            Some(BackendNetworkEvent::StoreCookie { key, payload })
        }
        NetworkEvent::SelectKnownPacks { known_packs } => {
            Some(BackendNetworkEvent::SelectKnownPacks { known_packs })
        }
        NetworkEvent::RegistryData {
            registry_id,
            entries_count,
        } => Some(BackendNetworkEvent::RegistryData {
            registry_id,
            entries_count,
        }),
        NetworkEvent::UpdateEnabledFeatures { features } => {
            Some(BackendNetworkEvent::UpdateEnabledFeatures { features })
        }
        NetworkEvent::CodeOfConduct { text } => Some(BackendNetworkEvent::CodeOfConduct { text }),
        NetworkEvent::CookieRequest { key, payload } => {
            Some(BackendNetworkEvent::CookieRequest { key, payload })
        }
        NetworkEvent::ChatAck { offset } => Some(BackendNetworkEvent::ChatAck { offset }),
        NetworkEvent::CustomReportDetails { details } => {
            Some(BackendNetworkEvent::CustomReportDetails { details })
        }
        NetworkEvent::DebugSample {
            sample,
            sample_type,
        } => Some(BackendNetworkEvent::DebugSample {
            sample,
            sample_type,
        }),
        NetworkEvent::Waypoint {
            operation,
            waypoint,
        } => Some(BackendNetworkEvent::Waypoint {
            operation,
            waypoint,
        }),
        NetworkEvent::LowDiskSpaceWarning => Some(BackendNetworkEvent::LowDiskSpaceWarning),
        NetworkEvent::GameRuleValues { values } => {
            Some(BackendNetworkEvent::GameRuleValues { values })
        }
    };
    Ok(mapped)
}
#[allow(dead_code)]
fn unsupported(
    kind: &'static str,
    unsupported_events: &mut HashSet<&'static str>,
) -> Option<BackendNetworkEvent> {
    if unsupported_events.insert(kind) {
        eprintln!("[EltharnProtocolBackend] unsupported inbound event {kind}");
    }
    None
}
fn map_biome(biome: BiomeRenderData) -> BackendBiomeRenderData {
    BackendBiomeRenderData {
        id: biome.id,
        has_precipitation: biome.has_precipitation,
        temperature: biome.temperature,
        temperature_modifier: biome.temperature_modifier,
        downfall: biome.downfall,
        water_color: biome.water_color,
        foliage_color: biome.foliage_color,
        dry_foliage_color: biome.dry_foliage_color,
        grass_color: biome.grass_color,
        grass_color_modifier: biome.grass_color_modifier,
    }
}
fn map_block_update(update: BlockUpdateEvent) -> BackendBlockUpdateEvent {
    BackendBlockUpdateEvent {
        x: update.x,
        y: update.y,
        z: update.z,
        state_id: update.state_id,
    }
}
fn map_remote_entity(entity: RemoteEntityEvent) -> BackendRemoteEntityEvent {
    BackendRemoteEntityEvent {
        id: entity.id,
        uuid: entity.uuid,
        kind: entity.kind,
        x: entity.x,
        y: entity.y,
        z: entity.z,
        yaw_degrees: entity.yaw_degrees,
        pitch_degrees: entity.pitch_degrees,
        head_yaw_degrees: entity.head_yaw_degrees,
        spawn_data: entity.spawn_data,
        falling_block_state_id: entity.falling_block_state_id,
        velocity_x: entity.velocity_x,
        velocity_y: entity.velocity_y,
        velocity_z: entity.velocity_z,
    }
}
fn map_chat_suggestion(suggestion: NetworkChatSuggestion) -> BackendNetworkChatSuggestion {
    BackendNetworkChatSuggestion {
        text: suggestion.text,
        tooltip: suggestion.tooltip,
    }
}
fn map_chat_completion_action(action: ChatCompletionAction) -> BackendChatCompletionAction {
    match action {
        ChatCompletionAction::Add => BackendChatCompletionAction::Add,
        ChatCompletionAction::Remove => BackendChatCompletionAction::Remove,
        ChatCompletionAction::Set => BackendChatCompletionAction::Set,
    }
}
fn map_chat_trust(trust: ChatTrust) -> BackendChatTrust {
    match trust {
        ChatTrust::Secure => BackendChatTrust::Secure,
        ChatTrust::Modified => BackendChatTrust::Modified,
        ChatTrust::NotSecure => BackendChatTrust::NotSecure,
    }
}
fn map_nbt_to_backend_nbt(tag: el_nbt::Tag) -> el_protocol_backend::BackendNbtTag {
    match tag {
        el_nbt::Tag::End => el_protocol_backend::BackendNbtTag::End,
        el_nbt::Tag::Byte(v) => el_protocol_backend::BackendNbtTag::Byte(v),
        el_nbt::Tag::Short(v) => el_protocol_backend::BackendNbtTag::Short(v),
        el_nbt::Tag::Int(v) => el_protocol_backend::BackendNbtTag::Int(v),
        el_nbt::Tag::Long(v) => el_protocol_backend::BackendNbtTag::Long(v),
        el_nbt::Tag::Float(v) => el_protocol_backend::BackendNbtTag::Float(v),
        el_nbt::Tag::Double(v) => el_protocol_backend::BackendNbtTag::Double(v),
        el_nbt::Tag::ByteArray(v) => el_protocol_backend::BackendNbtTag::ByteArray(v),
        el_nbt::Tag::String(v) => el_protocol_backend::BackendNbtTag::String(v),
        el_nbt::Tag::List(v) => el_protocol_backend::BackendNbtTag::List(
            v.into_iter().map(map_nbt_to_backend_nbt).collect(),
        ),
        el_nbt::Tag::Compound(v) => el_protocol_backend::BackendNbtTag::Compound(
            v.into_iter()
                .map(|(k, t)| (k, map_nbt_to_backend_nbt(t)))
                .collect(),
        ),
        el_nbt::Tag::IntArray(v) => el_protocol_backend::BackendNbtTag::IntArray(v),
        el_nbt::Tag::LongArray(v) => el_protocol_backend::BackendNbtTag::LongArray(v),
    }
}
fn map_item_stack_to_backend(stack: el_core::ItemStack) -> el_protocol_backend::BackendItemStack {
    let tag = stack.to_nbt();
    el_protocol_backend::BackendItemStack {
        tag: map_nbt_to_backend_nbt(tag),
    }
}
fn map_backend_nbt_to_nbt(tag: el_protocol_backend::BackendNbtTag) -> el_nbt::Tag {
    match tag {
        el_protocol_backend::BackendNbtTag::End => el_nbt::Tag::End,
        el_protocol_backend::BackendNbtTag::Byte(v) => el_nbt::Tag::Byte(v),
        el_protocol_backend::BackendNbtTag::Short(v) => el_nbt::Tag::Short(v),
        el_protocol_backend::BackendNbtTag::Int(v) => el_nbt::Tag::Int(v),
        el_protocol_backend::BackendNbtTag::Long(v) => el_nbt::Tag::Long(v),
        el_protocol_backend::BackendNbtTag::Float(v) => el_nbt::Tag::Float(v),
        el_protocol_backend::BackendNbtTag::Double(v) => el_nbt::Tag::Double(v),
        el_protocol_backend::BackendNbtTag::ByteArray(v) => el_nbt::Tag::ByteArray(v),
        el_protocol_backend::BackendNbtTag::String(v) => el_nbt::Tag::String(v),
        el_protocol_backend::BackendNbtTag::List(v) => {
            el_nbt::Tag::List(v.into_iter().map(map_backend_nbt_to_nbt).collect())
        }
        el_protocol_backend::BackendNbtTag::Compound(v) => el_nbt::Tag::Compound(
            v.into_iter()
                .map(|(k, t)| (k, map_backend_nbt_to_nbt(t)))
                .collect(),
        ),
        el_protocol_backend::BackendNbtTag::IntArray(v) => el_nbt::Tag::IntArray(v),
        el_protocol_backend::BackendNbtTag::LongArray(v) => el_nbt::Tag::LongArray(v),
    }
}
fn map_backend_item_stack(b_stack: el_protocol_backend::BackendItemStack) -> el_core::ItemStack {
    let tag = map_backend_nbt_to_nbt(b_stack.tag);
    el_core::ItemStack::from_nbt(&tag).unwrap_or_else(el_core::ItemStack::empty)
}
fn map_blocks_container_to_backend(
    container: &el_world::PalettedContainer<el_core::BlockState>,
) -> el_protocol_backend::BackendPalettedContainer<usize> {
    el_protocol_backend::BackendPalettedContainer {
        palette: container.palette.iter().map(|state| state.id).collect(),
        storage: container.storage.clone(),
        bits_per_entry: container.bits_per_entry,
        capacity: container.capacity,
        default_value: container.default_value.id,
    }
}
fn map_biomes_container_to_backend(
    container: &el_world::PalettedContainer<el_core::resource_location::ResourceLocation>,
) -> el_protocol_backend::BackendPalettedContainer<String> {
    el_protocol_backend::BackendPalettedContainer {
        palette: container
            .palette
            .iter()
            .map(|loc| loc.to_string())
            .collect(),
        storage: container.storage.clone(),
        bits_per_entry: container.bits_per_entry,
        capacity: container.capacity,
        default_value: container.default_value.to_string(),
    }
}
fn map_heightmap_type_to_string(h: el_world::chunk::HeightmapType) -> String {
    match h {
        el_world::chunk::HeightmapType::WorldSurfaceWg => "world_surface_wg".to_string(),
        el_world::chunk::HeightmapType::WorldSurface => "world_surface".to_string(),
        el_world::chunk::HeightmapType::OceanFloorWg => "ocean_floor_wg".to_string(),
        el_world::chunk::HeightmapType::OceanFloor => "ocean_floor".to_string(),
        el_world::chunk::HeightmapType::MotionBlocking => "motion_blocking".to_string(),
        el_world::chunk::HeightmapType::MotionBlockingNoLeaves => {
            "motion_blocking_no_leaves".to_string()
        }
    }
}
fn map_chunk_to_backend(chunk: el_world::chunk::Chunk) -> el_protocol_backend::BackendChunk {
    let sections = chunk
        .sections
        .into_iter()
        .map(|sec| el_protocol_backend::BackendChunkSection {
            y_index: sec.y_index,
            non_empty_block_count: sec.non_empty_block_count,
            fluid_count: sec.fluid_count,
            ticking_block_count: sec.ticking_block_count,
            ticking_fluid_count: sec.ticking_fluid_count,
            blocks: map_blocks_container_to_backend(&sec.blocks),
            biomes: map_biomes_container_to_backend(&sec.biomes),
        })
        .collect::<Vec<_>>();
    let mut heightmaps = std::collections::HashMap::new();
    for (k, v) in chunk.heightmaps {
        heightmaps.insert(map_heightmap_type_to_string(k), v);
    }
    let mut block_entities = Vec::new();
    for (pos, data) in chunk.block_entities {
        block_entities.push(el_protocol_backend::BackendBlockEntityEntry {
            pos: BackendBlockPos {
                x: pos.x,
                y: pos.y,
                z: pos.z,
            },
            data: el_protocol_backend::BackendBlockEntityData {
                kind: data.kind,
                nbt: data.nbt,
            },
        });
    }
    let sky_light = chunk
        .sky_light
        .iter()
        .map(|opt| opt.as_ref().map(|layer| layer.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    let block_light = chunk
        .block_light
        .iter()
        .map(|opt| opt.as_ref().map(|layer| layer.as_bytes().to_vec()))
        .collect::<Vec<_>>();
    el_protocol_backend::BackendChunk {
        x: chunk.x,
        z: chunk.z,
        sections,
        min_y_section: chunk.min_y_section,
        heightmaps,
        block_entities,
        sky_light,
        block_light,
        is_light_correct: chunk.is_light_correct,
    }
}
fn map_light_update_to_backend(
    update: crate::LightUpdateEvent,
) -> el_protocol_backend::BackendLightUpdateEvent {
    let sky_layers = update
        .sky_layers
        .iter()
        .map(|(idx, opt)| (*idx, opt.as_ref().map(|layer| layer.as_bytes().to_vec())))
        .collect::<Vec<_>>();
    let block_layers = update
        .block_layers
        .iter()
        .map(|(idx, opt)| (*idx, opt.as_ref().map(|layer| layer.as_bytes().to_vec())))
        .collect::<Vec<_>>();
    el_protocol_backend::BackendLightUpdateEvent {
        x: update.x,
        z: update.z,
        sky_layers,
        block_layers,
    }
}
fn map_biome_update_to_backend(
    update: crate::ChunkBiomeUpdate,
) -> el_protocol_backend::BackendChunkBiomeUpdate {
    let sections = update
        .sections
        .into_iter()
        .map(|sec| sec.into_iter().map(|loc| loc.to_string()).collect())
        .collect::<Vec<_>>();
    el_protocol_backend::BackendChunkBiomeUpdate {
        x: update.x,
        z: update.z,
        sections,
    }
}
fn map_to_backend_boss_event_operation(
    op: crate::BossEventOperation,
) -> el_protocol_backend::BackendBossEventOperation {
    use crate::BossEventOperation as Aop;
    use el_protocol_backend::BackendBossEventOperation as Bop;
    match op {
        Aop::Add(add) => Bop::Add(el_protocol_backend::BackendBossEventAdd {
            name: add.name,
            progress: add.progress,
            style: el_protocol_backend::BackendBossEventStyle {
                color: match add.style.color {
                    crate::BossBarColor::Pink => el_protocol_backend::BackendBossBarColor::Pink,
                    crate::BossBarColor::Blue => el_protocol_backend::BackendBossBarColor::Blue,
                    crate::BossBarColor::Red => el_protocol_backend::BackendBossBarColor::Red,
                    crate::BossBarColor::Green => el_protocol_backend::BackendBossBarColor::Green,
                    crate::BossBarColor::Yellow => el_protocol_backend::BackendBossBarColor::Yellow,
                    crate::BossBarColor::Purple => el_protocol_backend::BackendBossBarColor::Purple,
                    crate::BossBarColor::White => el_protocol_backend::BackendBossBarColor::White,
                },
                overlay: match add.style.overlay {
                    crate::BossBarOverlay::Progress => {
                        el_protocol_backend::BackendBossBarOverlay::Progress
                    }
                    crate::BossBarOverlay::Notched6 => {
                        el_protocol_backend::BackendBossBarOverlay::Notched6
                    }
                    crate::BossBarOverlay::Notched10 => {
                        el_protocol_backend::BackendBossBarOverlay::Notched10
                    }
                    crate::BossBarOverlay::Notched12 => {
                        el_protocol_backend::BackendBossBarOverlay::Notched12
                    }
                    crate::BossBarOverlay::Notched20 => {
                        el_protocol_backend::BackendBossBarOverlay::Notched20
                    }
                },
            },
            properties: el_protocol_backend::BackendBossEventProperties {
                darken_screen: add.properties.darken_screen,
                play_music: add.properties.play_music,
                create_world_fog: add.properties.create_world_fog,
            },
        }),
        Aop::Remove => Bop::Remove,
        Aop::UpdateProgress(p) => Bop::UpdateProgress(p),
        Aop::UpdateName(n) => Bop::UpdateName(n),
        Aop::UpdateStyle(style) => Bop::UpdateStyle(el_protocol_backend::BackendBossEventStyle {
            color: match style.color {
                crate::BossBarColor::Pink => el_protocol_backend::BackendBossBarColor::Pink,
                crate::BossBarColor::Blue => el_protocol_backend::BackendBossBarColor::Blue,
                crate::BossBarColor::Red => el_protocol_backend::BackendBossBarColor::Red,
                crate::BossBarColor::Green => el_protocol_backend::BackendBossBarColor::Green,
                crate::BossBarColor::Yellow => el_protocol_backend::BackendBossBarColor::Yellow,
                crate::BossBarColor::Purple => el_protocol_backend::BackendBossBarColor::Purple,
                crate::BossBarColor::White => el_protocol_backend::BackendBossBarColor::White,
            },
            overlay: match style.overlay {
                crate::BossBarOverlay::Progress => {
                    el_protocol_backend::BackendBossBarOverlay::Progress
                }
                crate::BossBarOverlay::Notched6 => {
                    el_protocol_backend::BackendBossBarOverlay::Notched6
                }
                crate::BossBarOverlay::Notched10 => {
                    el_protocol_backend::BackendBossBarOverlay::Notched10
                }
                crate::BossBarOverlay::Notched12 => {
                    el_protocol_backend::BackendBossBarOverlay::Notched12
                }
                crate::BossBarOverlay::Notched20 => {
                    el_protocol_backend::BackendBossBarOverlay::Notched20
                }
            },
        }),
        Aop::UpdateProperties(props) => {
            Bop::UpdateProperties(el_protocol_backend::BackendBossEventProperties {
                darken_screen: props.darken_screen,
                play_music: props.play_music,
                create_world_fog: props.create_world_fog,
            })
        }
    }
}
