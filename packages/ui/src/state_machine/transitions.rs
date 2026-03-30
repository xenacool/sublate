use tween::{Tween, Linear};

pub struct AnimationStep {
    pub from: f32,
    pub to: f32,
    pub duration: f32,
}

impl AnimationStep {
    pub fn new(from: f32, to: f32, duration: f32) -> Self {
        Self { from, to, duration }
    }
}
