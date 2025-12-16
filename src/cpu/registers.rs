// src/cpu/registers.rs

//! Representa o conjunto de registradores da CPU Motorola 68000.

#[derive(Clone, Copy, Debug, Default)]
pub struct CCR {
    pub extend: bool,   // Bit 4 (X)
    pub negative: bool, // Bit 3 (N)
    pub zero: bool,     // Bit 2 (Z)
    pub overflow: bool, // Bit 1 (V)
    pub carry: bool,    // Bit 0 (C)
}

impl CCR {
    /// Retorna o valor binário dos bits do CCR (5 bits inferiores do SR).
    pub fn to_u16(&self) -> u16 {
        ((self.extend as u16) << 4)
            | ((self.negative as u16) << 3)
            | ((self.zero as u16) << 2)
            | ((self.overflow as u16) << 1)
            | (self.carry as u16)
    }

    /// Define o CCR a partir de um valor binário.
    pub fn from_u16(value: u16) -> Self {
        Self {
            extend: (value & 0x10) != 0,
            negative: (value & 0x08) != 0,
            zero: (value & 0x04) != 0,
            overflow: (value & 0x02) != 0,
            carry: (value & 0x01) != 0,
        }
    }
}

/// Representa o conjunto completo de registradores da CPU M68000.
#[derive(Clone, Debug)]
pub struct Registers {
    pub d: [u32; 8], // Data registers D0–D7
    pub a: [u32; 8], // Address registers A0–A7
    pub pc: u32,     // Program Counter
    pub sr: u16,     // Status Register (inclui CCR + bits de modo)
    pub ccr: CCR,    // Condition Code Register
}

impl Registers {
    pub fn new() -> Self {
        Self {
            d: [0; 8],
            a: [0; 8],
            pc: 0,
            sr: 0x2700, // Supervisor mode padrão (após reset)
            ccr: CCR::default(),
        }
    }

    /// Atualiza o SR (Status Register) a partir do CCR atual.
    pub fn update_sr_from_ccr(&mut self) {
        self.sr = (self.sr & 0xFFF0) | self.ccr.to_u16();
    }

    /// Atualiza o CCR com base no SR atual.
    pub fn update_ccr_from_sr(&mut self) {
        self.ccr = CCR::from_u16(self.sr & 0x1F);
    }

    /// Define uma flag do CCR.
    pub fn set_flag(&mut self, flag: &str, value: bool) {
        match flag {
            "X" => self.ccr.extend = value,
            "N" => self.ccr.negative = value,
            "Z" => self.ccr.zero = value,
            "V" => self.ccr.overflow = value,
            "C" => self.ccr.carry = value,
            _ => (),
        }
        self.update_sr_from_ccr();
    }

    /// Lê o valor de uma flag.
    pub fn get_flag(&self, flag: &str) -> bool {
        match flag {
            "X" => self.ccr.extend,
            "N" => self.ccr.negative,
            "Z" => self.ccr.zero,
            "V" => self.ccr.overflow,
            "C" => self.ccr.carry,
            _ => false,
        }
    }

    /// Lê um registrador de dados.
    pub fn get_data(&self, index: usize) -> u32 {
        self.d[index]
    }

    /// Lê um registrador de endereço.
    pub fn get_address(&self, index: usize) -> u32 {
        self.a[index]
    }

    /// Define um registrador de dados.
    pub fn set_data(&mut self, index: usize, value: u32) {
        self.d[index] = value;
    }

    /// Define um registrador de endereço.
    pub fn set_address(&mut self, index: usize, value: u32) {
        self.a[index] = value;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ccr_conversion() {
        let ccr = CCR {
            extend: true,
            negative: false,
            zero: true,
            overflow: false,
            carry: true,
        };
        let val = ccr.to_u16();
        assert_eq!(val, 0b10101);
        let back = CCR::from_u16(val);
        assert_eq!(back.carry, true);
        assert_eq!(back.zero, true);
        assert_eq!(back.extend, true);
    }

    #[test]
    fn test_registers_flags() {
        let mut r = Registers::new();
        r.set_flag("C", true);
        assert!(r.get_flag("C"));
        r.set_flag("Z", true);
        assert!(r.ccr.zero);
    }

    #[test]
    fn test_registers_set_get() {
        let mut r = Registers::new();
        r.set_data(0, 0x12345678);
        assert_eq!(r.get_data(0), 0x12345678);
        r.set_address(7, 0xFF00);
        assert_eq!(r.get_address(7), 0xFF00);
    }
}
