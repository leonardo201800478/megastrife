//! Mapeamento de memória do Genesis

use super::rom::Cartridge;
use log::debug;

/// Regiões de memória do Genesis
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MemoryRegion {
    Bios,           // BIOS (0x000000-0x0003FF)
    WorkRam,        // RAM de trabalho (0xFF0000-0xFFFFFF)
    VideoRam,       // VRAM (0xC00000-0xC00003)
    SoundRam,       // RAM de som (0xA00000-0xA0FFFF)
    CartridgeRom,   // ROM do cartucho (0x000000-0x3FFFFF)
    CartridgeRam,   // RAM do cartucho (0x200000-0x20FFFF)
    IoRegisters,    // Registradores de I/O (0xA10000-0xA10FFF)
    Unmapped,       // Área não mapeada
}

/// Endereço mapeado
#[derive(Debug, Clone, Copy)]
pub struct MappedAddress {
    pub region: MemoryRegion,
    pub offset: u16,
}

/// Mapeador de memória do Genesis
pub struct MemoryMapper {
    /// TMSS lock (para consoles com TMSS)
    tmss_locked: bool,
    
    /// Configuração de mapeamento
    mapping_config: MappingConfig,
}

/// Configuração de mapeamento
#[derive(Debug, Clone)]
struct MappingConfig {
    cartridge_start: u32,
    cartridge_end: u32,
    sram_start: u32,
    sram_end: u32,
    sram_enabled: bool,
}

impl MemoryMapper {
    /// Cria um novo mapeador
    pub fn new() -> Self {
        Self {
            tmss_locked: false,
            mapping_config: MappingConfig {
                cartridge_start: 0x000000,
                cartridge_end: 0x3FFFFF,
                sram_start: 0x200000,
                sram_end: 0x20FFFF,
                sram_enabled: false,
            },
        }
    }
    
    /// Configura o mapeamento baseado no cartucho
    pub fn configure(&mut self, cartridge: &Cartridge) {
        let header: &crate::memory::RomHeader = cartridge.get_header();
        
        debug!("Configuring memory mapper for cartridge:");
        debug!("  ROM: {}KB", header.rom_size_kb);
        debug!("  RAM: {}KB", header.ram_size_kb);
        
        // Ativa SRAM se o cartucho tiver
        self.mapping_config.sram_enabled = header.ram_size_kb > 0;
        
        if header.ram_size_kb > 0 {
            debug!("SRAM enabled at {:08X}-{:08X}", 
                   self.mapping_config.sram_start,
                   self.mapping_config.sram_end);
        }
    }
    
    /// Mapeia um endereço de 24 bits para uma região
    pub fn map_address(&self, address: u32) -> MappedAddress {
        let addr_24bit: u32 = address & 0x00FFFFFF; // Endereços são 24 bits no Genesis
        
        match addr_24bit {
            // BIOS/TMSS Area
            0x000000..=0x0003FF => {
                if self.tmss_locked {
                    // TMSS mostra "LICENSED BY SEGA" quando bloqueado
                    MappedAddress {
                        region: MemoryRegion::Bios,
                        offset: (addr_24bit - 0x000000) as u16,
                    }
                } else {
                    // Normalmente mapeado para cartucho
                    self.map_cartridge_address(addr_24bit)
                }
            }
            
            // Cartridge ROM Area (0x000000-0x1FFFFF and 0x210000-0x3FFFFF)
            0x000000..=0x1FFFFF | 0x210000..=0x3FFFFF => {
                self.map_cartridge_address(addr_24bit)
            }
            
            // Cartridge RAM/SRAM Area (0x200000-0x20FFFF)
            0x200000..=0x20FFFF => {
                if self.mapping_config.sram_enabled {
                    MappedAddress {
                        region: MemoryRegion::CartridgeRam,
                        offset: (addr_24bit - 0x200000) as u16,
                    }
                } else {
                    // Sem SRAM, volta para ROM
                    self.map_cartridge_address(addr_24bit)
                }
            }
            
            // I/O Registers (0xA10000-0xA10FFF)
            0xA10000..=0xA10FFF => {
                let offset = (addr_24bit - 0xA10000) as u16;
                MappedAddress {
                    region: MemoryRegion::IoRegisters,
                    offset,
                }
            }
            
            // Z80 Address Space / Sound RAM (0xA00000-0xA0FFFF)
            0xA00000..=0xA0FFFF => {
                if addr_24bit <= 0xA01FFF {
                    // Sound RAM (8KB)
                    MappedAddress {
                        region: MemoryRegion::SoundRam,
                        offset: (addr_24bit - 0xA00000) as u16,
                    }
                } else {
                    // Z80 I/O ou não mapeado
                    MappedAddress {
                        region: MemoryRegion::Unmapped,
                        offset: 0,
                    }
                }
            }
            
            // VDP Registers (0xC00000-0xC0001F)
            0xC00000..=0xC0001F => {
                // Acesso ao VDP - tratado pelo módulo VDP
                MappedAddress {
                    region: MemoryRegion::VideoRam,
                    offset: (addr_24bit - 0xC00000) as u16,
                }
            }
            
            // Work RAM (0xFF0000-0xFFFFFF)
            0xFF0000..=0xFFFFFF => {
                MappedAddress {
                    region: MemoryRegion::WorkRam,
                    offset: (addr_24bit - 0xFF0000) as u16,
                }
            }
            
            // Unmapped areas
            _ => {
                MappedAddress {
                    region: MemoryRegion::Unmapped,
                    offset: 0,
                }
            }
        }
    }
    
    /// Mapeia endereço da área do cartucho
    fn map_cartridge_address(&self, address: u32) -> MappedAddress {
        if address >= self.mapping_config.sram_start && 
           address <= self.mapping_config.sram_end &&
           self.mapping_config.sram_enabled {
            
            MappedAddress {
                region: MemoryRegion::CartridgeRam,
                offset: (address - self.mapping_config.sram_start) as u16,
            }
        } else {
            MappedAddress {
                region: MemoryRegion::CartridgeRom,
                offset: address as u16, // Limite de 64KB, mas o cartucho lida com bank switching
            }
        }
    }
    
    /// Ativa/desativa TMSS lock
    pub fn set_tmss_lock(&mut self, locked: bool) {
        self.tmss_locked = locked;
        debug!("TMSS lock {}", if locked { "enabled" } else { "disabled" });
    }
}