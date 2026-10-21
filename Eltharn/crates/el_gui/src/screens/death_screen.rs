use crate::FontWidthMeasurer;
use crate::components::{BIG_WIDTH, Button, DEFAULT_HEIGHT};
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::narration::NarrationElementOutput;
use crate::framework::navigation::ScreenRectangle;
use crate::framework::screen::{Screen, ScreenBase};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use crate::screens::warning_screen::SharedWidget;
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
const TITLE_KEY: &str = "deathScreen.title";
const HARDCORE_TITLE_KEY: &str = "deathScreen.title.hardcore";
const RESPAWN_KEY: &str = "deathScreen.respawn";
const SPECTATE_KEY: &str = "deathScreen.spectate";
const TITLE_SCREEN_KEY: &str = "deathScreen.titleScreen";
const QUIT_CONFIRM_KEY: &str = "deathScreen.quit.confirm";
const SCORE_VALUE_KEY: &str = "deathScreen.score.value";
const TITLE_FALLBACK: &str = "You Died!";
const HARDCORE_TITLE_FALLBACK: &str = "Game Over!";
const RESPAWN_FALLBACK: &str = "Respawn";
const SPECTATE_FALLBACK: &str = "Spectate World";
const TITLE_SCREEN_FALLBACK: &str = "Title Screen";
const QUIT_CONFIRM_FALLBACK: &str = "Are you sure you want to quit?";
const SCORE_VALUE_FALLBACK: &str = "Score: %s";
const DEFAULT_FONT: &str = "minecraft:default";
const WHITE: i32 = 0xFFFF_FFFFu32 as i32;
const YELLOW: i32 = 0xFFFF_FF55u32 as i32;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DeathScreenLayout {
    pub respawn_button: ScreenRectangle,
    pub title_button: ScreenRectangle,
    pub title_anchor_x: i32,
    pub text_center_x: i32,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeathScreenAction {
    Respawn,
    RequestTitleExit,
    ExitToTitle,
    ShowTitleConfirmation {
        title: String,
        message: String,
        yes_button: String,
        no_button: String,
        delay_ticks: u32,
    },
    RespawnAndClose,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ScoreTextPart {
    text: String,
    yellow: bool,
}
pub struct DeathScreen {
    base: ScreenBase,
    font: Arc<dyn FontWidthMeasurer>,
    cause_of_death: Option<String>,
    hardcore: bool,
    score: Option<i32>,
    delay_ticker: u32,
    has_draft_report: bool,
    respawn_button: Rc<RefCell<Button<DeathScreenAction>>>,
    title_button: Rc<RefCell<Button<DeathScreenAction>>>,
}
impl DeathScreen {
    pub fn new(
        cause_of_death: Option<String>,
        hardcore: bool,
        score: Option<i32>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title = translated(
            if hardcore {
                HARDCORE_TITLE_KEY
            } else {
                TITLE_KEY
            },
            if hardcore {
                HARDCORE_TITLE_FALLBACK
            } else {
                TITLE_FALLBACK
            },
        );
        let respawn_label = translated(
            if hardcore { SPECTATE_KEY } else { RESPAWN_KEY },
            if hardcore {
                SPECTATE_FALLBACK
            } else {
                RESPAWN_FALLBACK
            },
        );
        let title_label = translated(TITLE_SCREEN_KEY, TITLE_SCREEN_FALLBACK);
        let respawn_button = Rc::new(RefCell::new(
            Button::<DeathScreenAction>::builder(Component::literal(&respawn_label))
                .bounds(0, 0, BIG_WIDTH, DEFAULT_HEIGHT)
                .with_action(DeathScreenAction::Respawn)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        let title_button = Rc::new(RefCell::new(
            Button::<DeathScreenAction>::builder(Component::literal(&title_label))
                .bounds(0, 0, BIG_WIDTH, DEFAULT_HEIGHT)
                .with_action(DeathScreenAction::RequestTitleExit)
                .with_font(Arc::clone(&font))
                .build(),
        ));
        let mut base = ScreenBase::new(Component::literal(&title));
        base.set_close_on_esc(false);
        base.set_pause_screen(false);
        Self {
            base,
            font,
            cause_of_death,
            hardcore,
            score,
            delay_ticker: 0,
            has_draft_report: false,
            respawn_button,
            title_button,
        }
    }
    pub fn layout(width: i32, height: i32) -> DeathScreenLayout {
        let middle_line = width / 2;
        DeathScreenLayout {
            respawn_button: ScreenRectangle::new(
                middle_line - BIG_WIDTH / 2,
                height / 4 + 72,
                BIG_WIDTH,
                DEFAULT_HEIGHT,
            ),
            title_button: ScreenRectangle::new(
                middle_line - BIG_WIDTH / 2,
                height / 4 + 96,
                BIG_WIDTH,
                DEFAULT_HEIGHT,
            ),
            title_anchor_x: middle_line / 2,
            text_center_x: middle_line,
        }
    }
    pub fn set_has_draft_report(&mut self, has_draft_report: bool) {
        self.has_draft_report = has_draft_report;
    }
    pub fn delay_ticker(&self) -> u32 {
        self.delay_ticker
    }
    pub fn take_action(&mut self) -> Option<DeathScreenAction> {
        let respawn_action = self.respawn_button.borrow_mut().take_press_action();
        if let Some(action) = respawn_action {
            self.respawn_button.borrow_mut().base.active = false;
            return Some(action);
        }
        self.title_button.borrow_mut().take_press_action()
    }
    pub fn after_report_handled(&self) -> DeathScreenAction {
        if self.hardcore {
            DeathScreenAction::ExitToTitle
        } else {
            DeathScreenAction::ShowTitleConfirmation {
                title: translated(QUIT_CONFIRM_KEY, QUIT_CONFIRM_FALLBACK),
                message: String::new(),
                yes_button: translated(TITLE_SCREEN_KEY, TITLE_SCREEN_FALLBACK),
                no_button: translated(RESPAWN_KEY, RESPAWN_FALLBACK),
                delay_ticks: 20,
            }
        }
    }
    pub fn resolve_title_confirmation(&self, confirmed: bool) -> DeathScreenAction {
        if confirmed {
            DeathScreenAction::ExitToTitle
        } else {
            DeathScreenAction::RespawnAndClose
        }
    }
    pub const fn is_allowed_in_portal(&self) -> bool {
        true
    }
    pub fn extract_death_background(graphics: &mut GuiGraphicsExtractor, width: i32, height: i32) {
        Self::draw_death_background(graphics, width, height);
    }
    fn set_buttons_active(&mut self, active: bool) {
        self.respawn_button.borrow_mut().base.active = active;
        self.title_button.borrow_mut().base.active = active;
    }
    fn draw_death_background(graphics: &mut GuiGraphicsExtractor, width: i32, height: i32) {
        graphics.fill_gradient(0, 0, width, height, 1_615_855_616, -1_602_211_792);
    }
    fn draw_text(&self, graphics: &mut GuiGraphicsExtractor, layout: DeathScreenLayout) {
        let title = self.base.title().get_string();
        let title_width = java_font_width(self.font.as_ref(), &title);
        graphics.pose_mut().push_pose();
        graphics.pose_mut().scale(2.0, 2.0);
        graphics.text(
            default_font(),
            title,
            layout.title_anchor_x - title_width / 2,
            30,
            WHITE,
        );
        graphics.pose_mut().pop_pose();
        if let Some(cause) = &self.cause_of_death {
            let x = layout.text_center_x - java_font_width(self.font.as_ref(), cause) / 2;
            graphics.text(default_font(), cause.clone(), x, 85, WHITE);
        }
        if let Some(score) = self.score {
            let template = translated(SCORE_VALUE_KEY, SCORE_VALUE_FALLBACK);
            let parts = format_score_value(&template, &score.to_string());
            let full_text = parts
                .iter()
                .map(|part| part.text.as_str())
                .collect::<String>();
            let mut x = layout.text_center_x - java_font_width(self.font.as_ref(), &full_text) / 2;
            for part in parts {
                if part.text.is_empty() {
                    continue;
                }
                graphics.text(
                    default_font(),
                    part.text.clone(),
                    x,
                    100,
                    if part.yellow { YELLOW } else { WHITE },
                );
                x += java_font_width(self.font.as_ref(), &part.text);
            }
        }
    }
}
impl Screen for DeathScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let layout = Self::layout(self.base.width, self.base.height);
        self.delay_ticker = 0;
        self.base.clear_widgets();
        {
            let mut button = self.respawn_button.borrow_mut();
            button.base.set_x(layout.respawn_button.position.x);
            button.base.set_y(layout.respawn_button.position.y);
            button.base.active = false;
        }
        {
            let mut button = self.title_button.borrow_mut();
            button.base.set_x(layout.title_button.position.x);
            button.base.set_y(layout.title_button.position.y);
            button.base.active = false;
        }
        self.base
            .add_widget(SharedWidget::new(Rc::clone(&self.respawn_button)));
        self.base
            .add_widget(SharedWidget::new(Rc::clone(&self.title_button)));
    }
    fn tick(&mut self) {
        self.delay_ticker = self.delay_ticker.saturating_add(1);
        if self.delay_ticker == 20 {
            self.set_buttons_active(true);
        }
    }
    fn should_close_on_esc(&self) -> bool {
        false
    }
    fn is_pause_screen(&self) -> bool {
        false
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let layout = Self::layout(self.base.width, self.base.height);
        Self::extract_death_background(graphics, self.base.width, self.base.height);
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        if self.has_draft_report {
            let button = self.title_button.borrow();
            graphics.blit(
                ResourceLocation::from_string("minecraft:icon/draft_report"),
                button.base.x() + button.base.width() - 17,
                button.base.y() + 3,
                15,
                15,
            );
        }
        self.draw_text(graphics, layout);
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        crate::framework::screen::screen_key_pressed(self, event)
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        crate::framework::screen::screen_mouse_clicked(self, event, double_click)
    }
    fn update_narration(&mut self, output: &mut NarrationElementOutput) {
        self.base.update_narration(output);
    }
}
fn translated(key: &str, fallback: &str) -> String {
    let value = crate::translate(key);
    if value == key {
        fallback.to_string()
    } else {
        value
    }
}
fn default_font() -> ResourceLocation {
    ResourceLocation::from_string(DEFAULT_FONT)
}
fn java_font_width(font: &dyn FontWidthMeasurer, text: &str) -> i32 {
    font.measure_width(text, 1.0).ceil() as i32
}
fn format_score_value(template: &str, score: &str) -> Vec<ScoreTextPart> {
    let bytes = template.as_bytes();
    let mut index = 0;
    let mut automatic_argument = 0usize;
    let mut plain = String::new();
    let mut parts = Vec::new();
    let mut saw_argument = false;
    while index < bytes.len() {
        if bytes[index] != b'%' {
            let Some(ch) = template[index..].chars().next() else {
                break;
            };
            plain.push(ch);
            index += ch.len_utf8();
            continue;
        }
        if bytes.get(index + 1) == Some(&b'%') {
            plain.push('%');
            index += 2;
            continue;
        }
        let token_start = index;
        index += 1;
        let explicit_argument = if bytes.get(index).is_some_and(u8::is_ascii_digit) {
            let digits_start = index;
            while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                index += 1;
            }
            if bytes.get(index) != Some(&b'$') {
                return vec![ScoreTextPart {
                    text: template.to_string(),
                    yellow: false,
                }];
            }
            let parsed = template[digits_start..index].parse::<usize>().ok();
            index += 1;
            parsed.map(|one_based| one_based.saturating_sub(1))
        } else {
            let argument = automatic_argument;
            automatic_argument += 1;
            Some(argument)
        };
        if bytes.get(index) != Some(&b's') || explicit_argument != Some(0) {
            return vec![ScoreTextPart {
                text: template.to_string(),
                yellow: false,
            }];
        }
        index += 1;
        if !plain.is_empty() {
            parts.push(ScoreTextPart {
                text: std::mem::take(&mut plain),
                yellow: false,
            });
        }
        parts.push(ScoreTextPart {
            text: score.to_string(),
            yellow: true,
        });
        saw_argument = true;
        debug_assert!(index > token_start);
    }
    if !plain.is_empty() {
        parts.push(ScoreTextPart {
            text: plain,
            yellow: false,
        });
    }
    if !saw_argument && parts.is_empty() {
        parts.push(ScoreTextPart {
            text: String::new(),
            yellow: false,
        });
    }
    parts
}
