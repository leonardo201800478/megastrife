//! Sprite Engine do VDP
//! Baseado na estrutura do hardware real do Mega Drive (SATB).
//! Cada entrada de 8 bytes define um sprite:
//!  Y, link, X, tamanho, tile index, atributos

use crate::vdp::{cram::Cram, vram::Vram};

#[derive(Clone, Copy, Debug, Default)]
pub struct Sprite {
    pub y: u16,
    pub size: (u8, u8), // largura, altura em tiles
    pub x: u16,
    pub tile_index: u16,
    pub palette: u8,
    pub priority: bool,
    pub flip_x: bool,
    pub flip_y: bool,
}

impl Sprite {
    /// Cria sprite a partir de 8 bytes da VRAM (SAT)
    pub fn from_bytes(data: &[u8]) -> Self {
        let y = ((data[0] as u16) << 8) | (data[1] as u16);
        let size_word = ((data[2] as u16) << 8) | (data[3] as u16);
        let x = ((data[4] as u16) << 8) | (data[5] as u16);
        let attr = ((data[6] as u16) << 8) | (data[7] as u16);

        let width = (((size_word >> 2) & 0x03) + 1) as u8;
        let height = ((size_word & 0x03) + 1) as u8;

        let tile_index = attr & 0x7FF;
        let palette = ((attr >> 13) & 0x03) as u8;
        let priority = attr & 0x8000 != 0;
        let flip_x = attr & 0x0800 != 0;
        let flip_y = attr & 0x1000 != 0;

        Self {
            y,
            size: (width, height),
            x,
            tile_index,
            palette,
            priority,
            flip_x,
            flip_y,
        }
    }
}

/// Tabela de Sprites (SAT)
pub struct SpriteTable {
    pub sprites: Vec<Sprite>,
}

impl SpriteTable {
    pub fn new() -> Self {
        Self { sprites: vec![] }
    }

    /// Lê a Sprite Attribute Table da VRAM a partir de `base_addr`
    pub fn load_from_vram(vram: &Vram, base_addr: u32, count: usize) -> Self {
        let mut sprites = Vec::new();
        for i in 0..count {
            let addr = base_addr + (i as u32) * 8;
            let mut bytes = [0u8; 8];
            for j in 0..8 {
                bytes[j] = vram.read8(addr + j as u32);
            }
            sprites.push(Sprite::from_bytes(&bytes));
        }
        Self { sprites }
    }

    /// Renderiza todos os sprites na tela
    pub fn render_all(
        &self,
        frame: &mut crate::vdp::renderer::FrameBuffer,
        vram: &Vram,
        cram: &Cram,
    ) {
        for sprite in &self.sprites {
            Self::draw_sprite(sprite, frame, vram, cram);
        }
    }

    fn draw_sprite(
        sprite: &Sprite,
        frame: &mut crate::vdp::renderer::FrameBuffer,
        vram: &Vram,
        cram: &Cram,
    ) {
        let tile_w = 8;
        let tile_h = 8;

        for ty in 0..sprite.size.1 {
            for tx in 0..sprite.size.0 {
                let tile_index = sprite.tile_index as usize + (ty as usize) * (sprite.size.0 as usize) + tx as usize;
                let tile = vram.read_tile(tile_index);

                let x_base = sprite.x as u32 + (tx as u32) * tile_w as u32;
                let y_base = sprite.y as u32 + (ty as u32) * tile_h as u32;

                for row in 0..8 {
                    for col in 0..8 {
                        let src_x = if sprite.flip_x { 7 - col } else { col };
                        let src_y = if sprite.flip_y { 7 - row } else { row };

                        let idx = (src_y * 4 + (src_x / 2)) as usize;
                        let byte = tile[idx];
                        let pix = if src_x & 1 == 0 { byte >> 4 } else { byte & 0x0F };

                        if pix != 0 {
                            let (r, g, b) = cram.rgb((sprite.palette as usize * 16) + pix as usize);
                            let color =
                                (0xFF << 24) | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                            frame.set_pixel(x_base + col as u32, y_base + row as u32, color);
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vdp::{cram::Cram, vram::Vram, renderer::FrameBuffer};

    #[test]
    fn test_sprite_decode() {
        let data = [0x00, 0x10, 0x00, 0x01, 0x01, 0x20, 0x84, 0x00];
        let s = Sprite::from_bytes(&data);
        assert_eq!(s.y, 0x0010);
        assert_eq!(s.x, 0x0120);
        assert_eq!(s.tile_index, 0x400);
        assert_eq!(s.priority, true);
    }

    #[test]
    fn test_sprite_render() {
        let vram = Vram::new();
        let cram = Cram::new();
        let mut fb = FrameBuffer::new(64, 64);

        let sprite = Sprite {
            x: 8,
            y: 8,
            size: (1, 1),
            tile_index: 0,
            palette: 0,
            priority: true,
            flip_x: false,
            flip_y: false,
        };

        let sat = SpriteTable { sprites: vec![sprite] };
        sat.render_all(&mut fb, &vram, &cram);
        assert_eq!(fb.pixels.len(), (64 * 64) as usize);
    }
}
