
use crate::vdp::{
    cram::Cram,
    modes::VdpMode,
    vram::Vram,
    planes::{Plane, PlaneType},
    sprite::SpriteTable,
    registers::VdpRegisters,
    vsram::Vsram,
};

pub struct FrameBuffer {
    pub width: usize,
    pub height: usize,
    pub pixels: Vec<u32>,
}

impl FrameBuffer {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![0; (width * height) as usize],
        }
    }

    pub fn clear(&mut self, color: u32) {
        self.pixels.fill(color);
    }

    pub fn set_pixel(&mut self, x: usize, y: usize, color: usize) {
        if x < self.width && y < self.height {
            let idx = (y * self.width + x) as usize;
            self.pixels[idx] = color as u32;
        }
    }
}

pub struct Renderer;

impl Renderer {
    pub fn render_full(
        regs: &VdpRegisters,
        cram: &Cram,
        vram: &Vram,
        vsram: &Vsram,
        sprites: &SpriteTable,
        mode: VdpMode,
    ) -> FrameBuffer {
        let (w, h) = mode.resolution();
        let mut frame = FrameBuffer::new(w as usize, h as usize);

        let (r, g, b) = cram.rgb(0);
        let bg_color = (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
        frame.clear(bg_color);

        let plane_a = Plane::new(PlaneType::A, regs);
        let plane_b = Plane::new(PlaneType::B, regs);

        let scroll_a_x = regs.read(16) as u16;
        let scroll_b_x = regs.read(18) as u16;

        // Scroll global Y (base vertical)
        let scroll_a_y_base = regs.read(17) as u16;
        let scroll_b_y_base = regs.read(19) as u16;

        // Render planos com prioridade baixa e alta
        for priority in [false, true] {
            plane_b.render_with_vsram(
                &mut frame,
                vram,
                cram,
                vsram,
                scroll_b_x,
                scroll_b_y_base,
                priority,
            );
            plane_a.render_with_vsram(
                &mut frame,
                vram,
                cram,
                vsram,
                scroll_a_x,
                scroll_a_y_base,
                priority,
            );

            if priority {
                sprites.render_all(&mut frame, vram, cram);
            }
        }

        frame
    }
}
