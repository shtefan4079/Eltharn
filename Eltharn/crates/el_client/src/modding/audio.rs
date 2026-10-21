use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use crate::client::audio::{AudioEngine, PlayResult, SoundHandle, SoundPlayback};
use crate::client::sound_engine::{
    SoundSource, SoundSourceVolumes, SourceGains, attenuation_distance, calculate_pitch,
    calculate_volume,
};
use crate::resources::manager::ClientResourceManager;

const MAX_TRACKED: usize = 64;
const MOD_ATTENUATION_DISTANCE: i32 = 16;

struct State {
    epoch: u64,
    last_sequence: u64,
    handles: Vec<(String, SoundHandle)>,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State {
        epoch: 0,
        last_sequence: 0,
        handles: Vec::new(),
    });
}

fn take_handles() -> Vec<(String, SoundHandle)> {
    STATE.with(|s| std::mem::take(&mut s.borrow_mut().handles))
}

fn replace_handles(handles: Vec<(String, SoundHandle)>) {
    STATE.with(|s| s.borrow_mut().handles = handles);
}

pub fn clear(engine: &mut AudioEngine) {
    let owned = take_handles();
    for (_, handle) in owned {
        engine.stop(handle);
    }
}

pub fn update(
    engine: &mut AudioEngine,
    resources: &ClientResourceManager,
    options: &HashMap<String, String>,
) -> Result<(), String> {
    if !super::client_content::active() {
        let owned = take_handles();
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.epoch = 0;
            st.last_sequence = 0;
        });
        for (_, handle) in owned {
            engine.stop(handle);
        }
        return Ok(());
    }
    let scene = super::client_content::scene();
    let scene_epoch = scene.epoch;
    let epoch_mismatch = STATE.with(|s| s.borrow().epoch != scene_epoch);
    if epoch_mismatch {
        let owned = take_handles();
        for (_, handle) in owned {
            engine.stop(handle);
        }
        STATE.with(|s| {
            let mut st = s.borrow_mut();
            st.epoch = scene_epoch;
            st.last_sequence = 0;
        });
    }
    let live_owners: HashSet<String> = scene.actors.iter().map(|a| a.owner.clone()).collect();
    let snapshot = STATE.with(|s| s.borrow().handles.clone());
    let mut kept = Vec::with_capacity(snapshot.len());
    let mut stale = Vec::new();
    for (owner, handle) in snapshot {
        if !live_owners.contains(&owner) {
            stale.push(handle);
            continue;
        }
        if !engine.is_active(handle) {
            continue;
        }
        kept.push((owner, handle));
    }
    for handle in stale {
        engine.stop(handle);
    }
    replace_handles(kept);
    let last = STATE.with(|s| s.borrow().last_sequence);
    let volumes = SoundSourceVolumes::from_options(options);
    let gains = SourceGains::default();
    let source = SoundSource::Players;
    for cue in scene.sounds.iter().filter(|c| c.sequence > last) {
        super::client_content::require_sound(&cue.sound)?;
        let event = super::material::sound_file(resources, &cue.sound)?;
        let instance_volume = cue.volume_milli as f32 / 1000.0;
        let instance_pitch = cue.pitch_milli as f32 / 1000.0;
        let playback = SoundPlayback {
            source,
            instance_volume,
            category_gain: calculate_volume(instance_volume, source, &volumes, &gains),
            pitch: calculate_pitch(instance_pitch),
            position: Some(glam::Vec3::new(
                cue.position_milli[0] as f32 / 1000.0,
                cue.position_milli[1] as f32 / 1000.0,
                cue.position_milli[2] as f32 / 1000.0,
            )),
            attenuation_max_distance: attenuation_distance(
                instance_volume,
                MOD_ATTENUATION_DISTANCE,
            ),
        };
        match engine.play_sound(resources, &event, playback) {
            PlayResult::Started(handle) => {
                STATE.with(|s| s.borrow_mut().handles.push((cue.owner.clone(), handle)));
            }
            PlayResult::StartedSilently | PlayResult::NotStarted => {}
        }
        STATE.with(|s| s.borrow_mut().last_sequence = cue.sequence);
    }
    let overflow = STATE.with(|s| {
        let mut st = s.borrow_mut();
        let mut overflow = Vec::new();
        while st.handles.len() > MAX_TRACKED {
            overflow.push(st.handles.remove(0).1);
        }
        overflow
    });
    for handle in overflow {
        engine.stop(handle);
    }
    Ok(())
}
