//! Implementação do sistema de interrupções do VDP (Mega Drive).
//!
//! O VDP gera dois tipos principais de interrupção para o M68000:
//! - **HBlank (Horizontal Blank):** ocorre a cada linha de varredura
//! - **VBlank (Vertical Blank):** ocorre ao fim do quadro
//!
//! As interrupções são controladas pelos bits dos registradores 0 e 1:
//! - Reg #0, bit 4 → HBlank interrupt enable
//! - Reg #1, bit 5 → VBlank interrupt enable
//!
//! HBlank é gerado a cada 488 ciclos de clock de vídeo (~15.7 kHz NTSC),
//! e VBlank a cada 262 linhas (NTSC) ou 313 (PAL).

use crate::vdp::registers::VdpRegisters;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VdpInterrupt {
    HBlank,
    VBlank,
}

#[derive(Clone, Debug)]
pub struct VdpInterruptController {
    pub hblank_enabled: bool,
    pub vblank_enabled: bool,
    pub h_counter: u16,
    pub v_counter: u16,
    pub h_trigger_line: u16,
    pub v_total_lines: u16,
    pub pending_interrupts: Vec<VdpInterrupt>,
}

impl VdpInterruptController {
    pub fn new() -> Self {
        Self {
            hblank_enabled: false,
            vblank_enabled: false,
            h_counter: 0,
            v_counter: 0,
            h_trigger_line: 0,
            v_total_lines: 262, // NTSC padrão
            pending_interrupts: Vec::new(),
        }
    }

    /// Atualiza o estado do controlador com base nos registradores do VDP.
    pub fn update_from_registers(&mut self, regs: &VdpRegisters) {
        let r0 = regs.read(0);
        let r1 = regs.read(1);

        self.hblank_enabled = (r0 & 0x10) != 0;
        self.vblank_enabled = (r1 & 0x20) != 0;

        // Linha de disparo configurável (Reg 10)
        self.h_trigger_line = regs.read(10) as u16;
    }

    /// Avança um "tick" de vídeo (~1 pixel clock)
    pub fn tick(&mut self, regs: &VdpRegisters) {
        self.update_from_registers(regs);

        self.h_counter += 1;

        // Fim de linha → HBlank
        if self.h_counter >= 488 {
            self.h_counter = 0;
            self.v_counter += 1;

            if self.hblank_enabled {
                self.pending_interrupts.push(VdpInterrupt::HBlank);
            }

            // Fim do quadro → VBlank
            if self.v_counter >= self.v_total_lines {
                self.v_counter = 0;
                if self.vblank_enabled {
                    self.pending_interrupts.push(VdpInterrupt::VBlank);
                }
            }
        }
    }

    /// Retorna se há interrupção pendente
    pub fn has_pending(&self) -> bool {
        !self.pending_interrupts.is_empty()
    }

    /// Retorna e remove a próxima interrupção pendente
    pub fn pop_pending(&mut self) -> Option<VdpInterrupt> {
        if !self.pending_interrupts.is_empty() {
            Some(self.pending_interrupts.remove(0))
        } else {
            None
        }
    }

    /// Reseta contadores e fila
    pub fn reset(&mut self) {
        self.h_counter = 0;
        self.v_counter = 0;
        self.pending_interrupts.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::registers::VdpRegisters;

    #[test]
    fn test_vblank_trigger() {
        let regs = VdpRegisters::new();
        let mut irq = VdpInterruptController::new();
        irq.vblank_enabled = true;
        irq.v_total_lines = 4;
        for _ in 0..(4 * 488) {
            irq.tick(&regs);
        }
        assert!(irq.has_pending());
    }

    #[test]
    fn test_hblank_trigger() {
        let regs = VdpRegisters::new();
        let mut irq = VdpInterruptController::new();
        irq.hblank_enabled = true;
        for _ in 0..500 {
            irq.tick(&regs);
        }
        assert!(irq.has_pending());
    }
}
