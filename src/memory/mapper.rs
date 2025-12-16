//! Mapper de memória do Mega Drive / Genesis
//!
//! Controla o roteamento de endereços entre ROM, SRAM e variações de mapeamento
//! (SEGA, Codemasters, etc). Fornece suporte básico a EEPROM serial.

use crate::memory::rom::Rom;
use std::sync::{Arc, Mutex};

/// Tipos de mapper suportados
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapperType {
    /// Mapeamento direto padrão (ROM linear de até 4 MB)
    Standard,
    /// Mapper SEGA — usado por jogos maiores (ex: Sonic 3, Virtua Racing)
    Sega,
    /// Mapper Codemasters — usado em jogos como "Micro Machines"
    Codemasters,
    /// Mapper com SRAM (bateria)
    Sram,
    /// Mapper com EEPROM (serial)
    Eeprom,
}

/// Estrutura principal de mapeamento de ROM/SRAM.
pub struct Mapper {
    pub rom: Arc<Mutex<Rom>>,
    pub sram: Option<Vec<u8>>,
    pub mapper_type: MapperType,
    pub bank: usize, // banco ativo para mappers SEGA/Codemasters
}

impl Mapper {
    /// Cria um novo mapper com a ROM e tipo desejado.
    pub fn new(rom: Rom, mapper_type: MapperType) -> Self {
        let sram = match mapper_type {
            MapperType::Sram => Some(vec![0; 0x10000]), // 64KB de SRAM
            _ => None,
        };

        Self {
            rom: Arc::new(Mutex::new(rom)),
            sram,
            mapper_type,
            bank: 0,
        }
    }

    /// Lê um byte (8 bits) de ROM ou SRAM conforme o tipo de mapper.
    pub fn read8(&self, addr: u32) -> u8 {
        match self.mapper_type {
            MapperType::Standard => self.read_rom(addr),
            MapperType::Sega => self.read_sega(addr),
            MapperType::Codemasters => self.read_codemasters(addr),
            MapperType::Sram => self.read_sram(addr),
            MapperType::Eeprom => self.read_eeprom(addr),
        }
    }

    /// Escrita de byte (8 bits) — SRAM, EEPROM ou troca de banco.
    pub fn write8(&mut self, addr: u32, value: u8) {
        match self.mapper_type {
            MapperType::Sram => self.write_sram(addr, value),
            MapperType::Eeprom => self.write_eeprom(addr, value),
            MapperType::Sega => self.handle_sega_bank_switch(addr, value),
            MapperType::Codemasters => self.handle_codemasters_bank_switch(addr, value),
            _ => {}
        }
    }

    /// Leitura direta de ROM padrão (espelhada até 4MB)
    fn read_rom(&self, addr: u32) -> u8 {
        let rom = self.rom.lock().unwrap();
        rom.read8(addr % rom.size() as u32)
    }

    /// Leitura de ROM com mapeamento SEGA (banco de 512 KB)
    fn read_sega(&self, addr: u32) -> u8 {
        let rom = self.rom.lock().unwrap();
        let bank_offset = (self.bank * 0x80000) as u32;
        let offset = (addr % 0x80000) + bank_offset;
        rom.read8(offset % rom.size() as u32)
    }

    /// Leitura Codemasters (banco de 256 KB)
    fn read_codemasters(&self, addr: u32) -> u8 {
        let rom = self.rom.lock().unwrap();
        let bank_offset = (self.bank * 0x40000) as u32;
        let offset = (addr % 0x40000) + bank_offset;
        rom.read8(offset % rom.size() as u32)
    }

    /// Leitura com SRAM (faixa 0x200000–0x20FFFF)
    fn read_sram(&self, addr: u32) -> u8 {
        if (0x200000..=0x20FFFF).contains(&addr) {
            if let Some(ref sram) = self.sram {
                return sram[(addr as usize - 0x200000) % sram.len()];
            }
        }
        self.read_rom(addr)
    }

    /// Escrita na SRAM (com bateria)
    fn write_sram(&mut self, addr: u32, value: u8) {
        if (0x200000..=0x20FFFF).contains(&addr) {
            if let Some(ref mut sram) = self.sram {
                let offset = (addr as usize - 0x200000) % sram.len();
                sram[offset] = value;
            }
        }
    }

    /// Leitura simulada de EEPROM serial (faixa 0x200000–0x200001)
    fn read_eeprom(&self, addr: u32) -> u8 {
        if addr & 1 == 0 {
            // bit de status
            0xFF
        } else {
            0x00
        }
    }

    /// Escrita simulada de EEPROM serial (não persiste, apenas emula ACK)
    fn write_eeprom(&mut self, _addr: u32, _value: u8) {
        // EEPROM fictícia — sem armazenamento persistente.
    }

    /// Troca de banco SEGA — escrita em 0xA130F1 define banco ativo
    fn handle_sega_bank_switch(&mut self, addr: u32, value: u8) {
        if addr == 0xA130F1 {
            self.bank = (value & 0x07) as usize;
        }
    }

    /// Troca de banco Codemasters — escrita em 0x0000 controla o banco
    fn handle_codemasters_bank_switch(&mut self, addr: u32, value: u8) {
        if addr & 0x400000 == 0x000000 {
            self.bank = (value & 0x0F) as usize;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::rom::Rom;

    #[test]
    fn test_standard_mapper_reads_rom() {
        let data = (0..255).collect::<Vec<u8>>();
        let rom = Rom::new(data.clone());
        let mapper = Mapper::new(rom, MapperType::Standard);
        assert_eq!(mapper.read8(0x10), 0x10);
        assert_eq!(mapper.read8(0x200000), data[0]);
    }

    #[test]
    fn test_sram_write_and_read() {
        let data = vec![0; 256];
        let rom = Rom::new(data);
        let mut mapper = Mapper::new(rom, MapperType::Sram);
        mapper.write8(0x200000, 0xAA);
        assert_eq!(mapper.read8(0x200000), 0xAA);
    }

    #[test]
    fn test_sega_bank_switch() {
        let data = (0..255).cycle().take(0x100000).collect::<Vec<u8>>();
        let rom = Rom::new(data);
        let mut mapper = Mapper::new(rom, MapperType::Sega);
        mapper.handle_sega_bank_switch(0xA130F1, 3);
        assert_eq!(mapper.bank, 3);
    }

    #[test]
    fn test_codemasters_bank_switch() {
        let data = vec![0; 0x80000];
        let rom = Rom::new(data);
        let mut mapper = Mapper::new(rom, MapperType::Codemasters);
        mapper.handle_codemasters_bank_switch(0x0000, 2);
        assert_eq!(mapper.bank, 2);
    }
}
