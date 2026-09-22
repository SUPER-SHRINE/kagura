use debug_io::DebugIo;
use kagura::{BusFault, Device};

#[test]
fn debug_io_rejects_multi_byte_accesses() {
    let mut io = DebugIo::new();

    assert_eq!(io.read16(0), Err(BusFault));
    assert_eq!(io.read32(0), Err(BusFault));
    assert_eq!(io.write16(0, 0x1234), Err(BusFault));
    assert_eq!(io.write32(0, 0x1234_5678), Err(BusFault));
}

#[test]
fn debug_io_input_is_consumed_on_read() {
    let mut io = DebugIo::new();
    io.push_input(b'A');
    io.push_input(b'B');

    assert_eq!(io.read8(DebugIo::DATA), Ok(b'A'));
    assert_eq!(io.read8(DebugIo::STATUS), Ok(0b11));
    assert_eq!(io.read8(DebugIo::DATA), Ok(b'B'));
    assert_eq!(io.read8(DebugIo::STATUS), Ok(0b10));
}
