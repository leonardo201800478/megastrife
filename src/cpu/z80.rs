// src/cpu/z80.rs
use std::sync::{Arc, Mutex};
use crate::sound::Sound;

// Endereço de escrita do PSG (SN76489) no espaço de endereçamento do Z80
// O endereço real é 0x7F11, mas a decodificação de endereço pode variar.
// Usamos 0x7F11 como o endereço de escrita do PSG.
const PSG_WRITE_ADDR: u16 = 0x7F11;

pub struct Z80 {
    pub ram: Vec<u8>,
    pub pc: u16,
    pub sp: u16,
    pub halted: bool,
    pub bus_taken: bool,
    // O Sound agora contém os chips PSG e YM2612
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

    /// Avança o emulador Z80 por um ciclo de clock.
    /// O Z80 no Mega Drive roda a 3.58 MHz.
    pub fn tick(&mut self) {
        // 1 ciclo Z80 (3.58MHz) = 15 ciclos do clock principal (53.69MHz)
        const MASTER_CYCLES_PER_Z80_TICK: u32 = 15;
        
        // O tick do Sound agora recebe o número de ciclos do clock principal
        // que o Z80 consumiu.
        let mut snd = self.sound.lock().unwrap();
        snd.tick(MASTER_CYCLES_PER_Z80_TICK);
    }

    pub fn read_byte(&self, addr: u16) -> u8 {
        // Lógica de leitura de memória (RAM do Z80)
        self.ram[addr as usize % self.ram.len()]
    }

    pub fn write_byte(&mut self, addr: u16, val: u8) {
        // --- Lógica de escrita nos chips de som ---
        
        // Escrita no PSG (SN76489)
        if addr == PSG_WRITE_ADDR {
            // O PSG é acessado através de um endereço de I/O.
            // A escrita é feita diretamente no chip.
            // O lock é necessário para acessar o PSG dentro do Sound
            self.sound.lock().unwrap().psg.write().write_data(val);
            return;
        }
        
        // Escrita normal na RAM do Z80
        let index = addr as usize % self.ram.len();
        self.ram[index] = val;
    }
}
