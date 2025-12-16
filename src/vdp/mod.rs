//! Subsistema completo do VDP (Video Display Processor) do Mega Drive / Sega Genesis
//!
//! Este módulo integra todos os componentes do sistema de vídeo:
//! - Registradores de controle
//! - Memória de vídeo (VRAM, CRAM, VSRAM)
//! - Sistema de DMA
//! - Controlador de interrupções
//! - Planos (A, B, Window)
//! - Sprites
//! - Modos de vídeo
//! - Renderizador
//! - Framebuffer

pub mod cram;
pub mod dma;
pub mod framebuffer;
pub mod interrupts;
pub mod video_modes;
pub mod planes;
pub mod registers;
pub mod renderer;
pub mod sprite;
pub mod vram;
pub mod vsram;

// Re-export de tipos importantes para uso externo
pub use cram::Cram;
pub use dma::{VdpDma, DmaMode};
pub use framebuffer::FrameBuffer;
pub use interrupts::{VdpInterruptController, VdpInterruptType, VdpStatus};
pub use video_modes::{VdpVideoMode, VdpResolution, VdpRenderMode};
pub use planes::{Plane, PlaneType, PlaneManager, TileEntry};
pub use registers::VdpRegisters;
pub use renderer::{VdpRenderer, RenderFlags, RenderStats};
pub use sprite::{Sprite, SpriteSize, SpriteTable};
pub use vram::Vram;
pub use vsram::{Vsram, ScrollMode};

use crate::memory::bus::Bus;
use std::sync::{Arc, Mutex};

/// Instância principal do VDP
#[derive(Clone)]
pub struct Vdp {
    // Componentes principais
    pub regs: VdpRegisters,
    pub cram: Cram,
    pub vram: Vram,
    pub vsram: Vsram,
    pub dma: VdpDma,
    pub interrupts: VdpInterruptController,
    pub renderer: VdpRenderer,
    
    // Estado interno
    pub video_mode: VdpVideoMode,
    pub plane_manager: Option<PlaneManager>,
    pub sprite_table: SpriteTable,
    pub is_pal: bool,               // false = NTSC, true = PAL
    pub frame_counter: u64,
    pub cycles_elapsed: u64,
    
    // Flags de controle
    pub display_enabled: bool,
    pub vblank_active: bool,
    pub hblank_active: bool,
    
    // Referências compartilhadas (para acesso de outros componentes)
    pub bus_ref: Option<Arc<Mutex<Bus>>>,
}

impl Vdp {
    /// Cria uma nova instância do VDP
    pub fn new(is_pal: bool) -> Self {
        let regs = VdpRegisters::new();
        let video_mode = VdpVideoMode::new_default();
        
        // Criar renderer com modo de vídeo padrão
        let renderer = VdpRenderer::new(video_mode.clone());
        
        // Configurar modo de vídeo (NTSC/PAL)
        let mut interrupts = VdpInterruptController::new();
        interrupts.set_video_mode(is_pal);
        
        // Criar sprite table com endereço padrão
        let sprite_table = SpriteTable::new(0xF800, false);
        
        Self {
            regs,
            cram: Cram::new(),
            vram: Vram::new(),
            vsram: Vsram::new(),
            dma: VdpDma::new(),
            interrupts,
            renderer,
            
            video_mode,
            plane_manager: None,
            sprite_table,
            is_pal,
            frame_counter: 0,
            cycles_elapsed: 0,
            
            display_enabled: true,
            vblank_active: false,
            hblank_active: false,
            
            bus_ref: None,
        }
    }
    
    /// Inicializa completamente o VDP (deve ser chamado após criação)
    pub fn initialize(&mut self) {
        // Atualizar modo de vídeo baseado nos registradores
        self.update_video_mode();
        
        // Criar gerenciador de planos
        self.plane_manager = Some(PlaneManager::new(&self.regs, &self.video_mode));
        
        // Configurar interrupções
        self.interrupts.update_from_registers(&self.regs);
        
        // Configurar DMA
        self.dma.setup_from_registers(&self.regs, 0);
    }
    
    /// Atualiza o modo de vídeo baseado nos registradores
    pub fn update_video_mode(&mut self) {
        self.video_mode = VdpVideoMode::from_registers(&self.regs, self.is_pal);
        
        // Atualizar renderer se necessário
        if self.renderer.video_mode.resolution() != self.video_mode.resolution() {
            let (width, height) = self.video_mode.resolution();
            self.renderer.resize(width as usize, height as usize);
        }
    }
    
    // =====================================================
    // CICLOS E ATUALIZAÇÃO
    // =====================================================
    
