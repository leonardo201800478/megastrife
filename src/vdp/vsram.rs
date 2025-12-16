//! Implementação da VSRAM (Vertical Scroll RAM) do VDP.
//!
//! A VSRAM é uma memória especial de 40 palavras (80 bytes) usada para:
//! 1. Scroll vertical global para os planos A e B
//! 2. Efeitos de line scroll (scroll vertical por linha)
//! 3. Efeitos de column scroll (scroll vertical por coluna)
//! 4. Configurações de split screen
//!
//! Organização da VSRAM:
//! - 0x00-0x4F: 80 bytes (40 palavras de 16 bits)
//! - Entradas 0-31: Scroll vertical para plano A (uma por célula de 8 pixels)
//! - Entradas 32-39: Scroll vertical para plano B (uma por linha de tile)
//! 
//! Nota: No modo H40 (320 pixels), apenas as primeiras 20 entradas são usadas.

use std::ops::{Index, IndexMut};

/// Tamanho total da VSRAM em bytes
pub const VSRAM_SIZE_BYTES: usize = 80;
/// Tamanho total da VSRAM em palavras (16 bits)
pub const VSRAM_SIZE_WORDS: usize = 40;
/// Número máximo de linhas suportadas (NTSC + PAL)
pub const MAX_VERTICAL_LINES: usize = 313;

/// Tipos de scroll vertical suportados
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollMode {
    /// Scroll vertical global (uma única posição para todo o plano)
    Global,
    /// Scroll por linha (cada linha pode ter scroll diferente)
    LineScroll,
    /// Scroll por coluna (cada coluna de 8 pixels pode ter scroll diferente)
    ColumnScroll,
    /// Modo split screen (duas regiões com scroll diferente)
    SplitScreen,
}

/// Estrutura da VSRAM com funcionalidades completas
#[derive(Clone)]
pub struct Vsram {
    pub data: [u16; VSRAM_SIZE_WORDS],  // 40 palavras de 16 bits
    pub plane_a_scroll: i16,             // Scroll vertical global do plano A
    pub plane_b_scroll: i16,             // Scroll vertical global do plano B
    pub split_line: u16,                 // Linha onde ocorre o split screen
    pub split_scroll_a: i16,             // Scroll após split para plano A
    pub split_scroll_b: i16,             // Scroll após split para plano B
    pub scroll_mode: ScrollMode,         // Modo de scroll ativo
    pub line_scroll_enabled: bool,       // Line scroll habilitado?
    pub column_scroll_enabled: bool,     // Column scroll habilitado?
}

impl Vsram {
    /// Cria uma nova VSRAM zerada
    pub fn new() -> Self {
        Self {
            data: [0; VSRAM_SIZE_WORDS],
            plane_a_scroll: 0,
            plane_b_scroll: 0,
            split_line: 0,
            split_scroll_a: 0,
            split_scroll_b: 0,
            scroll_mode: ScrollMode::Global,
            line_scroll_enabled: false,
            column_scroll_enabled: false,
        }
    }

    // =====================================================
    // ACESSO BÁSICO À MEMÓRIA
    // =====================================================

    /// Lê um byte (8 bits) da VSRAM
    /// A VSRAM é acessada como memória de 16 bits, então endereços ímpares retornam 0
    pub fn read8(&self, addr: u32) -> u8 {
        let addr = addr as usize;
        
        if addr >= VSRAM_SIZE_BYTES {
            return 0;
        }
        
        let word_addr = addr >> 1;
        let word = self.data[word_addr];
        
        if (addr & 1) == 0 {
            // Byte baixo
            (word & 0xFF) as u8
        } else {
            // Byte alto
            (word >> 8) as u8
        }
    }

    /// Lê uma palavra (16 bits) da VSRAM
    /// O endereço deve ser alinhado (par), caso contrário o comportamento é indefinido
    pub fn read16(&self, addr: u32) -> u16 {
        let addr = addr as usize;
        
        if addr >= VSRAM_SIZE_BYTES {
            return 0;
        }
        
        let word_addr = addr >> 1;
        self.data[word_addr]
    }

    /// Escreve um byte (8 bits) na VSRAM
    pub fn write8(&mut self, addr: u32, value: u8) {
        let addr = addr as usize;
        
        if addr >= VSRAM_SIZE_BYTES {
            return;
        }
        
        let word_addr = addr >> 1;
        let mut word = self.data[word_addr];
        
        if (addr & 1) == 0 {
            // Byte baixo
            word = (word & 0xFF00) | (value as u16);
        } else {
            // Byte alto
            word = (word & 0x00FF) | ((value as u16) << 8);
        }
        
        self.data[word_addr] = word;
        self.update_scroll_from_data();
    }

