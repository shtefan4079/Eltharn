use crate::FontWidthMeasurer;
use crate::components::Button;
use crate::framework::graphics::GuiGraphicsExtractor;
use crate::framework::navigation::ScreenRectangle;
use crate::framework::screen::{Screen, ScreenBase, screen_init, screen_key_pressed};
use crate::input::events::KeyEvent;
use crate::screens::confirm_screen::SharedButton;
use el_core::{Component, ResourceLocation};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
const TITLE_KEY: &str = "menu.server_links.title";
const BACK_KEY: &str = "gui.back";
const PREVIOUS_KEY: &str = "gui.previous";
const NEXT_KEY: &str = "gui.next";
const PAGE_SIZE: usize = 6;
const ROW_HEIGHT: i32 = 20;
const ROW_GAP: i32 = 4;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ServerLinksScreenAction {
    OpenLink(String),
    Done,
}
#[derive(Clone, Debug, PartialEq, Eq)]
enum ButtonAction {
    OpenLink(String),
    PreviousPage,
    NextPage,
    Done,
}
pub struct ServerLinksScreen {
    base: ScreenBase,
    in_world: bool,
    links: Vec<(String, String)>,
    page: usize,
    font: Arc<dyn FontWidthMeasurer>,
    buttons: Vec<Rc<RefCell<Button<ButtonAction>>>>,
    last_width: i32,
    last_height: i32,
}
impl ServerLinksScreen {
    pub fn new(
        in_world: bool,
        links: Vec<(String, String)>,
        font: Arc<dyn FontWidthMeasurer>,
    ) -> Self {
        let title = translated(TITLE_KEY, "Server Links");
        let mut base = ScreenBase::new(Component::literal(&title));
        base.set_close_on_esc(true);
        Self {
            base,
            in_world,
            links: links.into_iter().take(128).collect(),
            page: 0,
            font,
            buttons: Vec::new(),
            last_width: 0,
            last_height: 0,
        }
    }
    pub fn in_world(&self) -> bool {
        self.in_world
    }
    pub fn page(&self) -> usize {
        self.page
    }
    pub fn page_count(&self) -> usize {
        self.links.len().max(1).div_ceil(self.page_size())
    }
    fn page_size(&self) -> usize {
        let height = self.base.height;
        if height <= 0 {
            return PAGE_SIZE;
        }
        let footer_y = (height - 26).max(4);
        let rows_that_fit_without_pager =
            ((footer_y - 39 + ROW_GAP) / (ROW_HEIGHT + ROW_GAP)).clamp(1, PAGE_SIZE as i32);
        if self.links.len() <= rows_that_fit_without_pager as usize {
            return rows_that_fit_without_pager as usize;
        }
        let pager_y = footer_y - ROW_HEIGHT - ROW_GAP;
        ((pager_y - 39).max(ROW_HEIGHT) / (ROW_HEIGHT + ROW_GAP)).clamp(1, PAGE_SIZE as i32)
            as usize
    }
    pub fn button_rectangle(&self, visible_index: usize) -> Option<ScreenRectangle> {
        self.buttons
            .get(visible_index)
            .map(|button| crate::framework::layouts::LayoutElement::rectangle(&*button.borrow()))
    }
    pub fn take_action(&mut self) -> Option<ServerLinksScreenAction> {
        if self.base.take_close_request() {
            return Some(ServerLinksScreenAction::Done);
        }
        for button in &self.buttons {
            let action = button.borrow_mut().take_press_action();
            match action {
                Some(ButtonAction::OpenLink(url)) => {
                    return Some(ServerLinksScreenAction::OpenLink(url));
                }
                Some(ButtonAction::PreviousPage) => {
                    self.page = self.page.saturating_sub(1);
                    self.rebuild();
                    return None;
                }
                Some(ButtonAction::NextPage) => {
                    self.page = (self.page + 1).min(self.page_count().saturating_sub(1));
                    self.rebuild();
                    return None;
                }
                Some(ButtonAction::Done) => return Some(ServerLinksScreenAction::Done),
                None => {}
            }
        }
        None
    }
    fn rebuild(&mut self) {
        let width = self.base.width;
        let height = self.base.height;
        if width > 0 && height > 0 {
            screen_init(self, width, height);
        }
    }
    fn add_button(&mut self, label: String, action: ButtonAction, x: i32, y: i32, width: i32) {
        let button = Rc::new(RefCell::new(
            Button::<ButtonAction>::builder(Component::literal(&label))
                .bounds(x, y, width, ROW_HEIGHT)
                .with_action(action)
                .with_font(Arc::clone(&self.font))
                .build(),
        ));
        self.base.add_widget(SharedButton::new(Rc::clone(&button)));
        self.buttons.push(button);
    }
    fn init_widgets(&mut self) {
        self.base.clear_widgets();
        self.buttons.clear();
        let width = self.base.width;
        let height = self.base.height;
        self.last_width = width;
        self.last_height = height;
        if width <= 0 || height <= 0 {
            return;
        }
        let footer_y = (height - 26).max(4);
        let pager_y = footer_y - ROW_HEIGHT - ROW_GAP;
        let page_size = self.page_size();
        let page_count = self.links.len().max(1).div_ceil(page_size);
        self.page = self.page.min(page_count.saturating_sub(1));
        let first = self.page * page_size;
        let last = (first + page_size).min(self.links.len());
        let button_width = 310.min((width - 24).max(100));
        let x = (width - button_width) / 2;
        let page_links = self.links[first..last].to_vec();
        for (row, (label, url)) in page_links.iter().enumerate() {
            let y = 39 + row as i32 * (ROW_HEIGHT + ROW_GAP);
            self.add_button(
                server_link_label(label),
                ButtonAction::OpenLink(url.clone()),
                x,
                y,
                button_width,
            );
        }
        if page_count > 1 {
            let pager_width = 200.min(button_width);
            let pager_x = (width - pager_width) / 2;
            let half = ((pager_width - 4) / 2).max(1);
            self.add_button(
                translated(PREVIOUS_KEY, "Previous"),
                ButtonAction::PreviousPage,
                pager_x,
                pager_y,
                half,
            );
            self.add_button(
                translated(NEXT_KEY, "Next"),
                ButtonAction::NextPage,
                pager_x + half + 4,
                pager_y,
                pager_width - half - 4,
            );
        }
        self.add_button(
            translated(BACK_KEY, "Back"),
            ButtonAction::Done,
            (width - 200) / 2,
            footer_y,
            200.min(width.saturating_sub(8)),
        );
    }
    fn render_title(&self, graphics: &mut GuiGraphicsExtractor) {
        let title = translated(TITLE_KEY, "Server Links");
        let width = self.font.measure_width(&title, 1.0).round() as i32;
        graphics.text(
            ResourceLocation::from_string("minecraft:default"),
            title,
            (self.base.width - width) / 2,
            12,
            0xFFFF_FFFFu32 as i32,
        );
    }
}
impl Screen for ServerLinksScreen {
    fn base(&mut self) -> &mut ScreenBase {
        &mut self.base
    }
    fn base_ref(&self) -> &ScreenBase {
        &self.base
    }
    fn init(&mut self) {
        self.init_widgets();
    }
    fn reposition_elements(&mut self) {
        self.init_widgets();
    }
    fn on_close(&mut self) {
        self.base.request_close();
    }
    fn key_pressed(&mut self, event: KeyEvent) -> bool {
        screen_key_pressed(self, event)
    }
    fn extract_render_state(
        &mut self,
        graphics: &mut GuiGraphicsExtractor,
        mouse_x: i32,
        mouse_y: i32,
        partial_tick: f32,
    ) {
        self.base_ref()
            .extract_background_layers(graphics, self.in_world);
        self.base
            .extract_render_state(graphics, mouse_x, mouse_y, partial_tick);
        self.render_title(graphics);
    }
}
pub fn server_link_label(raw: &str) -> String {
    let (key, fallback) = match raw {
        "BugReport" => ("known_server_link.report_bug", "Report Server Bug"),
        "CommunityGuidelines" => (
            "known_server_link.community_guidelines",
            "Community Guidelines",
        ),
        "Support" => ("known_server_link.support", "Support"),
        "Status" => ("known_server_link.status", "Status"),
        "Feedback" => ("known_server_link.feedback", "Feedback"),
        "Community" => ("known_server_link.community", "Community"),
        "Website" => ("known_server_link.website", "Website"),
        "Forums" => ("known_server_link.forums", "Forums"),
        "News" => ("known_server_link.news", "News"),
        "Announcements" => ("known_server_link.announcements", "Announcements"),
        _ => return raw.to_string(),
    };
    translated(key, fallback)
}
fn translated(key: &str, fallback: &str) -> String {
    let value = crate::translate(key);
    if value == key {
        fallback.to_string()
    } else {
        value
    }
}
