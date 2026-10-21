use crate::codec::{
    AdvancementJson, BiomeJson, DamageTypeJson, DimensionTypeJson, LootTableJson, RecipeJson,
    TagJson,
};
use crate::pack::DataPack;
use el_core::ResourceLocation;
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub struct DataPackLoader;

impl DataPackLoader {
    pub fn new() -> Self {
        DataPackLoader
    }

    pub fn load_resources<T: DeserializeOwned>(
        &self,
        pack: &DataPack,
        subfolder: &str,
    ) -> io::Result<HashMap<ResourceLocation, T>> {
        let mut resources = HashMap::new();

        if !pack.data_path.exists() {
            return Ok(resources);
        }

        for entry in fs::read_dir(&pack.data_path)? {
            let entry = entry?;
            let ns_path = entry.path();
            if ns_path.is_dir() {
                let ns_name = ns_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("minecraft");

                let folder_path = ns_path.join(subfolder);
                if folder_path.exists() && folder_path.is_dir() {
                    self.scan_directory_and_load(
                        &folder_path,
                        ns_name,
                        &folder_path,
                        &mut resources,
                    )?;
                }
            }
        }

        Ok(resources)
    }

    pub fn load_tags(
        &self,
        pack: &DataPack,
        registry_type: &str,
    ) -> io::Result<HashMap<ResourceLocation, TagJson>> {
        let subfolder = format!("tags/{}", registry_type);
        self.load_resources::<TagJson>(pack, &subfolder)
    }

    pub fn load_recipes(
        &self,
        pack: &DataPack,
    ) -> io::Result<HashMap<ResourceLocation, RecipeJson>> {
        self.load_resources::<RecipeJson>(pack, "recipe")
    }

    pub fn load_loot_tables(
        &self,
        pack: &DataPack,
    ) -> io::Result<HashMap<ResourceLocation, LootTableJson>> {
        self.load_resources::<LootTableJson>(pack, "loot_table")
    }

    pub fn load_advancements(
        &self,
        pack: &DataPack,
    ) -> io::Result<HashMap<ResourceLocation, AdvancementJson>> {
        self.load_resources::<AdvancementJson>(pack, "advancement")
    }

    pub fn load_damage_types(
        &self,
        pack: &DataPack,
    ) -> io::Result<HashMap<ResourceLocation, DamageTypeJson>> {
        self.load_resources::<DamageTypeJson>(pack, "damage_type")
    }

    pub fn load_dimension_types(
        &self,
        pack: &DataPack,
    ) -> io::Result<HashMap<ResourceLocation, DimensionTypeJson>> {
        self.load_resources::<DimensionTypeJson>(pack, "dimension_type")
    }

    pub fn load_biomes(&self, pack: &DataPack) -> io::Result<HashMap<ResourceLocation, BiomeJson>> {
        self.load_resources::<BiomeJson>(pack, "worldgen/biome")
    }

    fn scan_directory_and_load<T: DeserializeOwned>(
        &self,
        dir: &Path,
        namespace: &str,
        base_dir: &Path,
        result: &mut HashMap<ResourceLocation, T>,
    ) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                self.scan_directory_and_load(&path, namespace, base_dir, result)?;
            } else if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                let rel = path
                    .relative_to(base_dir)
                    .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e.to_string()))?;
                let rel_str = rel.with_suffix("").to_string_lossy().replace("\\", "/");

                let loc_str = format!("{}:{}", namespace, rel_str);
                if let Some(loc) = ResourceLocation::try_from_string(&loc_str) {
                    if let Ok(content) = fs::read_to_string(&path) {
                        if let Ok(res) = serde_json::from_str::<T>(&content) {
                            result.insert(loc, res);
                        } else {
                            println!("Warning: failed to parse json at {:?}", path);
                        }
                    }
                }
            }
        }
        Ok(())
    }
}

trait RelativePath {
    fn relative_to(&self, base: &Path) -> Result<PathBuf, String>;
}

impl RelativePath for Path {
    fn relative_to(&self, base: &Path) -> Result<PathBuf, String> {
        let self_abs = self.canonicalize().map_err(|e| e.to_string())?;
        let base_abs = base.canonicalize().map_err(|e| e.to_string())?;
        self_abs
            .strip_prefix(&base_abs)
            .map(|p| p.to_path_buf())
            .map_err(|e| e.to_string())
    }
}

trait PathSuffix {
    fn with_suffix(&self, suffix: &str) -> PathBuf;
}

impl PathSuffix for Path {
    fn with_suffix(&self, suffix: &str) -> PathBuf {
        if suffix.is_empty() {
            self.with_extension("")
        } else {
            self.with_extension(suffix)
        }
    }
}
