// src/memory/bus.rs
use crate::cpu::z80::Z80;
use crate::memory::{Mapper, Ram, Rom};
use crate::sound::Sound;
use crate::vdp::Vdp;
use std::sync::{Arc, Mutex};

pub struct Bus {
    pub z80: Arc<Mutex<Z80>>,
    pub vdp: Arc<Mutex<Vdp>>,
    pub sound: Arc<Mutex<Sound>>,
    pub ram: Arc<Mutex<Ram>>,
    pub rom: Arc<Mutex<Rom>>,
    pub mapper: Arc<Mutex<Mapper>>,
}

impl Bus {
    pub fn new(
        z80: Arc<Mutex<Z80>>,
        vdp: Arc<Mutex<Vdp>>,
        sound: Arc<Mutex<Sound>>,
        ram: Arc<Mutex<Ram>>,
        rom: Arc<Mutex<Rom>>,
        mapper: Arc<Mutex<Mapper>>,
    ) -> Self {
        Self {
            z80,
            vdp,
            sound,
            ram,
            rom,
            mapper,
        }
    }

    pub fn read8(&self, addr: u32) -> u8 {
        match addr {
            0xA00000..=0xA0FFFF => self.z80.lock().unwrap().read_byte(addr as u16),
            0xC00000..=0xC0001F => self.vdp.lock().unwrap().bus_read(addr),
            0xFF0000..=0xFFFFFF => self.ram.lock().unwrap().read8(addr),
            _ => 0,
        }
    }

    pub fn write8(&self, addr: u32, value: u8) {
        match addr {
            0xA00000..=0xA0FFFF => self.z80.lock().unwrap().write_byte(addr as u16, value),
            0xC00000..=0xC0003F => self.vdp.lock().unwrap().bus_write(addr, value),
            0xFF0000..=0xFFFFFF => self.ram.lock().unwrap().write8(addr, value),
            _ => {}
        }
    }

    pub fn tick(&self) {
        self.vdp.lock().unwrap().tick();
        self.sound.lock().unwrap().tick();
    }

    pub fn render_frame(&self) -> Vec<u32> {
        let mut vdp = self.vdp.lock().unwrap();
        vdp.render_frame();
        vdp.framebuffer.pixels.clone()
    }

    pub fn read16(&self, addr: u32) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr + 1) as u16;
        (hi << 8) | lo
    }

    pub fn write16(&self, addr: u32, value: u16) {
        self.write8(addr, (value & 0xFF) as u8);
        self.write8(addr + 1, (value >> 8) as u8);
    }
}
