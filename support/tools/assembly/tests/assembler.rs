use kagura::{Cpu, DefaultBus};
use kagura_assembly::{AssemblerError, Program, assemble};
use ram::Ram;

fn bus_with_ram(size: usize) -> DefaultBus {
    let mut bus = DefaultBus::new();
    bus.map_device(0, size as u32, Ram::new(size)).unwrap();
    bus
}

fn load_program(bus: &mut DefaultBus, program: &Program) {
    for (i, word) in program.words().iter().copied().enumerate() {
        bus.load32((i as u32) * 4, word).unwrap();
    }
}

#[test]
fn assembles_real_instruction() {
    let program = assemble("ADD r1, r2, r3, -1").unwrap();
    assert_eq!(program.words(), &[0x0123_FFFF]);
}

#[test]
fn assembles_minimal_pseudo_set() {
    let source = r#"
        NOP
        LI r1, 42
        JR r2
        JR r3, -4
        CALLR r4
        CALLR r5, 8
        RET
    "#;

    let program = assemble(source).unwrap();
    assert_eq!(
        program.words(),
        &[
            0x0000_0000,
            0x0100_002A,
            0xC000_0000 | (2 << 16),
            0xC000_0000 | (3 << 16) | 0xFFFC,
            0xCF04_0000,
            0xCF05_0008,
            0xC00F_0000,
        ]
    );
}

#[test]
fn resolves_forward_and_backward_labels() {
    let source = r#"
start:
    BZ r1, done
    JMP start
done:
    CALL done
    RET
    "#;

    let program = assemble(source).unwrap();
    assert_eq!(
        program.words(),
        &[0xC010_0001, 0xC000_FFFE, 0xCF00_FFFF, 0xC00F_0000,]
    );
}

#[test]
fn duplicate_label_is_error() {
    let err = assemble("x:\nNOP\nx:\nNOP").unwrap_err();
    assert!(matches!(err, AssemblerError::DuplicateLabel { .. }));
}

#[test]
fn unknown_label_is_error() {
    let err = assemble("JMP missing").unwrap_err();
    assert!(matches!(err, AssemblerError::UnknownLabel { .. }));
}

#[test]
fn out_of_range_immediate_is_error() {
    let err = assemble("LI r1, 40000").unwrap_err();
    assert!(matches!(err, AssemblerError::ImmediateOutOfRange { .. }));
}

#[test]
fn assembled_program_runs_on_vm() {
    let source = r#"
        LI r1, 40
        LI r2, 2
        ADD r3, r1, r2, 0
        LI r4, 128
        STW r3, r4, r0, 0
        LDW r5, r4, r0, 0
    "#;

    let program = assemble(source).unwrap();
    let mut cpu = Cpu::new();
    let mut bus = bus_with_ram(256);
    load_program(&mut bus, &program);

    for _ in 0..program.words().len() {
        cpu.step(&mut bus).unwrap();
    }

    assert_eq!(cpu.reg(3), 42);
    assert_eq!(cpu.reg(5), 42);
    assert_eq!(bus.read32_at(128).unwrap(), 42);
}
