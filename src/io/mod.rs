// src/io/mod.rs
use std::sync::{Arc, Mutex};
use crate::cpu::z80::Z80;
use crate::sound::Sound;
use crate::vdp::Vdp;

/// Interface de entrada/saída (porta de controle entre CPU, VDP e Som)
pub struct Io {
    pub z80: Arc<Mutex<Z80>>,
    pub vdp: Arc<Mutex<Vdp>>,
    pub sound: Arc<Mutex<Sound>>,
}

impl Io {
    pub fn new(z80: Arc<Mutex<Z80>>, vdp: Arc<Mutex<Vdp>>, sound: Arc<Mutex<Sound>>) -> Self {
        Self { z80, vdp, sound }
    }

    pub fn tick(&mut self) {
        self.vdp.lock().unwrap().tick();
        self.sound.lock().unwrap().tick();
    }
}
