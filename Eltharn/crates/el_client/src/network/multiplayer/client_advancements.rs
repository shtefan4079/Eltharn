#![allow(dead_code)]

use el_core::resource_location::ResourceLocation;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Advancement {
    pub id: ResourceLocation,
    pub parent_id: Option<ResourceLocation>,
    pub title: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct AdvancementProgress {
    pub is_done: bool,
}

pub struct ClientAdvancements {
    pub advancements: HashMap<ResourceLocation, Advancement>,
    pub progress: HashMap<ResourceLocation, AdvancementProgress>,
    pub selected_tab: Option<ResourceLocation>,
}

impl Default for ClientAdvancements {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientAdvancements {
    pub fn new() -> Self {
        Self {
            advancements: HashMap::new(),
            progress: HashMap::new(),
            selected_tab: None,
        }
    }

    pub fn clear(&mut self) {
        self.advancements.clear();
        self.progress.clear();
        self.selected_tab = None;
    }

    pub fn select_tab(&mut self, tab: Option<ResourceLocation>) {
        self.selected_tab = tab;
    }

    pub fn get(&self, id: &ResourceLocation) -> Option<&Advancement> {
        self.advancements.get(id)
    }

    pub fn set_progress(&mut self, id: ResourceLocation, is_done: bool) {
        self.progress.insert(id, AdvancementProgress { is_done });
    }

    pub fn is_done(&self, id: &ResourceLocation) -> bool {
        self.progress.get(id).map_or(false, |p| p.is_done)
    }
}
