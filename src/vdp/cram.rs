//! CRAM (Color RAM) - Paleta de cores

use anyhow::{Context, Result};

pub struct Cram {
    colors: [u32; 64], // 64 cores (0-63)
}

impl Cram {
    pub fn new() -> Self {
        Self { colors: [0; 64] }
    }

    pub fn clear(&mut self) {
        self.colors = [0; 64];
    }

    pub fn read_color(&self, address: u16) -> Result<u32> {
        let index = (address & 0x3F) as usize;
        Ok(self.colors[index])
    }

    pub fn write_color(&mut self, address: u16, r: u8, g: u8, b: u8) -> Result<()> {
        let index = (address & 0x3F) as usize;

        // Formato ARGB 32-bit
        self.colors[index] = 0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);

        Ok(())
    }

    pub fn get_palette(&self, palette_num: u8) -> [u32; 16] {
        let mut palette = [0u32; 16];
        let base = (palette_num as usize * 16) % 64;

        for i in 0..16 {
            palette[i] = self.colors[(base + i) % 64];
        }

        palette
    }
}
