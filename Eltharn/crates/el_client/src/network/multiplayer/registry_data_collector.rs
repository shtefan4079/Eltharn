#![allow(dead_code)]

use el_core::registry::RegistryAccess;
use el_core::resource_location::ResourceLocation;
use std::collections::HashMap;

pub struct RegistryDataCollector {
    pub contents: HashMap<ResourceLocation, Vec<String>>,
    pub tags: HashMap<ResourceLocation, Vec<String>>,
}

impl Default for RegistryDataCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl RegistryDataCollector {
    pub fn new() -> Self {
        Self {
            contents: HashMap::new(),
            tags: HashMap::new(),
        }
    }

    pub fn append_contents(&mut self, registry: ResourceLocation, entries: Vec<String>) {
        self.contents.entry(registry).or_default().extend(entries);
    }

    pub fn append_tags(&mut self, tags: HashMap<ResourceLocation, Vec<String>>) {
        for (registry, tag_entries) in tags {
            self.tags.entry(registry).or_default().extend(tag_entries);
        }
    }

    pub fn collect_game_registries(&self, _original_registries: &RegistryAccess) -> RegistryAccess {
        RegistryAccess::new()
    }
}
