pub(crate) fn scaled_strength(user_strength: f64, scale_milli: u16) -> f64 {
    if scale_milli == 1000 {
        user_strength
    } else {
        user_strength * (scale_milli.min(1000) as f64 / 1000.0)
    }
}