    /// Executa um ciclo do VDP (~1 pixel clock)
    pub fn tick(&mut self) {
        self.cycles_elapsed += 1;
        
        // Atualizar interrupções
        self.interrupts.tick(&self.regs);
        
        // Atualizar DMA se ativo
        if self.dma.is_active() {
            // Em um sistema real, o DMA acessaria o barramento
            // Para simplificar, apenas processamos uma transferência
            if let Some(bus) = &self.bus_ref {
                let mut bus = bus.lock().unwrap();
                self.dma.tick(&mut bus, &mut self.vram, &mut self.cram, &mut self.vsram);
            }
        }
        
        // Atualizar contadores de linha/quadro
        self.update_counters();
        
        // Verificar se é hora de renderizar uma linha
        if self.interrupts.current_hpos() == 0 {
            self.render_scanline();
        }
    }
    
    /// Atualiza contadores de linha e quadro
    fn update_counters(&mut self) {
        // Verificar VBlank
        let in_vblank = self.interrupts.in_vblank();
        if in_vblank && !self.vblank_active {
            self.vblank_active = true;
            self.on_vblank_start();
        } else if !in_vblank && self.vblank_active {
            self.vblank_active = false;
            self.on_vblank_end();
        }
        
        // Verificar HBlank
        let in_hblank = self.interrupts.in_hblank();
        if in_hblank && !self.hblank_active {
            self.hblank_active = true;
            self.on_hblank_start();
        } else if !in_hblank && self.hblank_active {
            self.hblank_active = false;
        }
    }
    
    /// Chamado no início do VBlank
    fn on_vblank_start(&mut self) {
        // Incrementar contador de frames
        self.frame_counter += 1;
        
        // Carregar sprites para o próximo quadro
        self.sprite_table.load_from_vram(&self.vram);
        
        // Calcular sprites ativos por linha (para detecção de overflow)
        self.sprite_table.calculate_active_sprites(self.video_mode.visible_height);
        
        // Detectar colisões de sprites
        self.sprite_table.detect_collisions(
            self.video_mode.visible_width as usize,
            self.video_mode.visible_height as usize,
        );
        
        // Sinalizar interrupção de colisão/overflow se necessário
        if self.sprite_table.collision_detected {
            self.interrupts.signal_sprite_collision();
        }
        
        if self.sprite_table.overflow_line.is_some() {
            self.interrupts.signal_sprite_overflow(0); // Linha será atualizada
        }
    }
    
    /// Chamado no final do VBlank
    fn on_vblank_end(&mut self) {
        // Resetar estado para novo quadro
    }
    
    /// Chamado no início do HBlank
    fn on_hblank_start(&mut self) {
        // Processamento específico do HBlank
    }
    
    /// Renderiza a linha atual do scanline
    fn render_scanline(&mut self) {
        // Atualizar planos com scroll atual
        if let Some(plane_manager) = &mut self.plane_manager {
            plane_manager.update(&self.regs, &self.video_mode);
        }
        
        // Renderizar linha atual se display estiver habilitado
        if self.display_enabled && self.regs.display_enabled() {
            let current_line = self.interrupts.current_scanline();
            
            // Em um renderizador completo, renderizaríamos a linha aqui
            // Para simplificar, apenas marcamos que o framebuffer está sujo
            self.renderer.frame_buffer.mark_clean(); // Remover após implementação completa
        }
    }
    
    // =====================================================
    // ACESSO AO BARRAMENTO (para a CPU)
    // =====================================================
    
    /// Processa leitura do barramento pelo 68K
    pub fn bus_read(&mut self, addr: u32) -> u8 {
        match addr & 0xFC {
            0x00 => {  // Porta de dados (0xC00000)
                self.regs.read_data_port() as u8
            }
            0x02 => {  // Porta de dados (0xC00002) - byte alto
                (self.regs.read_data_port() >> 8) as u8
            }
            0x04 => {  // Porta de controle (0xC00004) - leitura de status
                self.regs.read_control_port()
            }
            0x06 => {  // Porta de controle (0xC00006) - sempre retorna 0
                0
            }
            _ => {
                // Endereço inválido
                0
            }
        }
    }
    
