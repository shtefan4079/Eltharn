use std::{path::PathBuf, sync::OnceLock};

use el_launcher_core::ProtocolBackendManifest;

static RUNTIME: OnceLock<BundledRuntime> = OnceLock::new();

pub struct BundledRuntime {
    pub executable: PathBuf,
    pub manifest: ProtocolBackendManifest,
}

pub(crate) fn register(runtime: BundledRuntime) {
    assert!(
        RUNTIME.set(runtime).is_ok(),
        "desktop runtime registered twice"
    );
}

pub fn bundled_runtime() -> Option<&'static BundledRuntime> {
    RUNTIME.get()
}
