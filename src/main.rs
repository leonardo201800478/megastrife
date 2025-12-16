// src/main.rs
mod cpu;
mod sound;
mod vdp;
mod io;
mod memory;

use std::sync::{Arc, Mutex};
use cpu::z80::Z80;
use sound::Sound;
use vdp::Vdp;
use io::Io;

fn main() {
    let sound = Arc::new(Mutex::new(Sound::new()));
    let z80 = Arc::new(Mutex::new(Z80::new(sound.clone())));
    let vdp = Arc::new(Mutex::new(Vdp::new(false)));

    let mut io = Io::new(z80.clone(), vdp.clone(), sound.clone());

    loop {
        io.tick();
        std::thread::sleep(std::time::Duration::from_millis(16));
    }
}
