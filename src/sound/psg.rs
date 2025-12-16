// src/sound/psg.rs
pub struct Psg {
    counter: u32,
}

impl Psg {
    pub fn new() -> Self {
        Self { counter: 0 }
    }

    pub fn tick(&mut self) {
        self.counter = self.counter.wrapping_add(1);
    }

    pub fn sample(&self) -> f32 {
        ((self.counter % 100) as f32 / 100.0) * 0.5
    }
}
