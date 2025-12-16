//! VRAM - Video RAM do VDP (64 KB)
//! Armazena tiles, planos, sprites e tabelas de nome.
//!
//! Organização da VRAM:
//! - 0x0000-0xFFFF: 64KB de memória de vídeo
//! - Tiles são armazenados como padrões de 8x8 pixels
//! - 4 bits por pixel (16 cores da CRAM) = 32 bytes por tile
//! - 8 bits por pixel (256 cores) = 64 bytes por tile
//! - Tabelas de nome mapeiam tiles para a tela

#[derive(Clone)]
pub struct Vram {
    pub data: Vec<u8>,
    pub size: usize,
}

impl Vram {
    pub const SIZE_64K: usize = 64 * 1024;
    pub const SIZE_128K: usize = 128 * 1024;
    pub const TILE_SIZE_4BPP: usize = 32;  // 8x8 pixels, 4 bits por pixel
    pub const TILE_SIZE_8BPP: usize = 64;  // 8x8 pixels, 8 bits por pixel
    pub const NAME_TABLE_ENTRY_SIZE: usize = 2;  // Cada entrada é 16 bits
    pub const SPRITE_ATTRIBUTE_SIZE: usize = 8;  // 4 palavras de 16 bits por sprite

    /// Cria uma nova VRAM com tamanho padrão de 64KB
    pub fn new() -> Self {
        Self::with_size(Self::SIZE_64K)
    }

    /// Cria uma nova VRAM com tamanho especificado
    pub fn with_size(size: usize) -> Self {
        Self {
            data: vec![0; size],
            size,
        }
    }

    // =====================================================
    // ACESSO BÁSICO À MEMÓRIA
    // =====================================================

    /// Leitura de byte (8 bits)
    pub fn read8(&self, addr: u32) -> u8 {
        let addr = addr as usize;
        if addr < self.size {
            self.data[addr]
        } else {
            0
        }
    }

    /// Leitura de word (16 bits) - little-endian (VDP é little-endian)
    /// CORREÇÃO: O VDP do Mega Drive usa little-endian para VRAM
    pub fn read16(&self, addr: u32) -> u16 {
        let lo = self.read8(addr) as u16;
        let hi = self.read8(addr + 1) as u16;
        (hi << 8) | lo
    }

    /// Leitura de word (16 bits) - big-endian (para compatibilidade)
    pub fn read16_be(&self, addr: u32) -> u16 {
        let hi = self.read8(addr) as u16;
        let lo = self.read8(addr + 1) as u16;
        (hi << 8) | lo
    }

    /// Leitura de dword (32 bits)
    pub fn read32(&self, addr: u32) -> u32 {
        let w1 = self.read16(addr) as u32;
        let w2 = self.read16(addr + 2) as u32;
        (w2 << 16) | w1
    }

    /// Escrita de byte (8 bits)
    pub fn write8(&mut self, addr: u32, value: u8) {
        let addr = addr as usize;
        if addr < self.size {
            self.data[addr] = value;
        }
    }

    /// Escrita de word (16 bits) - little-endian
    /// CORREÇÃO: O VDP do Mega Drive usa little-endian para VRAM
    pub fn write16(&mut self, addr: u32, value: u16) {
        self.write8(addr, (value & 0xFF) as u8);
        self.write8(addr + 1, (value >> 8) as u8);
    }

    /// Escrita de word (16 bits) - big-endian
    pub fn write16_be(&mut self, addr: u32, value: u16) {
        self.write8(addr, (value >> 8) as u8);
        self.write8(addr + 1, (value & 0xFF) as u8);
    }

    /// Escrita de dword (32 bits)
    pub fn write32(&mut self, addr: u32, value: u32) {
        self.write16(addr, (value & 0xFFFF) as u16);
        self.write16(addr + 2, (value >> 16) as u16);
    }

    // =====================================================
    // ACESSO A TILES
    // =====================================================

