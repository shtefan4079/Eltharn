use crate::java_random::java_string_hash;
use std::sync::Mutex;
use std::sync::atomic::{AtomicI64, Ordering};

pub trait RandomSource {
    fn next_bits(&mut self, bits: u32) -> i32;
    fn fork(&mut self) -> Box<dyn RandomSource + Send>;
    fn fork_positional(&mut self) -> Box<dyn PositionalRandomFactory + Send + Sync>;
    fn set_seed(&mut self, seed: i64);
    fn next_int(&mut self) -> i32;
    fn next_int_bound(&mut self, bound: i32) -> i32;
    fn next_long(&mut self) -> i64;
    fn next_boolean(&mut self) -> bool;
    fn next_float(&mut self) -> f32;
    fn next_double(&mut self) -> f64;
    fn next_gaussian(&mut self) -> f64;

    fn next_int_between_inclusive(&mut self, min: i32, max_inclusive: i32) -> i32 {
        self.next_int_bound(max_inclusive - min + 1) + min
    }

    fn triangle_double(&mut self, mean: f64, spread: f64) -> f64 {
        mean + spread * (self.next_double() - self.next_double())
    }

    fn triangle_float(&mut self, mean: f32, spread: f32) -> f32 {
        mean + spread * (self.next_float() - self.next_float())
    }

    fn consume_count(&mut self, count: usize) {
        for _ in 0..count {
            self.next_int();
        }
    }

    fn next_int_range(&mut self, origin: i32, bound: i32) -> i32 {
        assert!(origin < bound, "bound - origin is non positive");
        origin + self.next_int_bound(bound - origin)
    }

    fn is_legacy_random_source(&self) -> bool {
        false
    }
}

pub trait PositionalRandomFactory {
    fn at(&self, x: i32, y: i32, z: i32) -> Box<dyn RandomSource + Send>;
    fn from_hash_of(&self, name: &str) -> Box<dyn RandomSource + Send>;
    fn from_seed(&self, seed: i64) -> Box<dyn RandomSource + Send>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct GaussianSource {
    next_next_gaussian: f64,
    have_next_next_gaussian: bool,
}

impl GaussianSource {
    pub fn new() -> Self {
        GaussianSource {
            next_next_gaussian: 0.0,
            have_next_next_gaussian: false,
        }
    }

    pub fn reset(&mut self) {
        self.have_next_next_gaussian = false;
    }
}

pub struct LegacyRandomSource {
    seed: AtomicI64,
    gaussian: Mutex<GaussianSource>,
}

impl LegacyRandomSource {
    pub fn new(seed: i64) -> Self {
        let mut source = LegacyRandomSource {
            seed: AtomicI64::new(0),
            gaussian: Mutex::new(GaussianSource::new()),
        };
        source.set_seed(seed);
        source
    }

    fn next_bits_impl(&self, bits: u32) -> i32 {
        let old_seed = self.seed.load(Ordering::SeqCst);
        let new_seed = old_seed.wrapping_mul(25214903917).wrapping_add(11) & 281474976710655;
        if self
            .seed
            .compare_exchange(old_seed, new_seed, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            panic!("LegacyRandomSource concurrent modification detected");
        }
        (new_seed >> (48 - bits)) as i32
    }

    fn next_long_impl(&self) -> i64 {
        let upper = self.next_bits_impl(32) as i64;
        let lower = self.next_bits_impl(32) as i64;
        let shifted = upper << 32;
        shifted + lower
    }
}

impl RandomSource for LegacyRandomSource {
    fn next_bits(&mut self, bits: u32) -> i32 {
        self.next_bits_impl(bits)
    }

    fn fork(&mut self) -> Box<dyn RandomSource + Send> {
        Box::new(LegacyRandomSource::new(self.next_long()))
    }

    fn fork_positional(&mut self) -> Box<dyn PositionalRandomFactory + Send + Sync> {
        let seed = self.next_long_impl();
        Box::new(LegacyPositionalRandomFactory { seed })
    }