    /// Escreve uma palavra (16 bits) na VSRAM
    /// O endereço deve ser alinhado (par)
    pub fn write16(&mut self, addr: u32, value: u16) {
        let addr = addr as usize;
        
        if addr >= VSRAM_SIZE_BYTES {
            return;
        }
        
        let word_addr = addr >> 1;
        self.data[word_addr] = value;
        self.update_scroll_from_data();
    }

    // =====================================================
    // SCROLL VERTICAL
    // =====================================================

    /// Atualiza o scroll vertical baseado nos dados da VSRAM
    fn update_scroll_from_data(&mut self) {
        // As primeiras duas palavras são o scroll global do plano A e B
        self.plane_a_scroll = self.data[0] as i16;
        self.plane_b_scroll = self.data[1] as i16;
        
        // A terceira palavra é a linha de split (se suportado)
        if VSRAM_SIZE_WORDS > 2 {
            self.split_line = self.data[2];
        }
        
        // A quarta e quinta palavras são os scrolls após split
        if VSRAM_SIZE_WORDS > 4 {
            self.split_scroll_a = self.data[3] as i16;
            self.split_scroll_b = self.data[4] as i16;
        }
    }

    /// Obtém o scroll vertical para uma linha específica do plano A
    /// Considera line scroll, column scroll e split screen
    pub fn get_line_scroll_a(&self, line: u16, column: u16, mode: ScrollMode) -> i16 {
        match mode {
            ScrollMode::Global => {
                // Scroll global apenas
                self.plane_a_scroll
            }
            ScrollMode::LineScroll => {
                // Line scroll: cada linha tem scroll independente
                if self.line_scroll_enabled {
                    let entry = (line as usize) % VSRAM_SIZE_WORDS;
                    self.data[entry] as i16
                } else {
                    self.plane_a_scroll
                }
            }
            ScrollMode::ColumnScroll => {
                // Column scroll: cada coluna de 8 pixels tem scroll independente
                if self.column_scroll_enabled {
                    let column_entry = (column / 8) as usize % 20; // Máximo 20 colunas em H40
                    self.data[column_entry] as i16
                } else {
                    self.plane_a_scroll
                }
            }
            ScrollMode::SplitScreen => {
                // Split screen: scroll diferente antes e depois da linha de split
                if line < self.split_line {
                    self.plane_a_scroll
                } else {
                    self.split_scroll_a
                }
            }
        }
    }

    /// Obtém o scroll vertical para uma linha específica do plano B
    pub fn get_line_scroll_b(&self, line: u16, column: u16, mode: ScrollMode) -> i16 {
        match mode {
            ScrollMode::Global => self.plane_b_scroll,
            ScrollMode::LineScroll => {
                if self.line_scroll_enabled {
                    let entry = (line as usize) % VSRAM_SIZE_WORDS;
                    self.data[entry] as i16
                } else {
                    self.plane_b_scroll
                }
            }
            ScrollMode::ColumnScroll => {
                if self.column_scroll_enabled {
                    let column_entry = (column / 8) as usize % 20;
                    self.data[column_entry] as i16
                } else {
                    self.plane_b_scroll
                }
            }
            ScrollMode::SplitScreen => {
                if line < self.split_line {
                    self.plane_b_scroll
                } else {
                    self.split_scroll_b
                }
            }
        }
    }

    /// Configura o scroll vertical global
    pub fn set_global_scroll(&mut self, plane_a_scroll: i16, plane_b_scroll: i16) {
        self.plane_a_scroll = plane_a_scroll;
        self.plane_b_scroll = plane_b_scroll;
        self.data[0] = plane_a_scroll as u16;
        self.data[1] = plane_b_scroll as u16;
    }

    /// Configura o scroll de linha específica
    pub fn set_line_scroll(&mut self, line: usize, scroll: i16) {
        if line < VSRAM_SIZE_WORDS {
            self.data[line] = scroll as u16;
        }
    }

    /// Configura o scroll de coluna específica
    pub fn set_column_scroll(&mut self, column: usize, scroll: i16) {
        // Em H40, existem 40 células horizontais (320/8), mas apenas 20 entradas
        let entry = (column / 2) % 20; // Cada entrada controla 2 células de 8 pixels
        if entry < VSRAM_SIZE_WORDS {
            self.data[entry] = scroll as u16;
        }
    }