    /// Leitura de tile 4bpp (32 bytes)
    /// CORREÇÃO: Adicionada verificação de limites segura
    pub fn read_tile_4bpp(&self, tile_index: usize) -> [u8; 32] {
        let mut tile = [0u8; 32];
        let base = tile_index * Self::TILE_SIZE_4BPP;
        
        if base + 32 <= self.size {
            tile.copy_from_slice(&self.data[base..base + 32]);
        } else if base < self.size {
            let remaining = self.size - base;
            tile[..remaining].copy_from_slice(&self.data[base..]);
        }
        tile
    }

    /// Leitura de tile 8bpp (64 bytes)
    /// CORREÇÃO: Adicionada verificação de limites segura
    pub fn read_tile_8bpp(&self, tile_index: usize) -> [u8; 64] {
        let mut tile = [0u8; 64];
        let base = tile_index * Self::TILE_SIZE_8BPP;
        
        if base + 64 <= self.size {
            tile.copy_from_slice(&self.data[base..base + 64]);
        } else if base < self.size {
            let remaining = self.size - base;
            tile[..remaining].copy_from_slice(&self.data[base..]);
        }
        tile
    }

    /// Escrita de tile 4bpp
    /// CORREÇÃO: Adicionada verificação de limites segura
    pub fn write_tile_4bpp(&mut self, tile_index: usize, tile: &[u8; 32]) {
        let base = tile_index * Self::TILE_SIZE_4BPP;
        
        if base + 32 <= self.size {
            self.data[base..base + 32].copy_from_slice(tile);
        } else if base < self.size {
            let remaining = self.size - base;
            self.data[base..].copy_from_slice(&tile[..remaining]);
        }
    }

    /// Escrita de tile 8bpp
    /// CORREÇÃO: Adicionada verificação de limites segura
    pub fn write_tile_8bpp(&mut self, tile_index: usize, tile: &[u8; 64]) {
        let base = tile_index * Self::TILE_SIZE_8BPP;
        
        if base + 64 <= self.size {
            self.data[base..base + 64].copy_from_slice(tile);
        } else if base < self.size {
            let remaining = self.size - base;
            self.data[base..].copy_from_slice(&tile[..remaining]);
        }
    }

    /// Leitura de pixel de tile 4bpp (4 bits por pixel)
    /// CORREÇÃO: Cálculo correto do índice do byte
    pub fn read_tile_pixel_4bpp(&self, tile_index: usize, x: usize, y: usize) -> u8 {
        let base = tile_index * Self::TILE_SIZE_4BPP;
        let row_offset = y * 4; // 4 bytes por linha (8 pixels, 4 bits cada)
        let byte_index = base + row_offset + (x / 2);
        
        if byte_index < self.size {
            let byte = self.data[byte_index];
            if x % 2 == 0 {
                byte & 0x0F  // Pixel baixo (bits 0-3)
            } else {
                (byte >> 4) & 0x0F  // Pixel alto (bits 4-7)
            }
        } else {
            0
        }
    }

    /// Leitura de pixel de tile 8bpp (8 bits por pixel)
    /// CORREÇÃO: Cálculo correto do índice do pixel
    pub fn read_tile_pixel_8bpp(&self, tile_index: usize, x: usize, y: usize) -> u8 {
        let base = tile_index * Self::TILE_SIZE_8BPP;
        let pixel_index = base + (y * 8) + x;
        
        if pixel_index < self.size {
            self.data[pixel_index]
        } else {
            0
        }
    }

    // =====================================================
    // TABELAS DE NOME (NAME TABLES)
    // =====================================================

    /// Leitura de entrada na tabela de nome
    /// CORREÇÃO: Verificação de limites
    pub fn read_name_table_entry(&self, base_addr: usize, row: usize, col: usize, width: usize) -> u16 {
        let entry_addr = base_addr + (row * width + col) * Self::NAME_TABLE_ENTRY_SIZE;
        if entry_addr + 1 < self.size {
            self.read16(entry_addr as u32)
        } else {
            0
        }
    }

