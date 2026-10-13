//! Local system clock -> continuous Euler angles, without accumulating drift.
#[derive(Clone, Copy, Default)]
pub struct Angles {
    pub yaw: f32,
    pub pitch: f32,
    pub roll: f32,
}

impl Angles {
    pub fn at(local_seconds: f64) -> Self {
        let turn = std::f64::consts::TAU;
        Self {
            // Different rates prevent yaw and roll from cancelling near 03:00
            // and 15:00, when the hour-driven pitch aligns their axes.
            yaw: (turn * local_seconds.rem_euclid(120.0) / 120.0) as f32,
            pitch: (turn * local_seconds.rem_euclid(43200.0) / 43200.0) as f32,
            // Projected Y is inverted: negative Z rotation is clockwise on screen.
            roll: (-turn * local_seconds.rem_euclid(60.0) / 60.0) as f32,
        }
    }
}

#[cfg(target_os = "windows")]
pub fn local_seconds() -> f64 {
    use windows_sys::Win32::{Foundation::SYSTEMTIME, System::SystemInformation::GetLocalTime};
    let mut time = SYSTEMTIME::default();
    // SAFETY: GetLocalTime writes to a valid SYSTEMTIME owned by this function.
    unsafe { GetLocalTime(&mut time) };
    f64::from(time.wHour) * 3600.0
        + f64::from(time.wMinute) * 60.0
        + f64::from(time.wSecond)
        + f64::from(time.wMilliseconds) / 1000.0
}

#[cfg(unix)]
pub fn local_seconds() -> f64 {
    let epoch = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let seconds = epoch.as_secs() as libc::time_t;
    let mut local = std::mem::MaybeUninit::<libc::tm>::uninit();
    // SAFETY: localtime_r writes into an adequately sized tm; initialized data
    // is read only after checking its successful return value.
    unsafe {
        if !libc::localtime_r(&seconds, local.as_mut_ptr()).is_null() {
            let local = local.assume_init();
            return f64::from(local.tm_hour) * 3600.0
                + f64::from(local.tm_min) * 60.0
                + f64::from(local.tm_sec)
                + f64::from(epoch.subsec_nanos()) / 1e9;
        }
    }
    epoch.as_secs_f64().rem_euclid(86400.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::V3, scene::project};
    use eframe::egui::Pos2;

    #[test]
    fn clockwise_quarters_are_three_six_nine_twelve() {
        let expected = [(1.0, 0.0), (0.0, 1.0), (-1.0, 0.0), (0.0, -1.0)];
        for (i, (x, y)) in expected.into_iter().enumerate() {
            let roll = Angles::at(i as f64 * 15.0).roll;
            let p = project(V3(1.0, 0.0, 0.0).rotate(0.0, 0.0, roll), Pos2::ZERO, 1.0);
            assert!((p.x - x).abs() < 0.0001 && (p.y - y).abs() < 0.0001);
        }
    }

    #[test]
    fn axes_follow_seconds_and_hours_and_wrap_continuously() {
        let a = Angles::at(15.0);
        assert!((a.roll + std::f32::consts::FRAC_PI_2).abs() < 0.0001);
        assert!((Angles::at(15.0).yaw - std::f32::consts::FRAC_PI_4).abs() < 0.0001);
        assert!((Angles::at(10800.0).pitch - std::f32::consts::FRAC_PI_2).abs() < 0.0001);
        let before = Angles::at(59.999).roll;
        let after = Angles::at(60.001).roll;
        let a = V3(1.0, 0.0, 0.0).rotate(0.0, 0.0, before);
        let b = V3(1.0, 0.0, 0.0).rotate(0.0, 0.0, after);
        assert!((a - b).dot(a - b) < 1e-6);
    }

    #[test]
    fn rotation_remains_visible_at_every_hour() {
        let solid = crate::geometry::Solid::dodecahedron();
        for seconds in (0..86400).step_by(137).chain([10800, 54000]) {
            let a = Angles::at(f64::from(seconds));
            let b = Angles::at(f64::from(seconds) + 3.0);
            let mean_squared_motion: f32 = solid
                .vertices
                .iter()
                .map(|vertex| {
                    let first = project(vertex.rotate(a.yaw, a.pitch, a.roll), Pos2::ZERO, 24.0);
                    let second = project(vertex.rotate(b.yaw, b.pitch, b.roll), Pos2::ZERO, 24.0);
                    (second - first).length_sq()
                })
                .sum::<f32>()
                / solid.vertices.len() as f32;
            // Check projected geometry, rather than changing Euler values alone:
            // equal yaw/roll rates can cancel when pitch is 90 degrees.
            assert!(
                mean_squared_motion > 4.0,
                "Model appears stationary at {seconds}s: motion squared={mean_squared_motion}"
            );
        }
    }
}
