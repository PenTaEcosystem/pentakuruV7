use eframe::egui::{Pos2, Rect, Vec2};

pub struct Layout {
    pub unit: f32,
    pub center: Pos2,
    pub toolbar: Rect,
}

impl Layout {
    pub fn new(rect: Rect, active_dock_count: usize) -> Self {
        let unit = (rect.width() / 160.0).min(rect.height() / 190.0).max(0.01);
        let origin = rect.center() - Vec2::new(80.0, 95.0) * unit;
        let toolbar = if active_dock_count == 0 {
            Rect::NOTHING
        } else {
            let width = match active_dock_count {
                1 => 40.0,
                2 => 78.0,
                3 => 116.0,
                4 => 148.0,
                n => (n as f32 * 36.0 + 4.0).min(150.0),
            } * unit;
            Rect::from_center_size(
                origin + Vec2::new(80.0, 155.0) * unit,
                Vec2::new(width, 36.0 * unit),
            )
        };
        Self {
            unit,
            center: origin + Vec2::new(80.0, 70.0) * unit,
            toolbar,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{clockwork::Angles, geometry::Solid, scene::project};

    #[test]
    fn all_axes_and_aspect_ratios_fit_without_clipping() {
        let solid = Solid::dodecahedron();
        for size in [
            [120.0, 150.0],
            [160.0, 190.0],
            [320.0, 380.0],
            [600.0, 200.0],
            [150.0, 700.0],
        ] {
            let rect = Rect::from_min_size(Pos2::ZERO, Vec2::from(size));
            let layout = Layout::new(rect, 3);
            assert!(rect.contains_rect(layout.toolbar.expand(4.0 * layout.unit)));
            for count in [0, 1, 2, 3, 4, 5] {
                let dynamic_layout = Layout::new(rect, count);
                if count == 0 {
                    assert_eq!(dynamic_layout.toolbar, Rect::NOTHING);
                } else {
                    assert!(
                        rect.contains_rect(
                            dynamic_layout.toolbar.expand(4.0 * dynamic_layout.unit)
                        )
                    );
                }
            }
            for tick in 0..720 {
                let angles = Angles::at(tick as f64 * 61.17);
                for vertex in &solid.vertices {
                    let p = project(
                        vertex.rotate(angles.yaw, angles.pitch, angles.roll),
                        layout.center,
                        24.0 * 1.08 * layout.unit,
                    );
                    assert!(rect.shrink(5.0 * layout.unit).contains(p));
                    assert!(p.y + 5.0 * layout.unit < layout.toolbar.top());
                }
            }
        }
    }
}
