#![allow(dead_code)]

pub struct PreeditEvent {
    pub full_text: String,
    pub caret_position: usize,
    pub blocks: Vec<String>,
    pub focused_block: usize,
}

impl PreeditEvent {
    pub fn new(
        full_text: String,
        caret_position: usize,
        blocks: Vec<String>,
        focused_block: usize,
    ) -> Self {
        Self {
            full_text,
            caret_position,
            blocks,
            focused_block,
        }
    }

    pub unsafe fn create_from_callback(
        preedit_size: usize,
        preedit_ptr: *const i32,
        block_count: usize,
        block_sizes_ptr: *const i32,
        focused_block: usize,
        caret: usize,
    ) -> Option<Self> {
        unsafe {
            if preedit_size == 0 || preedit_ptr.is_null() || block_sizes_ptr.is_null() {
                return None;
            }

            let codepoints = std::slice::from_raw_parts(preedit_ptr, preedit_size);
            let block_sizes = std::slice::from_raw_parts(block_sizes_ptr, block_count);
            let mut full_text = String::new();
            let mut blocks = Vec::with_capacity(block_count);
            let mut offset = 0;
            let mut converted_caret = 0;

            for &block_size_val in block_sizes {
                let block_size = block_size_val as usize;
                let mut block_builder = String::new();

                for _ in 0..block_size {
                    if offset < preedit_size {
                        let codepoint = codepoints[offset] as u32;
                        if offset == caret {
                            converted_caret = full_text.len() + block_builder.len();
                        }
                        if let Some(c) = char::from_u32(codepoint) {
                            block_builder.push(c);
                        }
                        offset += 1;
                    }
                }

                let block = block_builder;
                full_text.push_str(&block);
                blocks.push(block);
            }

            if offset == caret {
                converted_caret = full_text.len();
            }

            if focused_block >= blocks.len() {
                return None;
            }

            Some(Self {
                full_text,
                caret_position: converted_caret,
                blocks,
                focused_block,
            })
        }
    }

    pub fn to_formatted_text(&self) -> String {
        let mut result = String::new();
        for block in &self.blocks {
            result.push_str(block);
        }
        result
    }
}
