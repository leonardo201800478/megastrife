// src/cpu/alu.rs

//! ALU (Arithmetic Logic Unit) para CPU Motorola 68000.
//! Responsável pelas operações aritméticas e lógicas e atualização de flags.

use thiserror::Error;

/// Erros possíveis da ALU.
#[derive(Debug, Error)]
pub enum AluError {
    #[error("Operação desconhecida: {0}")]
    UnknownOperation(String),
}

/// Tipo de operação suportada pela ALU.
#[derive(Debug, Clone, Copy)]
pub enum AluOp {
    Add,
    Sub,
    And,
    Or,
    Xor,
    Not,
    Shl,
    Shr,
}

/// Resultado de uma operação da ALU.
#[derive(Debug, Clone, Copy)]
pub struct AluResult {
    pub value: u32,
    pub carry: bool,
    pub overflow: bool,
    pub zero: bool,
    pub negative: bool,
    pub extend: bool,
}

impl Default for AluResult {
    fn default() -> Self {
        Self {
            value: 0,
            carry: false,
            overflow: false,
            zero: true,
            negative: false,
            extend: false,
        }
    }
}

/// Implementação principal da ALU.
pub struct Alu;

impl Alu {
    pub fn execute(op: AluOp, a: u32, b: u32, size_bits: u8) -> Result<AluResult, AluError> {
        let mask: u32 = match size_bits {
            8 => 0xFF,
            16 => 0xFFFF,
            32 => 0xFFFF_FFFF,
            _ => return Err(AluError::UnknownOperation(format!("Tamanho inválido: {}", size_bits))),
        };

        let mut result = AluResult::default();

        match op {
            AluOp::Add => {
                let res: u32 = a.wrapping_add(b) & mask;
                result.value = res;
                result.carry = (a as u64 + b as u64) > mask as u64;
                result.overflow = ((a ^ res) & (b ^ res) & 0x8000_0000) != 0;
            }
            AluOp::Sub => {
                let res: u32 = a.wrapping_sub(b) & mask;
                result.value = res;
                result.carry = a < b;
                result.overflow = ((a ^ b) & (a ^ res) & 0x8000_0000) != 0;
            }
            AluOp::And => result.value = (a & b) & mask,
            AluOp::Or => result.value = (a | b) & mask,
            AluOp::Xor => result.value = (a ^ b) & mask,
            AluOp::Not => result.value = (!a) & mask,
            AluOp::Shl => {
                let res: u32 = (a << (b & 0x1F)) & mask;
                result.value = res;
                result.carry = (a >> (32 - (b & 0x1F))) & 1 != 0;
            }
            AluOp::Shr => {
                let res: u32 = (a >> (b & 0x1F)) & mask;
                result.value = res;
                result.carry = (a >> ((b - 1) & 0x1F)) & 1 != 0;
            }
        }

        result.zero = result.value == 0;
        result.negative = (result.value & (1 << (size_bits - 1))) != 0;
        result.extend = result.carry;

        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_add_8bit() {
        let r: AluResult = Alu::execute(AluOp::Add, 0x10, 0x20, 8).unwrap();
        assert_eq!(r.value, 0x30);
        assert!(!r.carry);
        assert!(!r.zero);
        assert!(!r.negative);
    }

    #[test]
    fn test_sub_16bit() {
        let r = Alu::execute(AluOp::Sub, 0x1234, 0x0034, 16).unwrap();
        assert_eq!(r.value, 0x1200);
        assert!(!r.carry);
    }

    #[test]
    fn test_and_32bit() {
        let r = Alu::execute(AluOp::And, 0xFF00_FF00, 0x0F0F_F0F0, 32).unwrap();
        assert_eq!(r.value, 0x0F00_F000);
    }
}
