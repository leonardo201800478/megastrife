//! Implementação das memórias RAM do Genesis

use anyhow::{Result, Context};

/// RAM de trabalho (64KB) - Endereço: 0xFF0000-0xFFFFFF
pub struct WorkRam {
    memory: [u8; 65536], // 64KB
}

/// VRAM (Video RAM) - 64KB
pub struct VideoRam {
    memory: [u8; 65536], // 64KB
}

/// Sound RAM (8KB) - Para o Z80
pub struct SoundRam {
    memory: [u8; 8192], // 8KB
}

impl WorkRam {
    pub fn new() -> Self {
        Self {
            memory: [0; 65536],
        }
    }
    
    pub fn clear(&mut self) {
        self.memory = [0; 65536];
    }
    
    pub fn read_byte(&self, offset: u16) -> Result<u8> {
        Ok(self.memory[offset as usize])
    }
    
    pub fn write_byte(&mut self, offset: u16, value: u8) -> Result<()> {
        self.memory[offset as usize] = value;
        Ok(())
    }
    
    pub fn read_word(&self, offset: u16) -> Result<u16> {
        let high: u16 = self.read_byte(offset)? as u16;
        let low: u16 = self.read_byte(offset.wrapping_add(1))? as u16;
        Ok((high << 8) | low)
    }
    
    pub fn write_word(&mut self, offset: u16, value: u16) -> Result<()> {
        self.write_byte(offset, (value >> 8) as u8)?;
        self.write_byte(offset.wrapping_add(1), value as u8)?;
        Ok(())
    }
    
    pub fn get_slice(&self, offset: u16, length: u16) -> &[u8] {
        let start: usize = offset as usize;
        let end: usize = (start + length as usize).min(self.memory.len());
        &self.memory[start..end]
    }
    
    pub fn load_data(&mut self, offset: u16, data: &[u8]) -> Result<()> {
        let start: usize = offset as usize;
        let end: usize = start + data.len();
        
        if end > self.memory.len() {
            anyhow::bail!("Data exceeds work RAM bounds");
        }
        
        self.memory[start..end].copy_from_slice(data);
        Ok(())
    }
}

impl VideoRam {
    pub fn new() -> Self {
        Self {
            memory: [0; 65536],
        }
    }
    
    pub fn clear(&mut self) {
        self.memory = [0; 65536];
    }
    
    pub fn read_byte(&self, offset: u16) -> Result<u8> {
        // A VRAM é acessada como words, mas permitimos byte access para simplicidade
        Ok(self.memory[offset as usize])
    }
    
    pub fn write_byte(&mut self, offset: u16, value: u8) -> Result<()> {
        self.memory[offset as usize] = value;
        Ok(())
    }
    
    pub fn read_word(&self, offset: u16) -> Result<u16> {
        let high: u16 = self.read_byte(offset)? as u16;
        let low: u16 = self.read_byte(offset.wrapping_add(1))? as u16;
        Ok((high << 8) | low)
    }
    
    pub fn write_word(&mut self, offset: u16, value: u16) -> Result<()> {
        self.write_byte(offset, (value >> 8) as u8)?;
        self.write_byte(offset.wrapping_add(1), value as u8)?;
        Ok(())
    }
    
    /// Lê um tile (32 bytes) da VRAM
    pub fn read_tile(&self, tile_index: u16) -> Result<[u8; 32]> {
        let offset: u16 = tile_index * 32;
        let mut tile: [u8; 32] = [0; 32];
        
        for i in 0..32 {
            tile[i] = self.read_byte(offset.wrapping_add(i as u16))?;
        }
        
        Ok(tile)
    }
    
    /// Escreve um tile (32 bytes) na VRAM
    pub fn write_tile(&mut self, tile_index: u16, tile: &[u8; 32]) -> Result<()> {
        let offset: u16 = tile_index * 32;
        
        for i in 0..32 {
            self.write_byte(offset.wrapping_add(i as u16), tile[i])?;
        }
        
        Ok(())
    }
    
    /// Obtém um slice da VRAM (para renderização)
    pub fn get_slice(&self, offset: u16, length: u16) -> &[u8] {
        let start: usize = offset as usize;
        let end: usize = (start + length as usize).min(self.memory.len());
        &self.memory[start..end]
    }
}

impl SoundRam {
    pub fn new() -> Self {
        Self {
            memory: [0; 8192],
        }
    }
    
    pub fn clear(&mut self) {
        self.memory = [0; 8192];
    }
    
    pub fn read_byte(&self, offset: u16) -> Result<u8> {
        if offset as usize >= self.memory.len() {
            Ok(0)
        } else {
            Ok(self.memory[offset as usize])
        }
    }
    
    pub fn write_byte(&mut self, offset: u16, value: u8) -> Result<()> {
        if (offset as usize) < self.memory.len() {
            self.memory[offset as usize] = value;
        }
        Ok(())
    }
    
    pub fn read_word(&self, offset: u16) -> Result<u16> {
        let high: u16 = self.read_byte(offset)? as u16;
        let low: u16 = self.read_byte(offset.wrapping_add(1))? as u16;
        Ok((high << 8) | low)
    }
    
    pub fn write_word(&mut self, offset: u16, value: u16) -> Result<()> {
        self.write_byte(offset, (value >> 8) as u8)?;
        self.write_byte(offset.wrapping_add(1), value as u8)?;
        Ok(())
    }
    
    /// Carrega dados na Sound RAM (para testes)
    pub fn load_data(&mut self, offset: u16, data: &[u8]) -> Result<()> {
        let start: usize = offset as usize;
        let end: usize = start + data.len();
        
        if end > self.memory.len() {
            anyhow::bail!("Data exceeds sound RAM bounds");
        }
        
        self.memory[start..end].copy_from_slice(data);
        Ok(())
    }
}