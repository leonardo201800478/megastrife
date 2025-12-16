//! Sistema de interrupções do VDP (Mega Drive / Sega Genesis)
//!
//! Implementa todas as interrupções geradas pelo VDP:
//! - **VBlank**: Fim do quadro (Vertical Blank)
//! - **HBlank**: Fim da linha (Horizontal Blank)
//! - **Scanline**: Interrupção em linha específica
//! - **Sprite**: Overflow e Collision
//!
//! O VDP gera interrupções para o M68000 através da linha IRQ2.
//! O estado das interrupções é lido/escrito via portas de dados do VDP.

use crate::vdp::registers::VdpRegisters;
use bitflags::bitflags;

bitflags! {
    /// Registrador de status do VDP (lido pela CPU)
    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub struct VdpStatus: u8 {
        /// Interrupção de quadro (VBlank) ocorreu
        const VBLANK         = 0b10000000;
        /// Interrupção de linha (HBlank) ocorreu
        const HBLANK         = 0b01000000;
        /// Sprite overflow ocorreu (mais de 20 sprites por linha)
        const SPRITE_OVERFLOW = 0b00100000;
        /// Colisão de sprites ocorreu
        const SPRITE_COLLISION = 0b00010000;
        /// Interrupção de linha programável ocorreu
        const SCANLINE_IRQ   = 0b00001000;
        /// DMA em progresso
        const DMA_IN_PROGRESS = 0b00000100;
        /// FIFO vazio (pronto para receber comandos)
        const FIFO_EMPTY     = 0b00000010;
        /// FIFO cheio (não pode receber mais comandos)
        const FIFO_FULL      = 0b00000001;
    }
}

/// Tipos de interrupção do VDP
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VdpInterruptType {
    VBlank,
    HBlank,
    Scanline,
    SpriteOverflow,
    SpriteCollision,
    DmaComplete,
}

/// Estrutura de uma interrupção pendente
#[derive(Clone, Debug)]
pub struct PendingInterrupt {
    pub irq_type: VdpInterruptType,
    pub timestamp: u64,        // Timestamp em ciclos do VDP
    pub priority: u8,          // Prioridade (0 = mais alta)
}

/// Controlador de interrupções do VDP
#[derive(Clone)]
pub struct VdpInterruptController {
    // Estado atual
    pub status: VdpStatus,
    pub enabled: VdpStatus,     // Quais interrupções estão habilitadas
    pub pending: Vec<PendingInterrupt>,
    
    // Contadores e timing
    pub h_counter: u16,         // Contador horizontal (0-341)
    pub v_counter: u16,         // Contador vertical (0-261 NTSC, 0-311 PAL)
    pub cycle_counter: u64,     // Contador total de ciclos
    
    // Configuração
    pub h_blank_start: u16,     // Início do HBlank (normalmente 320)
    pub v_blank_start: u16,     // Início do VBlank (normalmente 224)
    pub scanline_irq_line: u16, // Linha para interrupção programável (R#10)
    
    // Sprite tracking
    pub sprite_overflow_line: Option<u16>,
    pub sprite_collision_detected: bool,
    
    // Modo de vídeo
    pub is_pal: bool,           // false = NTSC, true = PAL
    pub is_interlaced: bool,    // Modo entrelaçado
    
    // Debug e estatísticas
    pub vblank_count: u32,
    pub hblank_count: u32,
    pub scanline_irq_count: u32,
}

impl VdpInterruptController {
    /// Cria um novo controlador de interrupções
    pub fn new() -> Self {
        Self {
            status: VdpStatus::empty(),
            enabled: VdpStatus::empty(),
            pending: Vec::new(),
            
            h_counter: 0,
            v_counter: 0,
            cycle_counter: 0,
            
            h_blank_start: 320,   // 320 pixels visíveis
            v_blank_start: 224,   // 224 linhas visíveis (NTSC)
            scanline_irq_line: 0,
            
            sprite_overflow_line: None,
            sprite_collision_detected: false,
            
            is_pal: false,        // NTSC por padrão
            is_interlaced: false,
            
            vblank_count: 0,
            hblank_count: 0,
            scanline_irq_count: 0,
        }
    }
    
    /// Configura o modo de vídeo (NTSC ou PAL)
    pub fn set_video_mode(&mut self, is_pal: bool) {
        self.is_pal = is_pal;
        if is_pal {
            self.v_blank_start = 240;  // 240 linhas visíveis PAL
        } else {
            self.v_blank_start = 224;  // 224 linhas visíveis NTSC
        }
    }
    
