/// Color RAM (CRAM) — 64 cores de 9 bits (0–511)
/// Cada cor é armazenada no formato BBBGGGRRR (bits 0–8).
#[derive(Clone)]
pub struct Cram {
    colors: [u16; 64],
}

impl Cram {
    pub fn new() -> Self {
        Self { colors: [0; 64] }
    }

    pub fn write(&mut self, index: usize, value: u16) {
        if index < self.colors.len() {
            self.colors[index] = value & 0x1FF; // apenas 9 bits válidos
        }
    }

    pub fn read(&self, index: usize) -> u16 {
        self.colors[index % 64]
    }

    /// Converte uma cor CRAM em RGB (0–255 por canal).
    pub fn rgb(&self, index: usize) -> (u8, u8, u8) {
        let v = self.colors[index % 64];
        let r = ((v >> 0) & 0x7) as u8 * 32;
        let g = ((v >> 3) & 0x7) as u8 * 32;
        let b = ((v >> 6) & 0x7) as u8 * 32;
        (r, g, b)
    }
}