    /// Escrita de entrada na tabela de nome
    /// CORREÇÃO: Verificação de limites
    pub fn write_name_table_entry(&mut self, base_addr: usize, row: usize, col: usize, width: usize, value: u16) {
        let entry_addr = base_addr + (row * width + col) * Self::NAME_TABLE_ENTRY_SIZE;
        if entry_addr + 1 < self.size {
            self.write16(entry_addr as u32, value);
        }
    }

    /// Extrai informações de uma entrada da tabela de nome
    /// CORREÇÃO: Ordem dos parâmetros na tupla corrigida (hflip antes de vflip)
    pub fn decode_name_table_entry(entry: u16) -> (usize, bool, bool, u8, bool) {
        let tile_index = (entry & 0x07FF) as usize;      // Bits 0-10: índice do tile
        let priority = (entry & 0x8000) != 0;            // Bit 15: prioridade
        let palette = ((entry >> 13) & 0x03) as u8;      // Bits 13-14: paleta (0-3)
        let vflip = (entry & 0x0800) != 0;               // Bit 11: flip vertical
        let hflip = (entry & 0x0400) != 0;               // Bit 10: flip horizontal
        (tile_index, priority, hflip, palette, vflip)
    }

    // =====================================================
    // SPRITES
    // =====================================================

    /// Leitura de atributos de sprite (4 words = 8 bytes)
    /// CORREÇÃO: Verificação de limites
    pub fn read_sprite_attributes(&self, base_addr: usize, sprite_index: usize) -> [u16; 4] {
        let sprite_addr = base_addr + sprite_index * Self::SPRITE_ATTRIBUTE_SIZE;
        [
            if sprite_addr < self.size { self.read16(sprite_addr as u32) } else { 0 },
            if sprite_addr + 2 < self.size { self.read16((sprite_addr + 2) as u32) } else { 0 },
            if sprite_addr + 4 < self.size { self.read16((sprite_addr + 4) as u32) } else { 0 },
            if sprite_addr + 6 < self.size { self.read16((sprite_addr + 6) as u32) } else { 0 },
        ]
    }

    /// Escrita de atributos de sprite
    /// CORREÇÃO: Verificação de limites
    pub fn write_sprite_attributes(&mut self, base_addr: usize, sprite_index: usize, attributes: &[u16; 4]) {
        let sprite_addr = base_addr + sprite_index * Self::SPRITE_ATTRIBUTE_SIZE;
        for i in 0..4 {
            let addr = sprite_addr + i * 2;
            if addr + 1 < self.size {
                self.write16(addr as u32, attributes[i]);
            }
        }
    }

    /// Decodifica atributos de sprite
    /// CORREÇÃO: Retorna valores separados para hflip e vflip, não multiplicados
    pub fn decode_sprite_attributes(attrs: &[u16; 4]) -> (i16, i16, usize, u16, usize, u8, bool, bool) {
        let y_pos = attrs[0] as i16;                    // Posição Y (complemento de 2)
        let _size = (attrs[1] >> 8) as u8 & 0x03;       // Bits 8-9: tamanho do sprite
        let link = (attrs[1] & 0x7F) as u8;             // Bits 0-6: link para próximo sprite
        let tile_index = (attrs[2] & 0x07FF) as usize;  // Bits 0-10: índice do tile
        let priority = (attrs[2] >> 15) & 0x01;         // Bit 15: prioridade
        let palette = ((attrs[2] >> 13) & 0x03) as u8;  // Bits 13-14: paleta (0-3)
        let hflip = ((attrs[2] >> 11) & 0x01) != 0;     // Bit 11: flip horizontal
        let vflip = ((attrs[2] >> 12) & 0x01) != 0;     // Bit 12: flip vertical
        let x_pos = attrs[3] as i16;                    // Posição X
        
        (y_pos, x_pos, tile_index, priority, link as usize, palette, hflip, vflip)
    }

    // =====================================================
    // TABELAS DE SCROLL
    // =====================================================

    /// Leitura de tabela de scroll horizontal
    /// CORREÇÃO: Verificação de limites
    pub fn read_hscroll(&self, base_addr: usize, line: usize) -> u16 {
        let addr = base_addr + (line * 2);  // Cada entrada é 16 bits
        if addr + 1 < self.size {
            self.read16(addr as u32)
        } else {
            0
        }
    }

