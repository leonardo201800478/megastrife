//! Interface de barramento de memória

use anyhow::Result;

/// Interface para acesso à memória
pub trait MemoryBus {
    /// Lê um byte do endereço especificado
    fn read_byte(&self, address: u32) -> Result<u8>;
    
    /// Lê uma palavra (16 bits) do endereço especificado
    fn read_word(&self, address: u32) -> Result<u16>;
    
    /// Lê uma palavra longa (32 bits) do endereço especificado
    fn read_long(&self, address: u32) -> Result<u32>;
    
    /// Escreve um byte no endereço especificado
    fn write_byte(&mut self, address: u32, value: u8) -> Result<()>;
    
    /// Escreve uma palavra (16 bits) no endereço especificado
    fn write_word(&mut self, address: u32, value: u16) -> Result<()>;
    
    /// Escreve uma palavra longa (32 bits) no endereço especificado
    fn write_long(&mut self, address: u32, value: u32) -> Result<()>;
}

/// Implementação padrão para MemoryBus
impl MemoryBus for crate::memory::MemorySystem {
    fn read_byte(&self, address: u32) -> Result<u8> {
        self.read_byte(address)
    }
    
    fn read_word(&self, address: u32) -> Result<u16> {
        self.read_word(address)
    }
    
    fn read_long(&self, address: u32) -> Result<u32> {
        self.read_long(address)
    }
    
    fn write_byte(&mut self, address: u32, value: u8) -> Result<()> {
        self.write_byte(address, value)
    }
    
    fn write_word(&mut self, address: u32, value: u16) -> Result<()> {
        self.write_word(address, value)
    }
    
    fn write_long(&mut self, address: u32, value: u32) -> Result<()> {
        self.write_long(address, value)
    }
}