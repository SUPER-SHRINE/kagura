# Kagura Bus

本書は [Kagura CPU](./cpu.md) が使用する bus access を定義する。address map は machine が定義する。

## アクセス

- address は 32-bit byte address とする。
- 8-bit、16-bit、32-bitの read と write を提供する。
- multi-byte value は little-endian とする。
- 未割り当て address または device が受理しない access は fault を返す。
- 各 access は分割できない 1 回の操作とする。16-bit / 32-bit access を複数の byte access として扱わない。
- 成功した write は全体を commit する。
- fault した read / write は device に副作用を残さない。

CPU は bus fault を `BUS_FAULT` として扱う。
