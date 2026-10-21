use serde::Deserialize;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PackMetadata {
    pub pack: PackInfo,
}
#[derive(Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct PackInfo {
    pub pack_format: i32,
    pub description: String,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataPack {
    pub name: String,
    pub data_path: PathBuf,
    pub metadata: Option<PackMetadata>,
}
impl DataPack {
    pub fn new(name: String, data_path: PathBuf) -> Self {
        let metadata = data_path.parent().and_then(|parent| {
            let mcmeta_path = parent.join("pack.mcmeta");
            if mcmeta_path.exists() {
                fs::read_to_string(&mcmeta_path)
                    .ok()
                    .and_then(|content| serde_json::from_str::<PackMetadata>(&content).ok())
            } else {
                None
            }
        });
        DataPack {
            name,
            data_path,
            metadata,
        }
    }
}
pub fn discover_packs(root: &Path) -> io::Result<Vec<DataPack>> {
    let mut packs = Vec::new();
    let direct_data = root.join("data");
    if direct_data.exists() && direct_data.is_dir() {
        packs.push(DataPack::new("vanilla".to_string(), direct_data));
    } else if root.ends_with("data") && root.is_dir() {
        packs.push(DataPack::new("vanilla".to_string(), root.to_path_buf()));
    }
    let datapacks_dir = root.join("datapacks");
    if datapacks_dir.exists() && datapacks_dir.is_dir() {
        for entry in fs::read_dir(datapacks_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                let data_sub = path.join("data");
                if data_sub.exists() && data_sub.is_dir() {
                    let name = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();
                    packs.push(DataPack::new(name, data_sub));
                }
            }
        }
    }
    Ok(packs)
}
