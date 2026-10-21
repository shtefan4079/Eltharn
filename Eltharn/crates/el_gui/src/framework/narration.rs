use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NarratedElementType {
    Title,
    Position,
    Hint,
    Usage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum NarrationPriority {
    None,
    Hovered,
    Focused,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NarrationElementOutput {
    elements: BTreeMap<NarratedElementType, Vec<String>>,
}

impl NarrationElementOutput {
    pub fn add(&mut self, ty: NarratedElementType, message: impl Into<String>) {
        self.elements.entry(ty).or_default().push(message.into());
    }

    pub fn get(&self, ty: NarratedElementType) -> &[String] {
        self.elements.get(&ty).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn is_empty(&self) -> bool {
        self.elements.values().all(Vec::is_empty)
    }
}

pub trait NarratableEntry {
    fn narration_priority(&self) -> NarrationPriority {
        NarrationPriority::None
    }

    fn is_active(&self) -> bool {
        true
    }

    fn update_narration(&self, _output: &mut NarrationElementOutput) {}
}
