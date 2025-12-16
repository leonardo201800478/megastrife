//! Renderização dos planos A e B do VDP (Tilemaps).
//!
//! Cada plano é uma grade 64x32 ou 64x64 de tiles.
//! Cada entrada da tabela tem 2 bytes com o seguinte formato:
//!
//! Bits 15    - Prioridade (1 = acima dos planos de fundo)
//! Bits 14-13 - Paleta (0–3)
//! Bit  12    - Flip vertical
//! Bit  11    - Flip horizontal
//! Bits 10-0  - Índice do tile na VRAM

use crate::vdp::{cram::Cram, renderer::FrameBuffer, vram::Vram, registers::VdpRegisters};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlaneType {
    A,
    B,
}

#[derive(Clone)]
pub struct Plane {
    pub plane_type: PlaneType,
    pub base_addr: u32, // endereço base na VRAM
    pub width: usize,   // largura em tiles (64)
    pub height: usize,  // altura em tiles (32/64)
}

impl Plane {
    pub fn new(plane_type: PlaneType, regs: &VdpRegisters) -> Self {
        // Bits 0–2 do reg 2 = base do plano A
        // Bits 0–2 do reg 4 = base do plano B
        let base_reg = match plane_type {
            PlaneType::A => regs.read(2),
            PlaneType::B => regs.read(4),
        };
        let base_addr = ((base_reg as u32) & 0x38) << 10; // bits 0–5 * 0x400
        let width = 64;
        let height = if regs.read(16) & 0x08 != 0 { 64 } else { 32 };
        Self {
            plane_type,
            base_addr,
            width,
            height,
        }
    }

    /// Lê uma entrada da tabela de nome do plano (TileMap)
    pub fn read_entry(vram: &Vram, addr: u32) -> TileEntry {
        let word = vram.read16(addr);
        TileEntry::from_word(word)
    }

    /// Renderiza o plano completo para o framebuffer
    pub fn render(
        &self,
        frame: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        scroll_x: u16,
        scroll_y: u16,
        priority_layer: bool,
    ) {
        let plane_w_px = self.width * 8;
        let plane_h_px = self.height * 8;
        let frame_w = frame.width as usize;
        let frame_h = frame.height as usize;

        for y in 0..frame_h {
            for x in 0..frame_w {
                let tx = ((x + scroll_x as usize) / 8) % self.width;
                let ty = ((y + scroll_y as usize) / 8) % self.height;
                let tile_addr = self.base_addr + ((ty * self.width + tx) as u32) * 2;
                let entry = Self::read_entry(vram, tile_addr);

                if entry.priority != priority_layer {
                    continue;
                }

                let tile_x = ((x + scroll_x as usize) % 8) as u8;
                let tile_y = ((y + scroll_y as usize) % 8) as u8;

                let color_index =
                    entry.get_pixel_color(vram, tile_x, tile_y, entry.flip_x, entry.flip_y);
                if color_index == 0 {
                    continue;
                }

                let (r, g, b) =
                    cram.rgb((entry.palette as usize * 16) + (color_index as usize));
                let color =
                    (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                frame.set_pixel(x as u32, y as u32, color);
            }
        }
    }
}

/// Estrutura de uma entrada de tile (2 bytes)
#[derive(Clone, Copy, Debug)]
pub struct TileEntry {
    pub tile_index: u16,
    pub palette: u8,
    pub priority: bool,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl TileEntry {
    pub fn from_word(word: u16) -> Self {
        Self {
            tile_index: word & 0x7FF,
            palette: ((word >> 13) & 0x03) as u8,
            priority: (word & 0x8000) != 0,
            flip_y: (word & 0x1000) != 0,
            flip_x: (word & 0x0800) != 0,
        }
    }

    /// Retorna o índice de cor de um pixel dentro do tile.
    pub fn get_pixel_color(
        &self,
        vram: &Vram,
        x: u8,
        y: u8,
        flip_x: bool,
        flip_y: bool,
    ) -> u8 {
        let tile = vram.read_tile(self.tile_index as usize);
        let tx = if flip_x { 7 - x } else { x };
        let ty = if flip_y { 7 - y } else { y };
        let idx = (ty * 4 + (tx / 2)) as usize;
        let byte = tile[idx];
        if tx & 1 == 0 { byte >> 4 } else { byte & 0x0F }
    }
}

impl Plane {
    pub fn render_with_vsram(
        &self,
        frame: &mut FrameBuffer,
        vram: &Vram,
        cram: &Cram,
        vsram: &crate::vdp::vsram::Vsram,
        scroll_x: u16,
        scroll_y: u16,
        priority_layer: bool,
    ) {
        let plane_w_px = self.width * 8;
        let plane_h_px = self.height * 8;
        let frame_w = frame.width as usize;
        let frame_h = frame.height as usize;

        for y in 0..frame_h {
            for x in 0..frame_w {
                // Line scroll: cada bloco de 8 px pode ter scrollY independente
                let vs_offset = vsram.line_scroll_offset(x);
                let effective_y = (y as i32 + scroll_y as i32 + vs_offset as i32)
                    .rem_euclid(plane_h_px as i32) as usize;

                let tx = ((x + scroll_x as usize) / 8) % self.width;
                let ty = (effective_y / 8) % self.height;

                let tile_addr = self.base_addr + ((ty * self.width + tx) as u32) * 2;
                let entry = Self::read_entry(vram, tile_addr);
                if entry.priority != priority_layer {
                    continue;
                }

                let tile_x = ((x + scroll_x as usize) % 8) as u8;
                let tile_y = (effective_y % 8) as u8;

                let color_index =
                    entry.get_pixel_color(vram, tile_x, tile_y, entry.flip_x, entry.flip_y);
                if color_index == 0 {
                    continue;
                }

                let (r, g, b) =
                    cram.rgb((entry.palette as usize * 16) + (color_index as usize));
                let color =
                    (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                frame.set_pixel(x as u32, y as u32, color);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::vram::Vram;
    use crate::vdp::cram::Cram;

    #[test]
    fn test_tile_entry_decode() {
        let entry = TileEntry::from_word(0b1000_1100_1010_1111);
        assert_eq!(entry.tile_index, 0x2AF);
        assert_eq!(entry.priority, true);
        assert_eq!(entry.flip_x, true);
        assert_eq!(entry.flip_y, true);
        assert_eq!(entry.palette, 0b00);
    }

    #[test]
    fn test_plane_render_simple() {
        let mut vram = Vram::new();
        let cram = Cram::new();
        let regs = VdpRegisters::new();

        // Escrever um tile visível na posição (0,0)
        let tile = [0x11; 32];
        vram.write_tile(0, &tile);
        vram.write16(0x0000, 0x0000); // tile index 0, sem flips

        let plane = Plane::new(PlaneType::A, &regs);
        let mut frame = FrameBuffer::new(32, 32);
        plane.render(&mut frame, &vram, &cram, 0, 0, false);
        assert!(frame.pixels.iter().any(|&c| c != 0));
    }
}
