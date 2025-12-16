// src/cpu/decoder.rs

//! Decodificador de instruções da CPU Motorola 68000.
//! Traduz o opcode de 16 bits em uma operação de alto nível executável pela ALU.

use crate::cpu::alu::{Alu, AluOp, AluResult};
use crate::cpu::bus::Bus;
use crate::cpu::registers::Registers;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("Opcode desconhecido: 0x{0:04X}")]
    UnknownOpcode(u16),

    #[error("Endereço inválido de acesso: 0x{0:08X}")]
    InvalidAddress(u32),
}

/// Enum de instruções decodificadas.
#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Move { size: u8, src: usize, dst: usize },
    Add { size: u8, src: usize, dst: usize },
    Sub { size: u8, src: usize, dst: usize },
    And { size: u8, src: usize, dst: usize },
    Or  { size: u8, src: usize, dst: usize },
    Eor { size: u8, src: usize, dst: usize },
    Not { size: u8, reg: usize },
    Clr { size: u8, reg: usize },
    Cmp { size: u8, src: usize, dst: usize },
}

impl Instruction {
    /// Decodifica um opcode de 16 bits em uma `Instruction`.
    pub fn decode(opcode: u16) -> Result<Self, DecodeError> {
        let op_major: u16 = (opcode >> 12) & 0xF;
        let src: usize = ((opcode >> 9) & 0x7) as usize;
        let dst: usize = (opcode & 0x7) as usize;
        let size_code: u8 = ((opcode >> 6) & 0x3) as u8;
        let size: u8 = match size_code {
            0b00 => 8,
            0b01 => 16,
            0b10 => 32,
            _ => 16,
        };

        match op_major {
            0x0 => Ok(Self::Or { size, src, dst }),
            0x1 => Ok(Self::And { size, src, dst }),
            0x2 => Ok(Self::Sub { size, src, dst }),
            0x3 => Ok(Self::Add { size, src, dst }),
            0x4 => Ok(Self::Eor { size, src, dst }),
            0x5 => Ok(Self::Not { size, reg: dst }),
            0x6 => Ok(Self::Clr { size, reg: dst }),
            0x7 => Ok(Self::Cmp { size, src, dst }),
            0x8 => Ok(Self::Move { size, src, dst }),
            _ => Err(DecodeError::UnknownOpcode(opcode)),
        }
    }

    /// Executa a instrução decodificada.
    pub fn execute(
        &self,
        regs: &mut Registers,
        bus: &mut Bus,
    ) -> Result<(), DecodeError> {
        match *self {
            Instruction::Add { size, src, dst } => {
                let a: u32 = regs.get_data(dst);
                let b: u32 = regs.get_data(src);
                let alu_res: AluResult = Alu::execute(AluOp::Add, a, b, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                regs.set_data(dst, alu_res.value);
                Self::update_flags(regs, alu_res);
            }
            Instruction::Sub { size, src, dst } => {
                let a: u32 = regs.get_data(dst);
                let b: u32 = regs.get_data(src);
                let alu_res: AluResult = Alu::execute(AluOp::Sub, a, b, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                regs.set_data(dst, alu_res.value);
                Self::update_flags(regs, alu_res);
            }
            Instruction::And { size, src, dst } => {
                let a = regs.get_data(dst);
                let b = regs.get_data(src);
                let alu_res = Alu::execute(AluOp::And, a, b, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                regs.set_data(dst, alu_res.value);
                Self::update_flags(regs, alu_res);
            }
            Instruction::Or { size, src, dst } => {
                let a = regs.get_data(dst);
                let b = regs.get_data(src);
                let alu_res = Alu::execute(AluOp::Or, a, b, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                regs.set_data(dst, alu_res.value);
                Self::update_flags(regs, alu_res);
            }
            Instruction::Eor { size, src, dst } => {
                let a = regs.get_data(dst);
                let b = regs.get_data(src);
                let alu_res = Alu::execute(AluOp::Xor, a, b, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                regs.set_data(dst, alu_res.value);
                Self::update_flags(regs, alu_res);
            }
            Instruction::Not { size, reg } => {
                let a = regs.get_data(reg);
                let alu_res = Alu::execute(AluOp::Not, a, 0, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                regs.set_data(reg, alu_res.value);
                Self::update_flags(regs, alu_res);
            }
            Instruction::Clr { size, reg } => {
                regs.set_data(reg, 0);
                regs.set_flag("Z", true);
                regs.set_flag("N", false);
                regs.set_flag("C", false);
                regs.set_flag("V", false);
                regs.set_flag("X", false);
            }
            Instruction::Cmp { size, src, dst } => {
                let a = regs.get_data(dst);
                let b = regs.get_data(src);
                let alu_res = Alu::execute(AluOp::Sub, a, b, size)
                    .map_err(|_| DecodeError::UnknownOpcode(0xFFFF))?;
                Self::update_flags(regs, alu_res);
            }
            Instruction::Move { size, src, dst } => {
                let val = regs.get_data(src);
                regs.set_data(dst, val);
                regs.set_flag("Z", val == 0);
                regs.set_flag("N", (val & (1 << (size - 1))) != 0);
            }
        }
        Ok(())
    }

    /// Atualiza os flags do CCR conforme resultado da ALU.
    fn update_flags(regs: &mut Registers, res: AluResult) {
        regs.set_flag("Z", res.zero);
        regs.set_flag("N", res.negative);
        regs.set_flag("V", res.overflow);
        regs.set_flag("C", res.carry);
        regs.set_flag("X", res.extend);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::bus::Bus;

    #[test]
    fn test_decode_add() {
        let opcode = 0x3000; // ADD
        let instr = Instruction::decode(opcode).unwrap();
        if let Instruction::Add { size, src, dst } = instr {
            assert_eq!(size, 16);
            assert_eq!(src, 0);
            assert_eq!(dst, 0);
        }
    }

    #[test]
    fn test_execute_add() {
        let mut regs = Registers::new();
        let mut bus = Bus::new(vec![0; 4], 64 * 1024);
        regs.set_data(0, 10);
        regs.set_data(1, 5);
        let instr = Instruction::Add { size: 32, src: 1, dst: 0 };
        instr.execute(&mut regs, &mut bus).unwrap();
        assert_eq!(regs.get_data(0), 15);
        assert!(!regs.get_flag("Z"));
    }
}
