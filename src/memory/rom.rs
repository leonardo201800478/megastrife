//! Implementação da ROM do Mega Drive / Sega Genesis.
//!
//! A ROM contém o código do jogo e o cabeçalho no formato padrão SEGA.
//! Este módulo permite ler dados e extrair informações do cabeçalho.

use std::fmt;

/// Representa uma ROM de cartucho do Mega Drive.
#[derive(Clone)]
pub struct Rom {
    data: Vec<u8>,
    header: RomHeader,
}

/// Estrutura do cabeçalho da ROM (64 bytes principais do cartucho).
#[derive(Debug, Clone)]
pub struct RomHeader {
    pub console_name: String,
    pub domestic_name: String,
    pub overseas_name: String,
    pub serial: String,
    pub checksum: u16,
    pub rom_start: u32,
    pub rom_end: u32,
    pub ram_start: u32,
    pub ram_end: u32,
    pub region: String,
}

impl fmt::Debug for Rom {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Rom")
            .field("size", &self.data.len())
            .field("header", &self.header)
            .finish()
    }
}

impl Rom {
    /// Cria uma nova ROM a partir de um vetor de bytes.
    pub fn new(data: Vec<u8>) -> Self {
        let header = RomHeader::parse(&data);
        Self { data, header }
    }

    /// Retorna o tamanho total da ROM.
    pub fn size(&self) -> usize {
        self.data.len()
    }

    /// Lê um byte (8 bits) da ROM.
    pub fn read8(&self, addr: u32) -> u8 {
        let index = (addr as usize) % self.data.len();
        self.data[index]
    }

    /// Lê uma palavra (16 bits) da ROM.
    pub fn read16(&self, addr: u32) -> u16 {
        let hi = self.read8(addr) as u16;
        let lo = self.read8(addr + 1) as u16;
        (hi << 8) | lo
    }

    /// Lê um bloco de dados (para DMA, leitura rápida etc.).
    pub fn read_block(&self, addr: u32, size: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(size);
        for i in 0..size {
            out.push(self.read8(addr + i as u32));
        }
        out
    }

    /// Retorna o cabeçalho parseado.
    pub fn header(&self) -> &RomHeader {
        &self.header
    }
}

/// Implementação de leitura e decodificação do cabeçalho SEGA.
impl RomHeader {
    pub fn parse(data: &[u8]) -> Self {
        // O cabeçalho começa em 0x100 no formato de ROM do Mega Drive.
        let safe_get_str = |offset: usize, len: usize| -> String {
            if offset + len <= data.len() {
                String::from_utf8_lossy(&data[offset..offset + len])
                    .trim_matches(char::from(0))
                    .trim()
                    .to_string()
            } else {
                String::from("UNKNOWN")
            }
        };

        let get_u16 = |offset: usize| -> u16 {
            if offset + 1 < data.len() {
                ((data[offset] as u16) << 8) | (data[offset + 1] as u16)
            } else {
                0
            }
        };

        let get_u32 = |offset: usize| -> u32 {
            if offset + 3 < data.len() {
                ((data[offset] as u32) << 24)
                    | ((data[offset + 1] as u32) << 16)
                    | ((data[offset + 2] as u32) << 8)
                    | (data[offset + 3] as u32)
            } else {
                0
            }
        };

        RomHeader {
            console_name: safe_get_str(0x100, 16),
            domestic_name: safe_get_str(0x120, 48),
            overseas_name: safe_get_str(0x150, 48),
            serial: safe_get_str(0x180, 14),
            checksum: get_u16(0x18E),
            rom_start: get_u32(0x1A0),
            rom_end: get_u32(0x1A4),
            ram_start: get_u32(0x1A8),
            ram_end: get_u32(0x1AC),
            region: safe_get_str(0x1F0, 3),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rom_read_basic() {
        let data = (0..256).collect::<Vec<u8>>();
        let rom = Rom::new(data.clone());
        assert_eq!(rom.read8(0x10), 0x10);
        assert_eq!(rom.read16(0x10), 0x1011);
        assert_eq!(rom.size(), 256);
    }

    #[test]
    fn test_rom_header_parsing_minimal() {
        // ROM mínima com cabeçalho "SEGA"
        let mut data = vec![0; 512];
        data[0x100..0x104].copy_from_slice(b"SEGA");
        data[0x18E..0x190].copy_from_slice(&[0x12, 0x34]); // checksum
        data[0x1A0..0x1A4].copy_from_slice(&[0x00, 0x00, 0x00, 0x00]);
        data[0x1A4..0x1A8].copy_from_slice(&[0x00, 0x3F, 0xFF, 0xFF]);
        data[0x1F0..0x1F3].copy_from_slice(b"JUE"); // Japão, EUA, Europa

        let rom = Rom::new(data);
        let header = rom.header();

        assert_eq!(header.console_name.starts_with("SEGA"), true);
        assert_eq!(header.checksum, 0x1234);
        assert_eq!(header.region, "JUE");
    }

    #[test]
    fn test_read_block() {
        let data = (0..64).collect::<Vec<u8>>();
        let rom = Rom::new(data.clone());
        let block = rom.read_block(0x10, 8);
        assert_eq!(block, data[0x10..0x18].to_vec());
    }
}
