use super::ClientSession;
use el_protocol_backend::{BackendMapDecoration, BackendMapPatch};
impl ClientSession {
    pub(crate) fn apply_map_update(
        &mut self,
        map_id: u32,
        scale: u8,
        locked: bool,
        tracking_position: bool,
        patch: Option<&BackendMapPatch>,
        decorations: Option<&[BackendMapDecoration]>,
    ) -> bool {
        if let Some(patch) = patch {
            let width = usize::from(patch.width);
            let height = usize::from(patch.height);
            if width == 0
                || height == 0
                || usize::from(patch.start_x) + width > 128
                || usize::from(patch.start_y) + height > 128
                || patch.colors.len() != width * height
            {
                return false;
            }
        }
        let map = self
            .maps
            .entry(map_id)
            .or_insert_with(|| crate::render::map::MapData {
                scale,
                locked,
                tracking_position,
                ..Default::default()
            });
        if let Some(patch) = patch {
            let width = usize::from(patch.width);
            for y in 0..usize::from(patch.height) {
                let dest = usize::from(patch.start_x) + (usize::from(patch.start_y) + y) * 128;
                map.colors[dest..dest + width]
                    .copy_from_slice(&patch.colors[y * width..(y + 1) * width]);
            }
        }
        if let Some(decorations) = decorations {
            map.decorations = decorations.to_vec();
        }
        map.revision = map.revision.wrapping_add(1);
        true
    }
}
