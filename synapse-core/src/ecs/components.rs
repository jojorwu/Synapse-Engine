
use crate::ecs::Component;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct PositionComponent {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl Component for PositionComponent {}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ColorComponent {
    pub r: f32,
    pub g: f32,
    pub b: f32,
}

impl Component for ColorComponent {}
