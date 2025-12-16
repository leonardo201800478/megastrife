//! Implementação da VSRAM (Vertical Scroll RAM) do VDP.
//!
//! Cada entrada de 16 bits controla o scroll vertical de um bloco de 8 pixels
//! em um plano A ou B. Existem 40 entradas (80 bytes), cada uma aplicada a uma
//! célula de 8 pixels na horizontal.
//!
//! Usado para efeitos de Line Scroll e paralaxe avançada.

use std::ops::{Index, IndexMut};

pub const VSRAM_SIZE: usize = 0x50; // 80 bytes = 40 entradas de 16 bits

#[derive(Clone)]
pub struct Vsram {
    data: [u16; VSRAM_SIZE / 2],
}

impl Vsram {
    pub fn new() -> Self {
        Self { data: [0; VSRAM_SIZE / 2] }
    }

    pub fn read16(&self, addr: u32) -> u16 {
        let idx = ((addr >> 1) as usize) % self.data.len();
        self.data[idx]
    }

    pub fn write16(&mut self, addr: u32, value: u16) {
        let idx = ((addr >> 1) as usize) % self.data.len();
        self.data[idx] = value;
    }

    /// Retorna o deslocamento vertical para uma coluna X (em pixels)
    pub fn line_scroll_offset(&self, x: usize) -> i16 {
        let idx = (x / 8) % self.data.len();
        self.data[idx] as i16
    }

    /// Zera a VSRAM
    pub fn reset(&mut self) {
        self.data.fill(0);
    }
}

impl Default for Vsram {
    fn default() -> Self {
        Self::new()
    }
}

impl Index<usize> for Vsram {
    type Output = u16;
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index % self.data.len()]
    }
}

impl IndexMut<usize> for Vsram {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index % self.data.len()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vsram_write_read() {
        let mut vs = Vsram::new();
        vs.write16(0, 0x1234);
        assert_eq!(vs.read16(0), 0x1234);
    }

    #[test]
    fn test_line_scroll_offset() {
        let mut vs = Vsram::new();
        vs.write16(0, 5);
        vs.write16(2, 10);
        assert_eq!(vs.line_scroll_offset(0), 5);
        assert_eq!(vs.line_scroll_offset(16), 10);
    }
}
