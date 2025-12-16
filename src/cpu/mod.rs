// src/cpu/mod.rs

pub mod z80;
use std::sync::{Arc, Mutex};
use crate::sound::Sound;
use crate::vdp::Vdp;
use crate::vdp::VdpInterruptType;
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
                VdpInterruptType::VBlank => {
                    // trata interrupção de VBlank
                }
                VdpInterruptType::HBlank => {
                    // trata interrupção de HBlank
                }
                VdpInterruptType::Scanline => {
                    // trata interrupção de Scanline
                }
                VdpInterruptType::SpriteOverflow => {
                    // trata interrupção de SpriteOverflow
                }
                VdpInterruptType::SpriteCollision => {
                    // trata interrupção de SpriteCollision
                }
                VdpInterruptType::DmaComplete => {
                    // trata interrupção de DmaComplete
                }
            }
        }
    }
}
