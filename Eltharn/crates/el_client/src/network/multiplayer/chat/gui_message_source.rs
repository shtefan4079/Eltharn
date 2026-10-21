#![allow(dead_code)]

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GuiMessageSource {
    Player,
    SystemServer,
    SystemClient,
}
