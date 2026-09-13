//! Hash-neutral day/night light scale. Native window only.

pub use sim_core::light_for_tod;

#[cfg(test)]
mod tests {
    use super::light_for_tod;

    #[test]
    fn day_is_bright_night_is_dim() {
        let day = light_for_tod(60, 240);
        let night = light_for_tod(200, 240);
        assert!(day > 0.9, "day={day}");
        assert!((night - 0.15).abs() < 0.01, "night={night}");
    }
}
