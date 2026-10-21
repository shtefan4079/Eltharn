pub const NOT_DIGIT: i32 = -1;

pub trait InputWithModifiers {
    fn input(&self) -> i32;
    fn _modifiers(&self) -> u32;

    fn is_escape(&self) -> bool {
        self.input() == 256
    }

    fn get_digit(&self) -> i32 {
        let value = self.input() - 48;
        if value >= 0 && value <= 9 { value } else { -1 }
    }
}
