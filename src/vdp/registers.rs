//! Registradores do VDP

bitflags! {
    /// Status Register do VDP
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct VdpStatus: u16 {
        const FIFO_EMPTY    = 0b0000000100000000;
        const FIFO_FULL     = 0b0000001000000000;
        const VBLANK        = 0b0000100000000000;
        const HBLANK        = 0b0001000000000000;
        const ODD_FRAME     = 0b0010000000000000;
        const COLLISION     = 0b0100000000000000;
        const OVERFLOW      = 0b1000000000000000;
    }
}

/// Modos de vídeo do VDP
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VdpMode {
    Mode0, // 256x192, 64 colors
    Mode1, // 320x192, 16 colors
    Mode2, // 256x192, 16 colors
    Mode3, // 320x192, 4 colors
    Mode4, // 256x224, 16 colors
    Mode5, // 320x224, 16 colors (MODE DO GENESIS)
}

/// Estrutura dos registradores do VDP
#[derive(Debug, Clone)]
pub struct VdpRegisters {
    registers: [u8; 24],
    address: u16,
    code: u8,
    increment: u16,
}

impl VdpRegisters {
    pub fn new() -> Self {
        Self {
            registers: [0; 24],
            address: 0,
            code: 0,
            increment: 2, // Padrão: incremento de 2 bytes
        }
    }

    pub fn reset(&mut self) {
        self.registers = [0; 24];
        self.address = 0;
        self.code = 0;
        self.increment = 2;

        // Valores padrão do Genesis
        self.registers[0x00] = 0x04; // HInterrupt off
        self.registers[0x01] = 0x04; // Display off
        self.registers[0x02] = 0x30; // Plane A address
        self.registers[0x03] = 0x3C; // Window address
        self.registers[0x04] = 0x07; // Plane B address
        self.registers[0x05] = 0x6C; // Sprite table address
        self.registers[0x07] = 0x00; // Background color
        self.registers[0x0C] = 0x81; // H40 mode, shadow/highlight off
    }

    pub fn write(&mut self, reg: u8, value: u8) {
        if reg < 24 {
            self.registers[reg as usize] = value;
        }
    }

    pub fn read(&self, reg: u8) -> u8 {
        if reg < 24 {
            self.registers[reg as usize]
        } else {
            0
        }
    }

    pub fn process_control_word(&mut self, value: u16) -> anyhow::Result<()> {
        if value & 0x8000 == 0x8000 {
            // Comando de registro
            let reg = ((value >> 8) & 0x1F) as u8;
            let data = value as u8;
            self.write(reg, data);
        } else {
            // Comando de endereço
            self.address = value & 0x3FFF;
            self.code = ((value >> 14) & 0x03) as u8;

            // Configura incremento baseado no modo
            match self.code {
                0x00 | 0x01 => self.increment = 1, // VRAM byte access
                _ => self.increment = 2,           // Word access
            }
        }

        Ok(())
    }

    pub fn get_address(&self) -> u16 {
        self.address
    }

    pub fn get_code(&self) -> u8 {
        self.code
    }

    pub fn increment_address(&mut self) {
        self.address = (self.address + self.increment) & 0xFFFF;
    }

    pub fn get_mode(&self) -> VdpMode {
        let mode_bits = (self.registers[0x0C] as u16) << 8;

        match mode_bits & 0x0C00 {
            0x0000 => VdpMode::Mode0,
            0x0400 => VdpMode::Mode1,
            0x0800 => VdpMode::Mode2,
            0x0C00 => VdpMode::Mode3,
            _ => VdpMode::Mode5, // Padrão
        }
    }

    pub fn hblank_interrupt_enabled(&self) -> bool {
        (self.registers[0x00] & 0x10) != 0
    }

    pub fn vblank_interrupt_enabled(&self) -> bool {
        (self.registers[0x01] & 0x20) != 0
    }

    pub fn get_background_a_base(&self) -> u16 {
        ((self.registers[0x02] & 0x38) as u16) << 10
    }

    pub fn get_background_b_base(&self) -> u16 {
        ((self.registers[0x04] & 0x07) as u16) << 13
    }

    pub fn get_window_base(&self) -> u16 {
        ((self.registers[0x03] & 0x3E) as u16) << 10
    }

    pub fn get_pattern_base(&self) -> u16 {
        ((self.registers[0x04] & 0x07) as u16) << 13
    }

    pub fn get_window_x(&self) -> u16 {
        ((self.registers[0x11] & 0x1F) as u16) << 3
    }

    pub fn get_window_y(&self) -> u16 {
        (self.registers[0x12] as u16) << 3
    }
}
