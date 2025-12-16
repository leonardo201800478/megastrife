//! Decodificador de instruções M68000

#[derive(Debug, Clone, Copy)]
pub enum Instruction {
    Move(AddressingMode, AddressingMode, Size),
    Add(AddressingMode, AddressingMode, Size),
    Sub(AddressingMode, AddressingMode, Size),
    And(AddressingMode, AddressingMode, Size),
    Or(AddressingMode, AddressingMode, Size),
    Xor(AddressingMode, AddressingMode, Size),  // Renomeado de Eor para Xor
    Cmp(AddressingMode, AddressingMode, Size),
    Nop,
    Illegal(u16),
}

#[derive(Debug, Clone, Copy)]
pub enum AddressingMode {
    DataRegister(u8),    // Dn
    AddressRegister(u8), // An
    Immediate(u32),      // #<data>
                         // Outros modos...
}

#[derive(Debug, Clone, Copy)]
pub enum Size {
    Byte,
    Word,
    Long,
}

/// Decodifica um opcode
pub fn decode(opcode: u16) -> Result<Instruction, String> {
    let high_nibble = (opcode >> 12) & 0xF;

    match high_nibble {
        0x0 => {
            // MOVEA, etc.
            Ok(Instruction::Nop)
        }
        0x1..=0x3 => {
            // MOVE
            decode_move(opcode)
        }
        0x4 => {
            // Várias instruções
            if opcode == 0x4E71 {
                Ok(Instruction::Nop)
            } else {
                Ok(Instruction::Illegal(opcode))
            }
        }
        _ => Ok(Instruction::Illegal(opcode)),
    }
}

fn decode_move(opcode: u16) -> Result<Instruction, String> {
    let size = match (opcode >> 12) & 0x3 {
        0x1 => Size::Byte,
        0x2 => Size::Word,
        0x3 => Size::Long,
        _ => return Err("Invalid move size".to_string()),
    };

    // Simplificado para exemplo
    let dst_mode = AddressingMode::DataRegister((opcode & 0x7) as u8);
    let src_mode = AddressingMode::DataRegister(((opcode >> 9) & 0x7) as u8);

    Ok(Instruction::Move(src_mode, dst_mode, size))
}
