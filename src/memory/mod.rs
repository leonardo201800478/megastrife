//! Sistema de memória do Sega Genesis/Mega Drive

mod bus;
mod ram;
mod rom;
mod mapper;

pub use bus::MemoryBus;
pub use ram::{WorkRam, VideoRam, SoundRam};
pub use rom::{Cartridge, RomHeader};
pub use mapper::MemoryMapper;

use anyhow::{Result, Context, bail};
use log::{debug, info, warn, trace};

/// Sistema completo de memória do Genesis
pub struct MemorySystem {
    /// RAM de trabalho (64KB)
    work_ram: WorkRam,
    
    /// RAM de vídeo (64KB)
    vram: VideoRam,
    
    /// RAM de som (8KB)
    sound_ram: SoundRam,
    
    /// Cartucho ROM
    cartridge: Option<Cartridge>,
    
    /// Mapeador de memória
    mapper: MemoryMapper,
    
    /// BIOS do sistema (opcional)
    bios: Option<Vec<u8>>,
    
    /// Registradores de I/O
    io_registers: [u8; 0x100],
}

impl MemorySystem {
    /// Cria um novo sistema de memória
    pub fn new() -> Self {
        Self {
            work_ram: WorkRam::new(),
            vram: VideoRam::new(),
            sound_ram: SoundRam::new(),
            cartridge: None,
            mapper: MemoryMapper::new(),
            bios: None,
            io_registers: [0; 0x100],
        }
    }
    
    /// Carrega uma ROM de cartucho
    pub fn load_cartridge(&mut self, rom_data: Vec<u8>) -> Result<()> {
        info!("Loading cartridge ROM: {} bytes", rom_data.len());
        
        let cartridge: Cartridge = Cartridge::new(rom_data)?;
        let header: &RomHeader = cartridge.get_header();
        
        info!("Cartridge Info:");
        info!("  Title: {}", header.title);
        info!("  Region: {:?}", header.region);
        info!("  ROM Size: {} KB", header.rom_size_kb);
        info!("  RAM Size: {} KB", header.ram_size_kb);
        info!("  Checksum: {:04X}", header.checksum);
        
        self.cartridge = Some(cartridge);
        
        // Configura mapeamento baseado no cartucho
        self.mapper.configure(&self.cartridge.as_ref().unwrap());
        
        Ok(())
    }
    
    /// Carrega BIOS (opcional)
    pub fn load_bios(&mut self, bios_data: Vec<u8>) -> Result<()> {
        if bios_data.len() != 0x2000 && bios_data.len() != 0x4000 {
            bail!("BIOS must be 8KB or 16KB, got {} bytes", bios_data.len());
        }
        
        self.bios = Some(bios_data);
        debug!("BIOS loaded: {} bytes", self.bios.as_ref().unwrap().len());
        Ok(())
    }
    
    /// Reseta o sistema de memória
    pub fn reset(&mut self) {
        debug!("Resetting memory system");
        
        self.work_ram.clear();
        self.vram.clear();
        self.sound_ram.clear();
        self.io_registers = [0; 0x100];
        
        // Inicializa alguns registros de I/O com valores padrão
        self.io_registers[0x00] = 0x00; // Version register
        self.io_registers[0x01] = 0x00; // Peripheral control
        self.io_registers[0x02] = 0x00; // TMSS register
    }
    
    /// Obtém informações sobre o cartucho
    pub fn get_cartridge_info(&self) -> Option<&RomHeader> {
        self.cartridge.as_ref().map(|c| c.get_header())
    }
    
    /// Lê um byte da memória (interface para CPU)
    pub fn read_byte(&self, address: u32) -> Result<u8> {
        let mapped: mapper::MappedAddress = self.mapper.map_address(address);
        
        match mapped.region {
            mapper::MemoryRegion::Bios => {
                if let Some(bios) = &self.bios {
                    let offset: usize = mapped.offset as usize;
                    if offset < bios.len() {
                        Ok(bios[offset])
                    } else {
                        Ok(0xFF)
                    }
                } else {
                    Ok(0xFF) // Sem BIOS, retorna 0xFF
                }
            }
            mapper::MemoryRegion::WorkRam => {
                self.work_ram.read_byte(mapped.offset)
            }
            mapper::MemoryRegion::VideoRam => {
                self.vram.read_byte(mapped.offset)
            }
            mapper::MemoryRegion::SoundRam => {
                self.sound_ram.read_byte(mapped.offset)
            }
            mapper::MemoryRegion::CartridgeRom => {
                if let Some(cart) = &self.cartridge {
                    cart.read_byte(mapped.offset)
                } else {
                    Ok(0xFF) // Sem cartucho
                }
            }
            mapper::MemoryRegion::CartridgeRam => {
                if let Some(cart) = &self.cartridge {
                    cart.read_sram(mapped.offset)
                } else {
                    Ok(0xFF)
                }
            }
            mapper::MemoryRegion::IoRegisters => {
                let offset: usize = mapped.offset as usize;
                if offset < self.io_registers.len() {
                    Ok(self.io_registers[offset])
                } else {
                    Ok(0)
                }
            }
            mapper::MemoryRegion::Unmapped => {
                Ok(0xFF)
            }
        }
    }
    