    fn set_seed(&mut self, seed: i64) {
        let val = (seed ^ 25214903917) & 281474976710655;
        self.seed.store(val, Ordering::SeqCst);
        self.gaussian.lock().unwrap().reset();
    }

    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return ((bound as i64 * self.next_bits(31) as i64) >> 31) as i32;
        }
        let mut sample;
        let mut modulo;
        loop {
            sample = self.next_bits(31);
            modulo = sample % bound;
            if sample - modulo + (bound - 1) >= 0 {
                break;
            }
        }
        modulo
    }

    fn next_long(&mut self) -> i64 {
        self.next_long_impl()
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.9604645E-8
    }

    fn next_double(&mut self) -> f64 {
        let upper = self.next_bits(26) as i64;
        let lower = self.next_bits(27) as i64;
        let combined = (upper << 27) + lower;
        combined as f64 * 1.1102230246251565E-16
    }

    fn next_gaussian(&mut self) -> f64 {
        let mut g = { *self.gaussian.lock().unwrap() };
        if g.have_next_next_gaussian {
            g.have_next_next_gaussian = false;
            *self.gaussian.lock().unwrap() = g;
            return g.next_next_gaussian;
        }

        let mut x;
        let mut y;
        let mut radius_squared;
        loop {
            x = 2.0 * self.next_double() - 1.0;
            y = 2.0 * self.next_double() - 1.0;
            radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                break;
            }
        }

        let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
        g.next_next_gaussian = y * multiplier;
        g.have_next_next_gaussian = true;
        *self.gaussian.lock().unwrap() = g;
        x * multiplier
    }

    fn is_legacy_random_source(&self) -> bool {
        true
    }
}

pub struct LegacyPositionalRandomFactory {
    seed: i64,
}

impl PositionalRandomFactory for LegacyPositionalRandomFactory {
    fn at(&self, x: i32, y: i32, z: i32) -> Box<dyn RandomSource + Send> {
        let positional_seed = crate::mth::get_seed(x, y, z);
        let random_seed = positional_seed ^ self.seed;
        Box::new(LegacyRandomSource::new(random_seed))
    }

    fn from_hash_of(&self, name: &str) -> Box<dyn RandomSource + Send> {
        let positional_seed = java_string_hash(name);
        Box::new(LegacyRandomSource::new(
            (positional_seed as i64) ^ self.seed,
        ))
    }

    fn from_seed(&self, seed: i64) -> Box<dyn RandomSource + Send> {
        Box::new(LegacyRandomSource::new(seed))
    }
}

pub struct SingleThreadedRandomSource {
    seed: i64,
    gaussian: GaussianSource,
}

impl SingleThreadedRandomSource {
    pub fn new(seed: i64) -> Self {
        let mut source = SingleThreadedRandomSource {
            seed: 0,
            gaussian: GaussianSource::new(),
        };
        source.set_seed(seed);
        source
    }
}

impl RandomSource for SingleThreadedRandomSource {
    fn next_bits(&mut self, bits: u32) -> i32 {
        let new_seed = self.seed.wrapping_mul(25214903917).wrapping_add(11) & 281474976710655;
        self.seed = new_seed;
        (new_seed >> (48 - bits)) as i32
    }

    fn fork(&mut self) -> Box<dyn RandomSource + Send> {
        Box::new(SingleThreadedRandomSource::new(self.next_long()))
    }

    fn fork_positional(&mut self) -> Box<dyn PositionalRandomFactory + Send + Sync> {
        let seed = self.next_long();
        Box::new(LegacyPositionalRandomFactory { seed })
    }

    fn set_seed(&mut self, seed: i64) {
        self.seed = (seed ^ 25214903917) & 281474976710655;
        self.gaussian.reset();
    }

    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return ((bound as i64 * self.next_bits(31) as i64) >> 31) as i32;
        }
        let mut sample;
        let mut modulo;
        loop {
            sample = self.next_bits(31);
            modulo = sample % bound;
            if sample - modulo + (bound - 1) >= 0 {
                break;
            }
        }
        modulo
    }

    fn next_long(&mut self) -> i64 {
        let upper = self.next_bits(32) as i64;
        let lower = self.next_bits(32) as i64;
        let shifted = upper << 32;
        shifted + lower
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.9604645E-8
    }

    fn next_double(&mut self) -> f64 {
        let upper = self.next_bits(26) as i64;
        let lower = self.next_bits(27) as i64;
        let combined = (upper << 27) + lower;
        combined as f64 * 1.1102230246251565E-16
    }

    fn next_gaussian(&mut self) -> f64 {
        let mut g = self.gaussian;
        if g.have_next_next_gaussian {
            g.have_next_next_gaussian = false;
            self.gaussian = g;
            return g.next_next_gaussian;
        }

        let mut x;
        let mut y;
        let mut radius_squared;
        loop {
            x = 2.0 * self.next_double() - 1.0;
            y = 2.0 * self.next_double() - 1.0;
            radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                break;
            }
        }

        let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
        g.next_next_gaussian = y * multiplier;
        g.have_next_next_gaussian = true;
        self.gaussian = g;
        x * multiplier
    }
}

