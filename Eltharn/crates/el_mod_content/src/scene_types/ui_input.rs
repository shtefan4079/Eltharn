use super::{InputEvent, UiView, Widget};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_UI_REVISION: u64 = 9_007_199_254_740_991;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiTargetRevisions {
    pub screen: u64,
    pub widgets: BTreeMap<u32, u64>,
}

fn interactive(widget: &Widget) -> bool {
    matches!(widget, Widget::Button { .. } | Widget::Slot { .. })
}
fn same_target(left: &Widget, right: &Widget) -> bool {
    match (left, right) {
        (
            Widget::Button {
                id: a, rect: ar, ..
            },
            Widget::Button {
                id: b, rect: br, ..
            },
        ) => a == b && ar == br,
        (
            Widget::Slot {
                id: a,
                rect: ar,
                inventory: ai,
                slot: aslot,
            },
            Widget::Slot {
                id: b,
                rect: br,
                inventory: bi,
                slot: bslot,
            },
        ) => a == b && ar == br && ai == bi && aslot == bslot,
        _ => false,
    }
}
fn overlap(left: &Widget, right: &Widget) -> bool {
    let [lx, ly, lw, lh] = left.rect();
    let [rx, ry, rw, rh] = right.rect();
    lx < rx + rw && rx < lx + lw && ly < ry + rh && ry < ly + lh
}
fn occluders<'a>(ui: &'a UiView, target: &Widget) -> Vec<&'a Widget> {
    let index = ui
        .widgets
        .iter()
        .position(|widget| widget.id() == target.id())
        .unwrap_or(ui.widgets.len());
    ui.widgets
        .iter()
        .skip(index.saturating_add(1))
        .filter(|widget| interactive(widget) && overlap(widget, target))
        .collect()
}
fn same_occlusion(previous: &UiView, view: &UiView, target: &Widget) -> bool {
    let old = occluders(previous, target);
    let new = occluders(view, target);
    old.len() == new.len() && old.iter().zip(new).all(|(old, new)| same_target(old, new))
}
fn next(clock: &mut u64) -> Result<u64, String> {
    *clock = clock
        .checked_add(1)
        .filter(|v| *v <= MAX_UI_REVISION)
        .ok_or("World UI revision exhausted")?;
    Ok(*clock)
}

impl UiTargetRevisions {
    pub fn validate(&self) -> Result<(), String> {
        if self.screen > MAX_UI_REVISION
            || self.widgets.len() > 128
            || self
                .widgets
                .iter()
                .any(|(id, revision)| *id == 0 || *revision == 0 || *revision > MAX_UI_REVISION)
        {
            return Err("Invalid world UI revisions".into());
        }
        Ok(())
    }
    pub fn updated(
        &self,
        previous: &UiView,
        view: &UiView,
        clock: &mut u64,
    ) -> Result<Self, String> {
        let mut staged_clock = *clock;
        let mut result = Self {
            screen: self.screen,
            widgets: BTreeMap::new(),
        };
        if previous.screen != view.screen {
            result.screen = next(&mut staged_clock)?;
        }
        for widget in view.widgets.iter().filter(|w| interactive(w)) {
            let old = previous.widgets.iter().find(|old| old.id() == widget.id());
            let revision = old
                .filter(|old| same_target(old, widget) && same_occlusion(previous, view, widget))
                .and_then(|_| self.widgets.get(&widget.id()).copied());
            result.widgets.insert(
                widget.id(),
                match revision {
                    Some(revision) => revision,
                    None => next(&mut staged_clock)?,
                },
            );
        }
        result.validate()?;
        *clock = staged_clock;
        Ok(result)
    }
    pub fn matches_view(&self, view: &UiView) -> bool {
        self.widgets.len() == view.widgets.iter().filter(|w| interactive(w)).count()
            && view
                .widgets
                .iter()
                .filter(|w| interactive(w))
                .all(|w| self.widgets.contains_key(&w.id()))
            && (!view.screen || self.screen != 0)
    }
    pub fn unchanged_event(&self, previous: &Self, event: &InputEvent) -> bool {
        if let Some(id) = event.widget {
            self.screen == previous.screen
                && self.widgets.get(&id).is_some()
                && self.widgets.get(&id) == previous.widgets.get(&id)
        } else if event.kind == "close" {
            self.screen != 0 && self.screen == previous.screen
        } else {
            true
        }
    }
    pub fn capture(&self, epoch: u64, events: &[InputEvent]) -> UiInputContext {
        UiInputContext {
            epoch,
            screen: self.screen,
            widgets: events
                .iter()
                .filter_map(|event| {
                    let id = event.widget?;
                    self.widgets
                        .get(&id)
                        .copied()
                        .map(|revision| (id, revision))
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UiInputContext {
    pub epoch: u64,
    pub screen: u64,
    pub widgets: BTreeMap<u32, u64>,
}
impl UiInputContext {
    pub fn validate(&self) -> Result<(), String> {
        if self.epoch == 0 || self.widgets.len() > 64 {
            return Err("Invalid captured world UI context".into());
        }
        UiTargetRevisions {
            screen: self.screen,
            widgets: self.widgets.clone(),
        }
        .validate()
    }
    pub fn permits(&self, epoch: u64, current: &UiTargetRevisions, event: &InputEvent) -> bool {
        if let Some(id) = event.widget {
            self.epoch == epoch
                && self.screen == current.screen
                && self.widgets.get(&id).is_some()
                && self.widgets.get(&id) == current.widgets.get(&id)
        } else if event.kind == "close" {
            self.epoch == epoch && self.screen != 0 && self.screen == current.screen
        } else {
            true
        }
    }
}
