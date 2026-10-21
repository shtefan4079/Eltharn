use el_protocol_backend::{BackendDebugSubscriptionEvent, BackendDebugSubscriptionUpdate};
use serde_json::Value;
use std::collections::HashMap;
fn debug_subscription_expire_after_ticks(subscription: &str) -> i64 {
    match subscription {
        "entity_block_intersections" => 100,
        "redstone_wire_orientations" => 200,
        "neighbor_updates" => 200,
        "game_events" => 60,
        _ => 0,
    }
}
const MAX_DEBUG_ENTRIES_PER_MAP: usize = 256;
const MAX_DEBUG_SUBSCRIPTIONS: usize = 256;
const UNKNOWN_DEBUG_SUBSCRIPTION_EXPIRE_TICKS: i64 = 6_000;
fn effective_debug_subscription_expire_after_ticks(subscription: &str) -> i64 {
    let expire = debug_subscription_expire_after_ticks(subscription);
    if expire == 0 {
        UNKNOWN_DEBUG_SUBSCRIPTION_EXPIRE_TICKS
    } else {
        expire
    }
}
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct DebugValueWrapper {
    pub(crate) value: Value,
    expires_at: i64,
    stored_seq: u64,
}
impl DebugValueWrapper {
    fn has_expired(&self, game_time: i64) -> bool {
        if self.expires_at == -1 {
            false
        } else {
            game_time >= self.expires_at
        }
    }
}
#[derive(Clone, Debug, Default)]
struct DebugValueMap<K> {
    values: HashMap<K, DebugValueWrapper>,
    next_seq: u64,
}
impl<K: Eq + std::hash::Hash + Copy> DebugValueMap<K> {
    fn apply(&mut self, game_time: i64, key: K, value: Option<Value>, expire_after_ticks: i64) {
        match value {
            Some(value) => {
                if !self.values.contains_key(&key) && self.values.len() >= MAX_DEBUG_ENTRIES_PER_MAP
                {
                    self.evict_oldest();
                }
                let stored_seq = self.next_seq;
                self.next_seq = self.next_seq.saturating_add(1);
                self.values.insert(
                    key,
                    DebugValueWrapper {
                        value,
                        expires_at: game_time + expire_after_ticks,
                        stored_seq,
                    },
                );
            }
            None => {
                self.values.remove(&key);
            }
        }
    }
    fn evict_oldest(&mut self) {
        if let Some(key) = self
            .values
            .iter()
            .min_by_key(|(_, wrapper)| wrapper.stored_seq)
            .map(|(key, _)| *key)
        {
            self.values.remove(&key);
        }
    }
    fn remove_expired(&mut self, game_time: i64) {
        self.values
            .retain(|_, wrapper| !wrapper.has_expired(game_time));
    }
}
#[derive(Clone, Debug, Default)]
struct DebugSubscriptionValues {
    chunk_values: DebugValueMap<(i32, i32)>,
    block_values: DebugValueMap<[i32; 3]>,
    entity_values: DebugValueMap<i32>,
    events: Vec<DebugValueWrapper>,
}
impl DebugSubscriptionValues {
    fn purge_expired(&mut self, game_time: i64) {
        self.chunk_values.remove_expired(game_time);
        self.block_values.remove_expired(game_time);
        self.entity_values.remove_expired(game_time);
        self.events.retain(|event| !event.has_expired(game_time));
    }
    fn drop_chunk(&mut self, chunk_pos: (i32, i32)) {
        let (chunk_x, chunk_z) = chunk_pos;
        self.chunk_values.values.remove(&chunk_pos);
        self.block_values
            .values
            .retain(|pos, _| !chunk_contains(chunk_x, chunk_z, *pos));
    }
}
fn chunk_contains(chunk_x: i32, chunk_z: i32, pos: [i32; 3]) -> bool {
    let block_x = pos[0] >> 4;
    let block_z = pos[2] >> 4;
    block_x == chunk_x && block_z == chunk_z
}
#[derive(Clone, Debug, Default)]
pub struct DebugSubscriptionSubscriber {
    values_by_subscription: HashMap<String, DebugSubscriptionValues>,
    warned_capacity: bool,
}
impl DebugSubscriptionSubscriber {
    fn subscription_values_mut(
        &mut self,
        subscription: &str,
    ) -> Option<&mut DebugSubscriptionValues> {
        if !self.values_by_subscription.contains_key(subscription)
            && self.values_by_subscription.len() >= MAX_DEBUG_SUBSCRIPTIONS
        {
            if !self.warned_capacity {
                self.warned_capacity = true;
                log::warn!(
                    "[DebugSubscriber] subscription cap ({MAX_DEBUG_SUBSCRIPTIONS}) reached; dropping further debug subscriptions"
                );
            }
            return None;
        }
        Some(
            self.values_by_subscription
                .entry(subscription.to_string())
                .or_default(),
        )
    }
    pub fn update_block(
        &mut self,
        game_time: i64,
        pos: [i32; 3],
        update: BackendDebugSubscriptionUpdate,
    ) {
        let expire = effective_debug_subscription_expire_after_ticks(&update.subscription);
        let Some(values) = self.subscription_values_mut(&update.subscription) else {
            return;
        };
        values
            .block_values
            .apply(game_time, pos, update.value, expire);
    }
    pub fn update_chunk(
        &mut self,
        game_time: i64,
        chunk_pos: (i32, i32),
        update: BackendDebugSubscriptionUpdate,
    ) {
        let expire = effective_debug_subscription_expire_after_ticks(&update.subscription);
        let Some(values) = self.subscription_values_mut(&update.subscription) else {
            return;
        };
        values
            .chunk_values
            .apply(game_time, chunk_pos, update.value, expire);
    }
    pub fn update_entity(
        &mut self,
        game_time: i64,
        entity_id: i32,
        update: BackendDebugSubscriptionUpdate,
    ) {
        let expire = effective_debug_subscription_expire_after_ticks(&update.subscription);
        let Some(values) = self.subscription_values_mut(&update.subscription) else {
            return;
        };
        values
            .entity_values
            .apply(game_time, entity_id, update.value, expire);
    }
    pub fn push_event(&mut self, game_time: i64, event: BackendDebugSubscriptionEvent) {
        let BackendDebugSubscriptionEvent {
            subscription,
            value,
        } = event;
        let expire = effective_debug_subscription_expire_after_ticks(&subscription);
        let Some(values) = self.subscription_values_mut(&subscription) else {
            return;
        };
        if values.events.len() >= MAX_DEBUG_ENTRIES_PER_MAP {
            values.events.remove(0);
        }
        values.events.push(DebugValueWrapper {
            value,
            expires_at: game_time + expire,
            stored_seq: 0,
        });
    }
    pub fn purge_expired(&mut self, game_time: i64) {
        for values in self.values_by_subscription.values_mut() {
            values.purge_expired(game_time);
        }
    }
    pub fn drop_level(&mut self) {
        self.values_by_subscription.clear();
        self.warned_capacity = false;
    }
    pub fn drop_chunk(&mut self, chunk_pos: (i32, i32)) {
        for values in self.values_by_subscription.values_mut() {
            values.drop_chunk(chunk_pos);
        }
    }
    pub fn drop_entity(&mut self, entity_id: i32) {
        for values in self.values_by_subscription.values_mut() {
            values.entity_values.values.remove(&entity_id);
        }
    }
    pub fn block_value(&self, subscription: &str, pos: [i32; 3]) -> Option<&Value> {
        self.values_by_subscription
            .get(subscription)
            .and_then(|values| values.block_values.values.get(&pos))
            .map(|wrapper| &wrapper.value)
    }
}
pub struct GameTestBlockHighlightRenderer {
    markers: HashMap<[i32; 3], GameTestMarker>,
}
#[derive(Clone, Debug, PartialEq)]
struct GameTestMarker {
    color: u32,
    text: String,
    remove_at_time: u64,
}
impl Default for GameTestBlockHighlightRenderer {
    fn default() -> Self {
        Self::new()
    }
}
impl GameTestBlockHighlightRenderer {
    pub fn new() -> Self {
        Self {
            markers: HashMap::new(),
        }
    }
    pub fn highlight_pos(
        &mut self,
        now_millis: u64,
        absolute_pos: [i32; 3],
        relative_pos: [i32; 3],
    ) {
        let text = block_pos_short_string(relative_pos);
        self.markers.insert(
            absolute_pos,
            GameTestMarker {
                color: 0x6000FF00,
                text,
                remove_at_time: now_millis + 10000,
            },
        );
    }
    pub fn clear(&mut self) {
        self.markers.clear();
    }
    pub fn remove_expired(&mut self, now_millis: u64) {
        self.markers
            .retain(|_, marker| now_millis <= marker.remove_at_time);
    }
    pub fn is_empty(&self) -> bool {
        self.markers.is_empty()
    }
    pub fn marker_text(&self, absolute_pos: [i32; 3]) -> Option<&str> {
        self.markers
            .get(&absolute_pos)
            .map(|marker| marker.text.as_str())
    }
}
fn block_pos_short_string(pos: [i32; 3]) -> String {
    format!("{}, {}, {}", pos[0], pos[1], pos[2])
}
