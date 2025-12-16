//! Sistema de cartuchos ROM do Genesis

use anyhow::{Result, Context, bail};
use log::{debug, info, warn};
use std::fmt;

/// Regiões suportadas
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Region {
    Japan,
    USA,
    Europe,
    Brazil,
    Korea,
    Unknown,
}

/// Cabeçalho da ROM do Genesis
#[derive(Debug, Clone)]
pub struct RomHeader {
    /// Nome do jogo (máx 48 caracteres)
    pub title: String,
    
    /// Nomes dos copyright holders
    pub copyright: String,
    
    /// Nome do jogo no exterior
    pub overseas_title: String,
    
    /// Tipo do produto
    pub product_type: String,
    
    /// Código do produto
    pub product_code: String,
    
    /// Checksum
    pub checksum: u16,
    
    /// Região
    pub region: Region,
    
    /// Tamanho da ROM em KB
    pub rom_size_kb: u32,
    
    /// Tamanho da RAM em KB (0 = sem RAM)
    pub ram_size_kb: u32,
    
    /// Endereço de início da RAM
    pub ram_start: u32,
    
    /// Endereço de fim da RAM
    pub ram_end: u32,
}

/// Tipos de mapeamento de cartucho
#[derive(Debug, Clone, Copy, PartialEq)]
enum CartridgeType {
    Standard,      // ROM simples
    Sram,          // Com RAM de save
    SramWithBattery, // Com RAM + bateria
    Custom,        // Mapeamento customizado
}

/// Cartucho do Genesis
pub struct Cartridge {
    /// Dados da ROM
    rom_data: Vec<u8>,
    
    /// RAM de save (se existir)
    sram: Option<Vec<u8>>,
    
    /// Cabeçalho da ROM
    header: RomHeader,
    
    /// Tipo do cartucho
    cart_type: CartridgeType,
    
    /// Bancos de memória ativos
    bank_registers: [u8; 8],
    
    /// Banco atual (para ROMs maiores que 4MB)
    current_bank: usize,
}

impl Cartridge {
    /// Cria um novo cartucho a partir dos dados da ROM
    pub fn new(rom_data: Vec<u8>) -> Result<Self> {
        if rom_data.len() < 0x200 {
            bail!("ROM too small ({} bytes)", rom_data.len());
        }
        
        // Extrai cabeçalho
        let header: RomHeader = Self::parse_header(&rom_data)?;
        
        info!("ROM Header:");
        info!("  Title: '{}'", header.title);
        info!("  Region: {:?}", header.region);
        info!("  ROM Size: {} KB", header.rom_size_kb);
        info!("  RAM Size: {} KB", header.ram_size_kb);
        
        // Determina tipo do cartucho
        let cart_type: CartridgeType = Self::determine_cartridge_type(&header, &rom_data);
        
        // Aloca RAM de save se necessário
        let sram: Option<Vec<u8>> = if header.ram_size_kb > 0 {
            let sram_size: usize = header.ram_size_kb as usize * 1024;
            info!("Allocating {} KB of SRAM", header.ram_size_kb);
            Some(vec![0xFF; sram_size])
        } else {
            None
        };
        
        Ok(Self {
            rom_data,
            sram,
            header,
            cart_type,
            bank_registers: [0; 8],
            current_bank: 0,
        })
    }
    
    /// Parseia o cabeçalho da ROM
    fn parse_header(rom_data: &[u8]) -> Result<RomHeader> {
        let mut title: String = String::new();
        for i in 0x100..0x110 {
            if rom_data[i] != 0 {
                title.push(rom_data[i] as char);
            }
        }
        
        let mut copyright: String = String::new();
        for i in 0x110..0x120 {
            if rom_data[i] != 0 {
                copyright.push(rom_data[i] as char);
            }
        }
        
        let mut overseas_title: String = String::new();
        for i in 0x120..0x150 {
            if rom_data[i] != 0 {
                overseas_title.push(rom_data[i] as char);
            }
        }
        
        // Determina região
        let region_code: u8 = rom_data[0x1F0];
        let region: Region = match region_code {
            b'J' | b'j' => Region::Japan,
            b'U' | b'u' => Region::USA,
            b'E' | b'e' => Region::Europe,
            b'B' | b'b' => Region::Brazil,
            b'K' | b'k' => Region::Korea,
            _ => Region::Unknown,
        };
        
        // Tamanhos
        let rom_size_words: u32 = (rom_data[0x1A4] as u32) << 8 | rom_data[0x1A5] as u32;
        let rom_size_kb: u32 = (rom_size_words * 2) / 1024;
        
        let ram_size_words: u32 = (rom_data[0x1A8] as u32) << 8 | rom_data[0x1A9] as u32;
        let ram_size_kb: u32 = if ram_size_words > 0 {
            (ram_size_words * 2) / 1024
        } else {
            0
        };
        
        // Checksum
        let checksum: u16 = (rom_data[0x18E] as u16) << 8 | rom_data[0x18F] as u16;
        
        Ok(RomHeader {
            title,
            copyright,
            overseas_title,
            product_type: String::from_utf8_lossy(&rom_data[0x180..0x188]).to_string(),
            product_code: String::from_utf8_lossy(&rom_data[0x188..0x190]).to_string(),
            checksum,
            region,
            rom_size_kb,
            ram_size_kb,
            ram_start: 0xFF0000, // Padrão do Genesis
            ram_end: 0xFFFFFF,
        })
    }
    
