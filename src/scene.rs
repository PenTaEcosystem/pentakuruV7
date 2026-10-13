//! Perspective projection and monochrome drawing; all motion is time based.
use crate::{
    clockwork::Angles,
    geometry::{Solid, V3},
};
use eframe::egui::{Color32, Painter, Pos2, Shape, Stroke, Vec2};
use std::collections::BTreeSet;

pub fn project(v: V3, center: Pos2, scale: f32) -> Pos2 {
    let perspective = 5.8 / (5.8 - v.2);
    center + Vec2::new(v.0, -v.1) * (scale * perspective)
}

pub struct Scene {
    pub center: Pos2,
    pub scale: f32,
    pub time: f32,
    pub angles: Angles,
    pub gaze: Vec2,
    pub openness: f32,
    pub unit: f32,
    pub glow: f32,
}

pub fn draw(p: &Painter, solid: &Solid, scene: &Scene) {
    let Scene {
        center,
        scale,
        time: t,
        angles,
        gaze,
        openness,
        unit,
        glow,
    } = *scene;
    let Angles { yaw, pitch, roll } = angles;
    let vertices: Vec<V3> = solid
        .vertices
        .iter()
        .map(|v| v.rotate(yaw, pitch, roll))
        .collect();
    let projected: Vec<Pos2> = vertices
        .iter()
        .map(|v| project(*v, center, scale))
        .collect();
    let mut faces: Vec<&Vec<usize>> = solid
        .faces
        .iter()
        .filter(|f| {
            let c = f.iter().fold(V3::default(), |s, i| s + vertices[*i]) * 0.2;
            let n = (vertices[f[1]] - vertices[f[0]]).cross(vertices[f[2]] - vertices[f[0]]);
            n.dot(V3(0.0, 0.0, 5.8) - c) > 0.0
        })
        .collect();
    faces.sort_by(|a, b| {
        let depth = |f: &Vec<usize>| f.iter().map(|i| vertices[*i].2).sum::<f32>();
        depth(a).total_cmp(&depth(b))
    });
    // The body is opaque black; only the desktop outside its silhouette is alpha.
    for face in &faces {
        p.add(Shape::convex_polygon(
            face.iter().map(|i| projected[*i]).collect(),
            Color32::BLACK,
            Stroke::NONE,
        ));
    }
    let eye = V3(
        0.17 * (t * 0.57).sin(),
        0.13 * (t * 0.41).cos(),
        0.18 * (t * 0.36).sin(),
    );
    let eye_pos = project(eye, center, scale) + gaze * scale * 0.14;
    let radius = scale * 0.46 * (5.8 / (5.8 - eye.2));
    draw_eye(p, eye_pos, radius, openness, unit, glow);
    let mut edges = BTreeSet::new();
    for face in &faces {
        for i in 0..5 {
            let a = face[i];
            let b = face[(i + 1) % 5];
            edges.insert((a.min(b), a.max(b)));
        }
    }
    let pulse = 0.88 + 0.12 * (t * 0.85).sin();
    for (a, b) in edges {
        let line = [projected[a], projected[b]];
        for (width, alpha) in [(10.0, 7.0), (6.0, 16.0), (3.5, 28.0)] {
            p.line_segment(
                line,
                Stroke::new(
                    width * unit,
                    Color32::from_white_alpha((alpha * glow * pulse) as u8),
                ),
            );
        }
        p.line_segment(line, Stroke::new(1.15 * unit, Color32::WHITE));
    }
}

fn draw_eye(p: &Painter, center: Pos2, radius: f32, openness: f32, unit: f32, glow: f32) {
    let points: Vec<Pos2> = (0..64)
        .map(|i| {
            let angle = i as f32 / 64.0 * std::f32::consts::TAU;
            center + Vec2::new(angle.cos(), angle.sin() * openness) * radius
        })
        .collect();
    for (width, alpha) in [(7.0, 7.0), (4.0, 14.0), (2.5, 24.0)] {
        p.add(Shape::closed_line(
            points.clone(),
            Stroke::new(
                width * unit,
                Color32::from_white_alpha((alpha * glow) as u8),
            ),
        ));
    }
    p.add(Shape::closed_line(
        points,
        Stroke::new(1.15 * unit, Color32::WHITE),
    ));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn projection_respects_depth_and_stays_inside_window() {
        let c = Pos2::new(80.0, 70.0);
        assert!(project(V3(1.0, 0.0, 1.0), c, 65.0).x > project(V3(1.0, 0.0, -1.0), c, 65.0).x);
        let solid = Solid::dodecahedron();
        for i in 0..720 {
            let angles = Angles::at(i as f64 * 61.17);
            for v in &solid.vertices {
                let pos = project(
                    v.rotate(angles.yaw, angles.pitch, angles.roll),
                    c,
                    24.0 * 1.08,
                );
                assert!(pos.x > 8.0 && pos.x < 152.0 && pos.y > 8.0 && pos.y < 130.0);
            }
        }
    }
}