pub struct ThreadSafeLegacyRandomSource {
    seed: AtomicI64,
    gaussian: Mutex<GaussianSource>,
}

impl ThreadSafeLegacyRandomSource {
    pub fn new(seed: i64) -> Self {
        let mut source = ThreadSafeLegacyRandomSource {
            seed: AtomicI64::new(0),
            gaussian: Mutex::new(GaussianSource::new()),
        };
        source.set_seed(seed);
        source
    }

    fn next_bits_impl(&self, bits: u32) -> i32 {
        let mut old_seed = self.seed.load(Ordering::SeqCst);
        loop {
            let new_seed = old_seed.wrapping_mul(25214903917).wrapping_add(11) & 281474976710655;
            match self.seed.compare_exchange_weak(
                old_seed,
                new_seed,
                Ordering::SeqCst,
                Ordering::SeqCst,
            ) {
                Ok(_) => {
                    return (new_seed >> (48 - bits)) as i32;
                }
                Err(actual) => {
                    old_seed = actual;
                }
            }
        }
    }

    fn next_long_impl(&self) -> i64 {
        let upper = self.next_bits_impl(32) as i64;
        let lower = self.next_bits_impl(32) as i64;
        let shifted = upper << 32;
        shifted + lower
    }
}

impl RandomSource for ThreadSafeLegacyRandomSource {
    fn next_bits(&mut self, bits: u32) -> i32 {
        self.next_bits_impl(bits)
    }

    fn fork(&mut self) -> Box<dyn RandomSource + Send> {
        Box::new(ThreadSafeLegacyRandomSource::new(self.next_long()))
    }

    fn fork_positional(&mut self) -> Box<dyn PositionalRandomFactory + Send + Sync> {
        let seed = self.next_long_impl();
        Box::new(LegacyPositionalRandomFactory { seed })
    }

    fn set_seed(&mut self, seed: i64) {
        let val = (seed ^ 25214903917) & 281474976710655;
        self.seed.store(val, Ordering::SeqCst);
        self.gaussian.lock().unwrap().reset();
    }

    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return ((bound as i64 * self.next_bits(31) as i64) >> 31) as i32;
        }
        let mut sample;
        let mut modulo;
        loop {
            sample = self.next_bits(31);
            modulo = sample % bound;
            if sample - modulo + (bound - 1) >= 0 {
                break;
            }
        }
        modulo
    }

    fn next_long(&mut self) -> i64 {
        self.next_long_impl()
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.9604645E-8
    }

    fn next_double(&mut self) -> f64 {
        let upper = self.next_bits(26) as i64;
        let lower = self.next_bits(27) as i64;
        let combined = (upper << 27) + lower;
        combined as f64 * 1.1102230246251565E-16
    }

    fn next_gaussian(&mut self) -> f64 {
        let mut g = { *self.gaussian.lock().unwrap() };
        if g.have_next_next_gaussian {
            g.have_next_next_gaussian = false;
            *self.gaussian.lock().unwrap() = g;
            return g.next_next_gaussian;
        }

        let mut x;
        let mut y;
        let mut radius_squared;
        loop {
            x = 2.0 * self.next_double() - 1.0;
            y = 2.0 * self.next_double() - 1.0;
            radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                break;
            }
        }

        let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
        g.next_next_gaussian = y * multiplier;
        g.have_next_next_gaussian = true;
        *self.gaussian.lock().unwrap() = g;
        x * multiplier
    }
}

#[derive(Clone, Debug)]
pub struct Xoroshiro128PlusPlus {
    seed_lo: u64,
    seed_hi: u64,
}

impl Xoroshiro128PlusPlus {
    pub fn new(seed_lo: u64, seed_hi: u64) -> Self {
        let mut xoroshiro = Xoroshiro128PlusPlus { seed_lo, seed_hi };
        if (xoroshiro.seed_lo | xoroshiro.seed_hi) == 0 {
            xoroshiro.seed_lo = 0x9E3779B97F4A7C15;
            xoroshiro.seed_hi = 0x6A09E667F3BCC909;
        }
        xoroshiro
    }

