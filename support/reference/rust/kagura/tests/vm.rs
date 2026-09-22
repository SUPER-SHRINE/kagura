use kagura::{
    AccessOperation, AccessWidth, Cpu, DefaultBus, Fault, FaultCode, MapError, REGISTER_COUNT,
    RESET_PC,
};
use ram::Ram;

fn encode(op: u8, rd: u8, rs1: u8, rs2: u8, imm: u16) -> u32 {
    ((op as u32) << 28)
        | ((rd as u32) << 24)
        | ((rs1 as u32) << 20)
        | ((rs2 as u32) << 16)
        | imm as u32
}

fn test_bus(size: usize) -> DefaultBus {
    let mut bus = DefaultBus::new();
    bus.map_device(0, size as u32, Ram::new(size)).unwrap();
    bus
}

#[test]
fn new_cpu_starts_at_reset_state() {
    let cpu = Cpu::new();
    assert_eq!(cpu.pc(), RESET_PC);
    for i in 0..REGISTER_COUNT {
        assert_eq!(cpu.reg(i), 0);
    }
}

#[test]
fn reset_restores_pc_and_registers() {
    let mut cpu = Cpu::new();
    cpu.set_pc(0x1234);
    cpu.set_reg(1, 10);
    cpu.set_reg(15, 20);

    cpu.reset();

    assert_eq!(cpu.pc(), RESET_PC);
    for i in 0..REGISTER_COUNT {
        assert_eq!(cpu.reg(i), 0);
    }
}

#[test]
fn r0_ignores_direct_writes() {
    let mut cpu = Cpu::new();
    cpu.set_reg(0, 123);
    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn unaligned_pc_faults_before_fetch() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_pc(2);

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(
        fault,
        Fault::UnalignedPc {
            faulting_pc: 2,
            addr: 2
        }
    );
    assert_eq!(cpu, before);
}

#[test]
fn fetch_bus_fault_is_reported() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_pc(64);

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(
        fault,
        Fault::BusFault {
            faulting_pc: 64,
            addr: 64,
            width: AccessWidth::Word,
            operation: AccessOperation::Fetch,
        }
    );
    assert_eq!(fault.code(), FaultCode::BusFault);
    assert_eq!(cpu, before);
}

#[test]
fn instruction_fetch_is_little_endian() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    let word = encode(0x0, 1, 0, 0, 0x1234);
    bus.load8(0, (word & 0xff) as u8).unwrap();
    bus.load8(1, ((word >> 8) & 0xff) as u8).unwrap();
    bus.load8(2, ((word >> 16) & 0xff) as u8).unwrap();
    bus.load8(3, ((word >> 24) & 0xff) as u8).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(1), 0x1234);
}

#[test]
fn add_uses_sign_extended_immediate_and_wrapping_arithmetic() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, u32::MAX);
    bus.load32(0, encode(0x0, 2, 1, 0, 1)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 0);
    assert_eq!(cpu.pc(), 4);
}

#[test]
fn add_supports_overlapping_registers() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 5);
    bus.load32(0, encode(0x0, 1, 1, 1, 1)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(1), 11);
}

#[test]
fn add_write_to_r0_is_discarded() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 7);
    bus.load32(0, encode(0x0, 0, 1, 0, 3)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(0), 0);
}

#[test]
fn nand_ignores_immediate() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0xf0f0_f0f0);
    cpu.set_reg(2, 0xffff_0000);
    bus.load32(0, encode(0x1, 3, 1, 2, 0x1234)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(3), !(0xf0f0_f0f0 & 0xffff_0000));
}

#[test]
fn mul_wraps_low_32_bits() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0xFFFF_FFFF);
    cpu.set_reg(2, 2);
    bus.load32(0, encode(0x3, 3, 1, 2, 0x1234)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(3), 0xFFFF_FFFE);
}

#[test]
fn cmp_eq_ltu_and_invert_forms_write_boolean_result() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 10);
    cpu.set_reg(2, 20);
    cpu.set_reg(3, 10);
    bus.load32(0, encode(0xD, 4, 1, 3, 0b000)).unwrap();
    bus.load32(4, encode(0xD, 5, 1, 2, 0b010)).unwrap();
    bus.load32(8, encode(0xD, 6, 1, 2, 0b110)).unwrap();

    cpu.step(&mut bus).unwrap();
    cpu.step(&mut bus).unwrap();
    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(4), 1);
    assert_eq!(cpu.reg(5), 1);
    assert_eq!(cpu.reg(6), 0);
}

