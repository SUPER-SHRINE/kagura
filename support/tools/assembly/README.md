# Kagura assembly tool構文

本書では付属assemblerが受理するassembly表記、pseudo instruction、labelを説明する。\
実命令のencodingと動作は[Kagura CPU仕様](../../../specs/kagura/v1/cpu.md)に従い、本書はassemblerの入力表記を定める。

この構文とassembler実装は非規範の開発ツールであり、Kagura適合実装にassembly sourceの解釈を要求しない。\
後方互換性はKagura VM仕様のversionとは独立して扱う。\
ABI、object format、link、executable formatは対象外とする。

## 1. 疑似命令

| 表記 | 展開結果 |
| --- | --- |
| `NOP` | `ADD r0, r0, r0, 0` |
| `LI rd, imm` | `ADD rd, r0, r0, imm` |
| `BZ rs, label` | `JZ r0, rs, r0, label` |
| `JMP label` | `JZ r0, r0, r0, label` |
| `CALL label` | `JZ r15, r0, r0, label` |
| `JR rs` | `JZ r0, r0, rs, 0` |
| `JR rs, imm` | `JZ r0, r0, rs, imm` |
| `CALLR rs` | `JZ r15, r0, rs, 0` |
| `CALLR rs, imm` | `JZ r15, r0, rs, imm` |
| `RET` | `JZ r0, r0, r15, 0` |

`LI`の`imm`はsigned 16-bitとする。\
`JR`と`CALLR`の`imm`は`JZ`と同じword offsetであり、省略値は`0`とする。

実命令は`ADD`、`NAND`、`SHIFT`、`MUL`、`LDB`、`LDH`、`LDW`、`STB`、`STH`、`STW`、`JZ`、`CMP`のraw formを受理する。\
`NAND`と`MUL`は`rd, rs1, rs2`の3 operandとし、CPU encodingで無視する`imm`の入力は省略する。\
その他の実命令は`rd, rs1, rs2, imm`の4 operandで指定する。

- `ADD`、load、store、`JZ`の数値`imm`はsigned 16-bit範囲の`-32768..32767`で指定する
- `SHIFT`と`CMP`の`imm`はunsigned 16-bit範囲の`0..65535`で指定し、16-bitのbit patternとしてencodingする

命令実行時に有効なencodingと、不正なencodingで発生するfaultは[CPU仕様](../../../specs/kagura/v1/cpu.md)に従う。

## 2. label解決

PC-relative label は次の値へ解決する。

```text
imm = (label_address - (instruction_address + 4)) / 4
```

- label addressと差分は4-byte alignedでなければならない
- `BZ`、`JMP`、`CALL`では、labelから計算した`imm`がsigned 16-bit range内でなければならない
- assembly完了時に未解決のlabelを残してはならない

sectionやdata directiveは実装していない。\
assemblerの出力は命令wordをlittle-endianで並べたflat binaryである。