    // =====================================================
    // CONFIGURAÇÃO DE MODO
    // =====================================================

    /// Configura o modo de scroll
    pub fn set_scroll_mode(&mut self, mode: ScrollMode) {
        self.scroll_mode = mode;
    }

    /// Habilita/desabilita line scroll
    pub fn set_line_scroll_enabled(&mut self, enabled: bool) {
        self.line_scroll_enabled = enabled;
        if enabled {
            self.scroll_mode = ScrollMode::LineScroll;
        }
    }

    /// Habilita/desabilita column scroll
    pub fn set_column_scroll_enabled(&mut self, enabled: bool) {
        self.column_scroll_enabled = enabled;
        if enabled {
            self.scroll_mode = ScrollMode::ColumnScroll;
        }
    }

    /// Configura split screen
    pub fn set_split_screen(&mut self, split_line: u16, scroll_a: i16, scroll_b: i16) {
        self.split_line = split_line;
        self.split_scroll_a = scroll_a;
        self.split_scroll_b = scroll_b;
        
        if VSRAM_SIZE_WORDS > 2 {
            self.data[2] = split_line;
        }
        if VSRAM_SIZE_WORDS > 3 {
            self.data[3] = scroll_a as u16;
        }
        if VSRAM_SIZE_WORDS > 4 {
            self.data[4] = scroll_b as u16;
        }
        
        self.scroll_mode = ScrollMode::SplitScreen;
    }

    // =====================================================
    // OPERAÇÕES EM BLOCO
    // =====================================================

    /// Limpa toda a VSRAM
    pub fn clear(&mut self) {
        self.data.fill(0);
        self.plane_a_scroll = 0;
        self.plane_b_scroll = 0;
        self.split_line = 0;
        self.split_scroll_a = 0;
        self.split_scroll_b = 0;
        self.scroll_mode = ScrollMode::Global;
    }

    /// Preenche a VSRAM com um valor específico
    pub fn fill(&mut self, value: u16) {
        self.data.fill(value);
        self.update_scroll_from_data();
    }

    /// Copia dados para a VSRAM
    pub fn copy_from(&mut self, addr: u32, data: &[u8]) {
        let start = addr as usize;
        if start >= VSRAM_SIZE_BYTES {
            return;
        }
        
        let end = (start + data.len()).min(VSRAM_SIZE_BYTES);
        
        for (i, &byte) in data.iter().enumerate().take(end - start) {
            self.write8((start + i) as u32, byte);
        }
    }

    /// Copia dados da VSRAM
    pub fn copy_to(&self, addr: u32, len: usize) -> Vec<u8> {
        let start = addr as usize;
        if start >= VSRAM_SIZE_BYTES {
            return vec![0; len];
        }
        
        let copy_len = len.min(VSRAM_SIZE_BYTES - start);
        let mut result = Vec::with_capacity(len);
        
        for i in 0..copy_len {
            result.push(self.read8((start + i) as u32));
        }
        
        // Preencher o restante com zeros
        while result.len() < len {
            result.push(0);
        }
        
        result
    }

    /// Retorna uma cópia completa dos dados da VSRAM
    pub fn dump(&self) -> Vec<u16> {
        self.data.to_vec()
    }

    /// Retorna os dados da VSRAM como bytes
    pub fn dump_bytes(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(VSRAM_SIZE_BYTES);
        for word in self.data.iter() {
            bytes.push((word & 0xFF) as u8);
            bytes.push((word >> 8) as u8);
        }
        bytes
    }

    // =====================================================
    // UTILITÁRIOS
    // =====================================================

    /// Retorna o tamanho da VSRAM em bytes
    pub fn size_bytes(&self) -> usize {
        VSRAM_SIZE_BYTES
    }

    /// Retorna o tamanho da VSRAM em palavras
    pub fn size_words(&self) -> usize {
        VSRAM_SIZE_WORDS
    }

    /// Verifica se um endereço está dentro dos limites da VSRAM
    pub fn is_valid_address(&self, addr: u32) -> bool {
        (addr as usize) < VSRAM_SIZE_BYTES
    }

    /// Retorna informações sobre o estado atual da VSRAM
    pub fn get_info(&self) -> String {
        format!(
            "VSRAM: {} words ({} bytes), Mode: {:?}, Plane A Scroll: {}, Plane B Scroll: {}",
            self.size_words(),
            self.size_bytes(),
            self.scroll_mode,
            self.plane_a_scroll,
            self.plane_b_scroll
        )
    }

