// src/cpu/bus.rs

//! Implementa o barramento (bus) de comunicação entre a CPU e os periféricos
//! como memória, ROM, VDP, e dispositivos de I/O.

use thiserror::Error;

/// Representa um erro de acesso ao barramento.
#[derive(Debug, Error)]
pub enum BusError {
    #[error("Tentativa de leitura inválida no endereço 0x{0:08X}")]
    InvalidRead(u32),

    #[error("Tentativa de escrita inválida no endereço 0x{0:08X}")]
    InvalidWrite(u32),

    #[error("Endereço fora do intervalo válido: 0x{0:08X}")]
    OutOfRange(u32),
}

/// Trait genérica para dispositivos mapeados em memória.
pub trait MemoryMappedDevice {
    fn read8(&mut self, addr: u32) -> Result<u8, BusError>;
    fn read16(&mut self, addr: u32) -> Result<u16, BusError>;
    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError>;
    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusError>;
}

/// Estrutura principal do barramento da CPU M68000.
pub struct Bus {
    pub ram: Vec<u8>,
    pub rom: Vec<u8>,
    pub rom_base: u32,
    pub ram_base: u32,
    pub ram_size: u32,
}

impl Bus {
    /// Cria um novo barramento com ROM e RAM inicializadas.
    pub fn new(rom_data: Vec<u8>, ram_size: usize) -> Self {
        Self {
            ram: vec![0; ram_size],
            rom: rom_data,
            rom_base: 0x000000,
            ram_base: 0xFF0000,
            ram_size: ram_size as u32,
        }
    }

    /// Lê um byte do barramento.
    pub fn read8(&mut self, addr: u32) -> Result<u8, BusError> {
        if addr < self.rom.len() as u32 {
            Ok(self.rom[addr as usize])
        } else if addr >= self.ram_base && addr < self.ram_base + self.ram_size {
            Ok(self.ram[(addr - self.ram_base) as usize])
        } else {
            Err(BusError::InvalidRead(addr))
        }
    }

    /// Lê uma palavra (16 bits).
    pub fn read16(&mut self, addr: u32) -> Result<u16, BusError> {
        let hi: u16 = self.read8(addr)? as u16;
        let lo: u16 = self.read8(addr + 1)? as u16;
        Ok((hi << 8) | lo)
    }

    /// Escreve um byte no barramento.
    pub fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusError> {
        if addr >= self.ram_base && addr < self.ram_base + self.ram_size {
            let offset = (addr - self.ram_base) as usize;
            self.ram[offset] = value;
            Ok(())
        } else {
            Err(BusError::InvalidWrite(addr))
        }
    }

    /// Escreve uma palavra (16 bits).
    pub fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusError> {
        let hi: u8 = (value >> 8) as u8;
        let lo: u8 = (value & 0xFF) as u8;
        self.write8(addr, hi)?;
        self.write8(addr + 1, lo)
    }

    /// Lê um valor de 32 bits (long word).
    pub fn read32(&mut self, addr: u32) -> Result<u32, BusError> {
        let w1: u32 = self.read16(addr)? as u32;
        let w2: u32 = self.read16(addr + 2)? as u32;
        Ok((w1 << 16) | w2)
    }

    /// Escreve um valor de 32 bits (long word).
    pub fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusError> {
        let hi: u16 = (value >> 16) as u16;
        let lo: u16 = (value & 0xFFFF) as u16;
        self.write16(addr, hi)?;
        self.write16(addr + 2, lo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bus_rom_read() {
        let rom: Vec<u8> = vec![0x12, 0x34, 0x56, 0x78];
        let mut bus: Bus = Bus::new(rom, 64 * 1024);
        assert_eq!(bus.read8(0x000000).unwrap(), 0x12);
        assert_eq!(bus.read16(0x000000).unwrap(), 0x1234);
    }

    #[test]
    fn test_bus_ram_write() {
        let rom: Vec<u8> = vec![0xFF; 4];
        let mut bus: Bus = Bus::new(rom, 64 * 1024);
        let addr: u32 = 0xFF0000;
        bus.write8(addr, 0xAA).unwrap();
        assert_eq!(bus.read8(addr).unwrap(), 0xAA);
    }

    #[test]
    fn test_bus_invalid_access() {
        let rom: Vec<u8> = vec![0xFF; 4];
        let mut bus: Bus = Bus::new(rom, 64 * 1024);
        assert!(bus.read8(0x200000).is_err());
    }
}
