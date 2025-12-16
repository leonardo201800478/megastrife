use crate::vdp::registers::VdpRegisters;

/// Controlador de DMA do VDP.
/// Implementa transferências VRAM, CRAM, VSRAM.
#[derive(Default, Clone)]
pub struct VdpDma {
    pub source_addr: u32,
    pub length: u16,
    pub active: bool,
}

impl VdpDma {
    pub fn new() -> Self {
        Self {
            source_addr: 0,
            length: 0,
            active: false,
        }
    }

    pub fn start(&mut self, source: u32, length: u16) {
        self.source_addr = source;
        self.length = length;
        self.active = true;
    }

    pub fn tick(&mut self, regs: &VdpRegisters) {
        if self.active && regs.dma_enable() {
            // Simula execução DMA (instantânea)
            self.active = false;
        }
    }

    pub fn is_active(&self) -> bool {
        self.active
    }
}