    /// Retorna o número total de linhas por quadro
    pub fn total_lines(&self) -> u16 {
        if self.is_pal {
            313  // PAL: 313 linhas
        } else {
            262  // NTSC: 262 linhas
        }
    }
    
    /// Retorna o número total de ciclos por linha
    pub fn cycles_per_line(&self) -> u16 {
        if self.is_pal {
            454  // PAL: ~454 ciclos por linha
        } else {
            342  // NTSC: ~342 ciclos por linha
        }
    }
    
    /// Atualiza a configuração a partir dos registradores do VDP
    pub fn update_from_registers(&mut self, regs: &VdpRegisters) {
        // Habilitar interrupções (R#0 bit 4, R#1 bit 5, R#0 bit 3)
        self.enabled = VdpStatus::empty();
        
        if (regs.get(0x00) & 0x10) != 0 {  // HBlank enable
            self.enabled.insert(VdpStatus::HBLANK);
        }
        
        if (regs.get(0x01) & 0x20) != 0 {  // VBlank enable
            self.enabled.insert(VdpStatus::VBLANK);
        }
        
        if (regs.get(0x00) & 0x08) != 0 {  // Scanline IRQ enable
            self.enabled.insert(VdpStatus::SCANLINE_IRQ);
        }
        
        // Linha de scanline IRQ (R#10)
        self.scanline_irq_line = regs.get(0x0A) as u16;
        
        // Modo de vídeo (R#12 bits 0-1)
        let mode = regs.get(0x0C) & 0x03;
        self.is_interlaced = (mode == 0x02) || (mode == 0x03);
        
        // Controle de HBlank/VBlank timing
        self.h_blank_start = match (regs.get(0x0C) >> 4) & 0x01 {
            0 => 320,  // 320 pixels visíveis
            1 => 256,  // 256 pixels visíveis
            _ => 320,
        };
    }
    
    /// Avança um ciclo de vídeo (~1 pixel clock)
    pub fn tick(&mut self, regs: &VdpRegisters) {
        self.cycle_counter += 1;
        self.h_counter += 1;
        
        let cycles_per_line = self.cycles_per_line();
        
        // Verificar fim da linha
        if self.h_counter >= cycles_per_line {
            self.h_counter = 0;
            self.end_of_line();
            
            self.v_counter += 1;
            let total_lines = self.total_lines();
            
            // Verificar fim do quadro
            if self.v_counter >= total_lines {
                self.v_counter = 0;
                self.end_of_frame();
            }
            
            // Verificar interrupção de linha programável
            self.check_scanline_irq();
            
            // Resetar estado por linha
            self.sprite_overflow_line = None;
        }
        
        // Verificar HBlank
        self.check_hblank();
        
        // Verificar VBlank
        self.check_vblank();
        
        // Atualizar estado DMA
        self.update_dma_status(regs);
        
        // Atualizar estado FIFO
        self.update_fifo_status(regs);
    }
    
    /// Processa o fim de uma linha de varredura
    fn end_of_line(&mut self) {
        // Resetar flags específicas por linha
        if self.status.contains(VdpStatus::HBLANK) {
            self.status.remove(VdpStatus::HBLANK);
        }
        
        self.hblank_count += 1;
    }
    
    /// Processa o fim de um quadro
    fn end_of_frame(&mut self) {
        self.vblank_count += 1;
        
        // Resetar flags de sprite para novo quadro
        self.sprite_collision_detected = false;
        if self.status.contains(VdpStatus::SPRITE_COLLISION) {
            self.status.remove(VdpStatus::SPRITE_COLLISION);
        }
        if self.status.contains(VdpStatus::SPRITE_OVERFLOW) {
            self.status.remove(VdpStatus::SPRITE_OVERFLOW);
        }
    }
    
    /// Verifica e gera interrupção HBlank
    fn check_hblank(&mut self) {
        // HBlank começa após os pixels visíveis
        if self.h_counter == self.h_blank_start {
            if self.enabled.contains(VdpStatus::HBLANK) {
                self.status.insert(VdpStatus::HBLANK);
                self.queue_interrupt(VdpInterruptType::HBlank, 1);
            }
        }
    }
    
    /// Verifica e gera interrupção VBlank
    fn check_vblank(&mut self) {
        // VBlank começa após as linhas visíveis
        if self.v_counter == self.v_blank_start && self.h_counter == 0 {
            if self.enabled.contains(VdpStatus::VBLANK) {
                self.status.insert(VdpStatus::VBLANK);
                self.queue_interrupt(VdpInterruptType::VBlank, 0); // Maior prioridade
            }
        }
    }
    
