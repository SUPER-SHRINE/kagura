#![forbid(unsafe_code)]

//! Four-byte read-only device at the top of the conformance address space.

use kagura::{BusFault, Device};
use std::any::Any;

#[derive(Debug, Clone, Copy, Default)]
pub struct HighTestRom {
    word: u32,
}

impl HighTestRom {
    pub fn word(&self) -> u32 {
        self.word
    }

    /// Host-only setup operation. It is not a bus transaction.
    pub fn set_word(&mut self, word: u32) {
        self.word = word;
    }
}

impl Device for HighTestRom {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn read8(&mut self, _addr: u32) -> Result<u8, BusFault> {
        Err(BusFault)
    }
    fn read16(&mut self, _addr: u32) -> Result<u16, BusFault> {
        Err(BusFault)
    }
    fn read32(&mut self, addr: u32) -> Result<u32, BusFault> {
        (addr == 0).then_some(self.word).ok_or(BusFault)
    }
    fn write8(&mut self, _addr: u32, _value: u8) -> Result<(), BusFault> {
        Err(BusFault)
    }
    fn write16(&mut self, _addr: u32, _value: u16) -> Result<(), BusFault> {
        Err(BusFault)
    }
    fn write32(&mut self, _addr: u32, _value: u32) -> Result<(), BusFault> {
        Err(BusFault)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_word_read_at_offset_zero_succeeds() {
        let mut rom = HighTestRom::default();
        rom.set_word(0x1122_3344);
        assert_eq!(rom.read32(0), Ok(0x1122_3344));
        assert_eq!(rom.read8(0), Err(BusFault));
        assert_eq!(rom.read16(0), Err(BusFault));
        assert_eq!(rom.read32(1), Err(BusFault));
        assert_eq!(rom.write32(0, 0), Err(BusFault));
    }
}
