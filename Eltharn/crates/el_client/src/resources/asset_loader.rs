use crate::resources::block_model::BlockModel;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[allow(dead_code)]
pub struct AssetLoader {
    base_path: PathBuf,
    model_cache: HashMap<String, BlockModel>,
}

#[allow(dead_code)]
impl AssetLoader {
    pub fn new<P: AsRef<Path>>(base_path: P) -> Self {
        Self {
            base_path: base_path.as_ref().to_path_buf(),
            model_cache: HashMap::new(),
        }
    }

    pub fn load_block_model(&mut self, name: &str) -> Option<BlockModel> {
        if let Some(model) = self.model_cache.get(name) {
            return Some(model.clone());
        }

        let clean_name = name.strip_prefix("minecraft:").unwrap_or(name);

        let path = self
            .base_path
            .join("models")
            .join(format!("{}.json", clean_name));
        let content = std::fs::read_to_string(&path).ok()?;

        let mut model: BlockModel = serde_json::from_str(&content).ok()?;

        if let Some(parent_name) = &model.parent {
            if !parent_name.starts_with("builtin/") {
                if let Some(parent_model) = self.load_block_model(parent_name) {
                    if let Some(parent_textures) = parent_model.textures {
                        let mut merged = parent_textures;
                        if let Some(self_textures) = model.textures.take() {
                            merged.extend(self_textures);
                        }
                        model.textures = Some(merged);
                    }

                    if model.elements.is_none() {
                        model.elements = parent_model.elements;
                    }
                }
            }
        }

        self.model_cache.insert(name.to_string(), model.clone());
        Some(model)
    }
}
