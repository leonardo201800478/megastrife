/// Registradores internos do VDP.
/// Existem 24 registradores de controle principais.
#[derive(Clone)]
pub struct VdpRegisters {
    pub regs: [u8; 24],
}

impl VdpRegisters {
    pub fn new() -> Self {
        Self { regs: [0; 24] }
    }

    pub fn write(&mut self, index: usize, value: u8) {
        if index < self.regs.len() {
            self.regs[index] = value;
        }
    }

    pub fn read(&self, index: usize) -> u8 {
        self.regs[index % self.regs.len()]
    }

    pub fn display_enable(&self) -> bool {
        self.regs[1] & 0x40 != 0
    }

    pub fn dma_enable(&self) -> bool {
        self.regs[1] & 0x10 != 0
    }

    pub fn mode_40_cell(&self) -> bool {
        self.regs[12] & 0x01 != 0
    }
}