    pub fn new_from_long(seed: i64) -> Self {
        let seed_128 = upgrade_seed_to_128bit(seed);
        Xoroshiro128PlusPlus::new(seed_128.lo, seed_128.hi)
    }

    pub fn next_long(&mut self) -> i64 {
        let s0 = self.seed_lo;
        let s1 = self.seed_hi;
        let result = (s0.wrapping_add(s1)).rotate_left(17).wrapping_add(s0);
        let next_s1 = s1 ^ s0;
        self.seed_lo = s0.rotate_left(49) ^ next_s1 ^ (next_s1 << 21);
        self.seed_hi = next_s1.rotate_left(28);
        result as i64
    }
}

pub struct XoroshiroRandomSource {
    rng: Xoroshiro128PlusPlus,
    gaussian: GaussianSource,
}

impl XoroshiroRandomSource {
    pub fn new(seed: i64) -> Self {
        XoroshiroRandomSource {
            rng: Xoroshiro128PlusPlus::new_from_long(seed),
            gaussian: GaussianSource::new(),
        }
    }

    pub fn new_from_seed128(seed: Seed128bit) -> Self {
        XoroshiroRandomSource {
            rng: Xoroshiro128PlusPlus::new(seed.lo, seed.hi),
            gaussian: GaussianSource::new(),
        }
    }

    pub fn new_from_hi_lo(lo: i64, hi: i64) -> Self {
        XoroshiroRandomSource {
            rng: Xoroshiro128PlusPlus::new(lo as u64, hi as u64),
            gaussian: GaussianSource::new(),
        }
    }
}

impl RandomSource for XoroshiroRandomSource {
    fn next_bits(&mut self, bits: u32) -> i32 {
        (self.next_long() as u64 >> (64 - bits)) as i32
    }

    fn fork(&mut self) -> Box<dyn RandomSource + Send> {
        let lo = self.rng.next_long();
        let hi = self.rng.next_long();
        Box::new(XoroshiroRandomSource::new_from_hi_lo(lo, hi))
    }

    fn fork_positional(&mut self) -> Box<dyn PositionalRandomFactory + Send + Sync> {
        let lo = self.rng.next_long();
        let hi = self.rng.next_long();
        Box::new(XoroshiroPositionalRandomFactory {
            seed_lo: lo as u64,
            seed_hi: hi as u64,
        })
    }

    fn set_seed(&mut self, seed: i64) {
        self.rng = Xoroshiro128PlusPlus::new_from_long(seed);
        self.gaussian.reset();
    }

    fn next_int(&mut self) -> i32 {
        self.rng.next_long() as i32
    }

    fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        let bound_u64 = bound as u64;
        let mut random_bits = (self.next_int() as u32) as u64;
        let mut multiplied_random_bits = random_bits.wrapping_mul(bound_u64);
        let mut fractional_part = multiplied_random_bits & 0xFFFFFFFF;

        if fractional_part < bound_u64 {
            let negated = (!(bound as u32)).wrapping_add(1);
            let unbiased_buckets_start_index = (negated % (bound as u32)) as u64;

            while fractional_part < unbiased_buckets_start_index {
                random_bits = (self.next_int() as u32) as u64;
                multiplied_random_bits = random_bits.wrapping_mul(bound_u64);
                fractional_part = multiplied_random_bits & 0xFFFFFFFF;
            }
        }

        let integer_part = multiplied_random_bits >> 32;
        integer_part as i32
    }

    fn next_long(&mut self) -> i64 {
        self.rng.next_long()
    }

    fn next_boolean(&mut self) -> bool {
        (self.rng.next_long() & 1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.9604645E-8
    }

    fn next_double(&mut self) -> f64 {
        ((self.next_long() as u64) >> 11) as f64 * 1.1102230246251565E-16
    }

    fn next_gaussian(&mut self) -> f64 {
        let mut g = self.gaussian;
        if g.have_next_next_gaussian {
            g.have_next_next_gaussian = false;
            self.gaussian = g;
            return g.next_next_gaussian;
        }

        let mut x;
        let mut y;
        let mut radius_squared;
        loop {
            x = 2.0 * self.next_double() - 1.0;
            y = 2.0 * self.next_double() - 1.0;
            radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                break;
            }
        }

        let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
        g.next_next_gaussian = y * multiplier;
        g.have_next_next_gaussian = true;
        self.gaussian = g;
        x * multiplier
    }

    fn consume_count(&mut self, count: usize) {
        for _ in 0..count {
            self.rng.next_long();
        }
    }
}

