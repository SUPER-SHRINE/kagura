# Kagura assembly tool構文

本書は付属assemblerが受理するassembly表記、pseudo instruction、labelを説明する。
実命令のoperandと動作はKagura CPU仕様に従う。

この構文とassembler実装は非規範の開発ツールであり、Kagura適合実装にassembly sourceの解釈を
要求しない。後方互換性はKagura VM仕様のversionとは独立して扱う。ABI、object format、link、
executable formatは対象外とする。

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

`LI` の `imm` は signed 16-bit とする。`JR` と `CALLR` の `imm` は `JZ` と同じ word offset で、省略値は `0` とする。

実命令は `ADD`、`NAND`、`SHIFT`、`MUL`、`LDB`、`LDH`、`LDW`、`STB`、`STH`、
`STW`、`JZ`、`CMP` のraw formを受理する。`imm` encodingはCPU仕様の値をそのまま指定する。

## 2. label解決

PC-relative label は次の値へ解決する。

```text
imm = (label_address - (instruction_address + 4)) / 4
```

- label address と差分は 4-byte aligned でなければならない。
- `BZ`、`JMP`、`CALL` target は signed 16-bit range 内でなければならない。
- assembly 完了時に未解決の label を残してはならない。

sectionやdata directiveは実装していない。assemblerの出力は命令wordをlittle-endianで並べた
flat binaryである。
