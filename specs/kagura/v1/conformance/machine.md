# Kagura適合試験machine

## 1. 目的

Conformance Test MachineはKagura仕様の一部ではなく、test binaryへ再現可能なbus environmentを与えるための固定profileである。

## 2. address map

| address範囲 | device |
| --- | --- |
| `0x00000000..0x0000FFFF` | Test RAM、64 KiB |
| `0xFFFFF000..0xFFFFF00F` | Test Reporter |
| `0xFFFFFFFC..0xFFFFFFFF` | High Test ROM、4 byte |
| その他 | unmapped |

unmapped addressへのread/writeはbus faultを返す。

## 3. Test RAM

- Test RAMは64 KiBのbyte-addressed memoryとする。
- 8-bit、16-bit、32-bitのread/writeを受理する。
- multi-byte valueはlittle-endianとする。
- range外へ一部でもはみ出すaccessはfaultし、memoryを変更しない。
- setup完了時に指定されていないbyteは`0`とする。
- hostによるbinary loadとinitial memory patchはCPU実行ではなく、bus traceへ記録しない。

CPUが要求するalignmentは[CPU ISA](../cpu.md)が検査する。Test RAM自体はrange内のunaligned accessを受理してもよいが、適合CPUからそのaccessが到達してはならない。

## 4. High Test ROM

High Test ROMは最上位addressでのPC/link wrapを検査するための4-byte read-only deviceである。

- host setupで正確に4 byteを配置する。
- `0xFFFFFFFC`へのaligned 32-bit readだけを受理する。
- 8-bit/16-bit read、全write、その他のoffsetへのaccessはfaultする。
- busにはfetchとdata readの区別がないため、`0xFFFFFFFC`への任意の32-bit readを同じように受理する。
- setupとinspectionはCPU bus traceへ含めない。

## 5. Test Reporter

Test Reporterはguest-driven caseの終了を通知するためのwrite-only deviceである。

| address | register | 意味 |
| --- | --- | --- |
| `0xFFFFF000` | `STATUS` | `1 = PASS`、`2 = FAIL` |
| `0xFFFFF004` | `CASE_ID` | caseが任意に使用する32-bit ID |
| `0xFFFFF008` | `DETAIL` | caseが任意に使用する32-bit detail |
| `0xFFFFF00C` | `RESERVED` | 予約領域 |

Reporterの規則:

- reset時の全register値は`0`とする。
- aligned 32-bit writeだけを受理する。
- read、8-bit write、16-bit write、unaligned write、`RESERVED`へのwriteはfaultする。
- `CASE_ID`と`DETAIL`への成功writeは値を保存する。
- `STATUS`への成功writeは値を保存し、その命令がcommitした後にReporter terminationを成立させる。
- `STATUS`の値`1`はPASS、`2`はFAILとする。その他の値によるterminationはinvalid reportとしてcaseを失敗させる。
- faultしたReporter accessはregister値を変更しない。
- runnerはsetup時にReporterの各registerを任意の32-bit値へ設定し、termination後に副作用なしでsnapshotできなければならない。このhost操作はbus transactionではない。

Reporterを使うこと自体はKagura適合条件ではない。Reporter accessに使用した命令も通常のKagura命令として実行されるため、その命令が不正ならcaseは成功しない。

## 6. device状態

Case TOMLのdevice snapshotで使用するdevice IDは次のとおりとする。

| ID | state field |
| --- | --- |
| `reporter` | `status`, `case_id`, `detail`, `reserved` |
| `high-rom` | `word` |

runnerは未知のdevice IDまたはstate fieldをsuite errorとして拒否する。device snapshotのsetupとinspectionはbus traceへ含めない。

## 7. CPU起動時状態

runnerはTest RAMとReporterを初期化した後、CPUをresetする。Level 1 caseは次の状態から開始する。

- `r0..r15 = 0`
- `PC = 0`

Level 2またはLevel 3 caseは、reset後にcase TOMLが指定したPCとregisterを注入してよい。`r0`へ`0`以外を指定してはならない。

## 8. 実行境界

1 stepは、現在のPCにある命令をfetchし、成功ならその命令のstate transitionをcommitする1回の試行である。

- 成功した命令だけをretired instructionとして数える。
- faultした命令はretired countへ含めない。
- Reporter terminationはReporter write命令のcommit後に成立し、その命令はretired countへ含める。
- setupと結果観測はretired countおよびbus traceへ含めない。