pub struct XoroshiroPositionalRandomFactory {
    seed_lo: u64,
    seed_hi: u64,
}

impl PositionalRandomFactory for XoroshiroPositionalRandomFactory {
    fn at(&self, x: i32, y: i32, z: i32) -> Box<dyn RandomSource + Send> {
        let positional_seed = crate::mth::get_seed(x, y, z);
        let random_seed = (positional_seed as u64) ^ self.seed_lo;
        Box::new(XoroshiroRandomSource::new_from_seed128(Seed128bit::new(
            random_seed,
            self.seed_hi,
        )))
    }

    fn from_hash_of(&self, name: &str) -> Box<dyn RandomSource + Send> {
        let seed = seed_from_hash_of(name);
        let lo = seed.lo ^ self.seed_lo;
        let hi = seed.hi ^ self.seed_hi;
        Box::new(XoroshiroRandomSource::new_from_seed128(Seed128bit::new(
            lo, hi,
        )))
    }

    fn from_seed(&self, seed: i64) -> Box<dyn RandomSource + Send> {
        Box::new(XoroshiroRandomSource::new_from_seed128(Seed128bit::new(
            (seed as u64) ^ self.seed_lo,
            (seed as u64) ^ self.seed_hi,
        )))
    }
}

pub struct WorldgenRandom {
    random_source: Box<dyn RandomSource + Send>,
    count: usize,
    gaussian: GaussianSource,
}

impl WorldgenRandom {
    pub fn new(random_source: Box<dyn RandomSource + Send>) -> Self {
        WorldgenRandom {
            random_source,
            count: 0,
            gaussian: GaussianSource::new(),
        }
    }

    pub fn count(&self) -> usize {
        self.count
    }

    pub fn set_decoration_seed(&mut self, seed: i64, chunk_x: i32, chunk_z: i32) -> i64 {
        self.set_seed(seed);
        let x_scale = self.next_long() | 1;
        let z_scale = self.next_long() | 1;
        let result = (chunk_x as i64)
            .wrapping_mul(x_scale)
            .wrapping_add((chunk_z as i64).wrapping_mul(z_scale))
            ^ seed;
        self.set_seed(result);
        result
    }

    pub fn set_feature_seed(&mut self, seed: i64, index: i32, step: i32) {
        let result = seed
            .wrapping_add(index as i64)
            .wrapping_add(10000 * step as i64);
        self.set_seed(result);
    }

    pub fn set_large_feature_seed(&mut self, seed: i64, chunk_x: i32, chunk_z: i32) {
        self.set_seed(seed);
        let x_scale = self.next_long();
        let z_scale = self.next_long();
        let result = ((chunk_x as i64).wrapping_mul(x_scale))
            ^ ((chunk_z as i64).wrapping_mul(z_scale))
            ^ seed;
        self.set_seed(result);
    }

    pub fn set_large_feature_with_salt(&mut self, seed: i64, x: i32, z: i32, blend: i32) {
        let result = (x as i64)
            .wrapping_mul(341873128712)
            .wrapping_add((z as i64).wrapping_mul(132897987541))
            .wrapping_add(seed)
            .wrapping_add(blend as i64);
        self.set_seed(result);
    }

    pub fn seed_slime_chunk(x: i32, z: i32, seed: i64, salt: i64) -> SingleThreadedRandomSource {
        let val = seed
            .wrapping_add((x.wrapping_mul(x).wrapping_mul(4987142)) as i64)
            .wrapping_add((x.wrapping_mul(5947611)) as i64)
            .wrapping_add((z.wrapping_mul(z) as i64).wrapping_mul(4392871))
            .wrapping_add((z.wrapping_mul(389711)) as i64)
            ^ salt;
        SingleThreadedRandomSource::new(val)
    }
}

impl RandomSource for WorldgenRandom {
    fn next_bits(&mut self, bits: u32) -> i32 {
        self.count += 1;
        if self.random_source.is_legacy_random_source() {
            self.random_source.next_bits(bits)
        } else {
            ((self.random_source.next_long() as u64) >> (64 - bits)) as i32
        }
    }

