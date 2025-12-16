//! Registradores do M68000

use bitflags::bitflags;

/// Registradores de dados D0-D7
pub type DataReg = [u32; 8];

/// Registradores de endereço A0-A7
pub type AddrReg = [u32; 8];

bitflags! {
    /// Status Register
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct StatusRegister: u16 {
        const CARRY     = 0b00000001;
        const OVERFLOW  = 0b00000010;
        const ZERO      = 0b00000100;
        const NEGATIVE  = 0b00001000;
        const EXTEND    = 0b00010000;
        const SUPER     = 0b00100000;
        const TRACE     = 0b01000000;
        const INTERRUPT = 0b11100000000; // 3 bits
    }
}

/// Todos os registradores do M68000
#[derive(Debug, Clone)]
pub struct Registers {
    /// D0-D7
    pub d: DataReg,
    
    /// A0-A6
    pub a: AddrReg,
    
    /// A7 (Stack Pointer)
    pub ssp: u32,  // Supervisor Stack Pointer
    pub usp: u32,  // User Stack Pointer
    
    /// Program Counter
    pub pc: u32,
    
    /// Status Register
    pub sr: StatusRegister,
}

impl Registers {
    pub fn new() -> Self {
        Self {
            d: [0; 8],
            a: [0; 8],
            ssp: 0,
            usp: 0,
            pc: 0,
            sr: StatusRegister::from_bits_truncate(0x2700), // Estado inicial
        }
    }
    
    pub fn get_a(&self, index: u8) -> u32 {
        if index < 7 {
            self.a[index as usize]
        } else {
            if self.sr.contains(StatusRegister::SUPER) {
                self.ssp
            } else {
                self.usp
            }
        }
    }
    
    pub fn set_a(&mut self, index: u8, value: u32) {
        if index < 7 {
            self.a[index as usize] = value;
        } else {
            if self.sr.contains(StatusRegister::SUPER) {
                self.ssp = value;
            } else {
                self.usp = value;
            }
        }
    }
}