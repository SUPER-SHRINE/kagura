use crate::bus::{AccessWidth, Bus};

pub const RESET_PC: u32 = 0x0000_0000;
pub const REGISTER_COUNT: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultCode {
    InvalidInstruction,
    UnalignedAccess,
    UnalignedPc,
    BusFault,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessOperation {
    Fetch,
    Load,
    Store,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Fault {
    InvalidInstruction {
        faulting_pc: u32,
    },
    UnalignedAccess {
        faulting_pc: u32,
        addr: u32,
        width: AccessWidth,
        operation: AccessOperation,
    },
    UnalignedPc {
        faulting_pc: u32,
        addr: u32,
    },
    BusFault {
        faulting_pc: u32,
        addr: u32,
        width: AccessWidth,
        operation: AccessOperation,
    },
}

impl Fault {
    pub fn code(self) -> FaultCode {
        match self {
            Self::InvalidInstruction { .. } => FaultCode::InvalidInstruction,
            Self::UnalignedAccess { .. } => FaultCode::UnalignedAccess,
            Self::UnalignedPc { .. } => FaultCode::UnalignedPc,
            Self::BusFault { .. } => FaultCode::BusFault,
        }
    }

    pub fn faulting_pc(self) -> u32 {
        match self {
            Self::InvalidInstruction { faulting_pc }
            | Self::UnalignedAccess { faulting_pc, .. }
            | Self::UnalignedPc { faulting_pc, .. }
            | Self::BusFault { faulting_pc, .. } => faulting_pc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cpu {
    regs: [u32; REGISTER_COUNT],
    pc: u32,
}

impl Default for Cpu {
    fn default() -> Self {
        Self::new()
    }
}

impl Cpu {
    pub fn new() -> Self {
        Self {
            regs: [0; REGISTER_COUNT],
            pc: RESET_PC,
        }
    }

    pub fn reset(&mut self) {
        self.regs = [0; REGISTER_COUNT];
        self.pc = RESET_PC;
    }

    pub fn pc(&self) -> u32 {
        self.pc
    }

    pub fn reg(&self, index: usize) -> u32 {
        self.regs[index]
    }

    pub fn set_pc(&mut self, pc: u32) {
        self.pc = pc;
    }

    pub fn set_reg(&mut self, index: usize, value: u32) {
        if index != 0 {
            self.regs[index] = value;
        }
    }

    pub fn step<B: Bus>(&mut self, bus: &mut B) -> Result<(), Fault> {
        if self.pc & 3 != 0 {
            return Err(Fault::UnalignedPc {
                faulting_pc: self.pc,
                addr: self.pc,
            });
        }

        let current_pc = self.pc;
        let instruction = bus.read32(current_pc).map_err(|_| Fault::BusFault {
            faulting_pc: current_pc,
            addr: current_pc,
            width: AccessWidth::Word,
            operation: AccessOperation::Fetch,
        })?;

        let decoded = DecodedInstruction::decode(instruction);
        let old_regs = self.regs;
        let next_pc = current_pc.wrapping_add(4);

        let mut staged = StagedState {
            regs: old_regs,
            pc: next_pc,
        };

        match decoded.opcode {
            Opcode::Add => {
                let value = old_regs[decoded.rs1]
                    .wrapping_add(old_regs[decoded.rs2])
                    .wrapping_add(decoded.imm_sext());
                staged.write_reg(decoded.rd, value);
            }
            Opcode::Nand => {
                let value = !(old_regs[decoded.rs1] & old_regs[decoded.rs2]);
                staged.write_reg(decoded.rd, value);
            }
            Opcode::Mul => {
                let value = old_regs[decoded.rs1].wrapping_mul(old_regs[decoded.rs2]);
                staged.write_reg(decoded.rd, value);
            }
            Opcode::Shift => {
                let mode = ((decoded.imm_raw >> 14) & 0b11) as u8;
                let reserved = (decoded.imm_raw >> 5) & 0x01ff;
                if reserved != 0 || mode == 0b01 {
                    return Err(Fault::InvalidInstruction {
                        faulting_pc: current_pc,
                    });
                }

                let amount =
                    old_regs[decoded.rs2].wrapping_add((decoded.imm_raw & 0x1f) as u32) & 31;
                let src = old_regs[decoded.rs1];
                let value = match mode {
                    0b00 => src.wrapping_shl(amount),
                    0b10 => src.wrapping_shr(amount),
                    0b11 => ((src as i32) >> amount) as u32,
                    _ => unreachable!(),
                };
                staged.write_reg(decoded.rd, value);
            }
            Opcode::Cmp => {
                let mode = (decoded.imm_raw & 0b11) as u8;
                let invert = ((decoded.imm_raw >> 2) & 0b1) != 0;
                let reserved = decoded.imm_raw >> 3;
                if reserved != 0 || mode == 0b11 {
                    return Err(Fault::InvalidInstruction {
                        faulting_pc: current_pc,
                    });
                }

                let lhs = old_regs[decoded.rs1];
                let rhs = old_regs[decoded.rs2];
                let result = match mode {
                    0b00 => lhs == rhs,
                    0b01 => (lhs as i32) < (rhs as i32),
                    0b10 => lhs < rhs,
                    _ => unreachable!(),
                };
                let result = if invert { !result } else { result };
                staged.write_reg(decoded.rd, u32::from(result));
            }
            Opcode::Ldb => {
                let addr = compute_addr(
                    old_regs[decoded.rs1],
                    old_regs[decoded.rs2],
                    decoded.imm_sext(),
                );
                let value = bus.read8(addr).map_err(|_| Fault::BusFault {
                    faulting_pc: current_pc,
                    addr,
                    width: AccessWidth::Byte,
                    operation: AccessOperation::Load,
                })?;
                staged.write_reg(decoded.rd, value as u32);
            }
            Opcode::Ldh => {
                let addr = compute_addr(
                    old_regs[decoded.rs1],
                    old_regs[decoded.rs2],
                    decoded.imm_sext(),
                );
                ensure_alignment(current_pc, addr, AccessWidth::Half, AccessOperation::Load)?;
                let value = bus.read16(addr).map_err(|_| Fault::BusFault {
                    faulting_pc: current_pc,
                    addr,
                    width: AccessWidth::Half,
                    operation: AccessOperation::Load,
                })?;
                staged.write_reg(decoded.rd, value as u32);
            }
            Opcode::Ldw => {
                let addr = compute_addr(
                    old_regs[decoded.rs1],
                    old_regs[decoded.rs2],
                    decoded.imm_sext(),
                );
                ensure_alignment(current_pc, addr, AccessWidth::Word, AccessOperation::Load)?;
                let value = bus.read32(addr).map_err(|_| Fault::BusFault {
                    faulting_pc: current_pc,
                    addr,
                    width: AccessWidth::Word,
                    operation: AccessOperation::Load,
                })?;
                staged.write_reg(decoded.rd, value);
            }
            Opcode::Stb => {
                let addr = compute_addr(
                    old_regs[decoded.rs1],
                    old_regs[decoded.rs2],
                    decoded.imm_sext(),
                );
                let value = old_regs[decoded.rd] as u8;
                bus.write8(addr, value).map_err(|_| Fault::BusFault {
                    faulting_pc: current_pc,
                    addr,
                    width: AccessWidth::Byte,
                    operation: AccessOperation::Store,
                })?;
            }
            Opcode::Sth => {
                let addr = compute_addr(
                    old_regs[decoded.rs1],
                    old_regs[decoded.rs2],
                    decoded.imm_sext(),
                );
                ensure_alignment(current_pc, addr, AccessWidth::Half, AccessOperation::Store)?;
                let value = old_regs[decoded.rd] as u16;
                bus.write16(addr, value).map_err(|_| Fault::BusFault {
                    faulting_pc: current_pc,
                    addr,
                    width: AccessWidth::Half,
                    operation: AccessOperation::Store,
                })?;
            }
            Opcode::Stw => {
                let addr = compute_addr(
                    old_regs[decoded.rs1],
                    old_regs[decoded.rs2],
                    decoded.imm_sext(),
                );
                ensure_alignment(current_pc, addr, AccessWidth::Word, AccessOperation::Store)?;
                let value = old_regs[decoded.rd];
                bus.write32(addr, value).map_err(|_| Fault::BusFault {
                    faulting_pc: current_pc,
                    addr,
                    width: AccessWidth::Word,
                    operation: AccessOperation::Store,
                })?;
            }
            Opcode::Jz => {
                if old_regs[decoded.rs1] == 0 {
                    staged.write_reg(decoded.rd, next_pc);
                    let offset = decoded.imm_sext().wrapping_shl(2);
                    if decoded.rs2 == 0 {
                        staged.pc = next_pc.wrapping_add(offset);
                    } else {
                        let target = old_regs[decoded.rs2].wrapping_add(offset);
                        if target & 3 != 0 {
                            return Err(Fault::UnalignedPc {
                                faulting_pc: current_pc,
                                addr: target,
                            });
                        }
                        staged.pc = target;
                    }
                }
            }
            Opcode::Reserved => {
                return Err(Fault::InvalidInstruction {
                    faulting_pc: current_pc,
                });
            }
        }

        self.regs = staged.regs;
        self.regs[0] = 0;
        self.pc = staged.pc;
        Ok(())
    }
}

fn compute_addr(rs1: u32, rs2: u32, imm: u32) -> u32 {
    rs1.wrapping_add(rs2).wrapping_add(imm)
}

fn ensure_alignment(
    faulting_pc: u32,
    addr: u32,
    width: AccessWidth,
    operation: AccessOperation,
) -> Result<(), Fault> {
    let mask = match width {
        AccessWidth::Byte => 0,
        AccessWidth::Half => 1,
        AccessWidth::Word => 3,
    };
    if addr & mask != 0 {
        Err(Fault::UnalignedAccess {
            faulting_pc,
            addr,
            width,
            operation,
        })
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct StagedState {
    regs: [u32; REGISTER_COUNT],
    pc: u32,
}

impl StagedState {
    fn write_reg(&mut self, index: usize, value: u32) {
        if index != 0 {
            self.regs[index] = value;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Opcode {
    Add,
    Nand,
    Shift,
    Mul,
    Ldb,
    Ldh,
    Ldw,
    Stb,
    Sth,
    Stw,
    Jz,
    Cmp,
    Reserved,
}

#[derive(Debug, Clone, Copy)]
struct DecodedInstruction {
    opcode: Opcode,
    rd: usize,
    rs1: usize,
    rs2: usize,
    imm_raw: u16,
}

impl DecodedInstruction {
    fn decode(word: u32) -> Self {
        let opcode = match ((word >> 28) & 0x0f) as u8 {
            0x0 => Opcode::Add,
            0x1 => Opcode::Nand,
            0x2 => Opcode::Shift,
            0x3 => Opcode::Mul,
            0x4 => Opcode::Ldb,
            0x5 => Opcode::Ldh,
            0x7 => Opcode::Ldw,
            0x8 => Opcode::Stb,
            0x9 => Opcode::Sth,
            0xB => Opcode::Stw,
            0xC => Opcode::Jz,
            0xD => Opcode::Cmp,
            _ => Opcode::Reserved,
        };

        Self {
            opcode,
            rd: ((word >> 24) & 0x0f) as usize,
            rs1: ((word >> 20) & 0x0f) as usize,
            rs2: ((word >> 16) & 0x0f) as usize,
            imm_raw: (word & 0xffff) as u16,
        }
    }

    fn imm_sext(self) -> u32 {
        (self.imm_raw as i16 as i32) as u32
    }
}
