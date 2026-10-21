use crate::render::item_in_hand_renderer::ItemUseAnimation;
use crate::render::model::player::player_model::HandSide;
use el_core::{item::ItemStack, resource_location::ResourceLocation};
#[derive(Default)]
pub(crate) struct LocalItemUse {
    pub hand: Option<HandSide>,
    pub item_id: Option<ResourceLocation>,
    pub remaining: f32,
    pub duration: f32,
    pub animation: Option<ItemUseAnimation>,
    pub server_confirmed: bool,
}
impl LocalItemUse {
    pub fn start(
        &mut self,
        hand: HandSide,
        stack: &ItemStack,
        animation: Option<ItemUseAnimation>,
        duration: f32,
    ) -> bool {
        if stack.is_empty() || self.hand.is_some() {
            return false;
        }
        self.hand = Some(hand);
        self.item_id = Some(stack.item.location.clone());
        self.remaining = duration;
        self.duration = duration;
        self.animation = animation;
        self.server_confirmed = false;
        true
    }
    pub fn server_flag_transition(&mut self, flags: u8) -> Option<bool> {
        let server_using = flags & 1 != 0;
        if server_using {
            self.server_confirmed = true;
            if self.hand.is_none() {
                Some(true)
            } else {
                None
            }
        } else {
            if self.hand.is_some() && self.server_confirmed {
                self.server_confirmed = false;
                Some(false)
            } else {
                None
            }
        }
    }
    pub fn tick(&mut self) {
        if self.hand.is_some() {
            self.remaining -= 1.0;
        }
    }
    pub fn stop(&mut self) {
        *self = Self::default();
    }
    pub fn handle_entity_event(&mut self, event_id: u8) -> bool {
        if event_id != 9 || self.hand.is_none() {
            return false;
        }
        self.stop();
        true
    }
}