    /// Verifica interrupção de scanline programável
    fn check_scanline_irq(&mut self) {
        if self.enabled.contains(VdpStatus::SCANLINE_IRQ) {
            // Verificar linha atual
            let current_line = if self.is_interlaced {
                // Em modo entrelaçado, contamos linhas de campo
                (self.v_counter * 2) + if self.v_counter >= self.total_lines() / 2 { 1 } else { 0 }
            } else {
                self.v_counter
            };
            
            if current_line == self.scanline_irq_line {
                self.status.insert(VdpStatus::SCANLINE_IRQ);
                self.queue_interrupt(VdpInterruptType::Scanline, 2);
                self.scanline_irq_count += 1;
            }
        }
    }
    
    /// Atualiza o estado DMA no registrador de status
    fn update_dma_status(&mut self, regs: &VdpRegisters) {
        if regs.dma_enabled() && regs.dma_in_progress() {
            self.status.insert(VdpStatus::DMA_IN_PROGRESS);
        } else {
            self.status.remove(VdpStatus::DMA_IN_PROGRESS);
        }
    }
    
    /// Atualiza o estado FIFO no registrador de status
    fn update_fifo_status(&mut self, regs: &VdpRegisters) {
        // Simulação simplificada do FIFO
        if regs.fifo_empty() {
            self.status.insert(VdpStatus::FIFO_EMPTY);
            self.status.remove(VdpStatus::FIFO_FULL);
        } else if regs.fifo_full() {
            self.status.insert(VdpStatus::FIFO_FULL);
            self.status.remove(VdpStatus::FIFO_EMPTY);
        } else {
            self.status.remove(VdpStatus::FIFO_EMPTY | VdpStatus::FIFO_FULL);
        }
    }
    
    /// Adiciona uma interrupção à fila
    fn queue_interrupt(&mut self, irq_type: VdpInterruptType, priority: u8) {
        let interrupt = PendingInterrupt {
            irq_type,
            timestamp: self.cycle_counter,
            priority,
        };
        
        // Inserir mantendo ordenação por prioridade e timestamp
        let pos = self.pending.iter()
            .position(|i| i.priority > priority || 
                     (i.priority == priority && i.timestamp > self.cycle_counter))
            .unwrap_or(self.pending.len());
        
        self.pending.insert(pos, interrupt);
    }
    
    /// Registra overflow de sprites
    pub fn signal_sprite_overflow(&mut self, line: u16) {
        if self.sprite_overflow_line.is_none() {
            self.sprite_overflow_line = Some(line);
            self.status.insert(VdpStatus::SPRITE_OVERFLOW);
            self.queue_interrupt(VdpInterruptType::SpriteOverflow, 3);
        }
    }
    
    /// Registra colisão de sprites
    pub fn signal_sprite_collision(&mut self) {
        if !self.sprite_collision_detected {
            self.sprite_collision_detected = true;
            self.status.insert(VdpStatus::SPRITE_COLLISION);
            self.queue_interrupt(VdpInterruptType::SpriteCollision, 4);
        }
    }
    
    /// Sinaliza conclusão de DMA
    pub fn signal_dma_complete(&mut self) {
        self.queue_interrupt(VdpInterruptType::DmaComplete, 5);
    }
    
    // =====================================================
    // INTERFACE PARA A CPU
    // =====================================================
    
    /// Lê o registrador de status (limpa flags de interrupção)
    pub fn read_status(&mut self) -> u8 {
        let status = self.status.bits();
        
        // Clear interrupt flags on read (standard VDP behavior)
        self.status.remove(VdpStatus::VBLANK | VdpStatus::HBLANK | VdpStatus::SCANLINE_IRQ);
        
        status
    }
    
    /// Verifica se há uma interrupção pendente para a CPU
    pub fn has_interrupt(&self) -> bool {
        !self.pending.is_empty()
    }
    
    /// Obtém a próxima interrupção pendente (sem removê-la)
    pub fn peek_interrupt(&self) -> Option<&PendingInterrupt> {
        self.pending.first()
    }
    
    /// Remove e retorna a próxima interrupção pendente
    pub fn pop_interrupt(&mut self) -> Option<PendingInterrupt> {
        if !self.pending.is_empty() {
            Some(self.pending.remove(0))
        } else {
            None
        }
    }
    
