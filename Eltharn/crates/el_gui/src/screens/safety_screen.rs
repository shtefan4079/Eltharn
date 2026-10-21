use super::warning_screen::{SharedWidget, WarningScreen, position_footer_button};
use crate::FontWidthMeasurer;
use crate::components::Button;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::screen::{Screen, ScreenBase, screen_key_pressed, screen_mouse_clicked};
use crate::input::events::{KeyEvent, MouseButtonEvent};
use el_core::Component;
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
pub const TITLE_KEY: &str = "multiplayerWarning.header";
pub const MESSAGE_KEY: &str = "multiplayerWarning.message";
pub const CHECK_KEY: &str = "multiplayerWarning.check";
pub const PROCEED_KEY: &str = "gui.proceed";
pub const BACK_KEY: &str = "gui.back";
pub const CHECK_COLOR: i32 = 0xFFE0_E0E0u32 as i32;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SafetyScreenAction {
    Back,
    Proceed { skip_warning: bool },
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FooterPress {
    Proceed,
    Back,
}
pub struct SafetyScreen {
    base: ScreenBase,
    warning: WarningScreen,
    pub in_world: bool,
    pub skip_warning_checked: bool,
    font: Arc<dyn FontWidthMeasurer>,
    proceed_button: Rc<RefCell<Button<FooterPress>>>,
    back_button: Rc<RefCell<Button<FooterPress>>>,
}
impl SafetyScreen {
    pub fn new(in_world: bool, font: Arc<dyn FontWidthMeasurer>) -> Self {
        let warning = WarningScreen::new(
            TITLE_KEY,
            crate::translate(MESSAGE_KEY),
            Some(CHECK_KEY),
            Some(CHECK_COLOR),
            font.clone(),
        );
        Self {
            base: ScreenBase::new(Component::literal(&warning.narration_message())),
            warning,
            in_world,
            skip_warning_checked: false,
            proceed_button: Rc::new(RefCell::new(Self::build_button(
                &font,
                PROCEED_KEY,
                FooterPress::Proceed,
            ))),
            back_button: Rc::new(RefCell::new(Self::build_button(
                &font,
                BACK_KEY,
                FooterPress::Back,
            ))),
            font,
        }
    }
    fn build_button(
        font: &Arc<dyn FontWidthMeasurer>,
        key: &str,
        press: FooterPress,
    ) -> Button<FooterPress> {
        Button::<FooterPress>::builder(Component::literal(&crate::translate(key)))
            .with_action(press)
            .with_font(font.clone())
            .build()
    }
    pub fn take_action(&mut self) -> Option<SafetyScreenAction> {
        self.sync_checkbox();
        for button in [&self.proceed_button, &self.back_button] {
            let press = match button.try_borrow_mut() {
                Ok(mut button) => button.take_press_action(),
                Err(_) => None,
            };
            match press {
                Some(FooterPress::Proceed) => {
                    return Some(SafetyScreenAction::Proceed {
                        skip_warning: self.skip_warning_checked,
                    });
                }
                Some(FooterPress::Back) => return Some(SafetyScreenAction::Back),
                None => {}
            }
        }
        None
    }
    pub fn font(&self) -> &Arc<dyn FontWidthMeasurer> {
        &self.font
    }
    fn sync_checkbox(&mut self) {
        self.warning.drain_value_changed();
        self.skip_warning_checked = self.warning.is_checked();
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        let layout = self.warning.arrange(self.base.widgets_mut(), width, height);
        position_footer_button(
            &self.proceed_button,
            layout.first_button_x,
            layout.buttons_y,
        );
        position_footer_button(&self.back_button, layout.second_button_x, layout.buttons_y);
    }
}
impl std::fmt::Debug for SafetyScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SafetyScreen")
            .field("in_world", &self.in_world)
            .field("skip_warning_checked", &self.skip_warning_checked)
            .field("widgets", &self.base.widgets_len())
            .field("close_on_esc", &self.base.should_close_on_esc())
            .field("checked_key", &CHECK_KEY)
            .finish()
    }
}
impl Screen for SafetyScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        let title = self.warning.build_title();
        let message = self.warning.build_message(width, height);
        let checkbox = self.warning.checkbox();
        let title_handle = self.base.add_widget(title);
        let message_handle = self.base.add_widget(message);
        let checkbox_handle = match checkbox {
            Some(checkbox) => self.base.add_widget(SharedWidget::new(checkbox)),
            None => 0,
        };
        self.base
            .add_widget(SharedWidget::new(self.proceed_button.clone()));
        self.base
            .add_widget(SharedWidget::new(self.back_button.clone()));
        self.warning
            .set_handles(title_handle, message_handle, checkbox_handle);
        self.arrange();
    }
    fn reposition_elements(&mut self) {
        self.arrange();
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        let in_world = self.in_world;
        self.base().extract_background_layers(graphics, in_world);
        self.base()
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
    }
    fn mouse_clicked(&mut self, event: MouseButtonEvent, double_click: bool) -> bool {
        let handled = screen_mouse_clicked(self, event, double_click);
        self.sync_checkbox();
        handled
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        let handled = screen_key_pressed(self, event);
        self.sync_checkbox();
        handled
    }
}
