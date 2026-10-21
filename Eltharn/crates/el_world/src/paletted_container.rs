#[derive(Clone, Debug, PartialEq)]
pub struct PalettedContainer<T: Clone + PartialEq> {
    pub palette: Vec<T>,
    pub storage: Vec<u64>,
    pub bits_per_entry: usize,
    pub capacity: usize,
    pub default_value: T,
    minimum_bits: usize,
}
#[inline]
fn low_mask(bits: usize) -> u64 {
    if bits >= 64 {
        u64::MAX
    } else {
        (1u64 << bits) - 1
    }
}
fn warn_invalid_bits_per_entry(bits: usize) {
    static WARNED: std::sync::OnceLock<()> = std::sync::OnceLock::new();
    WARNED.get_or_init(|| {
        eprintln!(
            "el_world: paletted container: invalid bits_per_entry {bits}; \
             falling back to the palette-derived width"
        );
    });
}
impl<T: Clone + PartialEq> PalettedContainer<T> {
    pub fn new(capacity: usize, default_value: T) -> Self {
        Self::new_with_minimum_bits(capacity, default_value, 4)
    }
    pub fn new_with_minimum_bits(capacity: usize, default_value: T, minimum_bits: usize) -> Self {
        Self {
            palette: vec![default_value.clone()],
            storage: Vec::new(),
            bits_per_entry: 0,
            capacity,
            default_value,
            minimum_bits,
        }
    }
    pub fn from_raw(
        palette: Vec<T>,
        storage: Vec<u64>,
        bits_per_entry: usize,
        capacity: usize,
        default_value: T,
    ) -> Self {
        Self::from_raw_with_minimum_bits(
            palette,
            storage,
            bits_per_entry,
            capacity,
            default_value,
            4,
        )
    }
    pub fn from_raw_with_minimum_bits(
        mut palette: Vec<T>,
        storage: Vec<u64>,
        mut bits_per_entry: usize,
        capacity: usize,
        default_value: T,
        minimum_bits: usize,
    ) -> Self {
        if palette.is_empty() {
            palette.push(default_value.clone());
        }
        if bits_per_entry > 64 {
            warn_invalid_bits_per_entry(bits_per_entry);
            bits_per_entry = Self::bits_for_distinct_values(palette.len())
                .max(minimum_bits)
                .min(64);
        }
        let expected_words = Self::word_count(capacity, bits_per_entry);
        let mut normalized_storage = storage;
        normalized_storage.resize(expected_words, 0);
        normalized_storage.truncate(expected_words);
        Self {
            palette,
            storage: normalized_storage,
            bits_per_entry,
            capacity,
            default_value,
            minimum_bits,
        }
    }
    #[inline]
    fn values_per_long(bits: usize) -> usize {
        if bits == 0 { usize::MAX } else { 64 / bits }
    }
    #[inline]
    fn word_count(capacity: usize, bits: usize) -> usize {
        if bits == 0 {
            0
        } else {
            capacity.div_ceil(Self::values_per_long(bits))
        }
    }
    #[inline]
    fn palette_index(&self, index: usize) -> usize {
        assert!(
            index < self.capacity,
            "paletted-container index out of bounds"
        );
        if self.bits_per_entry == 0 {
            return 0;
        }
        let values_per_long = Self::values_per_long(self.bits_per_entry);
        let long_index = index / values_per_long;
        let bit_offset = (index % values_per_long) * self.bits_per_entry;
        let mask = low_mask(self.bits_per_entry);
        ((self.storage[long_index] >> bit_offset) & mask) as usize
    }
    pub fn get(&self, index: usize) -> T {
        self.palette
            .get(self.palette_index(index))
            .unwrap_or(&self.default_value)
            .clone()
    }
    #[inline]
    pub fn get_ref(&self, index: usize) -> &T {
        self.palette
            .get(self.palette_index(index))
            .unwrap_or(&self.default_value)
    }
    pub fn get_and_set(&mut self, index: usize, value: T) -> T {
        let previous = self.get(index);
        self.set(index, value);
        previous
    }
    pub fn set(&mut self, index: usize, value: T) {
        assert!(
            index < self.capacity,
            "paletted-container index out of bounds"
        );
        let palette_index = self
            .palette
            .iter()
            .position(|item| *item == value)
            .unwrap_or_else(|| {
                let new_index = self.palette.len();
                let required_bits =
                    Self::bits_for_distinct_values(new_index + 1).max(self.minimum_bits);
                if required_bits > self.bits_per_entry {
                    self.resize_storage(required_bits);
                }
                self.palette.push(value);
                new_index
            });
        if self.bits_per_entry == 0 {
            return;
        }
        let values_per_long = Self::values_per_long(self.bits_per_entry);
        let long_index = index / values_per_long;
        let bit_offset = (index % values_per_long) * self.bits_per_entry;
        let mask = low_mask(self.bits_per_entry);
        self.storage[long_index] = (self.storage[long_index] & !(mask << bit_offset))
            | ((palette_index as u64 & mask) << bit_offset);
    }
    pub fn maybe_has(&self, predicate: impl Fn(&T) -> bool) -> bool {
        self.palette.iter().any(predicate)
    }
    pub fn count(&self, mut consumer: impl FnMut(&T, usize)) {
        if self.palette.len() == 1 {
            consumer(&self.palette[0], self.capacity);
            return;
        }
        let mut counts = vec![0usize; self.palette.len()];
        for index in 0..self.capacity {
            if let Some(count) = counts.get_mut(self.palette_index(index)) {
                *count += 1;
            }
        }
        for (value, count) in self.palette.iter().zip(counts) {
            if count != 0 {
                consumer(value, count);
            }
        }
    }
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.capacity).map(|index| self.get(index))
    }
    fn bits_for_distinct_values(values: usize) -> usize {
        if values <= 1 {
            0
        } else {
            usize::BITS as usize - (values - 1).leading_zeros() as usize
        }
    }
    fn resize_storage(&mut self, new_bits: usize) {
        let old_bits = self.bits_per_entry;
        let old_storage = std::mem::take(&mut self.storage);
        self.bits_per_entry = new_bits;
        self.storage = vec![0; Self::word_count(self.capacity, new_bits)];
        if old_bits == 0 {
            return;
        }
        let old_values_per_long = Self::values_per_long(old_bits);
        let old_mask = low_mask(old_bits);
        let new_values_per_long = Self::values_per_long(new_bits);
        for index in 0..self.capacity {
            let old_word = index / old_values_per_long;
            let old_offset = (index % old_values_per_long) * old_bits;
            let palette_index = (old_storage[old_word] >> old_offset) & old_mask;
            let new_word = index / new_values_per_long;
            let new_offset = (index % new_values_per_long) * new_bits;
            self.storage[new_word] |= palette_index << new_offset;
        }
    }
}
