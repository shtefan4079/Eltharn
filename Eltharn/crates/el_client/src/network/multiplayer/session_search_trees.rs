#![allow(dead_code)]

use el_core::item::ItemStack;

pub struct SearchTree<T> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T> SearchTree<T> {
    pub fn empty() -> Self {
        Self {
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn search(&self, _query: &str) -> Vec<T> {
        Vec::new()
    }
}

pub struct SessionSearchTrees {
    creative_by_name_search: SearchTree<ItemStack>,
    creative_by_tag_search: SearchTree<ItemStack>,
}

impl Default for SessionSearchTrees {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionSearchTrees {
    pub fn new() -> Self {
        Self {
            creative_by_name_search: SearchTree::empty(),
            creative_by_tag_search: SearchTree::empty(),
        }
    }

    pub fn rebuild_after_language_change(&mut self) {}

    pub fn update_recipes(&mut self) {}

    pub fn update_creative_tags(&mut self, _items: &[ItemStack]) {}

    pub fn update_creative_tooltips(&mut self, _items: &[ItemStack]) {}

    pub fn creative_tag_search(&self) -> &SearchTree<ItemStack> {
        &self.creative_by_tag_search
    }

    pub fn creative_name_search(&self) -> &SearchTree<ItemStack> {
        &self.creative_by_name_search
    }
}
