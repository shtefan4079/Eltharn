use el_core::ResourceLocation;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WidgetSprites {
    pub enabled: &'static str,
    pub disabled: &'static str,
    pub enabled_focused: &'static str,
    pub disabled_focused: &'static str,
}
impl WidgetSprites {
    pub const fn new(
        enabled: &'static str,
        disabled: &'static str,
        enabled_focused: &'static str,
        disabled_focused: &'static str,
    ) -> Self {
        Self {
            enabled,
            disabled,
            enabled_focused,
            disabled_focused,
        }
    }
    pub const fn uniform(sprite: &'static str) -> Self {
        Self::new(sprite, sprite, sprite, sprite)
    }
    pub const fn with_focused(sprite: &'static str, focused: &'static str) -> Self {
        Self::new(sprite, sprite, focused, focused)
    }
    pub const fn three_state(
        enabled: &'static str,
        disabled: &'static str,
        focused: &'static str,
    ) -> Self {
        Self::new(enabled, disabled, focused, disabled)
    }
    pub const fn get(self, enabled: bool, focused: bool) -> &'static str {
        if enabled {
            if focused {
                self.enabled_focused
            } else {
                self.enabled
            }
        } else if focused {
            self.disabled_focused
        } else {
            self.disabled
        }
    }
    pub const fn resolve(self, active: bool, focused: bool) -> &'static str {
        self.get(active, focused)
    }
    pub fn resolve_location(self, active: bool, focused: bool) -> ResourceLocation {
        widget_sprite(self.resolve(active, focused))
    }
    pub const fn paths(self) -> [&'static str; 4] {
        [
            self.enabled,
            self.disabled,
            self.enabled_focused,
            self.disabled_focused,
        ]
    }
}
pub fn widget_sprite(path: &str) -> ResourceLocation {
    ResourceLocation::from_string(path)
}
pub mod sprites {
    use super::WidgetSprites;
    pub const BUTTON: WidgetSprites = WidgetSprites::three_state(
        "minecraft:widget/button",
        "minecraft:widget/button_disabled",
        "minecraft:widget/button_highlighted",
    );
    pub const CHECKBOX: WidgetSprites = WidgetSprites::new(
        "minecraft:widget/checkbox_selected",
        "minecraft:widget/checkbox",
        "minecraft:widget/checkbox_selected_highlighted",
        "minecraft:widget/checkbox_highlighted",
    );
    pub const SLIDER: WidgetSprites = WidgetSprites::new(
        "minecraft:widget/slider",
        "minecraft:widget/slider",
        "minecraft:widget/slider_highlighted",
        "minecraft:widget/slider",
    );
    pub const SLIDER_HANDLE: WidgetSprites = WidgetSprites::new(
        "minecraft:widget/slider_handle",
        "minecraft:widget/slider_handle",
        "minecraft:widget/slider_handle_highlighted",
        "minecraft:widget/slider_handle",
    );
    pub const CYCLE_BUTTON: WidgetSprites = BUTTON;
    pub const TEXT_FIELD: WidgetSprites = WidgetSprites::with_focused(
        "minecraft:widget/text_field",
        "minecraft:widget/text_field_highlighted",
    );
    pub const TAB: WidgetSprites = WidgetSprites::new(
        "minecraft:widget/tab_selected",
        "minecraft:widget/tab",
        "minecraft:widget/tab_selected_highlighted",
        "minecraft:widget/tab_highlighted",
    );
    pub const PAGE_FORWARD: WidgetSprites = WidgetSprites::with_focused(
        "minecraft:widget/page_forward",
        "minecraft:widget/page_forward_highlighted",
    );
    pub const PAGE_BACKWARD: WidgetSprites = WidgetSprites::with_focused(
        "minecraft:widget/page_backward",
        "minecraft:widget/page_backward_highlighted",
    );
    pub const LOCK_BUTTON: WidgetSprites = WidgetSprites::three_state(
        "minecraft:widget/locked_button",
        "minecraft:widget/locked_button_disabled",
        "minecraft:widget/locked_button_highlighted",
    );
    pub const UNLOCK_BUTTON: WidgetSprites = WidgetSprites::three_state(
        "minecraft:widget/unlocked_button",
        "minecraft:widget/unlocked_button_disabled",
        "minecraft:widget/unlocked_button_highlighted",
    );
    pub const SCROLLER: &str = "minecraft:widget/scroller";
    pub const SCROLLER_BACKGROUND: &str = "minecraft:widget/scroller_background";
    pub const TOOLTIP_BACKGROUND: &str = "minecraft:tooltip/background";
    pub const TOOLTIP_FRAME: &str = "minecraft:tooltip/frame";
    pub const MENU_LIST_BACKGROUND: &str = "minecraft:gui/menu_list_background";
    pub const INWORLD_MENU_LIST_BACKGROUND: &str = "minecraft:gui/inworld_menu_list_background";
    pub const BUTTON_DISABLED: &str = "minecraft:widget/button_disabled";
    pub const LOADING: &str = "minecraft:friends/loading";
    pub const ICON_LANGUAGE: WidgetSprites = WidgetSprites::uniform("minecraft:icon/language");
    pub const ICON_ACCESSIBILITY: WidgetSprites =
        WidgetSprites::uniform("minecraft:icon/accessibility");
    pub const FRIENDS_BUTTON: WidgetSprites = WidgetSprites::uniform("minecraft:friends/friends");
    pub const PAUSE_REPORT_BUGS: WidgetSprites = WidgetSprites::uniform("minecraft:pause_menu/bug");
    pub const PAUSE_FEEDBACK: WidgetSprites =
        WidgetSprites::uniform("minecraft:pause_menu/social_interactions");
    pub const PAUSE_PLAYER_REPORTING: WidgetSprites =
        WidgetSprites::uniform("minecraft:pause_menu/player_reporting");
    pub const FRIENDS_ACCEPT: WidgetSprites = WidgetSprites::with_focused(
        "minecraft:friends/accept",
        "minecraft:friends/accept_highlighted",
    );
    pub const FRIENDS_REJECT: WidgetSprites = WidgetSprites::with_focused(
        "minecraft:friends/reject",
        "minecraft:friends/reject_highlighted",
    );
    pub const FRIENDS_CANCEL: WidgetSprites = WidgetSprites::uniform("minecraft:friends/cancel");
    pub const FRIENDS_REMOVE: WidgetSprites = WidgetSprites::uniform("minecraft:friends/remove");
}
