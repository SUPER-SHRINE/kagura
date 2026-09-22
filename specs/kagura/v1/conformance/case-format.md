# Kagura CPU適合case TOML形式

## 1. fileの組

1 caseは`case.toml`とraw binary fileからなる。pathはcase TOMLが存在するdirectoryを基準に解決する。

runnerはUTF-8のTOMLを読み、未知のfield、重複field、型の不一致、範囲外の整数をerrorとして拒否しなければならない。

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

absolute path、case directory外へ解決されるpath、symbolic linkによるcase directory外への脱出は拒否する。

`spec` referenceは現行仕様のrootを基準とし、`cpu.md#...`または`bus.md#...`の形式で記述する。これはtraceability metadataであり、runnerがspecification fileを読み込む必要はない。

## 3. binaryのload

```toml
[image]
load_address = 0x00000000
```

`load_address`は`0x00000000`または`0xFFFFFFFC`とする。省略時は`0x00000000`とする。

- `0x00000000`ではbinaryの全byteがTest RAM内に収まらなければならない。binary sizeは`0`でもよく、4の倍数である必要はない。
- `0xFFFFFFFC`ではbinary sizeを正確に4 byteとし、High Test ROMへ配置する。

## 4. 初期状態

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

- `initial.pc`と`initial.registers`はLevel 2以上でだけ指定できる。
- 未指定のPC/registerはreset値を維持する。
- register名は`r0`から`r15`だけを認める。
- `r0`には`0`だけを指定できる。
- memory patchはLevel 1以上で指定できる。
- patchはTest RAM内に収まり、互いに重複してはならない。
- patchはbinary imageと重複してはならない。
- `bytes`の各要素は`0..255`とし、空配列を禁止する。

## 5. 実行

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

- `max_steps`は正のintegerで必須とする。
- `steps`は`until = "steps"`の場合だけ必須で、`1..max_steps`とする。
- `until = "reporter"`または`fault`で`steps`を指定してはならない。
- 指定したstop conditionより先に別のterminationが発生した場合、caseは失敗する。
- `max_steps`回のstep試行後もstop conditionが成立しなければstep limit failureとする。

## 6. 期待する終了状態

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

`status`は`pass`または`fail`とする。`case_id`、`detail`、`retired`は省略可能であり、省略時は比較しない。

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

`fault`は`INVALID_INSTRUCTION`、`UNALIGNED_ACCESS`、`UNALIGNED_PC`、`BUS_FAULT`のいずれかとする。metadata fieldは対応するKagura faultが保持する場合に必須とし、該当しないfieldを指定してはならない。

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

- 記載した値だけを比較し、未記載のPC/register/memoryは比較しない。
- `expected.state`はLevel 2以上でだけ使用できる。
- memory rangeはTest RAM内に収まり、互いに重複してはならない。
- fault caseのstateはfault処理が完了し、CPUがhostへ制御を返した直後に観測する。

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

- `operation`は`read`または`write`とする。
- `width`は`8`、`16`、`32`とする。
- `result`は`ok`または`fault`とする。
- 成功readと全writeには`value`を指定する。
- fault readに`value`を指定してはならない。
- traceにはinstruction fetchを通常の32-bit `read`として含める。
- setup、binary load、initial patch、結果観測はtraceに含めない。
- `expected.bus`を記載した場合、entry数と全entryを順序を含めて完全一致させる。

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

- `initial.devices`と`expected.devices`はLevel 3だけで使用できる。
- device IDは各list内で一意とする。
- stateは[machine specification](./machine.md)がdeviceごとに定義する全fieldを正確に1回ずつ含める。
- setupとsnapshotはhost操作であり、bus traceへ含めない。
- `expected.devices`を記載した場合、指定deviceの全state fieldを完全一致させる。

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
