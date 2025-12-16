// src/sound/mod.rs
use std::sync::{Arc, Mutex};

pub mod psg;
pub mod ym2612;

use psg::Psg;
use ym2612::Ym2612;

/// Representa o sistema de som do Mega Drive (PSG + YM2612)
pub struct Sound {
    pub psg: Arc<Mutex<Psg>>,
    pub fm: Arc<Mutex<Ym2612>>,
}

impl Sound {
    pub fn new() -> Self {
        Self {
            psg: Arc::new(Mutex::new(Psg::new())),
            fm: Arc::new(Mutex::new(Ym2612::new())),
        }
    }

    /// Atualiza os chips de som por um ciclo
    pub fn tick(&mut self) {
        self.psg.lock().unwrap().tick();
        self.fm.lock().unwrap().tick();
    }

    /// Combina o áudio do PSG e do YM2612
    pub fn mix(&self) -> f32 {
        let psg_sample = self.psg.lock().unwrap().sample();
        let fm_sample = self.fm.lock().unwrap().sample();
        (psg_sample + fm_sample) / 2.0
    }
}