    fn fork(&mut self) -> Box<dyn RandomSource + Send> {
        self.random_source.fork()
    }

    fn fork_positional(&mut self) -> Box<dyn PositionalRandomFactory + Send + Sync> {
        self.random_source.fork_positional()
    }

    fn set_seed(&mut self, seed: i64) {
        self.random_source.set_seed(seed);
        self.gaussian.reset();
    }

    fn next_int(&mut self) -> i32 {
        self.next_bits(32)
    }

    fn next_int_bound(&mut self, bound: i32) -> i32 {
        assert!(bound > 0, "Bound must be positive");
        if (bound & (bound - 1)) == 0 {
            return ((bound as i64 * self.next_bits(31) as i64) >> 31) as i32;
        }
        let mut sample;
        let mut modulo;
        loop {
            sample = self.next_bits(31);
            modulo = sample % bound;
            if sample - modulo + (bound - 1) >= 0 {
                break;
            }
        }
        modulo
    }

    fn next_long(&mut self) -> i64 {
        let upper = self.next_bits(32) as i64;
        let lower = self.next_bits(32) as i64;
        let shifted = upper << 32;
        shifted + lower
    }

    fn next_boolean(&mut self) -> bool {
        self.next_bits(1) != 0
    }

    fn next_float(&mut self) -> f32 {
        self.next_bits(24) as f32 * 5.9604645E-8
    }

    fn next_double(&mut self) -> f64 {
        let upper = self.next_bits(26) as i64;
        let lower = self.next_bits(27) as i64;
        let combined = (upper << 27) + lower;
        combined as f64 * 1.1102230246251565E-16
    }

    fn next_gaussian(&mut self) -> f64 {
        let mut g = self.gaussian;
        if g.have_next_next_gaussian {
            g.have_next_next_gaussian = false;
            self.gaussian = g;
            return g.next_next_gaussian;
        }

        let mut x;
        let mut y;
        let mut radius_squared;
        loop {
            x = 2.0 * self.next_double() - 1.0;
            y = 2.0 * self.next_double() - 1.0;
            radius_squared = x * x + y * y;
            if radius_squared < 1.0 && radius_squared != 0.0 {
                break;
            }
        }

        let multiplier = (-2.0 * radius_squared.ln() / radius_squared).sqrt();
        g.next_next_gaussian = y * multiplier;
        g.have_next_next_gaussian = true;
        self.gaussian = g;
        x * multiplier
    }

    fn is_legacy_random_source(&self) -> bool {
        true
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Seed128bit {
    pub lo: u64,
    pub hi: u64,
}

impl Seed128bit {
    pub fn new(lo: u64, hi: u64) -> Self {
        Seed128bit { lo, hi }
    }

    pub fn xor(&self, lo: u64, hi: u64) -> Self {
        Seed128bit {
            lo: self.lo ^ lo,
            hi: self.hi ^ hi,
        }
    }

    pub fn xor_other(&self, other: Seed128bit) -> Self {
        self.xor(other.lo, other.hi)
    }

    pub fn mixed(&self) -> Self {
        Seed128bit {
            lo: mix_stafford_13(self.lo),
            hi: mix_stafford_13(self.hi),
        }
    }
}

pub fn mix_stafford_13(mut z: u64) -> u64 {
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

pub fn upgrade_seed_to_128bit_unmixed(legacy_seed: i64) -> Seed128bit {
    let low_bits = (legacy_seed as u64) ^ 0x6A09E667F3BCC909;
    let high_bits = low_bits.wrapping_add(0x9E3779B97F4A7C15);
    Seed128bit::new(low_bits, high_bits)
}

pub fn upgrade_seed_to_128bit(legacy_seed: i64) -> Seed128bit {
    upgrade_seed_to_128bit_unmixed(legacy_seed).mixed()
}

pub fn seed_from_hash_of(input: &str) -> Seed128bit {
    let digest = md5::compute(input.as_bytes());
    let hash = digest.0;

    let lo = u64::from_be_bytes([
        hash[0], hash[1], hash[2], hash[3], hash[4], hash[5], hash[6], hash[7],
    ]);
    let hi = u64::from_be_bytes([
        hash[8], hash[9], hash[10], hash[11], hash[12], hash[13], hash[14], hash[15],
    ]);
    Seed128bit::new(lo, hi)
}
