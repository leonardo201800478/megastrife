// src/cpu/mod.rs

pub mod z80;
use std::sync::{Arc, Mutex};
use crate::sound::Sound;
use crate::vdp::{Vdp, interrupts::VdpInterrupt};
use crate::cpu::z80::Z80;

pub struct Cpu {
    pub z80: Arc<Mutex<Z80>>,
    pub vdp: Arc<Mutex<Vdp>>,
    pub sound: Arc<Mutex<Sound>>,
    
}

impl Cpu {
    pub fn new(z80: Arc<Mutex<Z80>>, vdp: Arc<Mutex<Vdp>>, sound: Arc<Mutex<Sound>>) -> Self {
        Self { z80, vdp, sound }
    }

    pub fn tick(&mut self) {
        self.z80.lock().unwrap().tick();
        self.vdp.lock().unwrap().tick();
        self.sound.lock().unwrap().tick();

        if let Some(interrupt) = self.vdp.lock().unwrap().poll_interrupt() {
            match interrupt {
                VdpInterrupt::VBlank => {
                    // trata interrupção de VBlank
                }
                VdpInterrupt::HBlank => {
                    // trata interrupção de HBlank
                }
            }
        }
    }
}