#[test]
fn cmp_reserved_encoding_faults_without_state_change() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 1);
    cpu.set_reg(2, 2);
    bus.load32(0, encode(0xD, 3, 1, 2, 0b011)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(fault, Fault::InvalidInstruction { faulting_pc: 0 });
    assert_eq!(fault.code(), FaultCode::InvalidInstruction);
    assert_eq!(cpu, before);
}

#[test]
fn shift_shl_uses_rs2_plus_imm_masked_to_five_bits() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x0000_0003);
    cpu.set_reg(2, 30);
    bus.load32(0, encode(0x2, 4, 1, 2, 3)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(4), 0x0000_0006);
}

#[test]
fn shift_shr_fills_with_zero() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x8000_0000);
    bus.load32(0, encode(0x2, 4, 1, 0, 0b10 << 14 | 1)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(4), 0x4000_0000);
}

#[test]
fn shift_sar_fills_with_sign_bit() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x8000_0000);
    bus.load32(0, encode(0x2, 4, 1, 0, 0b11 << 14 | 1)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(4), 0xC000_0000);
}

#[test]
fn shift_amount_zero_returns_original_value() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x1234_5678);
    bus.load32(0, encode(0x2, 4, 1, 0, 0b10 << 14)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(4), 0x1234_5678);
}

#[test]
fn shift_mode_01_faults_without_state_change() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x1234_5678);
    bus.load32(0, encode(0x2, 2, 1, 0, 0b01 << 14)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(fault, Fault::InvalidInstruction { faulting_pc: 0 });
    assert_eq!(fault.code(), FaultCode::InvalidInstruction);
    assert_eq!(cpu, before);
}

#[test]
fn shift_reserved_bits_fault_without_state_change() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x1234_5678);
    bus.load32(0, encode(0x2, 2, 1, 0, 0x0020)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(fault, Fault::InvalidInstruction { faulting_pc: 0 });
    assert_eq!(fault.code(), FaultCode::InvalidInstruction);
    assert_eq!(cpu, before);
}

#[test]
fn ldb_and_ldh_zero_extend() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(128);
    bus.load8(0x20, 0xFE).unwrap();
    bus.load16(0x22, 0x80FF).unwrap();
    cpu.set_reg(1, 0x20);
    bus.load32(0, encode(0x4, 2, 1, 0, 0)).unwrap();
    bus.load32(4, encode(0x5, 3, 1, 0, 2)).unwrap();

    cpu.step(&mut bus).unwrap();
    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 0x0000_00FE);
    assert_eq!(cpu.reg(3), 0x0000_80FF);
}

#[test]
fn stw_uses_rd_as_store_value() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(128);
    cpu.set_reg(1, 0x20);
    cpu.set_reg(3, 0xDEAD_BEEF);
    bus.load32(0, encode(0xB, 3, 1, 0, 0)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(bus.read32_at(0x20).unwrap(), 0xDEAD_BEEF);
}

#[test]
fn load_and_store_address_calculation_wraps() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(128);
    cpu.set_reg(1, u32::MAX);
    cpu.set_reg(2, 0x10);
    cpu.set_reg(3, 0xAB);
    bus.load32(0, encode(0x8, 3, 1, 2, 0xFFF1)).unwrap();
    bus.load32(4, encode(0x4, 4, 1, 2, 0xFFF1)).unwrap();

    cpu.step(&mut bus).unwrap();
    cpu.step(&mut bus).unwrap();

    assert_eq!(bus.read8_at(0).unwrap(), 0xAB);
    assert_eq!(cpu.reg(4), 0xAB);
}

