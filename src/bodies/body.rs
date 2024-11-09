use bytemuck::{ AnyBitPattern, NoUninit };
use ultraviolet::Vec2;

#[derive(Clone, Copy, NoUninit, AnyBitPattern)]
#[repr(C)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub acc: Vec2,
    pub mass: f32,
    pub radius: f32,
}

impl Body {
    pub fn new(pos: Vec2, vel: Vec2, mass: f32, radius: f32) -> Self {
        Self {
            pos,
            vel,
            acc: Vec2::zero(),
            mass,
            radius,
        }
    }
}