    /// Retorna detalhes do scroll atual
    pub fn get_scroll_info(&self) -> Vec<String> {
        vec![
            format!("Scroll Mode: {:?}", self.scroll_mode),
            format!("Plane A Scroll: {}", self.plane_a_scroll),
            format!("Plane B Scroll: {}", self.plane_b_scroll),
            format!("Split Line: {}", self.split_line),
            format!("Split Scroll A: {}", self.split_scroll_a),
            format!("Split Scroll B: {}", self.split_scroll_b),
            format!("Line Scroll Enabled: {}", self.line_scroll_enabled),
            format!("Column Scroll Enabled: {}", self.column_scroll_enabled),
        ]
    }
}

// Implementação de traits para acesso indexado
impl Index<usize> for Vsram {
    type Output = u16;
    
    fn index(&self, index: usize) -> &Self::Output {
        &self.data[index % VSRAM_SIZE_WORDS]
    }
}

impl IndexMut<usize> for Vsram {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.data[index % VSRAM_SIZE_WORDS]
    }
}

impl Default for Vsram {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vsram_creation() {
        let vsram = Vsram::new();
        assert_eq!(vsram.data.len(), VSRAM_SIZE_WORDS);
        assert_eq!(vsram.plane_a_scroll, 0);
        assert_eq!(vsram.plane_b_scroll, 0);
        assert_eq!(vsram.scroll_mode, ScrollMode::Global);
    }

    #[test]
    fn test_vsram_read_write_16bit() {
        let mut vsram = Vsram::new();
        
        // Teste escrita/leitura alinhada
        vsram.write16(0, 0x1234);
        assert_eq!(vsram.read16(0), 0x1234);
        
        // Teste escrita/leitura em outro endereço
        vsram.write16(4, 0x5678);
        assert_eq!(vsram.read16(4), 0x5678);
        
        // Verificar que o scroll foi atualizado
        assert_eq!(vsram.plane_a_scroll, 0x1234 as i16);
        assert_eq!(vsram.plane_b_scroll, 0x5678 as i16);
    }

    #[test]
    fn test_vsram_read_write_8bit() {
        let mut vsram = Vsram::new();
        
        // Escrever bytes individuais
        vsram.write8(0, 0x12); // Byte baixo
        vsram.write8(1, 0x34); // Byte alto
        
        assert_eq!(vsram.read8(0), 0x12);
        assert_eq!(vsram.read8(1), 0x34);
        assert_eq!(vsram.read16(0), 0x3412);
        
        // Atualização do scroll
        assert_eq!(vsram.plane_a_scroll, 0x3412 as i16);
    }

    #[test]
    fn test_vsram_global_scroll() {
        let mut vsram = Vsram::new();
        
        vsram.set_global_scroll(100, -50);
        
        assert_eq!(vsram.plane_a_scroll, 100);
        assert_eq!(vsram.plane_b_scroll, -50);
        assert_eq!(vsram.data[0], 100 as u16);
        assert_eq!(vsram.data[1], (-50 as i16) as u16);
        
        // Verificar leitura
        assert_eq!(vsram.get_line_scroll_a(0, 0, ScrollMode::Global), 100);
        assert_eq!(vsram.get_line_scroll_b(0, 0, ScrollMode::Global), -50);
    }

    #[test]
    fn test_vsram_line_scroll() {
        let mut vsram = Vsram::new();
        
        vsram.set_line_scroll_enabled(true);
        vsram.set_line_scroll(5, 25);
        vsram.set_line_scroll(10, -25);
        
        assert_eq!(vsram.data[5], 25 as u16);
        assert_eq!(vsram.data[10], (-25 as i16) as u16);
        
        // Verificar leitura por linha
        assert_eq!(vsram.get_line_scroll_a(5, 0, ScrollMode::LineScroll), 25);
        assert_eq!(vsram.get_line_scroll_a(10, 0, ScrollMode::LineScroll), -25);
        
        // Linha fora do range usa scroll global
        assert_eq!(vsram.get_line_scroll_a(50, 0, ScrollMode::LineScroll), 0);
    }

