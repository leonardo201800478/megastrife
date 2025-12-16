/// Color RAM (CRAM) — 64 cores de 9 bits (0–511)
/// Cada cor é armazenada no formato BBBGGGRRR (bits 0–8).
#[derive(Clone)]
pub struct Cram {
    pub data: [u16; 64],  // Alterado de `colors` para `data` para consistência com o dump
}

impl Cram {
    pub fn new() -> Self {
        Self { data: [0; 64] }
    }

    /// Escreve um valor de 16 bits na CRAM
    /// O endereço é tratado como índice de palavra (cada entrada é 2 bytes)
    pub fn write_word(&mut self, addr: u16, value: u16) {
        let index = (addr as usize) >> 1; // Endereço em bytes, converter para índice de palavra
        if index < 64 {
            self.data[index] = value & 0x0EEE; // Máscara: BBB0GGG0RRR0 (bits 11-0 válidos)
        }
    }

    /// Lê um valor de 16 bits da CRAM
    pub fn read_word(&self, addr: u16) -> u16 {
        let index = (addr as usize) >> 1; // Endereço em bytes, converter para índice de palavra
        if index < 64 {
            self.data[index]
        } else {
            0
        }
    }

    /// Escreve diretamente em um índice específico da CRAM
    pub fn write(&mut self, index: usize, value: u16) {
        if index < 64 {
            self.data[index] = value & 0x0EEE; // Apenas 12 bits válidos (BBB0GGG0RRR0)
        }
    }

    /// Lê diretamente de um índice específico da CRAM
    pub fn read(&self, index: usize) -> u16 {
        self.data[index % 64]
    }

    /// Converte uma cor CRAM em RGB (0–255 por canal)
    /// Formato original: 0000 BBB0 GGG0 RRR0 (12 bits, mas apenas 9 bits de cor)
    pub fn to_rgb(&self, index: usize) -> (u8, u8, u8) {
        let v = self.data[index % 64];
        
        // Extrair componentes de 3 bits cada e expandir para 8 bits
        // BBB está nos bits 8-6, GGG nos bits 4-2, RRR nos bits 0-2
        let r = ((v >> 0) & 0x7) as u8;  // 3 bits do vermelho
        let g = ((v >> 4) & 0x7) as u8;  // 3 bits do verde
        let b = ((v >> 8) & 0x7) as u8;  // 3 bits do azul
        
        // Expandir de 3 bits (0-7) para 8 bits (0-255)
        // Fórmula: valor * 255 / 7, arredondado
        let r = (r * 36) as u8;  // 36 ≈ 255/7
        let g = (g * 36) as u8;
        let b = (b * 36) as u8;
        
        (r, g, b)
    }

    /// Converte para RGB 888 (32-bit ARGB)
    pub fn to_rgb888(&self, index: usize) -> u32 {
        let (r, g, b) = self.to_rgb(index);
        0xFF000000 | ((r as u32) << 16) | ((g as u32) << 8) | (b as u32)
    }

    /// Retorna uma cópia de todos os dados da CRAM
    pub fn dump(&self) -> Vec<u16> {
        self.data.to_vec()
    }
}

impl Default for Cram {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cram_write_read() {
        let mut cram = Cram::new();
        
        // Teste escrita direta
        cram.write(0, 0x0ABC); // BBB=0x5, GGG=0x2, RRR=0x4
        assert_eq!(cram.read(0), 0x0ABC & 0x0EEE);
        
        // Teste escrita por endereço
        cram.write_word(0x02, 0x0123); // Endereço 0x02 = índice 1
        assert_eq!(cram.read(1), 0x0123 & 0x0EEE);
        
        // Teste índice fora dos limites
        cram.write(100, 0x0FFF);
        assert_eq!(cram.read(100), cram.read(100 % 64));
    }

    #[test]
    fn test_cram_to_rgb() {
        let mut cram = Cram::new();
        
        // Teste: cor branca (todos os bits = 1)
        // BBB=111 (0x7), GGG=111 (0x7), RRR=111 (0x7)
        cram.write(0, 0x0EEE);
        let (r, g, b) = cram.to_rgb(0);
        assert_eq!(r, 36 * 7);
        assert_eq!(g, 36 * 7);
        assert_eq!(b, 36 * 7);
        
        // Teste: cor vermelha máxima
        // RRR=111 (0x7), GGG=000 (0x0), BBB=000 (0x0)
        cram.write(1, 0x000E);
        let (r, g, b) = cram.to_rgb(1);
        assert_eq!(r, 36 * 7);
        assert_eq!(g, 0);
        assert_eq!(b, 0);
        
        // Teste: cor verde média
        // RRR=100 (0x4), GGG=100 (0x4), BBB=100 (0x4)
        cram.write(2, 0x0444);
        let (r, g, b) = cram.to_rgb(2);
        assert_eq!(r, 36 * 4);
        assert_eq!(g, 36 * 4);
        assert_eq!(b, 36 * 4);
    }

    #[test]
    fn test_cram_to_rgb888() {
        let mut cram = Cram::new();
        
        // Teste cor vermelha
        cram.write(0, 0x000E); // Vermelho máximo
        let rgb = cram.to_rgb888(0);
        assert_eq!(rgb, 0xFF0000FF & 0xFFFF00FF); // Apenas verifica componente vermelho
        
        // Teste cor branca
        cram.write(1, 0x0EEE);
        let rgb = cram.to_rgb888(1);
        assert_eq!(rgb, 0xFFFFFFFF);
    }
}