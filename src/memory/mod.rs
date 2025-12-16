//! Subsistema de memória principal do Mega Drive / Sega Genesis.
//!
//! Este módulo unifica todos os componentes de memória e periféricos
//! conectados ao barramento do Motorola 68000:
//!
//! - ROM (via Mapper, com suporte a cartuchos especiais)
//! - RAM principal
//! - VDP (vídeo e CRAM)
//! - Som (FM e PSG)
//! - I/O (portas, controladores, interface com Z80)
//!
//! Também fornece funções convenientes para leitura, escrita e
//! atualização por ciclo (tick/frame).

pub mod bus;
pub mod mapper;
pub mod ram;
pub mod rom;

use bus::*;
use mapper::*;
use ram::*;
use rom::*;
use crate::vdp::Vdp;
use crate::sound::Sound;
use crate::io::Io;
use crate::cpu::z80::Z80;
use std::sync::{Arc, Mutex};

/// Estrutura de alto nível que representa o sistema de memória
/// unificado do Mega Drive.
pub struct Memory {
    pub bus: Bus,
}

impl Memory {
    /// Cria uma nova instância completa do subsistema de memória,
    /// com barramento, VDP, som e I/O inicializados.
    ///
    /// # Parâmetros
    /// - `rom_data`: conteúdo bruto da ROM (carregado do cartucho)
    /// - `ram_size`: tamanho da RAM principal (normalmente 64 KB)
    /// - `mapper_type`: define o tipo de mapeamento (Standard, SegaMapper, etc.)
    /// - `sound_rate`: taxa de amostragem do áudio (ex: 44100 Hz)
    pub fn new(rom_data: Vec<u8>, ram_size: usize, mapper_type: MapperType, sound_rate: u32) -> Self {
        let rom = Rom::new(rom_data);
        let mapper = Mapper::new(rom, mapper_type);
        let vdp = Arc::new(Mutex::new(Vdp::new()));
        let sound = Arc::new(Mutex::new(Sound::new(sound_rate)));
        let io = Arc::new(Mutex::new(Io::new()));
        let z80 = Arc::new(Mutex::new(Z80::new()));
        
        let bus = Bus::new(mapper, ram_size, vdp, sound, io, z80);
        Self { bus }
    }

    // =====================================================
    // LEITURA / ESCRITA
    // =====================================================

    /// Lê um byte (8 bits) da memória mapeada.
    pub fn read8(&self, addr: u32) -> u8 {
        self.bus.read8(addr)
    }

    /// Lê uma palavra (16 bits) da memória mapeada.
    pub fn read16(&self, addr: u32) -> u16 {
        self.bus.read16(addr)
    }

    /// Escreve um byte na memória mapeada.
    pub fn write8(&mut self, addr: u32, value: u8) {
        self.bus.write8(addr, value);
    }

    /// Escreve uma palavra (16 bits) na memória mapeada.
    pub fn write16(&mut self, addr: u32, value: u16) {
        self.bus.write16(addr, value);
    }

    // =====================================================
    // CICLOS / ATUALIZAÇÃO
    // =====================================================

    /// Atualiza o barramento e todos os periféricos a cada ciclo.
    /// Deve ser chamado uma vez por passo da CPU.
    pub fn tick(&mut self) {
        self.bus.tick();
    }

    /// Renderiza um frame completo do vídeo (VDP) e retorna o framebuffer RGBA.
    pub fn render_frame(&mut self) -> Vec<u32> {
        self.bus.render_frame()
    }

    // =====================================================
    // DIAGNÓSTICO
    // =====================================================

    /// Retorna o estado atual da VRAM (para debug).
    pub fn dump_vram(&self) -> Vec<u8> {
        self.bus.vram_dump()
    }

    /// Retorna o estado da CRAM (paleta de cores).
    pub fn dump_cram(&self) -> Vec<u16> {
        self.bus.cram_dump()
    }

    /// Retorna o estado da RAM principal.
    pub fn dump_ram(&self) -> Vec<u8> {
        self.bus.ram_dump()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::mapper::MapperType;

    #[test]
    fn test_memory_read_write() {
        let mut mem = Memory::new(vec![0xAA, 0xBB, 0xCC, 0xDD], 64 * 1024, MapperType::Standard, 44100);

        mem.write8(0xFF0000, 0x42);
        let val = mem.read8(0xFF0000);
        assert_eq!(val, 0x42);
    }

    #[test]
    fn test_memory_vdp_integration() {
        let mut mem = Memory::new(vec![0; 8], 64 * 1024, MapperType::Standard, 44100);
        mem.write16(0xC00000, 0x1234);
        let val = mem.read16(0xC00000);
        assert_eq!(val & 0xFF, 0x34);
    }

    #[test]
    fn test_memory_frame_render() {
        let mut mem = Memory::new(vec![0; 8], 64 * 1024, MapperType::Standard, 44100);
        let frame = mem.render_frame();
        assert!(!frame.is_empty());
    }
}