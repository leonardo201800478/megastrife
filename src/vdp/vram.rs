//! VRAM - Video RAM do VDP (64 KB)
//! Armazena tiles, planos, sprites e tabelas de nome.

#[derive(Clone)]
pub struct Vram {
    data: Vec<u8>,
}

impl Vram {
    pub const SIZE: usize = 64 * 1024;

    pub fn new() -> Self {
        Self {
            data: vec![0; Self::SIZE],
        }
    }

    /// Leitura de byte
    pub fn read8(&self, addr: u32) -> u8 {
        self.data[(addr as usize) % Self::SIZE]
    }

    /// Leitura de word (16 bits)
    pub fn read16(&self, addr: u32) -> u16 {
        let hi = self.read8(addr) as u16;
        let lo = self.read8(addr + 1) as u16;
        (hi << 8) | lo
    }

    /// Escrita de byte
    pub fn write8(&mut self, addr: u32, value: u8) {
        let index = (addr as usize) % Self::SIZE;
        self.data[index] = value;
    }

    /// Escrita de word (16 bits)
    pub fn write16(&mut self, addr: u32, value: u16) {
        self.write8(addr, (value >> 8) as u8);
        self.write8(addr + 1, (value & 0xFF) as u8);
    }

    /// Leitura de tile (8x8 pixels, 4 bits por pixel)
    pub fn read_tile(&self, tile_index: usize) -> [u8; 32] {
        let base = tile_index * 32;
        let mut tile = [0u8; 32];
        tile.copy_from_slice(&self.data[base % Self::SIZE..(base + 32) % Self::SIZE]);
        tile
    }

    /// Escrita de tile
    pub fn write_tile(&mut self, tile_index: usize, tile: &[u8; 32]) {
        let base = tile_index * 32;
        self.data[base % Self::SIZE..(base + 32) % Self::SIZE].copy_from_slice(tile);
    }

    /// Limpa VRAM
    pub fn clear(&mut self) {
        self.data.fill(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_basic_rw() {
        let mut vram = Vram::new();
        vram.write16(0x100, 0xABCD);
        assert_eq!(vram.read16(0x100), 0xABCD);
    }

    #[test]
    fn test_vram_tile_rw() {
        let mut vram = Vram::new();
        let tile = [0xAA; 32];
        vram.write_tile(3, &tile);
        let t2 = vram.read_tile(3);
        assert_eq!(t2, tile);
    }
}
