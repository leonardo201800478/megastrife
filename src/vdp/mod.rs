// src/vdp/mod.rs

pub mod cram;
pub mod dma;
pub mod framebuffer; // <-- adicione esta linha
pub mod interrupts;
pub mod modes;
pub mod planes;
pub mod registers;
pub mod renderer;
pub mod sprite;
pub mod vram;
pub mod vsram;

use cram::Cram;
use dma::VdpDma;
use interrupts::{VdpInterrupt, VdpInterruptController};
use modes::VdpMode;
use registers::VdpRegisters;
use renderer::FrameBuffer;
use sprite::SpriteTable;
use vram::Vram;
use vsram::Vsram;
use crate::vdp::renderer::Renderer;
use framebuffer::FrameBuffer;
#[derive(Clone)]
pub struct Vdp {
    pub regs: VdpRegisters,
    pub cram: Cram,
    pub vram: Vram,
    pub vsram: Vsram,
    pub dma: VdpDma,
    pub mode: VdpMode,
    pub framebuffer: FrameBuffer,
    pub sprites: SpriteTable,
    pub interrupts: VdpInterruptController,
}

impl Vdp {
    pub fn new() -> Self {
        let regs = VdpRegisters::new();
        let cram = Cram::new();
        let vram = Vram::new();
        let vsram = Vsram::new();
        let dma = VdpDma::new();
        let mode = VdpMode::Mode40Cell;
        let (w, h) = mode.resolution();
        let framebuffer = FrameBuffer::new(w, h);
        let sprites = SpriteTable::new();
        let interrupts = VdpInterruptController::new();

        Self {
            regs,
            cram,
            vram,
            vsram,
            dma,
            mode,
            framebuffer,
            sprites,
            interrupts,
        }
    }

    pub fn tick(&mut self) {
        self.dma.tick(&self.regs);
        self.interrupts.tick(&self.regs);
    }

    pub fn bus_read(&mut self, addr: u32) -> u8 {
        // Acesso genérico ao barramento do VDP
        match addr & 0xFFFF {
            0xC00000..=0xC0001F => self.read_register((addr & 0x1F) as u8),
            _ => 0,
        }
    }

    pub fn bus_write(&mut self, addr: u32, value: u8) {
        match addr & 0xFFFF {
            0xC00000..=0xC0001F => self.write_register((addr & 0x1F) as u8, value),
            0xC00020..=0xC0003F => self.write_cram((addr & 0x1F) as u8, value),
            _ => {}
        }
    }

    pub fn read_register(&self, reg: u8) -> u8 {
        self.regs.read(reg as usize)
    }

    pub fn write_register(&mut self, reg: u8, value: u8) {
        self.regs.write(addr as u16, value);
    }

    pub fn write_cram(&mut self, addr: u8, value: u8) {
        self.cram.write(addr as usize, value);
    }

    pub fn render_frame(&mut self) {
        self.framebuffer = Renderer::render_full(
            &self.regs,
            &self.cram,
            &self.vram,
            &self.vsram,
            &self.sprites,
            self.mode,
        );
    }

    pub fn poll_interrupt(&mut self) -> Option<VdpInterrupt> {
        self.interrupts.pop_pending()
    }
}