    /// Remove todas as interrupções pendentes de um tipo específico
    pub fn clear_interrupts(&mut self, irq_type: VdpInterruptType) {
        self.pending.retain(|i| i.irq_type != irq_type);
    }
    
    /// Reseta completamente o controlador
    pub fn reset(&mut self) {
        self.status = VdpStatus::empty();
        self.enabled = VdpStatus::empty();
        self.pending.clear();
        
        self.h_counter = 0;
        self.v_counter = 0;
        self.cycle_counter = 0;
        
        self.vblank_count = 0;
        self.hblank_count = 0;
        self.scanline_irq_count = 0;
        
        self.sprite_overflow_line = None;
        self.sprite_collision_detected = false;
    }
    
    // =====================================================
    // DIAGNÓSTICO E DEBUG
    // =====================================================
    
    /// Retorna a linha atual de varredura
    pub fn current_scanline(&self) -> u16 {
        self.v_counter
    }
    
    /// Retorna a posição horizontal atual (em ciclos)
    pub fn current_hpos(&self) -> u16 {
        self.h_counter
    }
    
    /// Retorna true se estiver no período de blanking vertical
    pub fn in_vblank(&self) -> bool {
        self.v_counter >= self.v_blank_start
    }
    
    /// Retorna true se estiver no período de blanking horizontal
    pub fn in_hblank(&self) -> bool {
        self.h_counter >= self.h_blank_start
    }
    
    /// Retorna o tempo atual em ciclos
    pub fn current_cycle(&self) -> u64 {
        self.cycle_counter
    }
    
    /// Retorna estatísticas de contagem
    pub fn get_stats(&self) -> (u32, u32, u32) {
        (self.vblank_count, self.hblank_count, self.scanline_irq_count)
    }
    
    /// Retorna uma string descrevendo a interrupção
    pub fn interrupt_to_string(irq_type: VdpInterruptType) -> &'static str {
        match irq_type {
            VdpInterruptType::VBlank => "VBlank",
            VdpInterruptType::HBlank => "HBlank",
            VdpInterruptType::Scanline => "Scanline",
            VdpInterruptType::SpriteOverflow => "SpriteOverflow",
            VdpInterruptType::SpriteCollision => "SpriteCollision",
            VdpInterruptType::DmaComplete => "DmaComplete",
        }
    }
}

impl Default for VdpInterruptController {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    fn create_test_registers() -> VdpRegisters {
        let mut regs = VdpRegisters::new();
        regs.set(0x00, 0x14);  // HBlank + Scanline IRQ enabled
        regs.set(0x01, 0x20);  // VBlank enabled
        regs.set(0x0A, 100);   // Scanline IRQ na linha 100
        regs
    }
    
    #[test]
    fn test_interrupt_controller_creation() {
        let irq = VdpInterruptController::new();
        assert_eq!(irq.status, VdpStatus::empty());
        assert_eq!(irq.h_counter, 0);
        assert_eq!(irq.v_counter, 0);
        assert_eq!(irq.pending.len(), 0);
        assert!(!irq.is_pal);
    }
    
    #[test]
    fn test_video_mode_configuration() {
        let mut irq = VdpInterruptController::new();
        
        // NTSC por padrão
        assert_eq!(irq.total_lines(), 262);
        assert_eq!(irq.cycles_per_line(), 342);
        assert_eq!(irq.v_blank_start, 224);
        
        // Mudar para PAL
        irq.set_video_mode(true);
        assert_eq!(irq.total_lines(), 313);
        assert_eq!(irq.cycles_per_line(), 454);
        assert_eq!(irq.v_blank_start, 240);
    }
    
    #[test]
    fn test_registers_update() {
        let mut irq = VdpInterruptController::new();
        let regs = create_test_registers();
        
        irq.update_from_registers(&regs);
        
        assert!(irq.enabled.contains(VdpStatus::HBLANK));
        assert!(irq.enabled.contains(VdpStatus::VBLANK));
        assert!(irq.enabled.contains(VdpStatus::SCANLINE_IRQ));
        assert_eq!(irq.scanline_irq_line, 100);
    }
    
    #[test]
    fn test_hblank_interrupt() {
        let mut irq = VdpInterruptController::new();
        let regs = create_test_registers();
        
        irq.update_from_registers(&regs);
        
        // Avançar até o início do HBlank
        for _ in 0..irq.h_blank_start {
            irq.tick(&regs);
        }
        
        // Deveria ter gerado HBlank interrupt
        assert!(irq.status.contains(VdpStatus::HBLANK));
        assert!(irq.has_interrupt());
        
        if let Some(interrupt) = irq.peek_interrupt() {
            assert_eq!(interrupt.irq_type, VdpInterruptType::HBlank);
        } else {
            panic!("Expected HBlank interrupt");
        }
    }
    
