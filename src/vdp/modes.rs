/// Modos de exibição do VDP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdpMode {
    Mode32Cell, // 256x224
    Mode40Cell, // 320x224
}

impl VdpMode {
    pub fn from_registers(r: &crate::vdp::registers::VdpRegisters) -> Self {
        if r.mode_40_cell() {
            VdpMode::Mode40Cell
        } else {
            VdpMode::Mode32Cell
        }
    }

    pub fn resolution(&self) -> (u32, u32) {
        match self {
            VdpMode::Mode32Cell => (256, 224),
            VdpMode::Mode40Cell => (320, 224),
        }
    }
}
