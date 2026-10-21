use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};

use el_mod_network::{
    frame,
    wire::{self, ClientMessage, ServerMessage},
};
use el_mod_runtime::server_types::{NetworkOutput, validate_client_network};

use crate::network::protocol_backend::RemotePacketSender;

#[derive(Default)]
struct Remote {
    incoming: frame::Reassembler,
    nonce: Option<String>,
    world: Option<String>,
    revision: u64,
    next_message: u64,
    next_action: u64,
    ready: bool,
    pending: BTreeMap<u64, String>,
    server_actors: Vec<String>,
    outgoing_channels: VecDeque<NetworkOutput>,
    dropped_outgoing: u64,
    last_ack: Option<wire::Ack>,
    received_channels: u64,
    delivered_channels: u64,
    dropped_channels: u64,
}

thread_local! {
    static REMOTE: RefCell<Remote> = RefCell::new(Remote::default());
}

pub fn report() -> serde_json::Value {
    REMOTE.with(|cell| {
        let state = cell.borrow();
        serde_json::json!({
            "connected": state.nonce.is_some(),
            "ready": state.ready,
            "world": state.world,
            "revision": state.revision,
            "pending_actions": state.pending.len(),
            "queued_channels": state.outgoing_channels.len(),
            "dropped_outgoing_channels": state.dropped_outgoing,
            "last_ack": state.last_ack,
            "received_channels": state.received_channels,
            "delivered_channels": state.delivered_channels,
            "dropped_channels": state.dropped_channels,
        })
    })
}

pub fn queue_guest_outputs(report: &serde_json::Value) -> Result<(), String> {
    let Some(raw) = report.get("network") else {
        return Ok(());
    };
    let outputs: Vec<NetworkOutput> =
        serde_json::from_value(raw.clone()).map_err(|_| "invalid guest network report")?;
    if outputs.len() > 64 {
        return Err("too many client guest channel messages".into());
    }
    for output in &outputs {
        validate_client_network(
            &output.owner,
            output.to.as_deref(),
            &output.channel,
            &output.data,
        )?;
    }
    REMOTE.with(|cell| {
        let mut state = cell.borrow_mut();
        if state.outgoing_channels.len() + outputs.len() > 64 {
            return Err("client guest channel queue full".into());
        }
        state.outgoing_channels.extend(outputs);
        Ok(())
    })
}

pub fn reset() {
    REMOTE.with(|cell| *cell.borrow_mut() = Remote::default());
    super::client_content::clear_remote();
}

fn send(
    state: &mut Remote,
    sender: &RemotePacketSender,
    message: &ClientMessage,
) -> Result<(), String> {
    let bytes = wire::encode_client(message)?;
    let next = state
        .next_message
        .checked_add(1)
        .filter(|value| *value <= frame::MAX_MESSAGE_ID)
        .ok_or("mod message counter exhausted")?;
    for part in frame::fragment(next, &bytes)? {
        sender.send_custom_payload(frame::CHANNEL.into(), part)?;
    }
    state.next_message = next;
    Ok(())
}

