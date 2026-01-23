use crate::ecs::Component;
use bytemuck::{Pod, Zeroable};
use glam::{Mat4, Quat};

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq)]
pub struct TransformComponent {
    pub position: [f32; 3],
    pub rotation: [f32; 4],
    pub scale: [f32; 3],
}

impl Component for TransformComponent {}

impl TransformComponent {
    pub fn to_matrix(&self) -> Mat4 {
        Mat4::from_scale_rotation_translation(
            self.scale.into(),
            Quat::from_array(self.rotation),
            self.position.into(),
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq)]
pub struct ColorComponent {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Component for ColorComponent {}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable, Debug, PartialEq)]
pub struct RectangleComponent {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Component for RectangleComponent {}
