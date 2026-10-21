use crate::package::Package;
use serde_json::{Value, json};
use std::collections::BTreeMap;
impl Package {
    pub async fn shader_settings(&self, settings: &Value) -> Result<Value, String> {
        let manifest = crate::manifest::strict_json(
            self.files
                .get(el_extension_api::package_path::MANIFEST_FILE_NAME)
                .ok_or("missing manifest")?,
        )?;
        if manifest["shader_api"] == "1.3" {
            let pack = self.shader_v4_with_settings(settings).await?;
            return Ok(json!(
                { "definitions" : pack.param_definitions(), "values" : pack.settings
                }
            ));
        }
        self.shader(settings).await?;
        let descriptor_path = manifest["shader"]["descriptor"]
            .as_str()
            .ok_or("missing descriptor")?;
        let descriptor = crate::manifest::strict_json(
            self.files
                .get(descriptor_path)
                .ok_or("missing descriptor payload")?,
        )?;
        let definitions = legacy_definitions(&descriptor)?;
        let overrides = settings.as_object().ok_or("settings must be an object")?;
        let mut values = serde_json::Map::new();
        for definition in &definitions {
            let id = definition["id"].as_str().ok_or("invalid parameter ID")?;
            values.insert(
                id.into(),
                overrides.get(id).unwrap_or(&definition["default"]).clone(),
            );
        }
        Ok(json!({ "definitions" : definitions, "values" : values }))
    }
}
fn legacy_definitions(descriptor: &Value) -> Result<Vec<Value>, String> {
    let mut definitions: BTreeMap<String, Value> = BTreeMap::new();
    for variant in descriptor["variants"]
        .as_array()
        .ok_or("invalid variants")?
    {
        for definition in variant["params"]
            .as_array()
            .ok_or("invalid variant parameters")?
        {
            let mut definition = definition.clone();
            definition
                .as_object_mut()
                .ok_or("invalid parameter")?
                .remove("index");
            let id = definition["id"]
                .as_str()
                .ok_or("invalid parameter ID")?
                .to_owned();
            if let Some(previous) = definitions.get(&id) {
                if previous != &definition {
                    return Err(format!(
                        "Cannot edit {id}: settings differ between shader variants"
                    ));
                }
            } else {
                definitions.insert(id, definition);
            }
        }
    }
    Ok(definitions.into_values().collect())
}
