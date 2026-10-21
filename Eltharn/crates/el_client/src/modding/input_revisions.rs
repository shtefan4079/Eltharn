use el_mod_content::scene_types::{
    InputEvent, SceneView, UiView,
    ui_input::{UiInputContext, UiTargetRevisions},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(in crate::modding) struct UiCapture {
    epoch: u64,
    clock: u64,
    current: BTreeMap<String, UiTargetRevisions>,
}
impl UiCapture {
    pub(in crate::modding) fn reconcile(
        &mut self,
        previous: &SceneView,
        scene: &SceneView,
        local: Option<&BTreeMap<String, UiTargetRevisions>>,
        pending: &mut BTreeMap<String, Vec<InputEvent>>,
    ) -> Result<(), String> {
        let same_epoch = self.epoch == scene.epoch;
        let mut clock = if same_epoch { self.clock } else { 0 };
        if let Some(local) = local {
            for revisions in local.values() {
                clock = clock.max(revisions.screen);
                for revision in revisions.widgets.values() {
                    clock = clock.max(*revision);
                }
            }
        }
        let mut current = BTreeMap::new();
        for actor in &scene.actors {
            let revisions = if let Some(revisions) = local.and_then(|map| map.get(&actor.owner)) {
                revisions.clone()
            } else {
                let old = if same_epoch {
                    previous
                        .actors
                        .iter()
                        .find(|old| old.owner == actor.owner)
                        .map(|old| &old.ui)
                } else {
                    None
                };
                let prior = if same_epoch {
                    self.current.get(&actor.owner)
                } else {
                    None
                };
                prior.cloned().unwrap_or_default().updated(
                    old.unwrap_or(&UiView::default()),
                    &actor.ui,
                    &mut clock,
                )?
            };
            revisions.validate()?;
            if !revisions.matches_view(&actor.ui) {
                return Err("World UI revision targets differ from view".into());
            }
            current.insert(actor.owner.clone(), revisions);
        }
        pending.retain(|owner, events| {
            events.retain(|event| {
                if event.widget.is_none() && event.kind != "close" {
                    return true;
                }
                same_epoch
                    && self
                        .current
                        .get(owner)
                        .zip(current.get(owner))
                        .is_some_and(|(old, new)| new.unchanged_event(old, event))
            });
            !events.is_empty()
        });
        super::invalidate_ui(previous, scene, &self.current, &current);
        self.current = current;
        self.clock = clock;
        self.epoch = scene.epoch;
        Ok(())
    }
    pub(in crate::modding) fn context(
        &self,
        owner: &str,
        events: &[InputEvent],
    ) -> Option<UiInputContext> {
        (self.epoch != 0).then(|| {
            self.current
                .get(owner)
                .cloned()
                .unwrap_or_default()
                .capture(self.epoch, events)
        })
    }
}

pub(in crate::modding) fn report_revisions(
    report: &serde_json::Value,
    scene: &SceneView,
) -> Result<Option<BTreeMap<String, UiTargetRevisions>>, String> {
    let Some(value) = report
        .get("content")
        .and_then(|content| content.get("ui_revisions"))
    else {
        return Ok(None);
    };
    let revisions: BTreeMap<String, UiTargetRevisions> =
        serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
    if revisions.len() != scene.actors.len()
        || scene.actors.iter().any(|actor| {
            revisions
                .get(&actor.owner)
                .is_none_or(|r| r.validate().is_err() || !r.matches_view(&actor.ui))
        })
    {
        return Err("Invalid local world UI revision report".into());
    }
    Ok(Some(revisions))
}
