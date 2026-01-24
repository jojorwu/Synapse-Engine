use glam::{Mat4, Vec3, Vec4};

#[derive(Debug, Clone, Copy)]
pub struct Plane {
    normal: Vec3,
    distance: f32,
}

impl Plane {
    pub fn new(normal: Vec3, distance: f32) -> Self {
        Self { normal, distance }
    }

    pub fn normalize(&mut self) {
        let length = self.normal.length();
        self.normal /= length;
        self.distance /= length;
    }

    pub fn distance_to_point(&self, point: Vec3) -> f32 {
        self.normal.dot(point) + self.distance
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Frustum {
    planes: [Plane; 6],
}

impl Frustum {
    pub fn from_matrix(matrix: &Mat4) -> Self {
        let mut planes = [Plane::new(Vec3::ZERO, 0.0); 6];
        let row0 = matrix.row(0);
        let row1 = matrix.row(1);
        let row2 = matrix.row(2);
        let row3 = matrix.row(3);

        // Left
        planes[0] = Plane::new((row3 + row0).truncate(), (row3 + row0).w);
        // Right
        planes[1] = Plane::new((row3 - row0).truncate(), (row3 - row0).w);
        // Bottom
        planes[2] = Plane::new((row3 + row1).truncate(), (row3 + row1).w);
        // Top
        planes[3] = Plane::new((row3 - row1).truncate(), (row3 - row1).w);
        // Near
        planes[4] = Plane::new((row3 + row2).truncate(), (row3 + row2).w);
        // Far
        planes[5] = Plane::new((row3 - row2).truncate(), (row3 - row2).w);

        for plane in &mut planes {
            plane.normalize();
        }

        Self { planes }
    }

    pub fn is_sphere_visible(&self, center: Vec3, radius: f32) -> bool {
        for plane in &self.planes {
            if plane.distance_to_point(center) < -radius {
                return false;
            }
        }
        true
    }
}
