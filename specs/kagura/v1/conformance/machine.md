# Kagura適合試験machine

## 1. 目的

Conformance Test Machineは、適合試験におけるaddress mapとdeviceの動作を定義し、test binaryに再現可能なbus環境を提供する固定profileである。\
CPUとbusの動作は[CPU ISA](../cpu.md)と[Bus契約](../bus.md)に従い、本profileはrunner adapterが構成する試験環境に適用する。

## 2. address map

| address範囲 | device |
| --- | --- |
| `0x00000000..0x0000FFFF` | Test RAM、64 KiB |
| `0xFFFFF000..0xFFFFF00F` | Test Reporter |
| `0xFFFFFFFC..0xFFFFFFFF` | High Test ROM、4 byte |
| その他 | unmapped |

unmapped addressへの読み込みまたは書き込みはbus faultを返す。

## 3. Test RAM

- Test RAMは64 KiBのbyte-addressed memoryとする
- accessの全byteがrange内に収まる8-bit、16-bit、32-bitの読み込みと書き込みを、開始addressのalignmentによらず受理する
- 複数byteの値の表現は[Bus契約](../bus.md#アクセス)に従う
- range外へ一部でもはみ出すaccessはfaultし、memoryを変更しない
- setup完了時に指定されていないbyteは`0`とする
- hostによるbinary loadとinitial memory patchはCPU実行ではなく、bus traceにも記録しない

CPUが要求するalignmentは[CPU ISA](../cpu.md)が検査する。

## 4. High Test ROM

High Test ROMは、最上位addressにおけるPC/link wrapを検査する4-byte read-only deviceである。

- reset時の`word`は`0`とする
- host setupで正確に4 byteを配置する
- `0xFFFFFFFC`へのaligned 32-bit readだけを受理する
- 8-bitまたは16-bit read、すべてのwrite、その他のoffsetへのaccessはfaultする
- busにはfetchとdata readの区別がないため、`0xFFFFFFFC`への任意の32-bit readを同様に受理する
- setupとinspectionはCPU bus traceに含めない

device snapshotの`word`は、この4 byteを[Bus契約のlittle-endian表現](../bus.md#アクセス)で解釈した32-bit値である。\
CPU caseのbinary配置と`initial.devices`の適用順序は[runnerのsetup順序](./runner-semantics.md#4-setup順序)に従う。

## 5. Test Reporter

Test Reporterはguest-driven caseの終了を通知するwrite-only deviceである。

| address | register | 意味 |
| --- | --- | --- |
| `0xFFFFF000` | `STATUS` | `1 = PASS`、`2 = FAIL` |
| `0xFFFFF004` | `CASE_ID` | caseが任意に使用する32-bit ID |
| `0xFFFFF008` | `DETAIL` | caseが任意に使用する32-bit detail |
| `0xFFFFF00C` | `RESERVED` | 予約領域 |

Reporterの規則:

- reset時の全register値を`0`とする
- aligned 32-bit writeだけを受理する
- read、8-bit write、16-bit write、unaligned write、`RESERVED`へのwriteはfaultする
- `CASE_ID`と`DETAIL`への成功writeでは値を保存する
- `STATUS`への成功writeでは値を保存し、その命令のcommit後にReporter terminationを成立させる
- `STATUS`の値`1`はPASS、`2`はFAILとし、それ以外の値によるterminationはinvalid reportとしてcaseを失敗させる
- faultしたReporter accessではregister値を変更しない
- runnerはsetup時にReporterの各registerを任意の32-bit値に設定し、termination後に副作用なしでsnapshotできなければならず、このhost操作はbus transactionではない

Reporterの使用自体はKagura適合条件ではない。\
Reporter accessに使う命令も通常のKagura命令として実行されるため、その命令が不正ならcaseは成功しない。

## 6. device状態

Case TOMLのdevice snapshotで使用するdevice IDを次に示す。

| ID | state field |
| --- | --- |
| `reporter` | `status`, `case_id`, `detail`, `reserved` |
| `high-rom` | `word` |

CPU caseとBus caseの`initial.devices`および`expected.devices`の各entryは、指定deviceの全state fieldを正確に1回ずつ含める。\
device IDは各list内で一意とし、runnerは未知のdevice ID、未知・欠落・重複したstate field、重複device IDをsuite errorとして拒否する。\
device snapshotのsetupとinspectionはbus traceに含めない。

## 7. CPU起動時状態

CPU resetは[CPU ISAのアーキテクチャ状態](../cpu.md#1-アーキテクチャ状態)に従う。\
resetと初期値注入の順序は[runnerのsetup順序](./runner-semantics.md#4-setup順序)、注入できるfieldとlevelは[CPU caseの初期状態](./case-format.md#4-初期状態)で定義する。

## 8. 実行境界

1 stepは、現在のPCにある命令をfetchし、成功した場合にその命令のstate transitionをcommitする1回の試行である。

retired countとterminationの判定順序は[runnerのstep実行loop](./runner-semantics.md#5-step実行loop)に従う。\
setupと結果観測は命令の実行試行ではなく、retired countにもbus traceにも含めない。
