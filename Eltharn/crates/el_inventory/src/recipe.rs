use el_core::item::ItemStack;
use el_core::registry::BuiltInRegistries;
use el_core::resource_location::ResourceLocation;
use el_data::pack::DataPack;
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum TagEntry {
    Item(ResourceLocation),
    Tag(ResourceLocation),
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct Ingredient {
    entries: Vec<TagEntry>,
}
impl Ingredient {
    fn matches_stack(
        &self,
        stack: &ItemStack,
        tags: &HashMap<ResourceLocation, Vec<TagEntry>>,
    ) -> bool {
        if stack.is_empty() {
            return false;
        }
        self.matches_location(&stack.item.location, tags)
    }
    fn matches_location(
        &self,
        item: &ResourceLocation,
        tags: &HashMap<ResourceLocation, Vec<TagEntry>>,
    ) -> bool {
        self.entries.iter().any(|entry| match entry {
            TagEntry::Item(candidate) => candidate == item,
            TagEntry::Tag(tag) => tag_contains_item(tags, tag, item, &mut HashSet::new()),
        })
    }
}
#[derive(Clone, Debug, PartialEq)]
enum CraftingRecipe {
    Shaped {
        pattern: Vec<String>,
        key: HashMap<char, Ingredient>,
        result: ItemStack,
    },
    Shapeless {
        ingredients: Vec<Ingredient>,
        result: ItemStack,
    },
    Cooking {
        recipe_type: String,
        ingredient: Ingredient,
        result: ItemStack,
        experience: f32,
        cooking_time: i32,
    },
}
#[derive(Clone, Debug, Default)]
pub struct RecipeManager {
    recipes: Vec<CraftingRecipe>,
    item_tag_entries: HashMap<ResourceLocation, Vec<TagEntry>>,
    pub item_tags: HashMap<ResourceLocation, Vec<ResourceLocation>>,
}
impl RecipeManager {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn load_from_datapack(&mut self, pack: &DataPack) -> io::Result<()> {
        self.load_item_tags(pack).map_err(invalid_data)?;
        self.rebuild_public_item_tags();
        self.load_recipes(pack).map_err(invalid_data)
    }
    pub fn recipe_count(&self) -> usize {
        self.recipes.len()
    }
    pub fn match_recipe(
        &self,
        inputs: &[ItemStack],
        grid_width: usize,
        grid_height: usize,
    ) -> Option<ItemStack> {
        if inputs.len() < grid_width.saturating_mul(grid_height) {
            return None;
        }
        for recipe in &self.recipes {
            match recipe {
                CraftingRecipe::Shaped {
                    pattern,
                    key,
                    result,
                } => {
                    if self.matches_shaped(pattern, key, inputs, grid_width, grid_height) {
                        return Some(result.clone());
                    }
                }
                CraftingRecipe::Shapeless {
                    ingredients,
                    result,
                } => {
                    if self.matches_shapeless(ingredients, inputs, grid_width, grid_height) {
                        return Some(result.clone());
                    }
                }
                CraftingRecipe::Cooking { .. } => {}
            }
        }
        None
    }
    pub fn match_smelting(
        &self,
        input: &ItemStack,
        recipe_type: &str,
    ) -> Option<(ItemStack, f32, i32)> {
        if input.is_empty() {
            return None;
        }
        let requested_type = normalize_recipe_type(recipe_type);
        for recipe in &self.recipes {
            let CraftingRecipe::Cooking {
                recipe_type: candidate_type,
                ingredient,
                result,
                experience,
                cooking_time,
            } = recipe
            else {
                continue;
            };
            if candidate_type == &requested_type
                && ingredient.matches_stack(input, &self.item_tag_entries)
            {
                return Some((result.clone(), *experience, *cooking_time));
            }
        }
        None
    }
    fn load_item_tags(&mut self, pack: &DataPack) -> Result<(), String> {
        let mut files = Vec::new();
        collect_json_files(&pack.data_path, "tags/item", &mut files)?;
        files.sort();
        for file in files {
            let content = fs::read_to_string(&file)
                .map_err(|error| format!("failed to read {}: {}", file.display(), error))?;
            let value: Value = serde_json::from_str(&content)
                .map_err(|error| format!("failed to parse {}: {}", file.display(), error))?;
            let Some(tag_id) = id_from_data_file(&pack.data_path, &file, "tags/item") else {
                continue;
            };
            let Some(values) = value.get("values").and_then(Value::as_array) else {
                continue;
            };
            let replace = value
                .get("replace")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if replace {
                self.item_tag_entries.remove(&tag_id);
            }
            let entries = self.item_tag_entries.entry(tag_id).or_default();
            for entry in values {
                if let Some(parsed) = parse_tag_entry(entry) {
                    entries.push(parsed);
                }
            }
        }
        Ok(())
    }
    fn load_recipes(&mut self, pack: &DataPack) -> Result<(), String> {
        let mut files = Vec::new();
        collect_json_files(&pack.data_path, "recipe", &mut files)?;
        collect_json_files(&pack.data_path, "recipes", &mut files)?;
        files.sort();
        for file in files {
            let content = fs::read_to_string(&file)
                .map_err(|error| format!("failed to read {}: {}", file.display(), error))?;
            let value: Value = serde_json::from_str(&content)
                .map_err(|error| format!("failed to parse {}: {}", file.display(), error))?;
            let Some(recipe_type) = value.get("type").and_then(Value::as_str) else {
                continue;
            };
            let Some(result) = parse_result_stack(value.get("result")) else {
                continue;
            };
            match recipe_type {
                "minecraft:crafting_shaped" | "crafting_shaped" => {
                    if let Some(recipe) = parse_shaped_recipe(&value, result) {
                        self.recipes.push(recipe);
                    }
                }
                "minecraft:crafting_shapeless" | "crafting_shapeless" => {
                    if let Some(recipe) = parse_shapeless_recipe(&value, result) {
                        self.recipes.push(recipe);
                    }
                }
                "minecraft:smelting"
                | "minecraft:blasting"
                | "minecraft:smoking"
                | "minecraft:campfire_cooking"
                | "smelting"
                | "blasting"
                | "smoking"
                | "campfire_cooking" => {
                    if let Some(recipe) = parse_cooking_recipe(recipe_type, &value, result) {
                        self.recipes.push(recipe);
                    }
                }
                _ => {}
            }
        }
        Ok(())
    }
    fn rebuild_public_item_tags(&mut self) {
        self.item_tags.clear();
        let tags: Vec<ResourceLocation> = self.item_tag_entries.keys().cloned().collect();
        for tag in tags {
            let mut seen = HashSet::new();
            collect_flat_tag_items(&self.item_tag_entries, &tag, &mut seen, &mut HashSet::new());
            let mut items: Vec<ResourceLocation> = seen.into_iter().collect();
            items.sort_by_key(|loc| loc.to_string());
            self.item_tags.insert(tag, items);
        }
    }
    fn matches_shaped(
        &self,
        pattern: &[String],
        key: &HashMap<char, Ingredient>,
        inputs: &[ItemStack],
        grid_width: usize,
        grid_height: usize,
    ) -> bool {
        let Some(trimmed) = trim_grid(inputs, grid_width, grid_height) else {
            return false;
        };
        let recipe_height = pattern.len();
        let recipe_width = pattern
            .iter()
            .map(|line| line.chars().count())
            .max()
            .unwrap_or(0);
        if recipe_width != trimmed.width || recipe_height != trimmed.height {
            return false;
        }
        for row in 0..recipe_height {
            let chars: Vec<char> = pattern[row].chars().collect();
            for col in 0..recipe_width {
                let expected = chars.get(col).copied().unwrap_or(' ');
                let input = &trimmed.cells[row * trimmed.width + col];
                if expected == ' ' {
                    if input.is_some() {
                        return false;
                    }
                    continue;
                }
                let Some(ingredient) = key.get(&expected) else {
                    return false;
                };
                let Some(item) = input else {
                    return false;
                };
                if !ingredient.matches_location(item, &self.item_tag_entries) {
                    return false;
                }
            }
        }
        true
    }
    fn matches_shapeless(
        &self,
        ingredients: &[Ingredient],
        inputs: &[ItemStack],
        grid_width: usize,
        grid_height: usize,
    ) -> bool {
        let used_inputs: Vec<&ItemStack> = inputs
            .iter()
            .take(grid_width.saturating_mul(grid_height))
            .filter(|stack| !stack.is_empty())
            .collect();
        if used_inputs.len() != ingredients.len() {
            return false;
        }
        let mut used_ingredients = vec![false; ingredients.len()];
        match_shapeless_backtracking(
            0,
            &used_inputs,
            ingredients,
            &mut used_ingredients,
            &self.item_tag_entries,
        )
    }
}
fn parse_shaped_recipe(value: &Value, result: ItemStack) -> Option<CraftingRecipe> {
    let pattern: Vec<String> = value
        .get("pattern")?
        .as_array()?
        .iter()
        .filter_map(Value::as_str)
        .map(ToOwned::to_owned)
        .collect();
    if pattern.is_empty() {
        return None;
    }
    let key_obj = value.get("key")?.as_object()?;
    let mut key = HashMap::new();
    for (name, ingredient_value) in key_obj {
        let mut chars = name.chars();
        let Some(ch) = chars.next() else {
            continue;
        };
        if let Some(ingredient) = parse_ingredient(ingredient_value) {
            key.insert(ch, ingredient);
        }
    }
    Some(CraftingRecipe::Shaped {
        pattern,
        key,
        result,
    })
}
fn parse_shapeless_recipe(value: &Value, result: ItemStack) -> Option<CraftingRecipe> {
    let ingredients = value
        .get("ingredients")?
        .as_array()?
        .iter()
        .filter_map(parse_ingredient)
        .collect::<Vec<_>>();
    Some(CraftingRecipe::Shapeless {
        ingredients,
        result,
    })
}
fn parse_cooking_recipe(
    recipe_type: &str,
    value: &Value,
    result: ItemStack,
) -> Option<CraftingRecipe> {
    let ingredient = parse_ingredient(value.get("ingredient")?)?;
    let experience = value
        .get("experience")
        .and_then(Value::as_f64)
        .unwrap_or(0.0) as f32;
    let cooking_time = value
        .get("cookingtime")
        .or_else(|| value.get("cooking_time"))
        .and_then(Value::as_i64)
        .map(|time| time as i32)
        .unwrap_or_else(|| default_cooking_time(recipe_type));
    Some(CraftingRecipe::Cooking {
        recipe_type: normalize_recipe_type(recipe_type),
        ingredient,
        result,
        experience,
        cooking_time,
    })
}
fn normalize_recipe_type(recipe_type: &str) -> String {
    if recipe_type.contains(':') {
        recipe_type.to_string()
    } else {
        format!("minecraft:{}", recipe_type)
    }
}
fn default_cooking_time(recipe_type: &str) -> i32 {
    match normalize_recipe_type(recipe_type).as_str() {
        "minecraft:blasting" | "minecraft:smoking" | "minecraft:campfire_cooking" => 100,
        _ => 200,
    }
}
fn parse_result_stack(value: Option<&Value>) -> Option<ItemStack> {
    let value = value?;
    let (id, count) = if let Some(id) = value.as_str() {
        (id, 1)
    } else {
        let obj = value.as_object()?;
        let id = obj.get("id").or_else(|| obj.get("item"))?.as_str()?;
        let count = obj.get("count").and_then(Value::as_i64).unwrap_or(1) as i32;
        (id, count)
    };
    let loc = ResourceLocation::try_from_string(id)?;
    let item = BuiltInRegistries::items().get_by_location(&loc).clone();
    Some(ItemStack::new(item, count))
}
fn parse_ingredient(value: &Value) -> Option<Ingredient> {
    let mut entries = Vec::new();
    collect_ingredient_entries(value, &mut entries);
    if entries.is_empty() {
        None
    } else {
        Some(Ingredient { entries })
    }
}
fn collect_ingredient_entries(value: &Value, entries: &mut Vec<TagEntry>) {
    if let Some(s) = value.as_str() {
        if let Some(entry) = parse_entry_string(s) {
            entries.push(entry);
        }
        return;
    }
    if let Some(array) = value.as_array() {
        for entry in array {
            collect_ingredient_entries(entry, entries);
        }
        return;
    }
    let Some(obj) = value.as_object() else {
        return;
    };
    if let Some(item) = obj.get("item").and_then(Value::as_str) {
        if let Some(loc) = ResourceLocation::try_from_string(item) {
            entries.push(TagEntry::Item(loc));
        }
    }
    if let Some(tag) = obj.get("tag").and_then(Value::as_str) {
        let tag = tag.strip_prefix('#').unwrap_or(tag);
        if let Some(loc) = ResourceLocation::try_from_string(tag) {
            entries.push(TagEntry::Tag(loc));
        }
    }
    if let Some(items) = obj.get("items") {
        collect_ingredient_entries(items, entries);
    }
}
fn parse_tag_entry(value: &Value) -> Option<TagEntry> {
    if let Some(s) = value.as_str() {
        return parse_entry_string(s);
    }
    let obj = value.as_object()?;
    let id = obj
        .get("id")
        .or_else(|| obj.get("item"))
        .or_else(|| obj.get("tag"))?
        .as_str()?;
    parse_entry_string(id)
}
fn parse_entry_string(s: &str) -> Option<TagEntry> {
    if let Some(tag) = s.strip_prefix('#') {
        ResourceLocation::try_from_string(tag).map(TagEntry::Tag)
    } else {
        ResourceLocation::try_from_string(s).map(TagEntry::Item)
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct TrimmedGrid {
    width: usize,
    height: usize,
    cells: Vec<Option<ResourceLocation>>,
}
fn trim_grid(inputs: &[ItemStack], width: usize, height: usize) -> Option<TrimmedGrid> {
    let mut min_x = width;
    let mut min_y = height;
    let mut max_x = 0usize;
    let mut max_y = 0usize;
    let mut found = false;
    for y in 0..height {
        for x in 0..width {
            let idx = y * width + x;
            let Some(stack) = inputs.get(idx) else {
                continue;
            };
            if stack.is_empty() {
                continue;
            }
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !found {
        return None;
    }
    let trimmed_width = max_x - min_x + 1;
    let trimmed_height = max_y - min_y + 1;
    let mut cells = Vec::with_capacity(trimmed_width * trimmed_height);
    for y in min_y..=max_y {
        for x in min_x..=max_x {
            let idx = y * width + x;
            let stack = &inputs[idx];
            if stack.is_empty() {
                cells.push(None);
            } else {
                cells.push(Some(stack.item.location.clone()));
            }
        }
    }
    Some(TrimmedGrid {
        width: trimmed_width,
        height: trimmed_height,
        cells,
    })
}
fn match_shapeless_backtracking(
    input_idx: usize,
    inputs: &[&ItemStack],
    ingredients: &[Ingredient],
    used_ingredients: &mut [bool],
    tags: &HashMap<ResourceLocation, Vec<TagEntry>>,
) -> bool {
    if input_idx >= inputs.len() {
        return true;
    }
    for ingredient_idx in 0..ingredients.len() {
        if used_ingredients[ingredient_idx] {
            continue;
        }
        if ingredients[ingredient_idx].matches_stack(inputs[input_idx], tags) {
            used_ingredients[ingredient_idx] = true;
            if match_shapeless_backtracking(
                input_idx + 1,
                inputs,
                ingredients,
                used_ingredients,
                tags,
            ) {
                return true;
            }
            used_ingredients[ingredient_idx] = false;
        }
    }
    false
}
fn tag_contains_item(
    tags: &HashMap<ResourceLocation, Vec<TagEntry>>,
    tag: &ResourceLocation,
    item: &ResourceLocation,
    visiting: &mut HashSet<ResourceLocation>,
) -> bool {
    if !visiting.insert(tag.clone()) {
        return false;
    }
    let Some(entries) = tags.get(tag) else {
        visiting.remove(tag);
        return false;
    };
    for entry in entries {
        match entry {
            TagEntry::Item(candidate) if candidate == item => {
                visiting.remove(tag);
                return true;
            }
            TagEntry::Tag(child) if tag_contains_item(tags, child, item, visiting) => {
                visiting.remove(tag);
                return true;
            }
            _ => {}
        }
    }
    visiting.remove(tag);
    false
}
fn collect_flat_tag_items(
    tags: &HashMap<ResourceLocation, Vec<TagEntry>>,
    tag: &ResourceLocation,
    output: &mut HashSet<ResourceLocation>,
    visiting: &mut HashSet<ResourceLocation>,
) {
    if !visiting.insert(tag.clone()) {
        return;
    }
    if let Some(entries) = tags.get(tag) {
        for entry in entries {
            match entry {
                TagEntry::Item(item) => {
                    output.insert(item.clone());
                }
                TagEntry::Tag(child) => {
                    collect_flat_tag_items(tags, child, output, visiting);
                }
            }
        }
    }
    visiting.remove(tag);
}
fn invalid_data(message: String) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, message)
}
fn collect_json_files(
    root: &Path,
    relative_dir: &str,
    output: &mut Vec<PathBuf>,
) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    for namespace in fs::read_dir(root)
        .map_err(|error| format!("failed to list {}: {}", root.display(), error))?
    {
        let namespace =
            namespace.map_err(|error| format!("failed to read {}: {}", root.display(), error))?;
        let dir = namespace.path().join(relative_dir);
        if dir.is_dir() {
            collect_json_files_recursive(&dir, output)?;
        }
    }
    Ok(())
}
fn collect_json_files_recursive(dir: &Path, output: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in
        fs::read_dir(dir).map_err(|error| format!("failed to list {}: {}", dir.display(), error))?
    {
        let entry =
            entry.map_err(|error| format!("failed to read {}: {}", dir.display(), error))?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files_recursive(&path, output)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("json") {
            output.push(path);
        }
    }
    Ok(())
}
fn id_from_data_file(root: &Path, file: &Path, relative_dir: &str) -> Option<ResourceLocation> {
    let relative = file.strip_prefix(root).ok()?;
    let mut components = relative.components();
    let namespace = components.next()?.as_os_str().to_str()?;
    let relative_prefix = Path::new(relative_dir);
    let file_in_namespace = relative.strip_prefix(Path::new(namespace)).ok()?;
    let path = file_in_namespace.strip_prefix(relative_prefix).ok()?;
    let mut path = path.to_str()?.replace('\\', "/");
    path = path.strip_suffix(".json").unwrap_or(&path).to_string();
    ResourceLocation::try_new(namespace, &path)
}
