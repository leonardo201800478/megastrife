//! Controlador DMA do VDP

use super::vram::Vram;
use anyhow::Result;

pub struct DmaController {
    active: bool,
    mode: DmaMode,
    source: u32,
    length: u16,
    address: u16,
    fill_data: u16,
}

#[derive(Debug, Clone, Copy)]
enum DmaMode {
    MemoryToVram,
    VramFill,
    VramCopy,
    Disabled,
}

impl DmaController {
    pub fn new() -> Self {
        Self {
            active: false,
            mode: DmaMode::Disabled,
            source: 0,
            length: 0,
            address: 0,
            fill_data: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn step(&mut self, vram: &mut Vram) -> Result<()> {
        if !self.active || self.length == 0 {
            self.active = false;
            return Ok(());
        }

        match self.mode {
            DmaMode::MemoryToVram => {
                // DMA de memória para VRAM
                // Em uma implementação real, você leria da memória do sistema
                vram.write_word(self.address, 0)?;
            }
            DmaMode::VramFill => {
                // Preenchimento de VRAM
                vram.write_word(self.address, self.fill_data)?;
            }
            DmaMode::VramCopy => {
                // Cópia dentro da VRAM
                let data: () = vram.read_word(self.source as u16)?;
                vram.write_word(self.address, data)?;
            }
            DmaMode::Disabled => return Ok(()),
        }

        self.address = self.address.wrapping_add(2);
        self.length = self.length.wrapping_sub(1);

        if self.length == 0 {
            self.active = false;
        }

        Ok(())
    }

    pub fn start(&mut self, mode: DmaMode, source: u32, length: u16, address: u16) {
        self.active = true;
        self.mode = mode;
        self.source = source;
        self.length = length;
        self.address = address;
    }
}
