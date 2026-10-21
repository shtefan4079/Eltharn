#![allow(dead_code)]

use el_core::resource_location::ResourceLocation;
use std::collections::HashMap;

pub struct ClientRecipeContainer {
    pub recipes: HashMap<ResourceLocation, Vec<String>>,
}

impl Default for ClientRecipeContainer {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientRecipeContainer {
    pub fn new() -> Self {
        Self {
            recipes: HashMap::new(),
        }
    }

    pub fn clear(&mut self) {
        self.recipes.clear();
    }

    pub fn add_recipe(&mut self, key: ResourceLocation, items: Vec<String>) {
        self.recipes.insert(key, items);
    }

    pub fn get_recipe(&self, key: &ResourceLocation) -> Option<&Vec<String>> {
        self.recipes.get(key)
    }
}
