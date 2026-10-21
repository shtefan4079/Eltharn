#![allow(dead_code)]

use std::collections::HashSet;

pub struct ClientSuggestionProvider {
    pub custom_completion_suggestions: HashSet<String>,
    pub pending_suggestions_id: i32,
}

impl Default for ClientSuggestionProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl ClientSuggestionProvider {
    pub fn new() -> Self {
        Self {
            custom_completion_suggestions: HashSet::new(),
            pending_suggestions_id: -1,
        }
    }

    pub fn get_online_player_names(&self) -> Vec<String> {
        Vec::new()
    }

    pub fn get_custom_tab_suggestions(&self) -> Vec<String> {
        self.custom_completion_suggestions.iter().cloned().collect()
    }

    pub fn clear(&mut self) {
        self.custom_completion_suggestions.clear();
        self.pending_suggestions_id = -1;
    }
}