    /// Leitura de tabela de scroll vertical
    /// CORREÇÃO: Verificação de limites
    pub fn read_vscroll(&self, base_addr: usize, column: usize) -> u16 {
        let addr = base_addr + (column * 2);  // Cada entrada é 16 bits
        if addr + 1 < self.size {
            self.read16(addr as u32)
        } else {
            0
        }
    }

    // =====================================================
    // OPERAÇÕES EM BLOCO
    // =====================================================

    /// Limpa toda a VRAM
    pub fn clear(&mut self) {
        self.data.fill(0);
    }

    /// Preenche a VRAM com um valor específico
    pub fn fill(&mut self, value: u8) {
        self.data.fill(value);
    }

    /// Copia dados para a VRAM
    /// CORREÇÃO: Tratamento de overflow
    pub fn copy_from(&mut self, addr: u32, data: &[u8]) {
        let start = addr as usize;
        if start >= self.size {
            return;
        }
        
        let end = start + data.len();
        let copy_len = if end > self.size { self.size - start } else { data.len() };
        
        self.data[start..start + copy_len].copy_from_slice(&data[..copy_len]);
    }

    /// Copia dados da VRAM
    /// CORREÇÃO: Tratamento de overflow
    pub fn copy_to(&self, addr: u32, len: usize) -> Vec<u8> {
        let start = addr as usize;
        if start >= self.size {
            return vec![0; len];
        }
        
        let end = start + len;
        let copy_len = if end > self.size { self.size - start } else { len };
        
        let mut result = vec![0; len];
        result[..copy_len].copy_from_slice(&self.data[start..start + copy_len]);
        result
    }

    /// Verifica se um endereço está dentro dos limites
    pub fn is_valid_address(&self, addr: u32) -> bool {
        (addr as usize) < self.size
    }

    /// Retorna o tamanho da VRAM
    pub fn size(&self) -> usize {
        self.size
    }

    /// Retorna uma cópia dos dados da VRAM
    pub fn dump(&self) -> Vec<u8> {
        self.data.clone()
    }

    /// Retorna uma fatia da VRAM
    pub fn slice(&self, start: usize, end: usize) -> &[u8] {
        let start = start.min(self.size);
        let end = end.min(self.size);
        &self.data[start..end]
    }

    /// Retorna uma fatia mutável da VRAM
    pub fn slice_mut(&mut self, start: usize, end: usize) -> &mut [u8] {
        let start = start.min(self.size);
        let end = end.min(self.size);
        &mut self.data[start..end]
    }
}

impl Default for Vram {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vram_basic_rw() {
        let mut vram = Vram::new();
        
        // Teste escrita/leitura byte
        vram.write8(0x100, 0xAB);
        assert_eq!(vram.read8(0x100), 0xAB);
        
        // Teste escrita/leitura word little-endian
        vram.write16(0x200, 0xABCD);
        assert_eq!(vram.read16(0x200), 0xABCD);
        
        // Teste escrita/leitura word big-endian
        vram.write16_be(0x300, 0x1234);
        assert_eq!(vram.read16_be(0x300), 0x1234);
    }

    #[test]
    fn test_vram_tile_4bpp() {
        let mut vram = Vram::new();
        
        // Cria um tile de teste
        let tile = [0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44, 
                    0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44,
                    0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44,
                    0x11, 0x22, 0x33, 0x44, 0x11, 0x22, 0x33, 0x44];
        
        // Escreve e lê o tile
        vram.write_tile_4bpp(10, &tile);
        let read_tile = vram.read_tile_4bpp(10);
        assert_eq!(read_tile, tile);
        
        // Teste leitura de pixel
        let pixel = vram.read_tile_pixel_4bpp(10, 0, 0);
        assert_eq!(pixel, 0x01);  // Byte 0x11, pixel baixo = 0x01
        let pixel = vram.read_tile_pixel_4bpp(10, 1, 0);
        assert_eq!(pixel, 0x01);  // Byte 0x11, pixel alto = 0x01
    }

