use crate::FontWidthMeasurer;
use crate::components::click_sound_sink;
use crate::components::{
    Button, DEFAULT_HEIGHT, GuiEventListener, Renderable, TabOrderedElement, WidgetClickSound,
};
use crate::framework::graphics::{GuiGraphicsExtractor, WidgetTooltipPosition};
use crate::framework::layouts::LayoutElement;
use crate::framework::narration::{NarratableEntry, NarrationElementOutput, NarrationPriority};
use crate::framework::navigation::{ComponentPath, FocusNavigationEvent, ScreenRectangle};
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent, MouseButtonInfo};
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
pub const HEADER_HEIGHT: i32 = 33;
pub const FOOTER_HEIGHT: i32 = 33;
pub const ROW_HEIGHT: i32 = 20;
pub const ROW_SPACING: i32 = 25;
pub const BIG_BUTTON_WIDTH: i32 = 310;
pub const SMALL_BUTTON_WIDTH: i32 = 150;
pub const DONE_BUTTON_WIDTH: i32 = 150;
pub const FOOTER_BUTTON_Y_OFFSET: i32 = 26;
pub const FOOTER_SPACING: i32 = 8;
pub const SCROLLER_WIDTH: i32 = 6;
pub const SCROLLER_RIGHT_OFFSET: i32 = 166;
#[derive(Clone, Debug)]
pub struct AccessibilityOptionsOption {
    pub key: String,
    pub label: String,
    pub is_slider: bool,
    pub min_val: f32,
    pub max_val: f32,
    pub step: f32,
    pub value: f32,
}
impl AccessibilityOptionsOption {
    pub fn get_display_string(&self) -> String {
        let name = crate::translate(&self.label);
        match self.key.as_str() {
            "showSubtitles"
            | "highContrast"
            | "bobView"
            | "hideLightningFlash"
            | "darkMojangStudiosBackground"
            | "hideSplashTexts"
            | "narratorHotkey"
            | "rotateWithMinecart"
            | "highContrastBlockOutline" => {
                let state = if self.value > 0.5 {
                    crate::translate("options.on")
                } else {
                    crate::translate("options.off")
                };
                format!("{}: {}", name, state)
            }
            "backgroundForChatOnly" => {
                let mode = if self.value > 0.5 {
                    crate::translate("options.accessibility.text_background.chat")
                } else {
                    crate::translate("options.accessibility.text_background.everywhere")
                };
                format!("{}: {}", name, mode)
            }
            "narrator" => {
                let val = self.value.round() as i32;
                let mode = match val {
                    0 => crate::translate("options.narrator.off"),
                    1 => crate::translate("options.narrator.all"),
                    2 => crate::translate("options.narrator.chat"),
                    3 => crate::translate("options.narrator.system"),
                    _ => String::new(),
                };
                format!("{}: {}", name, mode)
            }
            "controls" => name,
            "menuBackgroundBlurriness" => {
                let val = self.value.round() as i32;
                if val == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}", name, val)
                }
            }
            "textBackgroundOpacity" => {
                let pct = (self.value * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "chatOpacity" => {
                let pct = ((self.value * 0.9 + 0.1) * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "chatLineSpacing" => {
                let pct = (self.value * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            "chatDelay" => {
                let val = self.value;
                if val <= 0.0 {
                    crate::translate("options.chat.delay_none")
                } else {
                    let formatted = format!("{:.1}", val);
                    let translation = crate::translate("options.chat.delay");
                    translation.replace("%s", &formatted)
                }
            }
            "notificationDisplayTime" => {
                let val = self.value;
                let formatted = format!("{:.1}", val);
                let translation = crate::translate("options.multiplier");
                let multiplier = translation.replace("%s", &formatted);
                format!("{}: {}", name, multiplier)
            }
            "screenEffectScale" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "fovEffectScale" => {
                let pct = (self.value * self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "darknessEffectScale" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "damageTiltStrength" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "glintSpeed" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "glintStrength" => {
                let pct = (self.value * 100.0).round() as i32;
                if pct == 0 {
                    format!("{}: {}", name, crate::translate("options.off"))
                } else {
                    format!("{}: {}%", name, pct)
                }
            }
            "panoramaSpeed" => {
                let pct = (self.value * 100.0).round() as i32;
                format!("{}: {}%", name, pct)
            }
            _ => format!("{}: {}", name, self.value),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AccessibilityOptionsAction {
    Done,
    AccessibilityLinkClicked,
    ControlsClicked,
}
#[derive(Clone, Debug)]
pub enum AccessibilityOptionsListItem {
    Header {
        label_key: &'static str,
    },
    Big {
        option_index: usize,
    },
    SmallPair {
        option_index_a: usize,
        option_index_b: Option<usize>,
    },
}
#[derive(Clone, Debug)]
pub struct LayoutItem {
    pub kind: AccessibilityOptionsListItem,
    pub relative_y: i32,
    pub height: i32,
}
struct RowWidget {
    rect: ScreenRectangle,
    option_index: usize,
    is_slider: bool,
    focused: bool,
    pressed: bool,
}
impl RowWidget {
    fn new(rect: ScreenRectangle, option_index: usize, is_slider: bool) -> Self {
        Self {
            rect,
            option_index,
            is_slider,
            focused: false,
            pressed: false,
        }
    }
    fn takes_press(&mut self) -> bool {
        let pressed = self.pressed;
        self.pressed = false;
        pressed
    }
}
#[derive(Clone)]
struct RowHandle {
    row: Rc<RefCell<RowWidget>>,
}
impl RowHandle {
    fn new(row: Rc<RefCell<RowWidget>>) -> Self {
        Self { row }
    }
    fn with_row<R>(&self, f: impl FnOnce(&RowWidget) -> R, fallback: R) -> R {
        self.row.try_borrow().map(|r| f(&r)).unwrap_or(fallback)
    }
}
impl Renderable for RowHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for RowHandle {
    fn tab_order_group(&self) -> i32 {
        0
    }
}
impl NarratableEntry for RowHandle {
    fn narration_priority(&self) -> NarrationPriority {
        NarrationPriority::None
    }
    fn update_narration(&self, _output: &mut NarrationElementOutput) {}
}
impl GuiEventListener for RowHandle {
    fn mouse_clicked(&mut self, event: MouseButtonEvent, _double_click: bool) -> bool {
        if event.button() != 0 {
            return false;
        }
        let Ok(mut row) = self.row.try_borrow_mut() else {
            return false;
        };
        if row.rect.contains_point(event.x as i32, event.y as i32) {
            row.pressed = true;
            return true;
        }
        false
    }
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.with_row(
            |r| r.rect.contains_point(mouse_x as i32, mouse_y as i32),
            false,
        )
    }
    fn set_focused(&mut self, focused: bool) {
        if let Ok(mut row) = self.row.try_borrow_mut() {
            row.focused = focused;
        }
    }
    fn is_focused(&self) -> bool {
        self.with_row(|r| r.focused, false)
    }
    fn rectangle(&self) -> ScreenRectangle {
        self.with_row(|r| r.rect, ScreenRectangle::empty())
    }
    fn next_focus_path(&self, _event: &FocusNavigationEvent) -> Option<ComponentPath> {
        Some(ComponentPath::leaf(0))
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.with_row(
            |r| {
                if r.is_slider {
                    None
                } else {
                    Some(WidgetClickSound::UiButtonClick)
                }
            },
            Some(WidgetClickSound::UiButtonClick),
        )
    }
}
impl LayoutElement for RowHandle {
    fn set_x(&mut self, x: i32) {
        if let Ok(mut r) = self.row.try_borrow_mut() {
            r.rect.position.x = x;
        }
    }
    fn set_y(&mut self, y: i32) {
        if let Ok(mut r) = self.row.try_borrow_mut() {
            r.rect.position.y = y;
        }
    }
    fn x(&self) -> i32 {
        self.with_row(|r| r.rect.position.x, 0)
    }
    fn y(&self) -> i32 {
        self.with_row(|r| r.rect.position.y, 0)
    }
    fn width(&self) -> i32 {
        self.with_row(|r| r.rect.width, 0)
    }
    fn height(&self) -> i32 {
        self.with_row(|r| r.rect.height, 0)
    }
}
struct FooterButtonHandle {
    button: Rc<RefCell<Button<AccessibilityOptionsAction>>>,
}
impl FooterButtonHandle {
    fn new(button: Rc<RefCell<Button<AccessibilityOptionsAction>>>) -> Self {
        Self { button }
    }
}
impl Renderable for FooterButtonHandle {
    fn extract_render_state(
        &mut self,
        _graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
    }
}
impl TabOrderedElement for FooterButtonHandle {
    fn tab_order_group(&self) -> i32 {
        self.button.borrow().tab_order_group()
    }
}
impl GuiEventListener for FooterButtonHandle {
    fn is_mouse_over(&self, mouse_x: f64, mouse_y: f64) -> bool {
        self.button.borrow().is_mouse_over(mouse_x, mouse_y)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        self.button.borrow_mut().mouse_clicked(event, double_click)
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        self.button.borrow_mut().mouse_released(event)
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        self.button.borrow_mut().key_pressed(event)
    }
    fn set_focused(&mut self, focused: bool) {
        self.button.borrow_mut().set_focused(focused);
    }
    fn is_focused(&self) -> bool {
        self.button.borrow().is_focused()
    }
    fn rectangle(&self) -> ScreenRectangle {
        GuiEventListener::rectangle(&*self.button.borrow())
    }
    fn next_focus_path(&self, event: &FocusNavigationEvent) -> Option<ComponentPath> {
        self.button.borrow().next_focus_path(event)
    }
    fn play_down_sound(&self) -> Option<WidgetClickSound> {
        self.button.borrow().play_down_sound()
    }
}
impl LayoutElement for FooterButtonHandle {
    fn set_x(&mut self, x: i32) {
        self.button.borrow_mut().set_x(x);
    }
    fn set_y(&mut self, y: i32) {
        self.button.borrow_mut().set_y(y);
    }
    fn x(&self) -> i32 {
        self.button.borrow().x()
    }
    fn y(&self) -> i32 {
        self.button.borrow().y()
    }
    fn width(&self) -> i32 {
        self.button.borrow().width()
    }
    fn height(&self) -> i32 {
        self.button.borrow().height()
    }
}
impl NarratableEntry for FooterButtonHandle {
    fn narration_priority(&self) -> NarrationPriority {
        self.button.borrow().narration_priority()
    }
    fn update_narration(&self, output: &mut NarrationElementOutput) {
        self.button.borrow().update_narration(output);
    }
}
pub struct AccessibilityOptionsScreen {
    base: ScreenBase,
    link_button: Rc<RefCell<Button<AccessibilityOptionsAction>>>,
    done_button: Rc<RefCell<Button<AccessibilityOptionsAction>>>,
    rows: Vec<Rc<RefCell<RowWidget>>>,
    pub scroll_y: f32,
    pub dragging_scrollbar: bool,
    pub dragging_slider_index: Option<usize>,
    pub focused_slider_index: Option<usize>,
    pub slider_can_change_value: bool,
    pub last_width: i32,
    pub last_height: i32,
    pub options: Vec<AccessibilityOptionsOption>,
    pub in_world: bool,
    pub parent_is_title: bool,
    value_changed: bool,
    font: Arc<dyn FontWidthMeasurer>,
    pending_action: Option<AccessibilityOptionsAction>,
}
impl AccessibilityOptionsScreen {
    pub fn new(
        in_world: bool,
        extra_options: &HashMap<String, String>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let mut options = Vec::new();
        let _get_bool = |key: &str, default: bool| -> bool {
            extra_options
                .get(key)
                .map(|s| s == "true")
                .unwrap_or(default)
        };
        let _get_slider = |key: &str, default: f32, min_val: f32, max_val: f32| -> f32 {
            if let Some(s) = extra_options.get(key) {
                if let Ok(v) = s.parse::<f32>() {
                    let real_val = if key == "notificationDisplayTime" {
                        if v < min_val {
                            min_val + v.clamp(0.0, 1.0) * (max_val - min_val)
                        } else {
                            v
                        }
                    } else if v > 1.0 && max_val > 1.0 {
                        v
                    } else {
                        min_val + v.clamp(0.0, 1.0) * (max_val - min_val)
                    };
                    real_val.clamp(min_val, max_val)
                } else {
                    default
                }
            } else {
                default
            }
        };
        let narrator_val = match extra_options.get("narrator").map(|s| s.as_str()) {
            Some("0") | Some("off") => 0.0,
            Some("1") | Some("all") => 1.0,
            Some("2") | Some("chat") => 2.0,
            Some("3") | Some("system") => 3.0,
            _ => 0.0,
        };
        options.push(AccessibilityOptionsOption {
            key: "narrator".to_string(),
            label: "options.narrator".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 3.0,
            step: 1.0,
            value: narrator_val,
        });
        options.push(AccessibilityOptionsOption {
            key: "controls".to_string(),
            label: "options.controls".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: 0.0,
        });
        options.push(AccessibilityOptionsOption {
            key: "showSubtitles".to_string(),
            label: "options.showSubtitles".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("showSubtitles", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "highContrast".to_string(),
            label: "options.accessibility.high_contrast".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("highContrast", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "menuBackgroundBlurriness".to_string(),
            label: "options.accessibility.menu_background_blurriness".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 10.0,
            step: 1.0,
            value: _get_slider("menuBackgroundBlurriness", 5.0, 0.0, 10.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "textBackgroundOpacity".to_string(),
            label: "options.accessibility.text_background_opacity".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("textBackgroundOpacity", 0.5, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "backgroundForChatOnly".to_string(),
            label: "options.accessibility.text_background".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("backgroundForChatOnly", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "chatOpacity".to_string(),
            label: "options.chat.opacity".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatOpacity", 1.0, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "chatLineSpacing".to_string(),
            label: "options.chat.line_spacing".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("chatLineSpacing", 0.0, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "chatDelay".to_string(),
            label: "options.chat.delay_instant".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 6.0,
            step: 0.1,
            value: _get_slider("chatDelay", 0.0, 0.0, 6.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "notificationDisplayTime".to_string(),
            label: "options.notifications.display_time".to_string(),
            is_slider: true,
            min_val: 0.5,
            max_val: 10.0,
            step: 0.1,
            value: _get_slider("notificationDisplayTime", 1.0, 0.5, 10.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "bobView".to_string(),
            label: "options.viewBobbing".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("bobView", true) { 1.0 } else { 0.0 },
        });
        options.push(AccessibilityOptionsOption {
            key: "screenEffectScale".to_string(),
            label: "options.screenEffectScale".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("screenEffectScale", 1.0, 0.0, 1.0),
        });
        let fov_effect_slider = _get_slider("fovEffectScale", 1.0, 0.0, 1.0);
        options.push(AccessibilityOptionsOption {
            key: "fovEffectScale".to_string(),
            label: "options.fovEffectScale".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: fov_effect_slider,
        });
        options.push(AccessibilityOptionsOption {
            key: "darknessEffectScale".to_string(),
            label: "options.darknessEffectScale".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("darknessEffectScale", 1.0, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "damageTiltStrength".to_string(),
            label: "options.damageTiltStrength".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("damageTiltStrength", 1.0, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "glintSpeed".to_string(),
            label: "options.glintSpeed".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("glintSpeed", 0.5, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "glintStrength".to_string(),
            label: "options.glintStrength".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("glintStrength", 0.75, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "hideLightningFlash".to_string(),
            label: "options.hideLightningFlashes".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("hideLightningFlash", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "darkMojangStudiosBackground".to_string(),
            label: "options.darkMojangStudiosBackgroundColor".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("darkMojangStudiosBackground", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "panoramaSpeed".to_string(),
            label: "options.accessibility.panorama_speed".to_string(),
            is_slider: true,
            min_val: 0.0,
            max_val: 1.0,
            step: 0.01,
            value: _get_slider("panoramaSpeed", 1.0, 0.0, 1.0),
        });
        options.push(AccessibilityOptionsOption {
            key: "hideSplashTexts".to_string(),
            label: "options.hideSplashTexts".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("hideSplashTexts", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "narratorHotkey".to_string(),
            label: "options.accessibility.narrator_hotkey".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("narratorHotkey", true) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "rotateWithMinecart".to_string(),
            label: "options.rotateWithMinecart".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("rotateWithMinecart", false) {
                1.0
            } else {
                0.0
            },
        });
        options.push(AccessibilityOptionsOption {
            key: "highContrastBlockOutline".to_string(),
            label: "options.accessibility.high_contrast_block_outline".to_string(),
            is_slider: false,
            min_val: 0.0,
            max_val: 1.0,
            step: 1.0,
            value: if _get_bool("highContrastBlockOutline", false) {
                1.0
            } else {
                0.0
            },
        });
        let link_btn = Button::builder(Component::literal(&crate::translate(
            "options.accessibility.link",
        )))
        .bounds(0, 0, SMALL_BUTTON_WIDTH, DEFAULT_HEIGHT)
        .with_action(AccessibilityOptionsAction::AccessibilityLinkClicked)
        .with_font(Arc::clone(&font))
        .build();
        let done_btn = Button::builder(Component::literal(&crate::translate("gui.done")))
            .bounds(0, 0, DONE_BUTTON_WIDTH, DEFAULT_HEIGHT)
            .with_action(AccessibilityOptionsAction::Done)
            .with_font(Arc::clone(&font))
            .build();
        Self {
            base: ScreenBase::new(Component::literal(&crate::translate(
                "options.accessibility.title",
            ))),
            link_button: Rc::new(RefCell::new(link_btn)),
            done_button: Rc::new(RefCell::new(done_btn)),
            rows: Vec::new(),
            scroll_y: 0.0,
            dragging_scrollbar: false,
            dragging_slider_index: None,
            focused_slider_index: None,
            slider_can_change_value: false,
            last_width: 0,
            last_height: 0,
            options,
            in_world,
            parent_is_title: false,
            value_changed: false,
            font,
            pending_action: None,
        }
    }
    pub fn take_action(&mut self) -> Option<AccessibilityOptionsAction> {
        self.pending_action.take()
    }
    pub fn take_value_changed(&mut self) -> bool {
        let changed = self.value_changed;
        self.value_changed = false;
        changed
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn row_rects(&self) -> Vec<ScreenRectangle> {
        self.rows.iter().map(|r| r.borrow().rect).collect()
    }
    fn reposition_rows(&mut self) {
        let scroll = self.scroll_y.round() as i32;
        let mut row_idx = 0;
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - scroll;
            match item.kind {
                AccessibilityOptionsListItem::Header { .. } => {}
                AccessibilityOptionsListItem::Big { .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
                AccessibilityOptionsListItem::SmallPair { option_index_b, .. } => {
                    if let Some(row) = self.rows.get(row_idx) {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                    if option_index_b.is_some()
                        && let Some(row) = self.rows.get(row_idx)
                    {
                        row.borrow_mut().rect.position.y = item_y;
                        row_idx += 1;
                    }
                }
            }
        }
    }
    fn hovered_row(&self, mouse_x: i32, mouse_y: i32) -> Option<(usize, ScreenRectangle)> {
        self.rows.iter().find_map(|row| {
            let r = row.borrow();
            r.rect
                .contains_point(mouse_x, mouse_y)
                .then_some((r.option_index, r.rect))
        })
    }
    fn focused_row(&self) -> Option<(usize, ScreenRectangle, WidgetTooltipPosition)> {
        self.rows.iter().find_map(|row| {
            let r = row.borrow();
            r.focused
                .then_some((r.option_index, r.rect, WidgetTooltipPosition::BelowOrAbove))
        })
    }
    pub fn get_layout_items(&self) -> Vec<LayoutItem> {
        let list_items = vec![
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 0,
                option_index_b: Some(1),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 2,
                option_index_b: Some(3),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 4,
                option_index_b: Some(5),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 6,
                option_index_b: Some(7),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 8,
                option_index_b: Some(9),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 10,
                option_index_b: Some(11),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 12,
                option_index_b: Some(13),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 14,
                option_index_b: Some(15),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 16,
                option_index_b: Some(17),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 18,
                option_index_b: Some(19),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 20,
                option_index_b: Some(21),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 22,
                option_index_b: Some(23),
            },
            AccessibilityOptionsListItem::SmallPair {
                option_index_a: 24,
                option_index_b: None,
            },
        ];
        let mut items = Vec::new();
        let mut current_y = 0;
        for (i, kind) in list_items.into_iter().enumerate() {
            let height = match &kind {
                AccessibilityOptionsListItem::Header { .. } => {
                    if i == 0 {
                        13
                    } else {
                        31
                    }
                }
                _ => 25,
            };
            items.push(LayoutItem {
                kind,
                relative_y: current_y,
                height,
            });
            current_y += height;
        }
        items
    }
    pub fn max_scroll_amount(&self, height: i32) -> f32 {
        let items = self.get_layout_items();
        if let Some(last) = items.last() {
            let total_height = last.relative_y + last.height + 4;
            let visible_height = height - (HEADER_HEIGHT + FOOTER_HEIGHT);
            (total_height - visible_height).max(0) as f32
        } else {
            0.0
        }
    }
    pub fn scrollbar_height(&self, height: i32) -> i32 {
        let scroll_area_h = height - (HEADER_HEIGHT + FOOTER_HEIGHT);
        if scroll_area_h <= 0 {
            return 0;
        }
        let items = self.get_layout_items();
        let total_content_height = if let Some(last) = items.last() {
            last.relative_y + last.height + 4
        } else {
            1
        };
        let bar_h =
            ((scroll_area_h * scroll_area_h) as f32 / total_content_height as f32).round() as i32;
        let upper = (scroll_area_h - 8).max(1);
        let lower = 32.min(upper);
        bar_h.clamp(lower, upper).min(scroll_area_h)
    }
    pub fn mouse_clicked(
        &mut self,
        mouse_x: f32,
        mouse_y: f32,
        width: i32,
        height: i32,
    ) -> Option<AccessibilityOptionsAction> {
        let event =
            MouseButtonEvent::new(mouse_x as f64, mouse_y as f64, MouseButtonInfo::new(0, 0));
        self.ensure_layout(width, height);
        let handled = Screen::mouse_clicked(self, event, false);
        if handled {
            if let Some(action) = self.take_action() {
                match action {
                    AccessibilityOptionsAction::Done => {
                        click_sound_sink::record_button_click();
                        return Some(AccessibilityOptionsAction::Done);
                    }
                    AccessibilityOptionsAction::AccessibilityLinkClicked => {
                        click_sound_sink::record_button_click();
                        return Some(AccessibilityOptionsAction::AccessibilityLinkClicked);
                    }
                    AccessibilityOptionsAction::ControlsClicked => {
                        click_sound_sink::record_button_click();
                        return Some(AccessibilityOptionsAction::ControlsClicked);
                    }
                }
            }
        }
        None
    }
    pub fn mouse_released(&mut self) {
        self.dragging_scrollbar = false;
        if self.dragging_slider_index.take().is_some() {
            click_sound_sink::record_button_click();
        }
    }
    fn update_slider_from_mouse(
        &mut self,
        option_idx: usize,
        mouse_x: f32,
        slider_x: i32,
        slider_w: i32,
    ) {
        let opt = &mut self.options[option_idx];
        let min_x = slider_x as f32;
        let max_x = slider_x as f32 + slider_w as f32 - 8.0;
        let click_x = mouse_x - 4.0;
        let pct = if max_x > min_x {
            (click_x - min_x) / (max_x - min_x)
        } else {
            0.0
        };
        let clamped = pct.clamp(0.0, 1.0);
        let raw_val = opt.min_val + clamped * (opt.max_val - opt.min_val);
        let steps = ((raw_val - opt.min_val) / opt.step).round();
        opt.value = (opt.min_val + steps * opt.step).clamp(opt.min_val, opt.max_val);
    }
    fn cycle_option(&mut self, option_idx: usize) {
        let opt = &mut self.options[option_idx];
        opt.value += opt.step;
        if opt.value > opt.max_val {
            opt.value = opt.min_val;
        }
    }
    fn ensure_layout(&mut self, width: i32, height: i32) {
        if self.last_width == width && self.last_height == height && !self.rows.is_empty() {
            return;
        }
        self.last_width = width;
        self.last_height = height;
        self.base.clear_widgets();
        {
            let mut l = self.link_button.borrow_mut();
            l.set_position(width / 2 - 155, height - FOOTER_BUTTON_Y_OFFSET);
            l.base.set_width(SMALL_BUTTON_WIDTH);
        }
        {
            let mut d = self.done_button.borrow_mut();
            d.set_position(width / 2 + 5, height - FOOTER_BUTTON_Y_OFFSET);
            d.base.set_width(DONE_BUTTON_WIDTH);
        }
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.link_button)));
        self.base
            .add_widget(FooterButtonHandle::new(Rc::clone(&self.done_button)));
        self.rows.clear();
        for item in self.get_layout_items() {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y;
            match item.kind {
                AccessibilityOptionsListItem::Header { .. } => {}
                AccessibilityOptionsListItem::Big { option_index } => {
                    self.push_row(width / 2 - 155, item_y, BIG_BUTTON_WIDTH, option_index);
                }
                AccessibilityOptionsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    self.push_row(width / 2 - 155, item_y, SMALL_BUTTON_WIDTH, option_index_a);
                    if let Some(option_index_b) = option_index_b {
                        self.push_row(width / 2 + 5, item_y, SMALL_BUTTON_WIDTH, option_index_b);
                    }
                }
            }
        }
        for row in &self.rows {
            self.base.add_widget(RowHandle::new(Rc::clone(row)));
        }
    }
    fn push_row(&mut self, x: i32, y: i32, width: i32, option_index: usize) {
        let is_slider = self
            .options
            .get(option_index)
            .is_some_and(|opt| opt.is_slider);
        self.rows.push(Rc::new(RefCell::new(RowWidget::new(
            ScreenRectangle::new(x, y, width, ROW_HEIGHT),
            option_index,
            is_slider,
        ))));
    }
    fn draw_button(
        &self,
        graphics: &mut GuiGraphicsExtractor,
        font: &dyn FontWidthMeasurer,
        option_idx: usize,
        x: i32,
        y: i32,
        w: i32,
    ) {
        let Some(option) = self.options.get(option_idx) else {
            return;
        };
        if option.is_slider {
            let pct = (option.value - option.min_val) / (option.max_val - option.min_val);
            graphics.draw_slider_with_interaction_state(
                font,
                x,
                y,
                w,
                ROW_HEIGHT,
                &option.get_display_string(),
                pct,
                true,
                self.focused_slider_index == Some(option_idx),
                self.slider_can_change_value && self.focused_slider_index == Some(option_idx),
                self.dragging_slider_index == Some(option_idx),
            );
        } else {
            graphics.draw_button(
                font,
                x,
                y,
                w,
                ROW_HEIGHT,
                &option.get_display_string(),
                true,
            );
        }
    }
    fn render_screen(&mut self, graphics: &mut GuiGraphicsExtractor, font: &dyn FontWidthMeasurer) {
        let width = graphics.gui_width();
        let height = graphics.gui_height();
        self.ensure_layout(width, height);
        self.reposition_rows();
        self.base
            .extract_background_backdrop(graphics, self.in_world);
        let list_bg_tex = if self.in_world {
            "minecraft:gui/inworld_menu_list_background"
        } else {
            "minecraft:gui/menu_list_background"
        };
        let list_background = ResourceLocation::from_string(list_bg_tex);
        let mut y = HEADER_HEIGHT;
        while y < height - FOOTER_HEIGHT {
            let tile_h = 32.min(height - FOOTER_HEIGHT - y);
            let mut x = 0;
            while x < width {
                let tile_w = 32.min(width - x);
                graphics.blit_region(
                    list_background.clone(),
                    x,
                    y,
                    tile_w,
                    tile_h,
                    0.0,
                    (y as f32 + self.scroll_y) / 32.0,
                    tile_w as f32 / 32.0,
                    tile_h as f32 / 32.0,
                    0xFFFF_FFFFu32 as i32,
                );
                x += 32;
            }
            y += tile_h;
        }
        graphics.enable_scissor(0, HEADER_HEIGHT, width, height - FOOTER_HEIGHT);
        let layout_items = self.get_layout_items();
        for item in layout_items {
            let item_y = HEADER_HEIGHT + 4 + item.relative_y - self.scroll_y as i32;
            match item.kind {
                AccessibilityOptionsListItem::Header { label_key } => {
                    let header_text = crate::translate(label_key);
                    let text_y = if item.height == 13 {
                        item_y
                    } else {
                        item_y + 18
                    };
                    graphics.text(
                        ResourceLocation::from_string("minecraft:default"),
                        header_text,
                        width / 2 - 155,
                        text_y,
                        0xFFFF_FFFFu32 as i32,
                    );
                }
                AccessibilityOptionsListItem::Big { option_index } => {
                    let btn_x = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_button(graphics, font, option_index, btn_x, btn_y, BIG_BUTTON_WIDTH);
                }
                AccessibilityOptionsListItem::SmallPair {
                    option_index_a,
                    option_index_b,
                } => {
                    let btn_x_a = width / 2 - 155;
                    let btn_y = item_y;
                    self.draw_button(
                        graphics,
                        font,
                        option_index_a,
                        btn_x_a,
                        btn_y,
                        SMALL_BUTTON_WIDTH,
                    );
                    if let Some(option_index_b) = option_index_b {
                        let btn_x_b = width / 2 + 5;
                        self.draw_button(
                            graphics,
                            font,
                            option_index_b,
                            btn_x_b,
                            btn_y,
                            SMALL_BUTTON_WIDTH,
                        );
                    }
                }
            }
        }
        graphics.disable_scissor();
        graphics.draw_menu_background(self.in_world, width, HEADER_HEIGHT);
        graphics.pose_mut().push_pose();
        graphics
            .pose_mut()
            .translate(0.0, (height - FOOTER_HEIGHT) as f32);
        graphics.draw_menu_background(self.in_world, width, FOOTER_HEIGHT);
        graphics.pose_mut().pop_pose();
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/header_separator"),
            0,
            HEADER_HEIGHT - 2,
            width,
            2,
        );
        graphics.blit(
            ResourceLocation::from_string("minecraft:gui/footer_separator"),
            0,
            height - FOOTER_HEIGHT,
            width,
            2,
        );
        let title_text = crate::translate("options.accessibility.title");
        let title_w = font.measure_width(&title_text, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title_text,
            (width - title_w) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
        let max_scroll = self.max_scroll_amount(height);
        if max_scroll > 0.0 {
            let scrollbar_x = width / 2 + SCROLLER_RIGHT_OFFSET;
            let scroll_area_h = height - (HEADER_HEIGHT + FOOTER_HEIGHT);
            let bar_h = self.scrollbar_height(height);
            let bar_y = HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32;
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller_background"),
                scrollbar_x,
                HEADER_HEIGHT,
                SCROLLER_WIDTH,
                scroll_area_h,
            );
            graphics.blit(
                ResourceLocation::from_string("minecraft:widget/scroller"),
                scrollbar_x,
                bar_y,
                SCROLLER_WIDTH,
                bar_h,
            );
        }
        let done_y = height - FOOTER_BUTTON_Y_OFFSET;
        let guide_x = width / 2 - 155;
        graphics.draw_button(
            font,
            guide_x,
            done_y,
            SMALL_BUTTON_WIDTH,
            ROW_HEIGHT,
            &crate::translate("options.accessibility.link"),
            true,
        );
        let done_x = width / 2 + 5;
        graphics.draw_button(
            font,
            done_x,
            done_y,
            DONE_BUTTON_WIDTH,
            ROW_HEIGHT,
            &crate::translate("gui.done"),
            true,
        );
        let mouse_x = graphics.mouse_x();
        let mouse_y = graphics.mouse_y();
        let target = if mouse_y >= HEADER_HEIGHT && mouse_y < height - FOOTER_HEIGHT {
            self.hovered_row(mouse_x, mouse_y)
                .map(|(idx, rect)| (idx, rect, WidgetTooltipPosition::Menu))
        } else {
            None
        }
        .or_else(|| self.focused_row());
        if let Some((option_idx, rect, position)) = target
            && let Some(lines) = crate::settings::option_tooltips::accessibility_tooltip(
                &self.options[option_idx].key,
            )
        {
            graphics.draw_widget_tooltip(font, &lines, mouse_x, mouse_y, rect, position);
        }
    }
}
impl Screen for AccessibilityOptionsScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn should_close_on_esc(&self) -> bool {
        true
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = 0;
        self.last_height = 0;
        self.rows.clear();
        self.ensure_layout(width, height);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        if event.is_escape() {
            self.pending_action = Some(AccessibilityOptionsAction::Done);
            return true;
        }
        let focused_idx = self.base_ref().focused_index();
        if let Some(index) = focused_idx {
            if index == 0 {
                if event.is_selection() {
                    self.pending_action =
                        Some(AccessibilityOptionsAction::AccessibilityLinkClicked);
                    click_sound_sink::record_button_click();
                    return true;
                }
            } else if index == 1 {
                if event.is_selection() {
                    self.pending_action = Some(AccessibilityOptionsAction::Done);
                    click_sound_sink::record_button_click();
                    return true;
                }
            } else if let Some(row) = self.rows.get(index - 2) {
                let (option_index, is_slider) = {
                    let r = row.borrow();
                    (r.option_index, r.is_slider)
                };
                if self.options[option_index].key == "controls" {
                    if event.is_selection() {
                        self.pending_action = Some(AccessibilityOptionsAction::ControlsClicked);
                        click_sound_sink::record_button_click();
                        return true;
                    }
                } else if is_slider {
                    if event.is_selection() {
                        self.slider_can_change_value = !self.slider_can_change_value;
                        return true;
                    }
                    if self.slider_can_change_value {
                        let dir = match event.key {
                            KeyEvent::KEY_LEFT => Some(-1.0),
                            KeyEvent::KEY_RIGHT => Some(1.0),
                            _ => None,
                        };
                        if let Some(dir) = dir {
                            let opt = &mut self.options[option_index];
                            opt.value =
                                (opt.value + dir * opt.step).clamp(opt.min_val, opt.max_val);
                            self.value_changed = true;
                            return true;
                        }
                    }
                } else if event.is_selection() {
                    click_sound_sink::record_button_click();
                    self.cycle_option(option_index);
                    self.value_changed = true;
                    return true;
                }
            }
        }
        let handled = crate::framework::screen::screen_key_pressed(self, event);
        if handled || (event.key == KeyEvent::KEY_TAB && self.base_ref().focused_index().is_some())
        {
            return true;
        }
        false
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let handled = crate::framework::screen::screen_mouse_clicked(self, event, double_click);
        if !handled {
            return false;
        }
        let width = self.base_ref().width;
        let height = self.base_ref().height;
        let scrollbar_x = width / 2 + SCROLLER_RIGHT_OFFSET;
        let bar_h = self.scrollbar_height(height);
        let max_scroll = self.max_scroll_amount(height);
        let bar_y = if max_scroll == 0.0 {
            HEADER_HEIGHT
        } else {
            let scroll_area_h = height - (HEADER_HEIGHT + FOOTER_HEIGHT);
            HEADER_HEIGHT
                + (self.scroll_y * (scroll_area_h - bar_h) as f32 / max_scroll).round() as i32
        };
        let (mx, my) = (event.x as i32, event.y as i32);
        if event.button() == 0
            && mx >= scrollbar_x
            && mx < scrollbar_x + SCROLLER_WIDTH
            && my >= HEADER_HEIGHT
            && my < height - FOOTER_HEIGHT
        {
            self.dragging_scrollbar = true;
            if my < bar_y {
                self.scroll_y = (self.scroll_y - 25.0).max(0.0);
            } else if my > bar_y + bar_h {
                self.scroll_y = (self.scroll_y + 25.0).min(max_scroll);
            }
            return true;
        }
        if let Some(action) = self.link_button.borrow_mut().take_press_action() {
            self.pending_action = Some(action);
            return true;
        }
        if let Some(action) = self.done_button.borrow_mut().take_press_action() {
            self.pending_action = Some(action);
            return true;
        }
        let pressed: Vec<(usize, bool, ScreenRectangle)> = self
            .rows
            .iter()
            .filter_map(|row| {
                let mut r = row.borrow_mut();
                r.takes_press()
                    .then(|| (r.option_index, r.is_slider, r.rect))
            })
            .collect();
        for (option_index, is_slider, rect) in pressed {
            if is_slider {
                click_sound_sink::record(None);
                self.focused_slider_index = Some(option_index);
                self.slider_can_change_value = true;
                self.dragging_slider_index = Some(option_index);
                self.update_slider_from_mouse(
                    option_index,
                    event.x as f32,
                    rect.left(),
                    rect.width,
                );
                self.value_changed = true;
            } else {
                let opt_key = self.options[option_index].key.clone();
                match opt_key.as_str() {
                    "controls" => {
                        click_sound_sink::record_button_click();
                        self.pending_action = Some(AccessibilityOptionsAction::ControlsClicked);
                    }
                    "narrator" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "showSubtitles" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "highContrast" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "backgroundForChatOnly" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "bobView" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "hideLightningFlash" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "darkMojangStudiosBackground" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "hideSplashTexts" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "narratorHotkey" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "rotateWithMinecart" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    "highContrastBlockOutline" => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                    _ => {
                        click_sound_sink::record_button_click();
                        self.cycle_option(option_index);
                        self.value_changed = true;
                    }
                }
            }
        }
        true
    }
    fn mouse_released(&mut self, event: MouseButtonEvent) -> bool {
        let handled = crate::framework::screen::screen_mouse_released(self, event);
        if self.dragging_slider_index.take().is_some() {
            click_sound_sink::record_button_click();
        }
        self.dragging_scrollbar = false;
        handled
    }
    fn mouse_dragged(&mut self, event: MouseButtonEvent, dx: f64, dy: f64) -> bool {
        let handled = crate::framework::screen::screen_mouse_dragged(self, event, dx, dy);
        if self.dragging_scrollbar {
            let height = self.base_ref().height;
            let max_scroll = self.max_scroll_amount(height);
            let bar_h = self.scrollbar_height(height);
            let scroll_area_h = height - (HEADER_HEIGHT + FOOTER_HEIGHT);
            let y_drag_scale = if scroll_area_h - bar_h > 0 {
                max_scroll / (scroll_area_h - bar_h) as f32
            } else {
                1.0
            };
            self.scroll_y = (self.scroll_y + dy as f32 * y_drag_scale).clamp(0.0, max_scroll);
            self.reposition_rows();
            true
        } else if let Some(slider_idx) = self.dragging_slider_index {
            let rect = self
                .rows
                .iter()
                .find(|r| r.borrow().option_index == slider_idx)
                .map(|r| r.borrow().rect)
                .unwrap_or_else(ScreenRectangle::empty);
            self.update_slider_from_mouse(slider_idx, event.x as f32, rect.left(), rect.width);
            self.value_changed = true;
            true
        } else {
            handled
        }
    }
    fn mouse_scrolled(&mut self, _x: f64, _y: f64, _scroll_x: f64, scroll_y: f64) -> bool {
        let height = self.base_ref().height;
        let max_scroll = self.max_scroll_amount(height);
        self.scroll_y = (self.scroll_y - scroll_y as f32 * 12.0).clamp(0.0, max_scroll);
        self.reposition_rows();
        true
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        _mouse_x: i32,
        _mouse_y: i32,
        _partial_tick: f32,
    ) {
        let font = Arc::clone(&self.font);
        self.render_screen(graphics, font.as_ref());
    }
}
