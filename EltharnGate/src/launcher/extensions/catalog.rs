use el_extension_manager::package::PackageInfo;
use el_launcher_core::extensions::ExtensionProfile;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PackageKind {
    Mod,
    Shaderpack,
}
impl PackageKind {
    pub fn of(info: &PackageInfo) -> Option<Self> {
        if info.kind.eq_ignore_ascii_case("mod") {
            Some(Self::Mod)
        } else if info.kind.eq_ignore_ascii_case("shaderpack") {
            Some(Self::Shaderpack)
        } else {
            None
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PackageStatus {
    NotSelected,
    SelectedDisabled,
    EnabledNextLaunch,
}
impl PackageStatus {
    pub fn selected(self) -> bool {
        self != Self::NotSelected
    }
    pub fn badge(self) -> &'static str {
        match self {
            Self::NotSelected => "Installed",
            Self::SelectedDisabled => "Paused",
            Self::EnabledNextLaunch => "Enabled",
        }
    }
}
pub(super) fn entries(
    catalog: &[PackageInfo],
    kind: PackageKind,
) -> impl Iterator<Item = (usize, &PackageInfo)> {
    catalog
        .iter()
        .enumerate()
        .filter(move |(_, info)| PackageKind::of(info) == Some(kind))
}
pub(super) fn filtered<'a>(
    catalog: &'a [PackageInfo],
    kind: PackageKind,
    search: &str,
) -> Vec<(usize, &'a PackageInfo)> {
    let query = search.trim().to_lowercase();
    let mut found: Vec<_> = entries(catalog, kind)
        .filter(|(_, info)| {
            query.is_empty()
                || [
                    &info.name,
                    &info.description,
                    &info.pin.id.to_string(),
                    &info.pin.version.to_string(),
                ]
                .iter()
                .any(|field| field.to_lowercase().contains(&query))
        })
        .collect();
    found.sort_by(|(_, a), (_, b)| {
        a.name
            .to_lowercase()
            .cmp(&b.name.to_lowercase())
            .then_with(|| a.pin.version.to_string().cmp(&b.pin.version.to_string()))
            .then_with(|| {
                a.pin
                    .content_id
                    .to_string()
                    .cmp(&b.pin.content_id.to_string())
            })
    });
    found
}
pub(super) fn status(info: &PackageInfo, profile: Option<&ExtensionProfile>) -> PackageStatus {
    let Some(profile) = profile else {
        return PackageStatus::NotSelected;
    };
    let (selected, enabled) = match PackageKind::of(info) {
        Some(PackageKind::Mod) => (
            profile.mods.iter().any(|pin| *pin == info.pin),
            profile.flags.mods_enabled,
        ),
        Some(PackageKind::Shaderpack) => (
            profile
                .shader
                .as_ref()
                .is_some_and(|selection| selection.package == info.pin),
            profile.flags.shaders_enabled,
        ),
        None => (false, false),
    };
    match (selected, enabled) {
        (false, _) => PackageStatus::NotSelected,
        (true, false) => PackageStatus::SelectedDisabled,
        (true, true) => PackageStatus::EnabledNextLaunch,
    }
}
