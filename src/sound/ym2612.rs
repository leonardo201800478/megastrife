// src/sound/ym2612.rs
pub struct Ym2612 {
    phase: f32,
}

impl Ym2612 {
    pub fn new() -> Self {
        Self { phase: 0.0 }
    }

    pub fn tick(&mut self) {
        self.phase = (self.phase + 0.01) % 1.0;
    }

    pub fn sample(&self) -> f32 {
        (self.phase.sin()) * 0.5
    }
}
