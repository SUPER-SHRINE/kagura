//! bus に接続される device の共通インターフェース。
//!
//! Kagura では CPU は RAM や MMIO の具体型を直接知らず、すべて bus 越しにアクセスする。
//! そのため bus 実装側が複数種類の device を同じ方法で保持・呼び出しできるよう、
//! `Device` trait を切っている。
//!
//! これにより、
//! - `DefaultBus` が RAM / debug I/O / test reporter などを同じ配線機構で扱える
//! - host が独自 device を差し込める
//! - CPU 本体を device 実装から分離できる
//!
//! という利点がある。
//!
//! `as_any` / `as_any_mut` は、主に host やテストコードが device を具体型へ downcast して
//! 状態を観察・回収するために使う。

use crate::bus::BusFault;
use core::any::Any;

pub trait Device {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn read8(&mut self, addr: u32) -> Result<u8, BusFault>;
    fn read16(&mut self, addr: u32) -> Result<u16, BusFault>;
    fn read32(&mut self, addr: u32) -> Result<u32, BusFault>;

    fn write8(&mut self, addr: u32, value: u8) -> Result<(), BusFault>;
    fn write16(&mut self, addr: u32, value: u16) -> Result<(), BusFault>;
    fn write32(&mut self, addr: u32, value: u32) -> Result<(), BusFault>;
}
