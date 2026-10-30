# Kagura CPU適合case TOML形式

## 1. fileの組

1つのcaseは`case.toml`とraw binary fileからなる。\
pathはcase TOMLがあるdirectoryを基準に解決する。

CPU case、Bus case、coverage fragmentのTOML表記は[TOML v1.0.0](https://toml.io/en/v1.0.0)に従う。\
文字encodingは同規格の[Spec](https://toml.io/en/v1.0.0#spec)に従うUTF-8、整数表記は[Integer](https://toml.io/en/v1.0.0#integer)、tableの表現は[Table](https://toml.io/en/v1.0.0#table)を適用する。\
runnerは未知のfield、重複field、型の不一致、各formatが定める範囲外の整数をerrorとして拒否しなければならない。

CPU caseとBus caseの整数fieldには、次の共通値域を適用する。\
表の`a..b`は両端を含む整数範囲を表し、各fieldの成立条件や指定可能な値は各節の規則も満たさなければならない。

| fieldの種類 | 値域 |
| --- | --- |
| address、PC、`faulting_pc`、register値、device stateの32-bit値、`case_id`、`detail`、`value` | `0..(2^32 - 1)` |
| `bytes`の各要素 | `0..255` |
| `max_steps` | `1..(2^63 - 1)` |
| `steps` | `1..max_steps` |
| `retired` | `0..(2^63 - 1)` |

これらの上限は適合試験caseの入力に適用する値域である。

## 2. 必須header

```toml
format = "kagura-conformance-case"
version = 1
id = "KAGURA-V1-ADD-001"
level = 2
binary = "program.bin"
spec = ["cpu.md#3-算術"]
```

| field | 規則 |
| --- | --- |
| `format` | literal値 `kagura-conformance-case` |
| `version` | 本仕様ではinteger `1` |
| `id` | suite内で一意なASCII identifier。空でなく、ASCIIの制御文字・空白を含めない |
| `level` | `1`、`2`、`3`のいずれか |
| `binary` | case directory内のregular fileへのrelative path |
| `spec` | このcaseが検査する規範節の1個以上のreference |

absolute path、case directory外へ解決されるpath、symbolic linkによるcase directory外への脱出を拒否する。

`spec` referenceは現行仕様のrootを基準とし、`cpu.md#...`または`bus.md#...`の形式で記述する。\
これはcaseと規範節の対応を示すmetadataである。\
caseの実行条件と比較対象は本書で定義するfieldから取得する。\
coverage manifestの`spec` referenceに対する文書とheading anchorの存在確認は、別途[coverage validation](./coverage-format.md#5-仕様参照)に従う。

## 3. binaryのload

CPU caseは論理的な`image` tableを必ず持つ。

```toml
[image]
load_address = 0x00000000
```

`load_address`は`0x00000000`または`0xFFFFFFFC`とする。\
省略時は`0x00000000`とする。

- `0x00000000`ではbinaryの全byteがTest RAM内に収まらなければならず、binary sizeは`0`でもよく、4の倍数である必要もない
- `0xFFFFFFFC`ではbinary sizeを正確に4 byteとし、High Test ROMへ配置する

## 4. 初期状態

`initial` tableは省略できる。

```toml
[initial]
pc = 0x00000000

[initial.registers]
r1 = 0x00000001
r2 = 0xFFFFFFFF

[[initial.memory]]
address = 0x00000100
bytes = [0x01, 0x02, 0x03, 0x04]
```

- `initial.pc`と`initial.registers`はLevel 2以上でだけ指定できる
- 未指定のPC/registerはreset値を維持する
- register名は`r0`から`r15`だけを認める
- `r0`には`0`だけを指定できる
- memory patchはLevel 1以上で指定できる
- patchはTest RAM内に収まり、互いに重複してはならない
- patchはbinary imageと重複してはならない
- `bytes`には[共通値域](#1-fileの組)を適用し、空配列を禁止する

## 5. 実行

CPU caseは論理的な`execution` tableを必ず持ち、停止条件を`until`で指定する。

```toml
[execution]
until = "steps"
steps = 1
max_steps = 1
```

`until`は次のいずれかとする。

| 値 | 停止条件 |
| --- | --- |
| `steps` | `steps`個の命令がretireした直後 |
| `reporter` | Reporter termination成立直後 |
| `fault` | CPU fault成立直後 |

規則:

- `max_steps`は必須とし、値域は[共通値域](#1-fileの組)に従う
- `steps`は`until = "steps"`の場合だけ必須とする
- `until = "reporter"`または`"fault"`の場合、`steps`を指定してはならない
- 指定したstop conditionより先に別のterminationが発生した場合、caseは失敗する
- `max_steps`回のstep試行後もstop conditionが成立しなければstep limit failureとする

## 6. 期待する終了状態

CPU caseは論理的な`expected.termination` tableを必ず持ち、`kind`で`steps`、`reporter`、`fault`のいずれかを指定する。\
`retired`はすべての`kind`で省略可能とし、指定した場合だけ終了時のretired countと比較する。\
値域は[共通値域](#1-fileの組)に従う。

### step数

```toml
[expected.termination]
kind = "steps"
retired = 1
```

### Reporter

```toml
[expected.termination]
kind = "reporter"
status = "pass"
case_id = 1
detail = 0
retired = 12
```

`status`は`pass`または`fail`とする。\
`case_id`と`detail`は省略でき、省略時は比較しない。

### fault

```toml
[expected.termination]
kind = "fault"
fault = "BUS_FAULT"
faulting_pc = 0x00000004
address = 0x00010000
width = 32
operation = "load"
retired = 1
```

`fault`は`INVALID_INSTRUCTION`、`UNALIGNED_ACCESS`、`UNALIGNED_PC`、`BUS_FAULT`のいずれかとする。\
metadata fieldは対応するKagura faultが保持する場合に必須とし、該当しないfieldを指定してはならない。

`operation`は`fetch`、`load`、`store`のいずれかとする。

## 7. 期待するアーキテクチャ状態

```toml
[expected.state]
pc = 0x00000004

[expected.state.registers]
r0 = 0
r3 = 3

[[expected.state.memory]]
address = 0x00000100
bytes = [0x03, 0x00, 0x00, 0x00]
```

- 記載した値だけを比較し、未記載のPC/register/memoryは比較しない
- `expected.state`は省略可能とし、Level 2以上でだけ使用できる
- memory rangeはTest RAM内に収まり、互いに重複してはならない
- fault caseのstateはfault処理が完了し、CPUがhostへ制御を返した直後に観測する

## 8. 期待するbus trace

Level 3 caseは順序付きbus traceを指定できる。

```toml
[[expected.bus]]
operation = "read"
address = 0x00000000
width = 32
result = "ok"
value = 0x00120000

[[expected.bus]]
operation = "write"
address = 0x00000100
width = 32
result = "ok"
value = 3
```

- `operation`は`read`または`write`とする
- `width`は`8`、`16`、`32`とする
- `result`は`ok`または`fault`とする
- 成功したreadとすべてのwriteには`value`を指定する
- faultしたreadに`value`を指定してはならない
- traceにはinstruction fetchを通常の32-bit `read`として含める
- setup、binary load、initial patch、結果観測はtraceに含めない
- `expected.bus`を記載した場合、entry数と各entryを順序も含めて完全一致させる

`expected.bus`の省略時はbus traceを比較しない。\
空配列を指定する場合は、`[expected]` tableに`bus = []`を記載し、bus accessが0件であることを比較する。

`value`の比較は[runnerのbus値比較](./runner-semantics.md#7-比較)に従う。

## 9. device状態

Level 3 caseは固定machine deviceの初期状態と期待snapshotを記述できる。

```toml
[[initial.devices]]
id = "reporter"
state = { status = 0, case_id = 0x11223344, detail = 0x55667788, reserved = 0 }

[[expected.devices]]
id = "reporter"
state = { status = 0, case_id = 0x11223344, detail = 0x55667788, reserved = 0 }
```

- `initial.devices`と`expected.devices`はLevel 3だけで使用できる
- device IDとstate fieldの完全性は[machineのdevice状態](./machine.md#6-device状態)に従う
- setupとsnapshotはhost操作であり、bus traceに含めない
- `expected.devices`を記載した場合、指定deviceの全state fieldを完全一致させる

## 10. 完全な例

```toml
format = "kagura-conformance-case"
version = 1
id = "KAGURA-V1-ADD-001"
level = 2
binary = "program.bin"
spec = ["cpu.md#3-算術"]

[image]
load_address = 0

[initial.registers]
r1 = 1
r2 = 2

[execution]
until = "steps"
steps = 1
max_steps = 1

[expected.termination]
kind = "steps"
retired = 1

[expected.state]
pc = 4

[expected.state.registers]
r0 = 0
r3 = 3
```
