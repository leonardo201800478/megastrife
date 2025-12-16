// src/cpu/z80.rs
use std::sync::{Arc, Mutex};
use crate::sound::Sound;

pub struct Z80 {
    pub ram: Vec<u8>,
    pub pc: u16,
    pub sp: u16,
    pub halted: bool,
    pub bus_taken: bool,
    pub sound: Arc<Mutex<Sound>>,
}

impl Z80 {
    pub fn new(sound: Arc<Mutex<Sound>>) -> Self {
        Self {
            ram: vec![0; 0x2000],
            pc: 0,
            sp: 0x1FFF,
            halted: false,
            bus_taken: false,
            sound,
        }
    }

    pub fn tick(&mut self) {
        let mut snd = self.sound.lock().unwrap();
        snd.tick();
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        self.ram[addr as usize % self.ram.len()]
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        let index = addr as usize % self.ram.len();
        self.ram[index] = val;
    }
}