    /// Determina o tipo do cartucho
    fn determine_cartridge_type(header: &RomHeader, rom_data: &[u8]) -> CartridgeType {
        // Verifica se tem SRAM
        if header.ram_size_kb > 0 {
            // Verifica se tem bateria (olhando alguns padrões comuns)
            let has_battery: bool = rom_data.len() > 0x1B0 && 
                             (rom_data[0x1B0] == 0x52 || rom_data[0x1B0] == 0x46); // 'R' ou 'F'
            
            if has_battery {
                CartridgeType::SramWithBattery
            } else {
                CartridgeType::Sram
            }
        } else {
            CartridgeType::Standard
        }
    }
    
    /// Lê um byte da ROM
    pub fn read_byte(&self, offset: u16) -> Result<u8> {
        let addr: usize = self.translate_address(offset);
        
        if addr < self.rom_data.len() {
            Ok(self.rom_data[addr])
        } else {
            Ok(0xFF) // Fora dos limites
        }
    }
    
    /// Lê um byte da SRAM
    pub fn read_sram(&self, offset: u16) -> Result<u8> {
        if let Some(sram) = &self.sram {
            let addr: usize = offset as usize;
            if addr < sram.len() {
                Ok(sram[addr])
            } else {
                Ok(0xFF)
            }
        } else {
            Ok(0xFF) // Sem SRAM
        }
    }
    
    /// Escreve um byte na SRAM
    pub fn write_sram(&mut self, offset: u16, value: u8) -> Result<()> {
        if let Some(sram) = &mut self.sram {
            let addr: usize = offset as usize;
            if addr < sram.len() {
                sram[addr] = value;
            }
        }
        Ok(())
    }
    
    /// Manipula troca de bancos de memória
    pub fn handle_bank_switch(&mut self, offset: u16, value: u8) -> Result<()> {
        // Implementação básica de bank switching
        // Em uma implementação real, isso dependeria do tipo de cartucho
        match offset {
            0x00..=0x07 => {
                self.bank_registers[offset as usize] = value;
                debug!("Bank register {} set to {:02X}", offset, value);
            }
            0x08..=0x0F => {
                // Alguns cartuchos usam escrita para trocar bancos
                self.current_bank = (value as usize) & 0x0F;
                debug!("Current bank switched to {}", self.current_bank);
            }
            _ => {
                // Ignora outras escritas em ROM
            }
        }
        Ok(())
    }
    
    /// Traduz endereço do espaço de memória para índice na ROM
    fn translate_address(&self, offset: u16) -> usize {
        let mut addr: usize = offset as usize;
        
        // Aplica bank switching se necessário
        if self.rom_data.len() > 0x400000 { // > 4MB
            let bank_size: usize = 0x400000; // 4MB por banco
            let bank: usize = self.current_bank;
            addr = (bank * bank_size) + (addr % bank_size);
        }
        
        // Garante que não ultrapassa os limites
        if addr >= self.rom_data.len() {
            addr %= self.rom_data.len();
        }
        
        addr
    }
    
    /// Obtém o cabeçalho da ROM
    pub fn get_header(&self) -> &RomHeader {
        &self.header
    }
    
    /// Salva a SRAM em um arquivo (para jogos com save)
    pub fn save_sram(&self, filename: &str) -> Result<()> {
        if let Some(sram) = &self.sram {
            std::fs::write(filename, sram)?;
            info!("SRAM saved to {}", filename);
        }
        Ok(())
    }
    
    /// Carrega a SRAM de um arquivo
    pub fn load_sram(&mut self, filename: &str) -> Result<()> {
        if let Some(sram) = &mut self.sram {
            let data: Vec<u8> = std::fs::read(filename)?;
            if data.len() == sram.len() {
                sram.copy_from_slice(&data);
                info!("SRAM loaded from {}", filename);
            } else {
                warn!("SRAM file size mismatch: expected {}, got {}", 
                      sram.len(), data.len());
            }
        }
        Ok(())
    }
    
    /// Verifica checksum da ROM
    pub fn verify_checksum(&self) -> bool {
        let mut calculated: u16 = 0u16;
        
        for chunk in self.rom_data.chunks(2) {
            if chunk.len() == 2 {
                let word: u16 = ((chunk[0] as u16) << 8) | (chunk[1] as u16);
                calculated = calculated.wrapping_add(word);
            }
        }
        
        calculated == self.header.checksum
    }
}

impl fmt::Debug for Cartridge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Cartridge")
            .field("title", &self.header.title)
            .field("rom_size", &self.header.rom_size_kb)
            .field("ram_size", &self.header.ram_size_kb)
            .field("type", &self.cart_type)
            .finish()
    }
}