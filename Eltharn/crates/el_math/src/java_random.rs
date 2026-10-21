#[derive(Clone, Debug)]
pub struct JavaRandom {
    seed: u64,
}
impl JavaRandom {
    const MULTIPLIER: u64 = 0x5DEECE66D;
    const ADDEND: u64 = 0xB;
    const MASK: u64 = (1 << 48) - 1;
    pub fn new(seed: u64) -> Self {
        let mut rng = JavaRandom { seed: 0 };
        rng.set_seed(seed);
        rng
    }
    pub fn set_seed(&mut self, seed: u64) {
        self.seed = (seed ^ Self::MULTIPLIER) & Self::MASK;
    }
    pub fn next(&mut self, bits: u32) -> i32 {
        self.seed = (self
            .seed
            .wrapping_mul(Self::MULTIPLIER)
            .wrapping_add(Self::ADDEND))
            & Self::MASK;
        (self.seed >> (48 - bits)) as i32
    }
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }
    pub fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "bound must be positive");
        let n = bound as u32;
        if (n & n.wrapping_sub(1)) == 0 {
            return ((n as u64 * self.next(31) as u64) >> 31) as i32;
        }
        let mut bits;
        let mut val;
        loop {
            bits = self.next(31);
            val = bits % bound;
            if bits.wrapping_sub(val).wrapping_add(bound - 1) >= 0 {
                break;
            }
        }
        val
    }
    pub fn next_long(&mut self) -> i64 {
        let high = (self.next(32) as i64) << 32;
        let low = self.next(32) as i64;
        high.wrapping_add(low)
    }
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }
    pub fn next_float(&mut self) -> f32 {
        self.next(24) as f32 / ((1 << 24) as f32)
    }
    pub fn next_double(&mut self) -> f64 {
        let high = (self.next(26) as i64) << 27;
        let low = self.next(27) as i64;
        (high.wrapping_add(low)) as f64 / ((1u64 << 53) as f64)
    }
}
pub fn java_string_hash(value: &str) -> i32 {
    let mut hash = 0i32;
    for unit in value.encode_utf16() {
        hash = hash.wrapping_mul(31).wrapping_add(unit as i32);
    }
    hash
}
