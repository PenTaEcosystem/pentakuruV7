use eframe::egui::Vec2;

pub struct EyeMotion {
    pub gaze: Vec2,
    next_blink: f64,
    blink_start: Option<f64>,
    random: u64,
}

impl EyeMotion {
    pub fn new(seed: u64) -> Self {
        Self {
            gaze: Vec2::ZERO,
            next_blink: 5.0,
            blink_start: None,
            random: seed.max(1),
        }
    }

    pub fn update(&mut self, seconds: f64, dt: f32, target: Vec2) -> f32 {
        let target = target / target.length().max(1.0);
        self.gaze += (target - self.gaze) * (1.0 - (-12.0 * dt).exp());
        if self.blink_start.is_none() && seconds >= self.next_blink {
            self.blink_start = Some(seconds);
        }
        if let Some(start) = self.blink_start {
            let phase = ((seconds - start) / 0.26) as f32;
            if phase < 1.0 {
                return (1.0 - (std::f32::consts::PI * phase).sin().powi(2)).max(0.035);
            }
            self.blink_start = None;
            self.random ^= self.random << 13;
            self.random ^= self.random >> 7;
            self.random ^= self.random << 17;
            self.next_blink = seconds + 5.0 + (self.random % 4000) as f64 / 1000.0;
        }
        1.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn blink_closes_reopens_and_gaze_stays_bounded_at_different_fps() {
        let mut eye = EyeMotion::new(42);
        assert_eq!(eye.update(0.0, 1.0 / 60.0, Vec2::ZERO), 1.0);
        assert_eq!(eye.update(5.0, 1.0 / 60.0, Vec2::ZERO), 1.0);
        assert!(eye.update(5.13, 1.0 / 60.0, Vec2::ZERO) < 0.04);
        assert_eq!(eye.update(5.27, 1.0 / 60.0, Vec2::ZERO), 1.0);
        let mut a = EyeMotion::new(42);
        let mut b = EyeMotion::new(42);
        for _ in 0..30 {
            a.update(0.0, 1.0 / 30.0, Vec2::splat(1000.0));
        }
        for _ in 0..60 {
            b.update(0.0, 1.0 / 60.0, Vec2::splat(1000.0));
        }
        assert!((a.gaze - b.gaze).length() < 0.0001);
        assert!(a.gaze.length() <= 1.0);
    }
}
