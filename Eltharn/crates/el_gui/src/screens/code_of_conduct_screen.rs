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
pub const TITLE_KEY: &str = "multiplayer.codeOfConduct.title";
pub const CHECK_KEY: &str = "multiplayer.codeOfConduct.check";
pub const ACKNOWLEDGE_KEY: &str = "gui.acknowledge";
pub const DISCONNECT_KEY: &str = "menu.disconnect";
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeOfConductAction {
    Acknowledge { skip_next_time: bool },
    Disconnect,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FooterPress {
    Acknowledge,
    Disconnect,
}
pub struct CodeOfConductScreen {
    base: ScreenBase,
    warning: WarningScreen,
    pub in_world: bool,
    pub code_of_conduct_text: String,
    pub skip_next_time_checked: bool,
    font: Arc<dyn FontWidthMeasurer>,
    acknowledge_button: Rc<RefCell<Button<FooterPress>>>,
    disconnect_button: Rc<RefCell<Button<FooterPress>>>,
}
impl CodeOfConductScreen {
    pub fn new(
        in_world: bool,
        code_of_conduct_text: String,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let warning = WarningScreen::new(
            TITLE_KEY,
            code_of_conduct_text.clone(),
            Some(CHECK_KEY),
            None,
            font.clone(),
        );
        let mut base = ScreenBase::new(Component::literal(&warning.narration_message()));
        base.set_close_on_esc(false);
        Self {
            base,
            warning,
            in_world,
            code_of_conduct_text,
            skip_next_time_checked: false,
            acknowledge_button: Rc::new(RefCell::new(Self::build_button(
                &font,
                ACKNOWLEDGE_KEY,
                FooterPress::Acknowledge,
            ))),
            disconnect_button: Rc::new(RefCell::new(Self::build_button(
                &font,
                DISCONNECT_KEY,
                FooterPress::Disconnect,
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
    pub fn take_action(&mut self) -> Option<CodeOfConductAction> {
        self.sync_checkbox();
        for button in [&self.acknowledge_button, &self.disconnect_button] {
            let press = match button.try_borrow_mut() {
                Ok(mut button) => button.take_press_action(),
                Err(_) => None,
            };
            match press {
                Some(FooterPress::Acknowledge) => {
                    return Some(CodeOfConductAction::Acknowledge {
                        skip_next_time: self.skip_next_time_checked,
                    });
                }
                Some(FooterPress::Disconnect) => {
                    return Some(CodeOfConductAction::Disconnect);
                }
                None => {}
            }
        }
        None
    }
    pub fn should_close_on_esc(&self) -> bool {
        self.base_ref().should_close_on_esc()
    }
    pub fn font(&self) -> &Arc<dyn FontWidthMeasurer> {
        &self.font
    }
    fn sync_checkbox(&mut self) {
        self.warning.drain_value_changed();
        self.skip_next_time_checked = self.warning.is_checked();
    }
    fn arrange(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        let layout = self.warning.arrange(self.base.widgets_mut(), width, height);
        position_footer_button(
            &self.acknowledge_button,
            layout.first_button_x,
            layout.buttons_y,
        );
        position_footer_button(
            &self.disconnect_button,
            layout.second_button_x,
            layout.buttons_y,
        );
    }
}
impl std::fmt::Debug for CodeOfConductScreen {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CodeOfConductScreen")
            .field("in_world", &self.in_world)
            .field("skip_next_time_checked", &self.skip_next_time_checked)
            .field("widgets", &self.base.widgets_len())
            .field("close_on_esc", &self.base.should_close_on_esc())
            .field("text_len", &self.code_of_conduct_text.chars().count())
            .finish()
    }
}
impl Screen for CodeOfConductScreen {
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
            .add_widget(SharedWidget::new(self.acknowledge_button.clone()));
        self.base
            .add_widget(SharedWidget::new(self.disconnect_button.clone()));
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
        self.base_ref()
            .extract_background_layers(graphics, in_world);
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
