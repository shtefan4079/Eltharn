#![allow(dead_code)]

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KnownPack {
    pub namespace: String,
    pub id: String,
    pub version: String,
}

pub struct KnownPacksManager {
    pub available_packs: Vec<KnownPack>,
}

impl Default for KnownPacksManager {
    fn default() -> Self {
        Self::new()
    }
}

impl KnownPacksManager {
    pub fn new() -> Self {
        Self {
            available_packs: Vec::new(),
        }
    }

    pub fn select_packs(&self, packs: &[KnownPack]) -> Vec<KnownPack> {
        let mut selected = Vec::new();
        for pack in packs {
            if pack.namespace == "minecraft" {
                selected.push(pack.clone());
            }
        }
        selected
    }
}