    /// Escreve um byte na memória
    pub fn write_byte(&mut self, address: u32, value: u8) -> Result<()> {
        let mapped: mapper::MappedAddress = self.mapper.map_address(address);
        
        match mapped.region {
            mapper::MemoryRegion::WorkRam => {
                self.work_ram.write_byte(mapped.offset, value)
            }
            mapper::MemoryRegion::VideoRam => {
                self.vram.write_byte(mapped.offset, value)
            }
            mapper::MemoryRegion::SoundRam => {
                self.sound_ram.write_byte(mapped.offset, value)
            }
            mapper::MemoryRegion::CartridgeRam => {
                if let Some(cart) = &mut self.cartridge {
                    cart.write_sram(mapped.offset, value)
                } else {
                    Ok(())
                }
            }
            mapper::MemoryRegion::IoRegisters => {
                let offset: usize = mapped.offset as usize;
                if offset < self.io_registers.len() {
                    // Tratamento especial para alguns registros
                    match offset {
                        0x00..=0x01 => {
                            // Registros somente leitura
                            debug!("Attempt to write read-only IO register {:02X}", offset);
                        }
                        0x11 => {
                            // TMSS register - precisa de tratamento especial
                            if value & 0x01 != 0 {
                                debug!("TMSS lock enabled");
                            }
                            self.io_registers[offset] = value;
                        }
                        _ => {
                            self.io_registers[offset] = value;
                            trace!("IO write: {:02X} = {:02X}", offset, value);
                        }
                    }
                }
                Ok(())
            }
            mapper::MemoryRegion::CartridgeRom => {
                // Tentativa de escrever em ROM - pode ser mapeamento de banco
                if let Some(cart) = &mut self.cartridge {
                    cart.handle_bank_switch(mapped.offset, value)?;
                }
                Ok(())
            }
            _ => {
                // Regiões não escritas (BIOS, unmapped)
                Ok(())
            }
        }
    }
    
    /// Lê uma palavra (16 bits)
    pub fn read_word(&self, address: u32) -> Result<u16> {
        // Alinhamento de palavra no 68000
        if address & 1 != 0 {
            warn!("Unaligned word read at {:08X}", address);
        }
        
        let high: u16 = self.read_byte(address)? as u16;
        let low: u16 = self.read_byte(address.wrapping_add(1))? as u16;
        Ok((high << 8) | low)
    }
    
    /// Escreve uma palavra (16 bits)
    pub fn write_word(&mut self, address: u32, value: u16) -> Result<()> {
        if address & 1 != 0 {
            warn!("Unaligned word write at {:08X}", address);
        }
        
        self.write_byte(address, (value >> 8) as u8)?;
        self.write_byte(address.wrapping_add(1), value as u8)?;
        Ok(())
    }
    
    /// Lê uma palavra longa (32 bits)
    pub fn read_long(&self, address: u32) -> Result<u32> {
        if address & 1 != 0 {
            warn!("Unaligned long read at {:08X}", address);
        }
        
        let high: u32 = self.read_word(address)? as u32;
        let low: u32 = self.read_word(address.wrapping_add(2))? as u32;
        Ok((high << 16) | low)
    }
    
    /// Escreve uma palavra longa (32 bits)
    pub fn write_long(&mut self, address: u32, value: u32) -> Result<()> {
        if address & 1 != 0 {
            warn!("Unaligned long write at {:08X}", address);
        }
        
        self.write_word(address, (value >> 16) as u16)?;
        self.write_word(address.wrapping_add(2), value as u16)?;
        Ok(())
    }
    
    /// Interface para o VDP acessar VRAM
    pub fn vram(&self) -> &VideoRam {
        &self.vram
    }
    
    /// Interface mutável para o VDP acessar VRAM
    pub fn vram_mut(&mut self) -> &mut VideoRam {
        &mut self.vram
    }
    
    /// Interface para o Z80 acessar Sound RAM
    pub fn sound_ram(&self) -> &SoundRam {
        &self.sound_ram
    }
    
    /// Interface mutável para o Z80 acessar Sound RAM
    pub fn sound_ram_mut(&mut self) -> &mut SoundRam {
        &mut self.sound_ram
    }
    
    /// Despeja um bloco de memória para debug
    pub fn dump_memory(&self, start: u32, length: usize) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::with_capacity(length);
        
        for i in 0..length {
            if let Ok(byte) = self.read_byte(start.wrapping_add(i as u32)) {
                result.push(byte);
            } else {
                result.push(0xFF);
            }
        }
        
        result
    }
}