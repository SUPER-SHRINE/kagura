use kagura::{BusFault, Device};
use test_reporter::{ReporterState, TestReporter};

#[test]
fn reporter_accepts_only_defined_aligned_word_writes() {
    let mut dev = TestReporter::new();
    dev.restore(ReporterState {
        status: 0,
        case_id: 0x1122_3344,
        detail: 0x5566_7788,
        reserved: 0,
    });

    assert_eq!(dev.write32(TestReporter::TEST_ID, 0xaabb_ccdd), Ok(()));
    assert_eq!(dev.test_id(), 0xaabb_ccdd);
    assert_eq!(dev.read32(TestReporter::TEST_ID), Err(BusFault));
    assert_eq!(dev.write8(TestReporter::STATUS, 1), Err(BusFault));
    assert_eq!(dev.write16(TestReporter::STATUS, 1), Err(BusFault));
    assert_eq!(dev.write32(TestReporter::RESERVED, 1), Err(BusFault));
    assert_eq!(
        dev.snapshot(),
        ReporterState {
            status: 0,
            case_id: 0xaabb_ccdd,
            detail: 0x5566_7788,
            reserved: 0,
        }
    );
}
