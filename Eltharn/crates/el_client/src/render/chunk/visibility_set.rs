use el_math::Direction;
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct VisibilitySet {
    data: u64,
}
impl VisibilitySet {
    const FACINGS: usize = 6;
    #[inline]
    fn bit(a: Direction, b: Direction) -> u64 {
        1u64 << (a.data_3d() as usize + b.data_3d() as usize * Self::FACINGS)
    }
    pub fn add(&mut self, directions: &[Direction]) {
        for &a in directions {
            for &b in directions {
                self.set(a, b, true);
            }
        }
    }
    pub fn set(&mut self, a: Direction, b: Direction, visible: bool) {
        let mask = Self::bit(a, b) | Self::bit(b, a);
        if visible {
            self.data |= mask;
        } else {
            self.data &= !mask;
        }
    }
    pub fn set_all(&mut self, visible: bool) {
        self.data = if visible { (1u64 << 36) - 1 } else { 0 };
    }
    #[cfg(not(target_arch = "wasm32"))]
    pub fn visibility_between(self, a: Direction, b: Direction) -> bool {
        self.data & Self::bit(a, b) != 0
    }
}
