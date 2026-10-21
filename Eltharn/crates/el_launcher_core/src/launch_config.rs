use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum NavLayout {
    #[default]
    TopBar,
    Sidebar,
}
pub fn ordered_protocol_versions(
    builtin_version: &str,
    selected_version: &str,
    installed_versions: impl IntoIterator<Item = String>,
) -> Vec<String> {
    let mut versions = std::collections::BTreeSet::new();
    versions.insert(builtin_version.to_string());
    versions.insert(selected_version.to_string());
    versions.extend(installed_versions);
    let mut ordered = vec![builtin_version.to_string()];
    ordered.extend(
        versions
            .into_iter()
            .filter(|version| version != builtin_version),
    );
    ordered
}