    /// Processa escrita do barramento pelo 68K
    pub fn bus_write(&mut self, addr: u32, value: u8) {
        match addr & 0xFC {
            0x00 => {  // Porta de dados (0xC00000) - byte baixo
                let current = self.regs.data_buffer & 0xFF00;
                self.regs.write_data_port(current | (value as u16));
            }
            0x02 => {  // Porta de dados (0xC00002) - byte alto
                let current = self.regs.data_buffer & 0x00FF;
                self.regs.write_data_port((value as u16) << 8 | current);
            }
            0x04 => {  // Porta de controle (0xC00004) - byte baixo
                let current = self.regs.address_buffer & 0xFF00;
                self.regs.write_control_port(current | (value as u16));
            }
            0x06 => {  // Porta de controle (0xC00006) - byte alto
                let current = self.regs.address_buffer & 0x00FF;
                self.regs.write_control_port((value as u16) << 8 | current);
            }
            _ => {
                // Endereço inválido - ignorar
            }
        }
        
        // Processar escrita se necessário
        self.process_pending_write();
    }
    
    /// Processa escrita pendente após configuração de endereço
    fn process_pending_write(&mut self) {
        // Se temos um endereço configurado e não estamos em modo de leitura
        if !self.regs.current_address.read_mode && self.regs.has_fifo_commands() {
            if let Some(data) = self.regs.pop_fifo() {
                match self.regs.current_address.access_type {
                    VdpAccessType::VRAM => {
                        let addr = self.regs.current_address.addr;
                        self.vram.write16(addr as u32, data);
                    }
                    VdpAccessType::CRAM => {
                        let addr = self.regs.current_address.addr;
                        self.cram.write(addr as usize, data);
                    }
                    VdpAccessType::VSRAM => {
                        let addr = self.regs.current_address.addr;
                        self.vsram.write16(addr as u32, data);
                    }
                }
                
                // Incrementar endereço
                self.regs.increment_address();
            }
        }
    }
    
    // =====================================================
    // RENDERIZAÇÃO
    // =====================================================
    
    /// Renderiza um quadro completo
    pub fn render_frame(&mut self) -> &FrameBuffer {
        // Atualizar modo de vídeo
        self.update_video_mode();
        
        // Atualizar gerenciador de planos
        if let Some(plane_manager) = &mut self.plane_manager {
            plane_manager.update(&self.regs, &self.video_mode);
        }
        
        // Renderizar usando o renderer
        let timestamp = self.cycles_elapsed as f64 / 7_670_000.0; // Clock do VDP
        self.renderer.render_frame(
            &self.regs,
            &self.cram,
            &self.vram,
            &self.vsram,
            &self.sprite_table,
            timestamp,
        )
    }
    
    /// Retorna uma referência ao framebuffer atual
    pub fn get_framebuffer(&self) -> &FrameBuffer {
        &self.renderer.frame_buffer
    }
    
    /// Retorna uma cópia do framebuffer atual
    pub fn copy_framebuffer(&self) -> FrameBuffer {
        self.renderer.frame_buffer.clone()
    }
    
    // =====================================================
    // INTERRUPÇÕES
    // =====================================================
    
    /// Verifica se há uma interrupção pendente para a CPU
    pub fn has_interrupt(&self) -> bool {
        self.interrupts.has_interrupt()
    }
    
    /// Obtém a próxima interrupção pendente
    pub fn poll_interrupt(&mut self) -> Option<VdpInterruptType> {
        if let Some(interrupt) = self.interrupts.pop_interrupt() {
            Some(interrupt.irq_type)
        } else {
            None
        }
    }
    
    /// Lê o registrador de status (limpa flags de interrupção)
    pub fn read_status(&mut self) -> u8 {
        self.interrupts.read_status()
    }
    
    // =====================================================
    // CONFIGURAÇÃO E DIAGNÓSTICO
    // =====================================================
    
    /// Configura referência ao barramento (para DMA)
    pub fn set_bus_reference(&mut self, bus: Arc<Mutex<Bus>>) {
        self.bus_ref = Some(bus);
    }
    
    /// Configura modo de vídeo (NTSC/PAL)
    pub fn set_video_mode(&mut self, is_pal: bool) {
        self.is_pal = is_pal;
        self.interrupts.set_video_mode(is_pal);
        self.update_video_mode();
    }
    
    /// Reseta completamente o VDP
    pub fn reset(&mut self) {
        self.regs.reset();
        self.cram = Cram::new();
        self.vram.clear();
        self.vsram.clear();
        self.dma = VdpDma::new();
        self.interrupts.reset();
        self.sprite_table.clear();
        
        self.frame_counter = 0;
        self.cycles_elapsed = 0;
        self.vblank_active = false;
        self.hblank_active = false;
        
        // Resetar renderer
        self.renderer = VdpRenderer::new(VdpVideoMode::new_default());
        
        // Re-inicializar
        self.initialize();
    }
    
