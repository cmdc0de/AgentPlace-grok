//! Hash-neutral free-camera pan. Native window only.

pub const PAN_TAP: f32 = 2.0;
pub const PAN_UNITS_PER_SEC: f32 = 12.0;
pub const HEIGHT_CLEARANCE: f32 = 2.0;

/// Tap → 2.0; hold (not just-pressed) → 12 units/sec × dt; else 0.
pub fn pan_step(just_pressed: bool, pressed: bool, dt: f32) -> f32 {
    if just_pressed {
        PAN_TAP
    } else if pressed {
        PAN_UNITS_PER_SEC * dt
    } else {
        0.0
    }
}

/// Ground-plane pan. `forward_xz` / `right_xz` are XZ (Y is not in this helper).
/// Amounts are already `pan_step` results (tap or hold).
pub fn pan_xz(
    forward_xz: [f32; 2],
    right_xz: [f32; 2],
    left: f32,
    right: f32,
    forward: f32,
    back: f32,
) -> [f32; 2] {
    [
        right_xz[0] * (right - left) + forward_xz[0] * (forward - back),
        right_xz[1] * (right - left) + forward_xz[1] * (forward - back),
    ]
}

/// New camera Y after up/down amounts. Never below `min_y` (terrain + clearance).
pub fn height_step(up: f32, down: f32, y: f32, min_y: f32) -> f32 {
    (y + up - down).max(min_y)
}

/// First non-zero pan/height step cancels follow.
pub fn follow_after_pan<T>(follow: Option<T>, moved: bool) -> Option<T> {
    if moved { None } else { follow }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pan_step_tap_hold_none() {
        assert_eq!(pan_step(true, true, 0.25), 2.0);
        assert!((pan_step(false, true, 0.25) - 3.0).abs() < 1e-5);
        assert_eq!(pan_step(false, false, 0.25), 0.0);
    }

    #[test]
    fn pan_xz_unit_steps_no_y() {
        let forward = [0.0, 1.0];
        let right = [1.0, 0.0];
        assert_eq!(pan_xz(forward, right, 0.0, 1.0, 0.0, 0.0), [1.0, 0.0]);
        assert_eq!(pan_xz(forward, right, 1.0, 0.0, 0.0, 0.0), [-1.0, 0.0]);
        assert_eq!(pan_xz(forward, right, 0.0, 0.0, 1.0, 0.0), [0.0, 1.0]);
        assert_eq!(pan_xz(forward, right, 0.0, 0.0, 0.0, 1.0), [0.0, -1.0]);
    }

    #[test]
    fn height_clamp_does_not_go_below_min() {
        assert_eq!(height_step(0.0, 2.0, 3.0, 5.0), 5.0);
        assert_eq!(height_step(0.0, 2.0, 10.0, 5.0), 8.0);
        assert_eq!(height_step(2.0, 0.0, 10.0, 5.0), 12.0);
    }

    #[test]
    fn follow_cancel_on_nonzero_pan() {
        assert_eq!(follow_after_pan(Some(3u64), true), None);
        assert_eq!(follow_after_pan(Some(3u64), false), Some(3));
        assert_eq!(follow_after_pan(None::<u64>, true), None);
    }
}
