use crate::translate;
fn lines_for(key: &str) -> Option<Vec<String>> {
    let text = translate(key);
    Some(text.split('\n').map(str::to_string).collect())
}
pub fn accessibility_tooltip(option_key: &str) -> Option<Vec<String>> {
    let key = match option_key {
        "showSubtitles" => "options.showSubtitles.tooltip",
        "highContrast" => "options.accessibility.high_contrast.tooltip",
        "menuBackgroundBlurriness" => "options.accessibility.menu_background_blurriness.tooltip",
        "notificationDisplayTime" => "options.notifications.display_time.tooltip",
        "screenEffectScale" => "options.screenEffectScale.tooltip",
        "fovEffectScale" => "options.fovEffectScale.tooltip",
        "darknessEffectScale" => "options.darknessEffectScale.tooltip",
        "glintSpeed" => "options.glintSpeed.tooltip",
        "glintStrength" => "options.glintStrength.tooltip",
        "damageTiltStrength" => "options.damageTiltStrength.tooltip",
        "hideLightningFlash" => "options.hideLightningFlashes.tooltip",
        "darkMojangStudiosBackground" => "options.darkMojangStudiosBackgroundColor.tooltip",
        "hideSplashTexts" => "options.hideSplashTexts.tooltip",
        "narratorHotkey" => {
            if cfg!(target_os = "macos") {
                "options.accessibility.narrator_hotkey.mac.tooltip"
            } else {
                "options.accessibility.narrator_hotkey.tooltip"
            }
        }
        "rotateWithMinecart" => "options.rotateWithMinecart.tooltip",
        "highContrastBlockOutline" => "options.accessibility.high_contrast_block_outline.tooltip",
        _ => return None,
    };
    lines_for(key)
}
pub fn sound_tooltip(option_key: &str, value: f32) -> Option<Vec<String>> {
    let key = match option_key {
        "directionalAudio" => {
            if value >= 0.5 {
                "options.directionalAudio.on.tooltip"
            } else {
                "options.directionalAudio.off.tooltip"
            }
        }
        "musicFrequency" => "options.music_frequency.tooltip",
        "musicToast" => match value.round() as i32 {
            1 => "options.musicToast.pauseMenu.tooltip",
            2 => "options.musicToast.pauseMenuAndToast.tooltip",
            _ => "options.musicToast.never.tooltip",
        },
        _ => return None,
    };
    lines_for(key)
}
pub fn video_tooltip(option_key: &str, value: f32) -> Option<Vec<String>> {
    let key = match option_key {
        "graphicsPreset" => "options.graphics.preset.tooltip",
        "weatherRadius" => "options.weatherRadius.tooltip",
        "cutoutLeaves" => "options.cutoutLeaves.tooltip",
        "vignette" => "options.vignette.tooltip",
        "improvedTransparency" => "options.improvedTransparency.tooltip",
        "chunkSectionFadeInTime" => "options.chunkFade.tooltip",
        "maxAnisotropyBit" => "options.maxAnisotropy.tooltip",
        "textureFiltering" => match value.round() as i32 {
            1 => "options.textureFiltering.rgss.tooltip",
            2 => "options.textureFiltering.anisotropic.tooltip",
            _ => "options.textureFiltering.none.tooltip",
        },
        "inactivityFpsLimit" => {
            if value >= 0.5 {
                "options.inactivityFpsLimit.afk.tooltip"
            } else {
                "options.inactivityFpsLimit.minimized.tooltip"
            }
        }
        "exclusiveFullscreen" => {
            if value >= 0.5 {
                "options.exclusiveFullscreen.on.tooltip"
            } else {
                "options.exclusiveFullscreen.off.tooltip"
            }
        }
        "preferredGraphicsBackend" => "options.graphicsApi.tooltip",
        "menuBackgroundBlurriness" => "options.accessibility.menu_background_blurriness.tooltip",
        _ => return None,
    };
    lines_for(key)
}
pub fn chat_tooltip(option_key: &str) -> Option<Vec<String>> {
    let key = match option_key {
        "hideMatchedNames" => "options.hideMatchedNames.tooltip",
        "onlyShowSecureChat" => "options.onlyShowSecureChat.tooltip",
        "saveChatDrafts" => "options.chat.drafts.tooltip",
        _ => return None,
    };
    lines_for(key)
}
pub fn controls_tooltip(option_key: &str) -> Option<Vec<String>> {
    let key = match option_key {
        "sprintWindow" => "options.sprintWindow.tooltip",
        _ => return None,
    };
    lines_for(key)
}
pub fn mouse_tooltip(option_key: &str) -> Option<Vec<String>> {
    let key = match option_key {
        "allowCursorChanges" => "options.allowCursorChanges.tooltip",
        _ => return None,
    };
    lines_for(key)
}
pub fn font_tooltip(option_key: &str) -> Option<Vec<String>> {
    let key = match option_key {
        "japaneseGlyphVariants" => "options.japaneseGlyphVariants.tooltip",
        _ => return None,
    };
    lines_for(key)
}
pub fn online_tooltip(option_key: &str) -> Option<Vec<String>> {
    let key = match option_key {
        "options.allowFriendRequests" => "options.allowFriendRequests.tooltip",
        "options.inGameNotification" => "options.inGameNotification.tooltip",
        "options.allowServerListing" => "options.allowServerListing.tooltip",
        "options.realmsNotifications.button" => "options.realmsNotifications.tooltip",
        _ => return None,
    };
    lines_for(key)
}