    #[test]
    fn test_vram_name_table() {
        let mut vram = Vram::new();
        
        // Escreve uma entrada na tabela de nome
        let base_addr = 0x4000;
        let entry_value = 0x8123;  // Prioridade=1, tile_index=0x123
        
        vram.write_name_table_entry(base_addr, 2, 3, 40, entry_value);
        
        // Lê a entrada
        let read_value = vram.read_name_table_entry(base_addr, 2, 3, 40);
        assert_eq!(read_value, entry_value);
        
        // Decodifica a entrada
        let (tile_index, priority, hflip, palette, vflip) = Vram::decode_name_table_entry(entry_value);
        assert_eq!(tile_index, 0x123);
        assert_eq!(priority, true);
        assert_eq!(palette, 0);
        assert_eq!(hflip, false);
        assert_eq!(vflip, false);
    }

    #[test]
    fn test_vram_sprite_attributes() {
        let mut vram = Vram::new();
        
        // Cria atributos de sprite com hflip e vflip ativados
        let sprite_attrs = [0x0080, 0x0302, 0xE567, 0x00A0];  // Bits 11 e 12 ativos
        let base_addr = 0x8000;
        
        // Escreve atributos
        vram.write_sprite_attributes(base_addr, 5, &sprite_attrs);
        
        // Lê atributos
        let read_attrs = vram.read_sprite_attributes(base_addr, 5);
        assert_eq!(read_attrs, sprite_attrs);
        
        // Decodifica atributos
        let (y, x, tile_index, priority, link, palette, hflip, vflip) = Vram::decode_sprite_attributes(&read_attrs);
        assert_eq!(tile_index, 0x567);
        assert_eq!(priority, 1);
        assert_eq!(palette, 3);
        assert_eq!(hflip, true);
        assert_eq!(vflip, true);
        assert_eq!(link, 2);
        assert_eq!(y, 0x80);
        assert_eq!(x, 0xA0);
    }

    #[test]
    fn test_vram_operations() {
        let mut vram = Vram::new();
        
        // Teste fill
        vram.fill(0xAA);
        assert_eq!(vram.read8(0), 0xAA);
        assert_eq!(vram.read8(1000), 0xAA);
        
        // Teste clear
        vram.clear();
        assert_eq!(vram.read8(0), 0);
        
        // Teste copy
        let data = vec![1, 2, 3, 4, 5];
        vram.copy_from(0x500, &data);
        assert_eq!(vram.copy_to(0x500, 5), data);
        
        // Teste copy com overflow
        vram.copy_from(0xFFFC, &[0xDE, 0xAD, 0xBE, 0xEF, 0x12, 0x34]);
        let result = vram.copy_to(0xFFFC, 6);
        assert_eq!(result[..4], [0xDE, 0xAD, 0xBE, 0xEF]);
    }

    #[test]
    fn test_vram_size_operations() {
        let vram = Vram::with_size(128 * 1024);
        assert_eq!(vram.size(), 128 * 1024);
        assert!(vram.is_valid_address(0x1FFFF));
        assert!(!vram.is_valid_address(0x20000));
    }
    
    #[test]
    fn test_vram_edge_cases() {
        let mut vram = Vram::with_size(256);  // VRAM pequena para teste
        
        // Teste escrita no limite
        vram.write8(255, 0xFF);
        assert_eq!(vram.read8(255), 0xFF);
        
        // Teste leitura além do limite
        assert_eq!(vram.read8(256), 0);
        
        // Teste tile além do limite
        vram.write_tile_4bpp(7, &[0xAA; 32]);  // 7*32 = 224, dentro do limite
        let tile = vram.read_tile_4bpp(7);
        assert_eq!(tile[0], 0xAA);
        
        // Teste sprite attributes além do limite
        let attrs = [0x1111, 0x2222, 0x3333, 0x4444];
        vram.write_sprite_attributes(200, 0, &attrs);
        let read_attrs = vram.read_sprite_attributes(200, 0);
        assert_eq!(read_attrs[0], 0x1111);
    }
}