    #[test]
    fn test_vblank_interrupt() {
        let mut irq = VdpInterruptController::new();
        let regs = create_test_registers();
        
        irq.update_from_registers(&regs);
        
        // Avançar até o início do VBlank
        let total_cycles = (irq.v_blank_start as u64) * (irq.cycles_per_line() as u64);
        for _ in 0..total_cycles {
            irq.tick(&regs);
        }
        
        // Deveria ter gerado VBlank interrupt
        assert!(irq.status.contains(VdpStatus::VBLANK));
        assert!(irq.has_interrupt());
        assert!(irq.in_vblank());
        
        // Ler status deve limpar o flag
        let status = irq.read_status();
        assert_eq!(status & VdpStatus::VBLANK.bits(), 0);
    }
    
    #[test]
    fn test_scanline_interrupt() {
        let mut irq = VdpInterruptController::new();
        let regs = create_test_registers();
        
        irq.update_from_registers(&regs);
        
        // Avançar até a linha 100
        for line in 0..100 {
            for _ in 0..irq.cycles_per_line() {
                irq.tick(&regs);
            }
            assert_eq!(irq.current_scanline(), line + 1);
        }
        
        // Deveria ter gerado scanline interrupt
        assert!(irq.status.contains(VdpStatus::SCANLINE_IRQ));
        assert!(irq.has_interrupt());
        assert_eq!(irq.scanline_irq_count, 1);
    }
    
    #[test]
    fn test_sprite_interrupts() {
        let mut irq = VdpInterruptController::new();
        let regs = VdpRegisters::new();
        
        // Teste sprite overflow
        irq.signal_sprite_overflow(50);
        assert!(irq.status.contains(VdpStatus::SPRITE_OVERFLOW));
        assert!(irq.has_interrupt());
        
        // Teste sprite collision
        irq.signal_sprite_collision();
        assert!(irq.status.contains(VdpStatus::SPRITE_COLLISION));
        assert_eq!(irq.pending.len(), 2); // Overflow + Collision
        
        // Limpar interrupções
        irq.clear_interrupts(VdpInterruptType::SpriteOverflow);
        assert_eq!(irq.pending.len(), 1);
    }
    
    #[test]
    fn test_interrupt_priority() {
        let mut irq = VdpInterruptController::new();
        
        // Adicionar interrupções em ordem diferente da prioridade
        irq.queue_interrupt(VdpInterruptType::DmaComplete, 5); // Baixa prioridade
        irq.queue_interrupt(VdpInterruptType::HBlank, 1);      // Alta prioridade
        irq.queue_interrupt(VdpInterruptType::VBlank, 0);      // Mais alta
        
        // Deveriam estar ordenadas por prioridade
        assert_eq!(irq.pending[0].irq_type, VdpInterruptType::VBlank);
        assert_eq!(irq.pending[1].irq_type, VdpInterruptType::HBlank);
        assert_eq!(irq.pending[2].irq_type, VdpInterruptType::DmaComplete);
        
        // Pop deve retornar a mais prioritária primeiro
        let first = irq.pop_interrupt().unwrap();
        assert_eq!(first.irq_type, VdpInterruptType::VBlank);
    }
    
    #[test]
    fn test_reset() {
        let mut irq = VdpInterruptController::new();
        let regs = create_test_registers();
        
        // Gerar algumas interrupções
        irq.update_from_registers(&regs);
        irq.signal_sprite_collision();
        
        // Avançar alguns ciclos
        for _ in 0..100 {
            irq.tick(&regs);
        }
        
        // Resetar
        irq.reset();
        
        assert_eq!(irq.status, VdpStatus::empty());
        assert_eq!(irq.enabled, VdpStatus::empty());
        assert_eq!(irq.pending.len(), 0);
        assert_eq!(irq.h_counter, 0);
        assert_eq!(irq.v_counter, 0);
        assert_eq!(irq.cycle_counter, 0);
        assert_eq!(irq.get_stats(), (0, 0, 0));
    }
    
    #[test]
    fn test_interlaced_mode() {
        let mut irq = VdpInterruptController::new();
        let mut regs = VdpRegisters::new();
        
        // Configurar modo entrelaçado
        regs.set(0x0C, 0x02); // Modo 2 = entrelaçado
        irq.update_from_registers(&regs);
        
        assert!(irq.is_interlaced);
    }
}