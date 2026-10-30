# Kagura CPU

本書ではKaguraの32-bit CPU ISAを定義する。\
memory accessは[Kagura Bus](./bus.md)に従う。

## 1. アーキテクチャ状態

- 32-bit little-endian方式、byte address方式
- 32-bit固定長命令
- 32-bit register `r0..r15`
- `r0`の読み込み値は常に`0`であり、書き込みは破棄する
- 独立した32-bit `PC`
- `PC`は実行中命令のaddressを指し、逐次実行では`(PC + 4) mod 2^32`へ進む

reset時は`r0..r15 = 0`, `PC = 0`とする。

## 2. 命令encoding

```text
31         28 27        24 23        20 19        16 15               0
+------------+------------+------------+------------+------------------+
| op (4 bit) | rd (4 bit) | rs1 (4bit) | rs2 (4bit) | imm (16 bit)     |
+------------+------------+------------+------------+------------------+
```

bit番号は最下位bitを`0`とし、`[h:l]`はbit `h`からbit `l`までを含む範囲を表す。\
`rd`、`rs1`、`rs2`の各4-bit fieldは、`0..15`をregister `r0..r15`へ対応付けるselectorである。

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

表にないopcodeは`INVALID_INSTRUCTION`とする。\
特記しない`imm`は`sext16(imm)`として扱う。

source registerは命令開始時のstateから読み、成功した命令の最後にdestinationとPCをcommitする。\
operand registerは重複してもよい。

以下の式における`rs1`と`rs2`は、各selectorが選ぶregisterの命令開始時の値を表す。\
代入先の`rd`はselectorが選ぶregisterであり、storeで値として使う`rd`は命令開始時の値とする。

`sext16(x)`は16-bit値`x`を2の補数表現の符号付き整数として解釈した値であり、`x < 2^15`なら`x`、それ以外なら`x - 2^16`となる。\
`low32(x)`は`x mod 2^32`とする。

## 3. 算術

```text
ADD  rd, rs1, rs2, imm   rd = (rs1 + rs2 + sext16(imm)) mod 2^32
NAND rd, rs1, rs2, imm   rd = ~(rs1 & rs2)
MUL  rd, rs1, rs2, imm   rd = low32(rs1 * rs2)
```

`NAND`と`MUL`は`imm`を無視する。\
overflow flagとoverflow faultは持たない。

## 4. 比較

```text
CMP rd, rs1, rs2, imm
```

- `imm[1:0]`は基本比較種別を表す
- `00`: `rs1 == rs2`
- `01`: signed less-thanであり、`rs1`と`rs2`を2の補数表現の`i32`として比較する
- `10`: unsigned less-thanであり、`rs1`と`rs2`を`u32`として比較する
- `11`: `INVALID_INSTRUCTION`
- `imm[2]`はresultを反転するbitとし、`0`ならそのまま、`1`なら論理否定する
- `imm[15:3]`は`0`でなければならず、違反は`INVALID_INSTRUCTION`とする
- `rd`にはfalseなら`0`、trueなら`1`を書く

## 5. shift命令

```text
SHIFT rd, rs1, rs2, imm
amount = (rs2 + imm[4:0]) mod 32
signed_source = rs1 - 2^32 * floor(rs1 / 2^31)
```

| `imm[15:14]` | 動作 |
| --- | --- |
| `00` | `rd = low32(rs1 * 2^amount)` |
| `10` | `rd = floor(rs1 / 2^amount)` |
| `11` | `rd = low32(floor(signed_source / 2^amount))` |
| `01` | `INVALID_INSTRUCTION` |

`floor(x)`は`x`以下の最大の整数とする。\
`imm[13:5]`は`0`でなければならず、違反は`INVALID_INSTRUCTION`とする。

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

次のalignment条件を満たす場合、各loadは指定幅のreadを1回、各storeは指定幅のwriteを1回busへ要求する。\
loadのdestinationが`r0`の場合もreadを実行し、読み込んだ値のregisterへの書き込みを破棄する。

8-bit accessはすべてのbyte addressでalignment条件を満たす。\
16-bit accessはaddressが2の倍数、32-bit accessは4の倍数であることをdata bus要求の成立条件とする。\
alignment違反ではdata bus要求を行わず、`UNALIGNED_ACCESS`とする。

## 7. jump命令

```text
JZ rd, rs1, rs2, imm
offset = sext16(imm) << 2
```

- `rs1 != 0`なら`PC = (PC + 4) mod 2^32`とし、`rd`を変更せず、jump targetのalignment検査を行わない
- `rs1 == 0`なら`rd = (PC + 4) mod 2^32`とする
- `rs1 == 0`かつ`rs2 == r0`なら`PC = (PC + 4 + offset) mod 2^32`とする
- `rs1 == 0`かつ`rs2 != r0`なら`PC = (rs2 + offset) mod 2^32`とする
- indirect targetが4-byte alignedでなければ`UNALIGNED_PC`とする

`rd == r0`のlink writeは破棄する。

本節の`rs1`と`rs2`の数値計算は[命令encoding](#2-命令encoding)で定義する旧値を使う。\
`rs2 == r0`と`rs2 != r0`はregister値ではなく`rs2` fieldが選ぶregisterの比較であり、field値が`0`の場合だけPC-relative形式を選ぶ。\
`rd == r0`もdestinationのselectorを比較する。

## 8. 命令fetch

fetchのbus要求は`PC`が4の倍数である場合に成立し、`PC`から32 bitを1回のbus accessで読む。\
alignment違反ではbus要求を行わず`UNALIGNED_PC`とし、busがfaultを返せば`BUS_FAULT`とする。\
命令encodingの解釈と各命令の実行は、fetchが成功して命令wordを得た場合に行う。

## 9. fault処理

| fault | 原因 |
| --- | --- |
| `INVALID_INSTRUCTION` | reserved opcode、`CMP` または `SHIFT` の不正な encoding |
| `UNALIGNED_ACCESS` | load / store の alignment 違反 |
| `UNALIGNED_PC` | fetch または indirect jump target の alignment 違反 |
| `BUS_FAULT` | fetch、load、store の bus fault |

faultした命令はregister、PC、bus deviceに副作用を残さない。\
CPUは停止してhostへ制御を返し、すべてのfault resultに`faulting_pc`を含める。

`faulting_pc`は、faultした命令の実行試行を開始した時点の`PC`である。

- `UNALIGNED_ACCESS`はaddress、access width、`load`または`store`のoperationを含む
- `UNALIGNED_PC`はunalignedなPCまたはtarget addressを含む
- `BUS_FAULT`はaddress、access width、`fetch`、`load`、`store`のoperationを含む