    #[test]
    fn test_vsram_column_scroll() {
        let mut vsram = Vsram::new();
        
        vsram.set_column_scroll_enabled(true);
        
        // Configurar scroll para diferentes colunas
        vsram.set_column_scroll(0, 10);   // Coluna 0-7
        vsram.set_column_scroll(8, 20);   // Coluna 8-15
        vsram.set_column_scroll(16, 30);  // Coluna 16-23
        
        // Verificar scroll por coluna
        assert_eq!(vsram.get_line_scroll_a(0, 0, ScrollMode::ColumnScroll), 10);
        assert_eq!(vsram.get_line_scroll_a(0, 8, ScrollMode::ColumnScroll), 20);
        assert_eq!(vsram.get_line_scroll_a(0, 16, ScrollMode::ColumnScroll), 30);
        
        // Coluna fora do range
        assert_eq!(vsram.get_line_scroll_a(0, 320, ScrollMode::ColumnScroll), 10); // Wrap-around
    }

    #[test]
    fn test_vsram_split_screen() {
        let mut vsram = Vsram::new();
        
        vsram.set_split_screen(100, 50, -50);
        
        assert_eq!(vsram.split_line, 100);
        assert_eq!(vsram.split_scroll_a, 50);
        assert_eq!(vsram.split_scroll_b, -50);
        
        // Verificar scroll antes e depois do split
        assert_eq!(vsram.get_line_scroll_a(50, 0, ScrollMode::SplitScreen), 0);  // Antes do split
        assert_eq!(vsram.get_line_scroll_a(150, 0, ScrollMode::SplitScreen), 50); // Depois do split
        
        assert_eq!(vsram.get_line_scroll_b(50, 0, ScrollMode::SplitScreen), 0);   // Antes do split
        assert_eq!(vsram.get_line_scroll_b(150, 0, ScrollMode::SplitScreen), -50); // Depois do split
    }

    #[test]
    fn test_vsram_operations() {
        let mut vsram = Vsram::new();
        
        // Teste clear
        vsram.set_global_scroll(100, 200);
        vsram.clear();
        assert_eq!(vsram.plane_a_scroll, 0);
        assert_eq!(vsram.plane_b_scroll, 0);
        assert!(vsram.data.iter().all(|&x| x == 0));
        
        // Teste fill
        vsram.fill(0x1234);
        assert!(vsram.data.iter().all(|&x| x == 0x1234));
        
        // Teste copy
        let data = vec![0x01, 0x02, 0x03, 0x04];
        vsram.copy_from(0, &data);
        assert_eq!(vsram.read8(0), 0x01);
        assert_eq!(vsram.read8(1), 0x02);
        assert_eq!(vsram.read8(2), 0x03);
        assert_eq!(vsram.read8(3), 0x04);
        
        // Teste copy_to
        let copied = vsram.copy_to(0, 8);
        assert_eq!(copied[0], 0x01);
        assert_eq!(copied[1], 0x02);
        assert_eq!(copied[2], 0x03);
        assert_eq!(copied[3], 0x04);
    }

    #[test]
    fn test_vsram_dump() {
        let mut vsram = Vsram::new();
        
        vsram.data[0] = 0x1234;
        vsram.data[1] = 0x5678;
        vsram.data[2] = 0x9ABC;
        
        let dump = vsram.dump();
        assert_eq!(dump.len(), VSRAM_SIZE_WORDS);
        assert_eq!(dump[0], 0x1234);
        assert_eq!(dump[1], 0x5678);
        assert_eq!(dump[2], 0x9ABC);
        
        let bytes = vsram.dump_bytes();
        assert_eq!(bytes.len(), VSRAM_SIZE_BYTES);
        assert_eq!(bytes[0], 0x34);
        assert_eq!(bytes[1], 0x12);
        assert_eq!(bytes[2], 0x78);
        assert_eq!(bytes[3], 0x56);
    }

    #[test]
    fn test_vsram_indexing() {
        let mut vsram = Vsram::new();
        
        // Teste Index trait
        vsram[0] = 0x1234;
        vsram[1] = 0x5678;
        
        assert_eq!(vsram[0], 0x1234);
        assert_eq!(vsram[1], 0x5678);
        
        // Teste IndexMut trait
        vsram[2] = 0x9ABC;
        assert_eq!(vsram[2], 0x9ABC);
        
        // Teste wrap-around
        let index = VSRAM_SIZE_WORDS + 5;
        vsram[index] = 0xDEAD;
        assert_eq!(vsram[5], 0xDEAD);
    }

    #[test]
    fn test_vsram_info() {
        let vsram = Vsram::new();
        
        let info = vsram.get_info();
        assert!(info.contains("VSRAM"));
        assert!(info.contains("Global"));
        
        let scroll_info = vsram.get_scroll_info();
        assert_eq!(scroll_info.len(), 8);
        assert!(scroll_info[0].contains("Global"));
        assert!(scroll_info[1].contains("Plane A Scroll"));
    }
}