pub fn on_payload(sender: &RemotePacketSender, data: &[u8]) -> Result<bool, String> {
    REMOTE.with(|cell| {
        let mut state = cell.borrow_mut();
        let Some(bytes) = state.incoming.push(data)? else {
            return Ok(false);
        };
        match wire::decode_server(&bytes)? {
            ServerMessage::Offer {
                nonce,
                world,
                packages,
                actors,
                catalog,
                fingerprint,
                ..
            } => {
                if state.nonce.is_some() {
                    return Err("duplicate server mod offer".into());
                }
                let selected = crate::extension_launch::selected_mod_pins();
                if !packages
                    .iter()
                    .all(|required| selected.iter().any(|pin| pin == required))
                {
                    return Err("server requires different mod packages".into());
                }
                let catalog_entries = catalog.len();
                let server_actors = actors.len();
                let changed = super::client_content::remote_offer(&catalog, actors.clone())?;
                let hello = ClientMessage::Hello {
                    nonce: nonce.clone(),
                    protocol: wire::PROTOCOL,
                    api: wire::API.into(),
                    packages: selected,
                    fingerprint,
                };
                send(&mut state, sender, &hello)?;
                state.nonce = Some(nonce);
                state.world = Some(world);
                state.server_actors = actors;
                state.revision = 0;
                log::info!("[ServerMods] offer verified packages={} actors={} catalog_entries={} reload={}",
                    packages.len(), server_actors, catalog_entries, changed);
                Ok(changed)
            }
            ServerMessage::State {
                nonce,
                revision,
                scene,
                ack,
                messages,
            } => {
                if state.nonce.as_deref() != Some(nonce.as_str()) {
                    return Err("server mod session mismatch".into());
                }
                if revision < state.revision {
                    return Err("server mod state went backwards".into());
                }
                let actor_count = scene.actors.len();
                super::client_content::remote_state(scene)?;
                state.revision = revision;
                let first_state = !state.ready;
                state.ready = true;
                if first_state {
                    log::info!("[ServerMods] authoritative state ready revision={} actors={}",
                        revision, actor_count);
                }
                if let Some(ack) = ack {
                    if state.pending.remove(&ack.sequence).is_none() {
                        return Err("unexpected server mod acknowledgement".into());
                    }
                    if !ack.accepted {
                        log::warn!(
                            "Server rejected mod action {}: {}",
                            ack.sequence,
                            ack.reason
                        );
                    }
                    state.last_ack = Some(ack);
                }
                if !messages.is_empty() {
                    let (delivered, dropped) =
                        super::client_content::remote_channels(&messages)?;
                    state.received_channels = state
                        .received_channels
                        .saturating_add(messages.len() as u64);
                    state.delivered_channels = state.delivered_channels.saturating_add(delivered as u64);
                    state.dropped_channels = state.dropped_channels.saturating_add(dropped as u64);
                    if dropped > 0 {
                        log::warn!("[ServerMods] {} channel deliveries had no receiver or exceeded the guest queue", dropped);
                    }
                }
                Ok(false)
            }
            ServerMessage::Reject { reason } => Err(format!("server rejected mods: {reason}")),
        }
    })
}

pub fn flush(sender: &RemotePacketSender) -> Result<(), String> {
    REMOTE.with(|cell| {
        let mut state = cell.borrow_mut();
        if !state.ready {
            return Ok(());
        }
        let nonce = state.nonce.clone().ok_or("mod session missing")?;
        let room = 64usize.saturating_sub(state.pending.len());
        for (owner, mut event) in super::client_content::take_remote_events(room) {
            let sequence = state
                .next_action
                .checked_add(1)
                .filter(|value| *value <= frame::MAX_MESSAGE_ID)
                .ok_or("mod action counter exhausted")?;
            event.sequence = sequence;
            let message = ClientMessage::Action {
                nonce: nonce.clone(),
                sequence,
                revision: state.revision,
                owner: owner.clone(),
                event,
            };
            send(&mut state, sender, &message)?;
            state.next_action = sequence;
            state.pending.insert(sequence, owner);
        }
        while state.pending.len() < 64 {
            let Some(output) = state.outgoing_channels.front().cloned() else {
                break;
            };
            let package = output
                .owner
                .rsplit_once('/')
                .ok_or("bad client channel owner")?
                .0;
            let target = match output.to.as_deref() {
                Some(owner)
                    if state
                        .server_actors
                        .iter()
                        .any(|candidate| candidate == owner) =>
                {
                    Some(owner.to_string())
                }
                Some(_) => None,
                None => [format!("{package}/server"), format!("{package}/shared")]
                    .into_iter()
                    .find(|candidate| state.server_actors.contains(candidate)),
            };
            let Some(target) = target else {
                state.outgoing_channels.pop_front();
                state.dropped_outgoing = state.dropped_outgoing.saturating_add(1);
                log::warn!(
                    "[ServerMods] client guest channel has no negotiated server actor: {}",
                    output.owner
                );
                continue;
            };
            let sequence = state
                .next_action
                .checked_add(1)
                .filter(|value| *value <= frame::MAX_MESSAGE_ID)
                .ok_or("mod channel counter exhausted")?;
            let message = ClientMessage::Channel {
                nonce: nonce.clone(),
                sequence,
                revision: state.revision,
                owner: target.clone(),
                channel: output.channel,
                data: output.data,
            };
            send(&mut state, sender, &message)?;
            state.outgoing_channels.pop_front();
            state.next_action = sequence;
            state.pending.insert(sequence, target);
        }
        Ok(())
    })
}
