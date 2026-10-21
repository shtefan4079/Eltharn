use el_core::item::{DataComponent, ItemStack};
use el_core::resource_location::ResourceLocation;
use el_nbt::Tag;
use std::collections::HashMap;
pub struct ItemCooldowns {
    cooldowns: HashMap<ResourceLocation, (i32, i32)>,
}
impl Default for ItemCooldowns {
    fn default() -> Self {
        Self {
            cooldowns: HashMap::new(),
        }
    }
}
impl ItemCooldowns {
    pub fn add_cooldown(&mut self, cooldown_group: ResourceLocation, duration_ticks: i32) {
        self.cooldowns
            .insert(cooldown_group, (duration_ticks, duration_ticks));
    }
    pub fn remove_cooldown(&mut self, cooldown_group: &ResourceLocation) {
        self.cooldowns.remove(cooldown_group);
    }
    pub fn apply_server_update(&mut self, cooldown_group: ResourceLocation, duration_ticks: i32) {
        if duration_ticks == 0 {
            self.remove_cooldown(&cooldown_group);
        } else {
            self.add_cooldown(cooldown_group, duration_ticks);
        }
    }
    pub fn cooldown_group(stack: &ItemStack) -> ResourceLocation {
        if let Some(DataComponent::Generic {
            value: Tag::Compound(fields),
            ..
        }) = stack.components.get("minecraft:use_cooldown")
            && let Some(Tag::String(group)) = el_nbt::compound_get(fields, "cooldown_group")
        {
            ResourceLocation::from_string(group)
        } else {
            stack.item.location.clone()
        }
    }
    pub fn tick(&mut self) {
        self.cooldowns.retain(|_, (remaining, _)| {
            if *remaining > 0 {
                *remaining -= 1;
                *remaining > 0
            } else {
                false
            }
        });
    }
    pub fn cooldown_percent(&self, cooldown_group: &ResourceLocation, partial_tick: f32) -> f32 {
        let Some(&(remaining, duration)) = self.cooldowns.get(cooldown_group) else {
            return 0.0;
        };
        if duration == 0 {
            return 0.0;
        }
        ((remaining as f32 - partial_tick) / duration as f32).clamp(0.0, 1.0)
    }
    pub fn is_on_cooldown(&self, stack: &ItemStack) -> bool {
        !stack.is_empty() && self.cooldown_percent(&Self::cooldown_group(stack), 0.0) > 0.0
    }
}
