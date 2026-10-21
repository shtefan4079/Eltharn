pub const MAX_URGENT_UPLOADS_PER_FRAME: usize = 2;
pub const TOTAL_UPLOAD_HEADROOM_OVER_NORMAL: usize = 4;
pub const WASM_MAX_TOTAL_UPLOADS: usize = 2;
pub fn max_total_uploads(is_wasm: bool, uploads_per_frame: usize) -> usize {
    if is_wasm {
        WASM_MAX_TOTAL_UPLOADS
    } else {
        uploads_per_frame + TOTAL_UPLOAD_HEADROOM_OVER_NORMAL
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UploadLane {
    Urgent,
    Normal,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UploadBudget {
    urgent_uploads: usize,
    normal_uploads: usize,
    normal_indices: usize,
    total_uploads: usize,
    max_urgent_uploads: usize,
    max_normal_uploads: usize,
    max_normal_indices: usize,
    max_total_uploads: usize,
}
impl UploadBudget {
    pub fn new(
        max_total_uploads: usize,
        max_normal_uploads: usize,
        max_normal_indices: usize,
    ) -> Self {
        Self {
            urgent_uploads: 0,
            normal_uploads: 0,
            normal_indices: 0,
            total_uploads: 0,
            max_urgent_uploads: MAX_URGENT_UPLOADS_PER_FRAME,
            max_normal_uploads,
            max_normal_indices,
            max_total_uploads,
        }
    }
    pub fn try_take(&mut self, lane: UploadLane, will_upload: bool, index_count: usize) -> bool {
        if !will_upload {
            return true;
        }
        if self.total_uploads >= self.max_total_uploads {
            return false;
        }
        match lane {
            UploadLane::Urgent => {
                if self.urgent_uploads >= self.max_urgent_uploads {
                    return false;
                }
                self.urgent_uploads += 1;
            }
            UploadLane::Normal => {
                if self.normal_uploads >= self.max_normal_uploads
                    || self.normal_indices >= self.max_normal_indices
                {
                    return false;
                }
                self.normal_uploads += 1;
                self.normal_indices += index_count;
            }
        }
        self.total_uploads += 1;
        true
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StagedUpload<A: Copy> {
    Published {
        live: A,
        release_after_publish: Option<A>,
    },
    KeepOld {
        live: Option<A>,
    },
}
pub fn resolve_staged_upload<A: Copy>(
    previous: Option<A>,
    staged: Result<A, ()>,
) -> StagedUpload<A> {
    match staged {
        Ok(live) => StagedUpload::Published {
            live,
            release_after_publish: previous,
        },
        Err(()) => StagedUpload::KeepOld { live: previous },
    }
}
