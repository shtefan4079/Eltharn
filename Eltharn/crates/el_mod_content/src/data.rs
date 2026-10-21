use serde::Serialize;

use crate::ContentId;
use crate::catalog::Catalog;
use crate::spec::{Components, ContentSpec, InventoryStack, Kind, Predicate};

const MAX_DEPTH: u32 = 32;
const MAX_NODES: u32 = 4096;
const MAX_SLOTS: usize = 64;

pub fn condition(
    catalog: &Catalog,
    id: &ContentId,
    components: &Components,
) -> Result<bool, String> {
    catalog.validate_components(components)?;
    let mut budget = MAX_NODES;
    eval(catalog, id, components, 0, &mut budget)
}

fn eval(
    catalog: &Catalog,
    id: &ContentId,
    components: &Components,
    depth: u32,
    budget: &mut u32,
) -> Result<bool, String> {
    if depth > MAX_DEPTH {
        return Err("condition depth exceeded".to_string());
    }
    if *budget == 0 {
        return Err("condition budget exceeded".to_string());
    }
    *budget -= 1;
    let pred = match catalog.get(Kind::Condition, id)? {
        ContentSpec::Condition { predicate, .. } => predicate,
        _ => return Err("not a condition".to_string()),
    };
    match pred {
        Predicate::Always => Ok(true),
        Predicate::ComponentEquals {
            component,
            field,
            value,
        } => {
            catalog.get(Kind::Component, component)?;
            Ok(components
                .get(component)
                .and_then(|m| m.get(field))
                .map(|v| v == value)
                .unwrap_or(false))
        }
        Predicate::InTag { tag, content } => Ok(catalog.tag_members(tag)?.contains(content)),
        Predicate::All { conditions } => {
            for c in conditions {
                if !eval(catalog, c, components, depth + 1, budget)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Predicate::Any { conditions } => {
            for c in conditions {
                if eval(catalog, c, components, depth + 1, budget)? {
                    return Ok(true);
                }
            }
            Ok(false)
        }
        Predicate::Not { condition } => {
            Ok(!eval(catalog, condition, components, depth + 1, budget)?)
        }
    }
}

fn max_for(catalog: &Catalog, item: &ContentId) -> Result<u16, String> {
    match catalog.get(Kind::Item, item)? {
        ContentSpec::Item { max_stack, .. } => Ok(*max_stack),
        _ => Err("not an item".to_string()),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Inventory {
    slots: Vec<Option<InventoryStack>>,
    #[serde(skip)]
    fingerprint: Option<String>,
}

impl Inventory {
    fn check_catalog(&self, catalog: &Catalog) -> Result<(), String> {
        if self
            .fingerprint
            .as_deref()
            .is_some_and(|fingerprint| fingerprint != catalog.fingerprint())
        {
            return Err("inventory belongs to another catalog snapshot".into());
        }
        Ok(())
    }
    pub fn new(size: usize) -> Result<Self, String> {
        if size < 1 || size > MAX_SLOTS {
            return Err("bad inventory size".to_string());
        }
        Ok(Self {
            slots: vec![None; size],
            fingerprint: None,
        })
    }
    pub fn from_slots(
        catalog: &Catalog,
        slots: Vec<Option<InventoryStack>>,
    ) -> Result<Self, String> {
        if slots.len() < 1 || slots.len() > MAX_SLOTS {
            return Err("bad slot count".to_string());
        }
        for s in slots.iter().flatten() {
            catalog.validate_stack(s)?;
        }
        Ok(Self {
            slots,
            fingerprint: Some(catalog.fingerprint().to_owned()),
        })
    }
    pub fn slots(&self) -> &[Option<InventoryStack>] {
        &self.slots
    }
    pub fn set(
        &mut self,
        catalog: &Catalog,
        slot: usize,
        stack: Option<InventoryStack>,
    ) -> Result<(), String> {
        self.check_catalog(catalog)?;
        if slot >= self.slots.len() {
            return Err("slot out of bounds".to_string());
        }
        if let Some(s) = &stack {
            catalog.validate_stack(s)?;
        }
        self.slots[slot] = stack;
        self.fingerprint = Some(catalog.fingerprint().to_owned());
        Ok(())
    }
    pub fn transfer(
        &mut self,
        catalog: &Catalog,
        from: usize,
        to: usize,
        count: u16,
    ) -> Result<(), String> {
        self.check_catalog(catalog)?;
        let len = self.slots.len();
        if from >= len || to >= len {
            return Err("slot out of bounds".to_string());
        }
        if from == to {
            return Err("self transfer".to_string());
        }
        if count == 0 {
            return Err("zero count".to_string());
        }
        let src = self.slots[from].clone().ok_or("empty source".to_string())?;
        if src.count < count {
            return Err("insufficient source".to_string());
        }
        catalog.validate_stack(&src)?;
        let max = max_for(catalog, &src.item)?;
        let dst = self.slots[to].clone();
        let mut next = self.slots.clone();
        match dst {
            None => {
                let left = src.count - count;
                next[from] = if left == 0 {
                    None
                } else {
                    Some(InventoryStack {
                        item: src.item.clone(),
                        count: left,
                    })
                };
                next[to] = Some(InventoryStack {
                    item: src.item,
                    count,
                });
            }
            Some(d) => {
                catalog.validate_stack(&d)?;
                if d.item != src.item {
                    return Err("item mismatch".to_string());
                }
                let merged = (d.count as u32) + (count as u32);
                if merged > max as u32 {
                    return Err("merge over max".to_string());
                }
                let left = src.count - count;
                next[from] = if left == 0 {
                    None
                } else {
                    Some(InventoryStack {
                        item: src.item.clone(),
                        count: left,
                    })
                };
                next[to] = Some(InventoryStack {
                    item: src.item,
                    count: merged as u16,
                });
            }
        }
        self.slots = next;
        self.fingerprint = Some(catalog.fingerprint().to_owned());
        Ok(())
    }
    pub fn insert(&mut self, catalog: &Catalog, stack: &InventoryStack) -> Result<(), String> {
        self.check_catalog(catalog)?;
        catalog.validate_stack(stack)?;
        let max = max_for(catalog, &stack.item)?;
        let mut rem = stack.count;
        let mut next = self.slots.clone();
        for slot in next.iter_mut() {
            if rem == 0 {
                break;
            }
            if let Some(s) = slot {
                if s.item == stack.item && s.count < max {
                    let take = rem.min(max - s.count);
                    s.count = s.count.checked_add(take).ok_or("overflow".to_string())?;
                    rem -= take;
                }
            }
        }
        for slot in next.iter_mut() {
            if rem == 0 {
                break;
            }
            if slot.is_none() {
                let take = rem.min(max);
                *slot = Some(InventoryStack {
                    item: stack.item.clone(),
                    count: take,
                });
                rem -= take;
            }
        }
        if rem != 0 {
            return Err("no capacity".to_string());
        }
        self.slots = next;
        self.fingerprint = Some(catalog.fingerprint().to_owned());
        Ok(())
    }
    pub fn remove(
        &mut self,
        catalog: &Catalog,
        item: &ContentId,
        count: u32,
    ) -> Result<(), String> {
        self.check_catalog(catalog)?;
        if count == 0 {
            return Err("zero count".to_string());
        }
        max_for(catalog, item)?;
        for s in self.slots.iter().flatten() {
            catalog.validate_stack(s)?;
        }
        let mut total: u32 = 0;
        for s in self.slots.iter().flatten() {
            if &s.item == item {
                total = total
                    .checked_add(s.count as u32)
                    .ok_or("overflow".to_string())?;
            }
        }
        if total < count {
            return Err("insufficient".to_string());
        }
        let mut rem = count;
        let mut next = self.slots.clone();
        for slot in next.iter_mut() {
            if rem == 0 {
                break;
            }
            if let Some(s) = slot {
                if &s.item == item {
                    let take = (s.count as u32).min(rem);
                    let left = (s.count as u32) - take;
                    *slot = if left == 0 {
                        None
                    } else {
                        Some(InventoryStack {
                            item: s.item.clone(),
                            count: left as u16,
                        })
                    };
                    rem -= take;
                }
            }
        }
        self.slots = next;
        self.fingerprint = Some(catalog.fingerprint().to_owned());
        Ok(())
    }
}

pub fn craft(
    catalog: &Catalog,
    recipe: &ContentId,
    inventory: &mut Inventory,
    components: &Components,
) -> Result<InventoryStack, String> {
    catalog.validate_components(components)?;
    let (inputs, output, cond) = match catalog.get(Kind::Recipe, recipe)? {
        ContentSpec::Recipe {
            inputs,
            output,
            condition,
            ..
        } => (inputs, output, condition),
        _ => return Err("not a recipe".to_string()),
    };
    if let Some(c) = cond {
        if !condition(catalog, c, components)? {
            return Err("condition rejected".to_string());
        }
    }
    let mut staged = inventory.clone();
    for inp in inputs {
        staged.remove(catalog, &inp.item, inp.count as u32)?;
    }
    staged.insert(catalog, output)?;
    *inventory = staged;
    Ok(output.clone())
}

pub fn loot(
    catalog: &Catalog,
    table: &ContentId,
    roll: u64,
    components: &Components,
) -> Result<Option<InventoryStack>, String> {
    catalog.validate_components(components)?;
    let (entries, cond) = match catalog.get(Kind::Loot, table)? {
        ContentSpec::Loot {
            entries, condition, ..
        } => (entries, condition),
        _ => return Err("not a loot table".to_string()),
    };
    if let Some(c) = cond {
        if !condition(catalog, c, components)? {
            return Ok(None);
        }
    }
    if entries.is_empty() {
        return Err("empty loot".to_string());
    }
    let mut sum: u64 = 0;
    for e in entries {
        sum = sum
            .checked_add(e.weight as u64)
            .ok_or("weight overflow".to_string())?;
    }
    let pick = roll.checked_rem(sum).ok_or("zero weight sum".to_string())?;
    let mut acc: u64 = 0;
    for e in entries {
        acc = acc
            .checked_add(e.weight as u64)
            .ok_or("weight overflow".to_string())?;
        if pick < acc {
            return Ok(Some(e.stack.clone()));
        }
    }
    Err("unreachable".to_string())
}