    /// Retorna informações de debug sobre o estado do VDP
    pub fn debug_info(&self) -> Vec<String> {
        let mut info = Vec::new();
        
        // Informações gerais
        info.push(format!("VDP State:"));
        info.push(format!("  Frame: {}, Cycles: {}", self.frame_counter, self.cycles_elapsed));
        info.push(format!("  Mode: {}", self.video_mode.name));
        info.push(format!("  Display: {}", if self.display_enabled { "ON" } else { "OFF" }));
        info.push(format!("  VBlank: {}, HBlank: {}", self.vblank_active, self.hblank_active));
        info.push(format!("  System: {}", if self.is_pal { "PAL" } else { "NTSC" }));
        
        // Interrupções
        info.push(format!("Interrupts:"));
        info.push(format!("  Pending: {}", self.interrupts.has_interrupt()));
        info.push(format!("  Status: {:08b}", self.interrupts.status.bits()));
        info.push(format!("  Line: {}, HPos: {}", 
            self.interrupts.current_scanline(),
            self.interrupts.current_hpos()
        ));
        
        // DMA
        info.push(format!("DMA:"));
        info.push(format!("  Active: {}", self.dma.is_active()));
        info.push(format!("  Mode: {:?}", self.dma.mode()));
        info.push(format!("  Words remaining: {}", self.dma.words_remaining()));
        
        // Memória
        info.push(format!("Memory:"));
        info.push(format!("  VRAM: {} bytes", self.vram.size()));
        info.push(format!("  CRAM: {} colors", self.cram.data.len()));
        info.push(format!("  VSRAM: {} words", self.vsram.size_words()));
        
        // Sprites
        info.push(format!("Sprites:"));
        info.push(format!("  Active: {}", self.sprite_table.sprite_count));
        info.push(format!("  Overflow: {:?}", self.sprite_table.overflow_line));
        info.push(format!("  Collision: {}", self.sprite_table.collision_detected));
        
        info
    }
    
    /// Retorna estatísticas de renderização
    pub fn get_render_stats(&self) -> RenderStats {
        self.renderer.get_stats()
    }
    
    /// Configura quais elementos renderizar
    pub fn set_render_flags(&mut self, flags: RenderFlags) {
        self.renderer.set_render_flags(flags);
    }
    
    /// Habilita/desabilita renderização
    pub fn set_render_enabled(&mut self, enabled: bool) {
        self.renderer.set_render_enabled(enabled);
    }
}

impl Default for Vdp {
    fn default() -> Self {
        Self::new(false) // NTSC por padrão
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_vdp_creation() {
        let vdp = Vdp::new(false);
        
        assert!(!vdp.is_pal);
        assert_eq!(vdp.frame_counter, 0);
        assert_eq!(vdp.cycles_elapsed, 0);
        assert!(vdp.display_enabled);
        assert!(!vdp.vblank_active);
        assert!(!vdp.hblank_active);
    }
    
    #[test]
    fn test_vdp_initialize() {
        let mut vdp = Vdp::new(false);
        vdp.initialize();
        
        assert!(vdp.plane_manager.is_some());
        assert_eq!(vdp.video_mode.name, "NTSC 320x224");
    }
    
    #[test]
    fn test_vdp_reset() {
        let mut vdp = Vdp::new(false);
        vdp.initialize();
        
        // Modificar estado
        vdp.frame_counter = 100;
        vdp.cycles_elapsed = 5000;
        vdp.vblank_active = true;
        
        // Resetar
        vdp.reset();
        
        // Verificar estado resetado
        assert_eq!(vdp.frame_counter, 0);
        assert_eq!(vdp.cycles_elapsed, 0);
        assert!(!vdp.vblank_active);
    }
    
    #[test]
    fn test_vdp_tick() {
        let mut vdp = Vdp::new(false);
        vdp.initialize();
        
        // Executar alguns ticks
        for _ in 0..1000 {
            vdp.tick();
        }
        
        // Verificar que ciclos foram incrementados
        assert!(vdp.cycles_elapsed > 0);
    }
    
    #[test]
    fn test_vdp_render_frame() {
        let mut vdp = Vdp::new(false);
        vdp.initialize();
        
        // Renderizar um frame
        let framebuffer = vdp.render_frame();
        
        // Verificar que um framebuffer foi retornado
        assert_eq!(framebuffer.width, 320);
        assert_eq!(framebuffer.height, 224);
    }
    
    #[test]
    fn test_vdp_debug_info() {
        let vdp = Vdp::new(false);
        let info = vdp.debug_info();
        
        // Verificar que informações foram geradas
        assert!(!info.is_empty());
        assert!(info[0].contains("VDP State"));
    }
}