use el_core::item::ItemStack;
use std::sync::LazyLock;

static EMPTY_STACK: LazyLock<ItemStack> = LazyLock::new(ItemStack::empty);

pub trait Container {
    fn get_container_size(&self) -> usize;

    fn get_item(&self, slot: usize) -> &ItemStack;

    fn set_item(&mut self, slot: usize, stack: ItemStack);

    fn get_max_stack_size(&self) -> i32 {
        99
    }

    fn get_max_stack_size_for(&self, stack: &ItemStack) -> i32 {
        self.get_max_stack_size().min(stack.get_max_stack_size())
    }

    fn can_place_item(&self, _slot: usize, _stack: &ItemStack) -> bool {
        true
    }

    fn is_empty(&self) -> bool {
        (0..self.get_container_size()).all(|slot| self.get_item(slot).is_empty())
    }

    fn remove_item(&mut self, slot: usize, amount: i32) -> ItemStack {
        if slot >= self.get_container_size() || amount <= 0 {
            return ItemStack::empty();
        }

        let mut current = self.get_item(slot).clone();
        if current.is_empty() {
            return ItemStack::empty();
        }

        let removed = current.split(amount);
        if current.count <= 0 {
            self.set_item(slot, ItemStack::empty());
        } else {
            self.set_item(slot, current);
        }
        self.set_changed();
        removed
    }

    fn remove_item_no_update(&mut self, slot: usize) -> ItemStack {
        if slot >= self.get_container_size() {
            return ItemStack::empty();
        }
        let current = self.get_item(slot).clone();
        self.set_item(slot, ItemStack::empty());
        current
    }

    fn set_changed(&mut self) {}
}

#[derive(Clone, Debug, PartialEq)]
pub struct SimpleContainer {
    items: Vec<ItemStack>,
    changed: bool,
    max_stack_size: i32,
    limit_stacks_on_set: bool,
}

impl SimpleContainer {
    pub fn new(size: usize) -> Self {
        Self {
            items: vec![ItemStack::empty(); size],
            changed: false,
            max_stack_size: 99,
            limit_stacks_on_set: true,
        }
    }

    pub fn new_player_inventory(size: usize) -> Self {
        Self {
            limit_stacks_on_set: false,
            ..Self::new(size)
        }
    }

    pub fn from_items(items: Vec<ItemStack>) -> Self {
        Self {
            items,
            changed: false,
            max_stack_size: 99,
            limit_stacks_on_set: true,
        }
    }

    pub fn with_max_stack_size(mut self, max_stack_size: i32) -> Self {
        self.max_stack_size = max_stack_size;
        self
    }

    pub fn items(&self) -> &[ItemStack] {
        &self.items
    }

    pub fn items_mut(&mut self) -> &mut [ItemStack] {
        &mut self.items
    }

    pub fn has_changed(&self) -> bool {
        self.changed
    }

    pub fn clear_changed(&mut self) {
        self.changed = false;
    }

    pub fn clear_content(&mut self) {
        for item in &mut self.items {
            *item = ItemStack::empty();
        }
        self.set_changed();
    }
}

impl Container for SimpleContainer {
    fn get_container_size(&self) -> usize {
        self.items.len()
    }

    fn get_item(&self, slot: usize) -> &ItemStack {
        self.items.get(slot).unwrap_or(&EMPTY_STACK)
    }

    fn set_item(&mut self, slot: usize, mut stack: ItemStack) {
        if slot >= self.items.len() {
            return;
        }
        if self.limit_stacks_on_set {
            let max = self.get_max_stack_size_for(&stack);
            if stack.count > max {
                stack.count = max;
            }
            if stack.count <= 0 {
                stack = ItemStack::empty();
            }
        }
        self.items[slot] = stack;
        self.set_changed();
    }

    fn get_max_stack_size(&self) -> i32 {
        self.max_stack_size
    }

    fn set_changed(&mut self) {
        self.changed = true;
    }
}