#[test]
fn unaligned_half_and_word_access_fault_without_state_change() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 1);
    bus.load32(0, encode(0x5, 2, 1, 0, 0)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(
        fault,
        Fault::UnalignedAccess {
            faulting_pc: 0,
            addr: 1,
            width: AccessWidth::Half,
            operation: AccessOperation::Load,
        }
    );
    assert_eq!(fault.code(), FaultCode::UnalignedAccess);
    assert_eq!(cpu, before);

    bus.load32(0, encode(0x7, 2, 1, 0, 0)).unwrap();
    let fault = cpu.step(&mut bus).unwrap_err();
    assert_eq!(
        fault,
        Fault::UnalignedAccess {
            faulting_pc: 0,
            addr: 1,
            width: AccessWidth::Word,
            operation: AccessOperation::Load,
        }
    );
    assert_eq!(cpu, before);
}

#[test]
fn load_bus_fault_preserves_state() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x80);
    bus.load32(0, encode(0x4, 2, 1, 0, 0)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(
        fault,
        Fault::BusFault {
            faulting_pc: 0,
            addr: 0x80,
            width: AccessWidth::Byte,
            operation: AccessOperation::Load,
        }
    );
    assert_eq!(fault.code(), FaultCode::BusFault);
    assert_eq!(cpu, before);
}

#[test]
fn store_bus_fault_preserves_cpu_state() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0x80);
    cpu.set_reg(2, 0x55);
    bus.load32(0, encode(0x8, 2, 1, 0, 0)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(
        fault,
        Fault::BusFault {
            faulting_pc: 0,
            addr: 0x80,
            width: AccessWidth::Byte,
            operation: AccessOperation::Store,
        }
    );
    assert_eq!(fault.code(), FaultCode::BusFault);
    assert_eq!(cpu, before);
}

#[test]
fn jz_not_taken_only_advances_pc_and_keeps_rd() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 1);
    cpu.set_reg(2, 0xDEAD_BEEF);
    bus.load32(0, encode(0xC, 2, 1, 0, 3)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(2), 0xDEAD_BEEF);
    assert_eq!(cpu.pc(), 4);
}

#[test]
fn jz_taken_pc_relative_writes_link_and_uses_pc_plus_four_base() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    bus.load32(0, encode(0xC, 5, 0, 0, 2)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(5), 4);
    assert_eq!(cpu.pc(), 12);
}

#[test]
fn jz_taken_with_rd_r0_discards_link() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    bus.load32(0, encode(0xC, 0, 0, 0, 1)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(0), 0);
    assert_eq!(cpu.pc(), 8);
}

#[test]
fn jz_indirect_form_adds_scaled_offset() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(128);
    cpu.set_reg(3, 0x20);
    bus.load32(0, encode(0xC, 5, 0, 3, 2)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(5), 4);
    assert_eq!(cpu.pc(), 0x28);
}

#[test]
fn jz_condition_uses_old_register_value_when_rd_equals_rs1() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(1, 0);
    bus.load32(0, encode(0xC, 1, 1, 0, 1)).unwrap();

    cpu.step(&mut bus).unwrap();

    assert_eq!(cpu.reg(1), 4);
    assert_eq!(cpu.pc(), 8);
}

#[test]
fn jz_indirect_unaligned_faults_without_state_change() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    cpu.set_reg(3, 6);
    bus.load32(0, encode(0xC, 5, 0, 3, 0)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(
        fault,
        Fault::UnalignedPc {
            faulting_pc: 0,
            addr: 6
        }
    );
    assert_eq!(fault.code(), FaultCode::UnalignedPc);
    assert_eq!(cpu, before);
}

#[test]
fn reserved_opcode_faults() {
    let mut cpu = Cpu::new();
    let mut bus = test_bus(64);
    bus.load32(0, encode(0xF, 0, 0, 0, 0)).unwrap();

    let before = cpu.clone();
    let fault = cpu.step(&mut bus).unwrap_err();

    assert_eq!(fault, Fault::InvalidInstruction { faulting_pc: 0 });
    assert_eq!(fault.code(), FaultCode::InvalidInstruction);
    assert_eq!(cpu, before);
}

#[test]
fn default_bus_rejects_overlapping_mappings() {
    let mut bus = DefaultBus::new();
    bus.map_device(0, 16, Ram::new(16)).unwrap();
    let err = bus.map_device(8, 16, Ram::new(16)).unwrap_err();
    assert_eq!(err, MapError::Overlap);
}
