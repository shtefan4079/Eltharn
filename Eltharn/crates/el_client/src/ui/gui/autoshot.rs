use std::path::PathBuf;
#[derive(Clone, Debug, PartialEq)]
pub struct AutoShot {
    pub screen: String,
    pub out: PathBuf,
    pub frames: u32,
    pub in_world: bool,
    pub keep_running: bool,
    pub open_after: u32,
    pub reload_after: Option<u32>,
    pub mouse: Option<(f32, f32)>,
}
impl AutoShot {
    #[cfg(not(target_arch = "wasm32"))]
    pub const SCREENS: &'static [&'static str] = &[
        "none",
        "title",
        "pause",
        "custom_pause",
        "pack_prompt",
        "pack_prompt_optional",
        "options",
        "video",
        "controls",
        "keybinds",
        "multiplayer",
        "resourcepacks",
        "chat",
        "chat_hover_text",
        "chat_hover_item",
        "chat_hover_entity",
        "language",
        "font",
        "sound",
        "chat_options",
        "accessibility",
        "mouse",
        "skin",
        "world_options",
        "credits",
        "credits_roll",
        "end_poem",
        "inventory",
        "mods",
        "shaderpacks",
        "mod-settings",
    ];
    #[cfg(not(target_arch = "wasm32"))]
    pub fn from_env() -> Option<Self> {
        Self::parse(&std::env::var("ELTHARN_AUTOSHOT").ok()?)
    }
    #[cfg(target_arch = "wasm32")]
    pub fn from_env() -> Option<Self> {
        None
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn parse(spec: &str) -> Option<Self> {
        let mut screen = None;
        let mut out = None;
        let mut frames = 90u32;
        let mut in_world = false;
        let mut keep_running = false;
        let mut open_after = 0;
        let mut reload_after = None;
        let mut mouse = None;
        for field in spec.split(',') {
            let field = field.trim();
            if field.is_empty() {
                continue;
            }
            let (key, value) = field.split_once('=')?;
            match key.trim() {
                "screen" => screen = Some(value.trim().to_string()),
                "out" => out = Some(PathBuf::from(value.trim())),
                "frames" => frames = value.trim().parse().ok()?,
                "in_world" => in_world = matches!(value.trim(), "1" | "true" | "yes"),
                "keep" => keep_running = matches!(value.trim(), "1" | "true" | "yes"),
                "open_after" => open_after = value.trim().parse().ok()?,
                "reload_after" => reload_after = Some(value.trim().parse().ok()?),
                "mouse" => {
                    let (x, y) = value.trim().split_once(':')?;
                    mouse = Some((x.trim().parse().ok()?, y.trim().parse().ok()?));
                }
                _ => return None,
            }
        }
        let screen = screen?;
        if !Self::SCREENS.contains(&screen.as_str()) {
            return None;
        }
        Some(Self {
            screen,
            out: out?,
            frames,
            in_world,
            keep_running,
            open_after,
            reload_after,
            mouse,
        })
    }
}
#[derive(Clone, Debug)]
pub struct AutoShotRunner {
    pub config: AutoShot,
    rendered_frames: u32,
    screen_applied: bool,
    captured: bool,
    reload_requested: bool,
}
impl AutoShotRunner {
    pub fn new(config: AutoShot) -> Self {
        Self {
            config,
            rendered_frames: 0,
            screen_applied: false,
            captured: false,
            reload_requested: false,
        }
    }
    pub fn take_screen_request(&mut self) -> Option<(String, bool)> {
        if self.screen_applied || self.rendered_frames < self.config.open_after {
            return None;
        }
        self.screen_applied = true;
        Some((self.config.screen.clone(), self.config.in_world))
    }
    pub fn take_reload_request(&mut self) -> bool {
        if self.reload_requested
            || !self
                .config
                .reload_after
                .is_some_and(|frame| self.rendered_frames >= frame)
        {
            return false;
        }
        self.reload_requested = true;
        true
    }
    pub fn tick(&mut self) -> Option<PathBuf> {
        if self.captured {
            return None;
        }
        self.rendered_frames += 1;
        if self.rendered_frames < self.config.frames {
            return None;
        }
        self.captured = true;
        Some(self.config.out.clone())
    }
    pub fn finished(&self) -> bool {
        self.captured
    }
}
