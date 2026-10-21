#![forbid(unsafe_code)]

pub mod catalog;
pub mod data;
mod definition;
mod id;
mod registry;
pub mod scene;
pub mod scene_types;
pub mod spec;
mod state_schema;
pub mod tooltip;

pub use state_schema::{
    MAX_PROPERTY_NAME_BYTES, MAX_PROPERTY_VALUES, MAX_STATE_PROPERTIES, MAX_STATES_PER_BLOCK,
    StateProperty, StateSchema,
};

pub use definition::{Definition, MAX_DEFINITION_BYTES, MAX_DEPTH};
pub use id::{ContentId, MAX_NAMESPACE_LEN, MAX_PATH_LEN, MAX_TOTAL_LEN};
pub use registry::{
    ContentHandle, Entry, FrozenRegistry, MAX_ENTRIES, MAX_OWNER_ENTRIES, MAX_TOTAL_BYTES,
    RegistryBuilder,
};
