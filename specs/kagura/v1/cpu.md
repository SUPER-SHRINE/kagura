# Kagura CPU

本書は Kagura の 32-bit CPU ISA を定義する。memory access は [Kagura Bus](./bus.md) に従う。

## 1. アーキテクチャ状態

- 32-bit little-endian、byte-addressed
- 32-bit 固定長命令
- 32-bit register `r0..r15`
- `r0` の read は常に `0`、write は破棄
- 独立した 32-bit `PC`
- `PC` は実行中命令の address を指し、逐次実行では `4` 増える

reset 時は `r0..r15 = 0`, `PC = 0` とする。

## 2. 命令encoding

```text
31         28 27        24 23        20 19        16 15               0
+------------+------------+------------+------------+------------------+
| op (4 bit) | rd (4 bit) | rs1 (4bit) | rs2 (4bit) | imm (16 bit)     |
+------------+------------+------------+------------+------------------+
```

| opcode | 命令 |
| --- | --- |
| `0000` | `ADD` |
| `0001` | `NAND` |
| `0010` | `SHIFT` |
| `0011` | `MUL` |
| `0100` | `LDB` |
| `0101` | `LDH` |
| `0111` | `LDW` |
| `1000` | `STB` |
| `1001` | `STH` |
| `1011` | `STW` |
| `1100` | `JZ` |
| `1101` | `CMP` |

表にない opcode は `INVALID_INSTRUCTION` とする。特記しない `imm` は `sext16(imm)` として扱う。

source register は命令開始時の state から読み、成功した命令の最後に destination と PC を commit する。operand register が重複してもよい。

## 3. 算術

```text
ADD  rd, rs1, rs2, imm   rd = (rs1 + rs2 + sext16(imm)) mod 2^32
NAND rd, rs1, rs2, imm   rd = ~(rs1 & rs2)
MUL  rd, rs1, rs2, imm   rd = low32(rs1 * rs2)
```

`NAND` と `MUL` は `imm` を無視する。overflow flag と overflow fault は持たない。

## 4. 比較

```text
CMP rd, rs1, rs2, imm
```

- `imm[1:0]` は基本比較種別を表す。
- `00`: `rs1 == rs2`
- `01`: signed less-than。`rs1` と `rs2` を two's complement `i32` として比較する。
- `10`: unsigned less-than。`rs1` と `rs2` を `u32` として比較する。
- `11`: `INVALID_INSTRUCTION`
- `imm[2]` は result の反転 bit とする。`0` はそのまま、`1` は論理否定する。
- `imm[15:3]` は `0` でなければならず、違反は `INVALID_INSTRUCTION` とする。
- `rd` には false なら `0`、true なら `1` を書く。

## 5. shift命令

```text
SHIFT rd, rs1, rs2, imm
amount = (rs2 + imm[4:0]) & 31
```

| `imm[15:14]` | 動作 |
| --- | --- |
| `00` | `rd = rs1 << amount` |
| `10` | 論理右shift |
| `11` | 算術右shift |
| `01` | `INVALID_INSTRUCTION` |

`imm[13:5]` は `0` でなければならず、違反は `INVALID_INSTRUCTION` とする。left shift は上位 bit を破棄し、logical right shift は `0`、arithmetic right shift は元の bit 31 で上位を埋める。

## 6. load/store命令

```text
address = (rs1 + rs2 + sext16(imm)) mod 2^32

LDB rd, rs1, rs2, imm    8 bitを読み込み、zero-extendする
LDH rd, rs1, rs2, imm    16 bitを読み込み、zero-extendする
LDW rd, rs1, rs2, imm    32 bitを読み込む
STB rd, rs1, rs2, imm    rdの下位8 bitを書き込む
STH rd, rs1, rs2, imm    rdの下位16 bitを書き込む
STW rd, rs1, rs2, imm    rdの32 bitを書き込む
```

16-bit access は 2-byte alignment、32-bit access は 4-byte alignment を要求する。違反は `UNALIGNED_ACCESS` とする。

## 7. jump命令

```text
JZ rd, rs1, rs2, imm
offset = sext16(imm) << 2
```

- `rs1 != 0` なら `PC = (PC + 4) mod 2^32` とし、`rd` を変更しない。
- `rs1 == 0` なら `rd = (PC + 4) mod 2^32` とする。
- `rs1 == 0` かつ `rs2 == r0` なら `PC = (PC + 4 + offset) mod 2^32` とする。
- `rs1 == 0` かつ `rs2 != r0` なら `PC = (rs2 + offset) mod 2^32` とする。
- indirect target が 4-byte aligned でなければ `UNALIGNED_PC` とする。

`rd == r0` の link write は破棄する。

## 8. 命令fetch

fetch は `PC` から 32 bit を 1 回の bus access で読む。`PC` が 4-byte aligned でなければ `UNALIGNED_PC`、bus が fault を返せば `BUS_FAULT` とする。

## 9. fault処理

| fault | 原因 |
| --- | --- |
| `INVALID_INSTRUCTION` | reserved opcode、`CMP` または `SHIFT` の不正な encoding |
| `UNALIGNED_ACCESS` | load / store の alignment 違反 |
| `UNALIGNED_PC` | fetch または indirect jump target の alignment 違反 |
| `BUS_FAULT` | fetch、load、store の bus fault |

faultした命令はregister、PC、bus deviceに副作用を残さず、CPUは停止してhostへ制御を返す。すべてのfault resultは`faulting_pc`を含む。

- `UNALIGNED_ACCESS`はaddress、access width、`load`または`store`のoperationを含む。
- `UNALIGNED_PC`はunalignedなPCまたはtarget addressを含む。
- `BUS_FAULT`はaddress、access width、`fetch`、`load`、`store`のoperationを含む。
