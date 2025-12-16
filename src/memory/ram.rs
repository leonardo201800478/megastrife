//! Implementação da RAM principal do Mega Drive / Sega Genesis.
//!
//! A RAM do sistema possui 64 KB endereçados entre 0xFF0000 e 0xFFFFFF.
//! Este módulo fornece acesso seguro e rápido à memória de trabalho do 68000.

use std::fmt;

/// Estrutura principal da RAM do sistema.
/// Armazena bytes lineares com suporte a leitura e escrita em 8, 16 e 32 bits.
#[derive(Clone)]
pub struct Ram {
    data: Vec<u8>,
}

impl fmt::Debug for Ram {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Ram")
            .field("size", &self.data.len())
            .finish()
    }
}

impl Ram {
    /// Cria uma nova RAM com o tamanho especificado (em bytes).
    pub fn new(size: usize) -> Self {
        Self {
            data: vec![0; size],
        }
    }

    /// Retorna o tamanho total da RAM em bytes.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Lê um byte (8 bits) da RAM.
    pub fn read8(&self, addr: u32) -> u8 {
        let index: usize = (addr as usize) % self.data.len();
        self.data[index]
    }

    /// Lê uma palavra (16 bits, big-endian) da RAM.
    pub fn read16(&self, addr: u32) -> u16 {
        let hi: u16 = self.read8(addr) as u16;
        let lo: u16 = self.read8(addr.wrapping_add(1)) as u16;
        (hi << 8) | lo
    }

    /// Lê um longword (32 bits, big-endian) da RAM.
    pub fn read32(&self, addr: u32) -> u32 {
        let b0: u32 = self.read8(addr) as u32;
        let b1: u32 = self.read8(addr.wrapping_add(1)) as u32;
        let b2: u32 = self.read8(addr.wrapping_add(2)) as u32;
        let b3: u32 = self.read8(addr.wrapping_add(3)) as u32;
        (b0 << 24) | (b1 << 16) | (b2 << 8) | b3
    }

    /// Escreve um byte (8 bits) na RAM.
    pub fn write8(&mut self, addr: u32, value: u8) {
        let index: usize = (addr as usize) % self.data.len();
        self.data[index] = value;
    }

    /// Escreve uma palavra (16 bits) na RAM.
    pub fn write16(&mut self, addr: u32, value: u16) {
        self.write8(addr, (value >> 8) as u8);
        self.write8(addr.wrapping_add(1), (value & 0xFF) as u8);
    }

    /// Escreve um longword (32 bits) na RAM.
    pub fn write32(&mut self, addr: u32, value: u32) {
        self.write8(addr, ((value >> 24) & 0xFF) as u8);
        self.write8(addr.wrapping_add(1), ((value >> 16) & 0xFF) as u8);
        self.write8(addr.wrapping_add(2), ((value >> 8) & 0xFF) as u8);
        self.write8(addr.wrapping_add(3), (value & 0xFF) as u8);
    }

    /// Preenche toda a RAM com um valor fixo (útil para reset).
    pub fn fill(&mut self, value: u8) {
        self.data.fill(value);
    }

    /// Retorna uma cópia do conteúdo interno da RAM.
    pub fn dump(&self) -> Vec<u8> {
        self.data.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ram_read_write_8bit() {
        let mut ram = Ram::new(1024);
        ram.write8(0x10, 0xAB);
        assert_eq!(ram.read8(0x10), 0xAB);
    }

    #[test]
    fn test_ram_read_write_16bit() {
        let mut ram = Ram::new(1024);
        ram.write16(0x10, 0xBEEF);
        assert_eq!(ram.read16(0x10), 0xBEEF);
    }

    #[test]
    fn test_ram_read_write_32bit() {
        let mut ram = Ram::new(1024);
        ram.write32(0x20, 0x12345678);
        assert_eq!(ram.read32(0x20), 0x12345678);
    }

    #[test]
    fn test_ram_wraparound() {
        let mut ram = Ram::new(16);
        ram.write8(0x20, 0x55); // endereço 0x20 deve espelhar
        assert_eq!(ram.read8(0x20 % 16), 0x55);
    }

    #[test]
    fn test_ram_fill_and_dump() {
        let mut ram = Ram::new(32);
        ram.fill(0xAA);
        let dump = ram.dump();
        assert_eq!(dump.iter().all(|&b| b == 0xAA), true);
    }
}
