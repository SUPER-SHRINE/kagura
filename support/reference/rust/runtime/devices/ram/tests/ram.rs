use kagura::{BusFault, Device};
use ram::Ram;

#[test]
fn ram_write32_out_of_range_has_no_partial_effect() {
    let mut ram = Ram::new(4);
    ram.write32(0, 0x1122_3344).unwrap();

    assert_eq!(ram.write32(1, 0x5566_7788), Err(BusFault));
    assert_eq!(ram.read32(0), Ok(0x1122_3344));
}
