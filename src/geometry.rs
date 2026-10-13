//! Regular dodecahedron: 20 vertices, 30 edges, 12 pentagonal faces.
use std::collections::BTreeSet;
use std::ops::{Add, Mul, Sub};

#[derive(Clone, Copy, Debug, Default)]
pub struct V3(pub f32, pub f32, pub f32);

impl Add for V3 {
    type Output = Self;
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1, self.2 + b.2)
    }
}
impl Sub for V3 {
    type Output = Self;
    fn sub(self, b: Self) -> Self {
        Self(self.0 - b.0, self.1 - b.1, self.2 - b.2)
    }
}
impl Mul<f32> for V3 {
    type Output = Self;
    fn mul(self, s: f32) -> Self {
        Self(self.0 * s, self.1 * s, self.2 * s)
    }
}
impl V3 {
    pub fn dot(self, b: Self) -> f32 {
        self.0 * b.0 + self.1 * b.1 + self.2 * b.2
    }
    pub fn cross(self, b: Self) -> Self {
        Self(
            self.1 * b.2 - self.2 * b.1,
            self.2 * b.0 - self.0 * b.2,
            self.0 * b.1 - self.1 * b.0,
        )
    }
    pub fn unit(self) -> Self {
        self * (1.0 / self.dot(self).sqrt())
    }
    pub fn rotate(self, yaw: f32, pitch: f32, roll: f32) -> Self {
        let (s, c) = yaw.sin_cos();
        let p = Self(c * self.0 + s * self.2, self.1, -s * self.0 + c * self.2);
        let (s, c) = pitch.sin_cos();
        let p = Self(p.0, c * p.1 - s * p.2, s * p.1 + c * p.2);
        let (s, c) = roll.sin_cos();
        Self(c * p.0 - s * p.1, s * p.0 + c * p.1, p.2)
    }
}

pub struct Solid {
    pub vertices: Vec<V3>,
    pub faces: Vec<Vec<usize>>,
}

impl Solid {
    pub fn dodecahedron() -> Self {
        let phi = (1.0 + 5.0_f32.sqrt()) / 2.0;
        let mut vertices = Vec::with_capacity(20);
        for x in [-1.0, 1.0] {
            for y in [-1.0, 1.0] {
                for z in [-1.0, 1.0] {
                    vertices.push(V3(x, y, z));
                }
            }
        }
        for a in [-1.0, 1.0] {
            for b in [-1.0, 1.0] {
                vertices.extend([
                    V3(0.0, a / phi, b * phi),
                    V3(a / phi, b * phi, 0.0),
                    V3(b * phi, 0.0, a / phi),
                ]);
            }
        }
        // Find supporting planes, collecting the five coplanar vertices only.
        // No triangulation edges are ever exposed to the renderer.
        let mut sets = BTreeSet::new();
        for a in 0..20 {
            for b in a + 1..20 {
                for c in b + 1..20 {
                    let n = (vertices[b] - vertices[a]).cross(vertices[c] - vertices[a]);
                    if n.dot(n) < 0.0001 {
                        continue;
                    }
                    let n = n.unit();
                    let distances: Vec<f32> =
                        vertices.iter().map(|v| (*v - vertices[a]).dot(n)).collect();
                    if distances.iter().any(|d| *d > 0.001) && distances.iter().any(|d| *d < -0.001)
                    {
                        continue;
                    }
                    let face: Vec<usize> = distances
                        .iter()
                        .enumerate()
                        .filter(|(_, d)| d.abs() < 0.001)
                        .map(|(i, _)| i)
                        .collect();
                    if face.len() == 5 {
                        sets.insert(face);
                    }
                }
            }
        }
        let mut faces: Vec<Vec<usize>> = sets.into_iter().collect();
        for face in &mut faces {
            let center = face.iter().fold(V3::default(), |s, i| s + vertices[*i]) * 0.2;
            let normal = center.unit();
            let u = (vertices[face[0]] - center).unit();
            let v = normal.cross(u);
            face.sort_by(|a, b| {
                let a = vertices[*a] - center;
                let b = vertices[*b] - center;
                a.dot(v)
                    .atan2(a.dot(u))
                    .total_cmp(&b.dot(v).atan2(b.dot(u)))
            });
        }
        // Aim one pentagonal face toward the viewer, with one point at the top.
        let face = &faces[0];
        let center = face.iter().fold(V3::default(), |s, i| s + vertices[*i]) * 0.2;
        let n = center.unit();
        let up = (vertices[face[0]] - center).unit();
        let right = up.cross(n);
        vertices = vertices
            .iter()
            .map(|p| V3(p.dot(right), p.dot(up), p.dot(n)))
            .collect();
        Self { vertices, faces }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regular_solid_has_only_pentagons_and_thirty_shared_edges() {
        let solid = Solid::dodecahedron();
        assert_eq!(solid.vertices.len(), 20);
        assert_eq!(solid.faces.len(), 12);
        let mut edges = std::collections::BTreeMap::new();
        for face in &solid.faces {
            assert_eq!(face.len(), 5);
            let a = solid.vertices[face[0]];
            let normal = (solid.vertices[face[1]] - a)
                .cross(solid.vertices[face[2]] - a)
                .unit();
            assert!(normal.dot(a) > 0.0, "faces must point outward");
            for i in 0..5 {
                let a = face[i];
                let b = face[(i + 1) % 5];
                *edges.entry((a.min(b), a.max(b))).or_insert(0) += 1;
                assert!((solid.vertices[a] - solid.vertices[b]).dot(normal).abs() < 0.001);
            }
        }
        assert_eq!(edges.len(), 30);
        assert!(edges.values().all(|count| *count == 2));
        let lengths: Vec<f32> = edges
            .keys()
            .map(|(a, b)| {
                let d = solid.vertices[*a] - solid.vertices[*b];
                d.dot(d).sqrt()
            })
            .collect();
        assert!(lengths.iter().all(|l| (*l - lengths[0]).abs() < 0.001));
        assert!(
            solid
                .vertices
                .iter()
                .all(|v| (v.dot(*v) - 3.0).abs() < 0.001)
        );
    }
}
