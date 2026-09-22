# Kagura Bus適合case TOML形式

## 1. 目的

Bus caseはCPUを実行せず、候補busへtransactionを直接適用する。規範的入力は`case.toml`だけであり、`program.bin`を持たない。

## 2. header情報

```toml
format = "kagura-bus-conformance-case"
version = 1
id = "KAGURA-V1-BUS-WRITE32-ATOMIC"
spec = ["bus.md#アクセス"]
```

`format`、`version`、`id`、`spec`の規則はCPU caseと同じとする。`id`は空でないASCII文字列で、ASCIIの制御文字・空白を含めてはならない。Bus caseに`level`または`binary`を指定してはならない。

## 3. deviceの初期状態

```toml
[[initial.devices]]
id = "reporter"
state = { status = 0, case_id = 0x11223344, detail = 0x55667788, reserved = 0 }
```

device IDとstate fieldは[Conformance Test Machine](./machine.md)に従う。記載しないdeviceはreset stateから開始する。

Test RAMの初期byteはCPU caseと同じ`initial.memory`形式で指定できる。指定されないbyteは`0`とし、rangeはTest RAM内に収まり互いに重複してはならない。

## 4. transaction一覧

caseは1件以上のtransactionを順番に指定する。

```toml
[[transactions]]
operation = "write"
address = 0xFFFFF00C
width = 32
value = 0xDEADBEEF

[[transactions]]
operation = "read"
address = 0x00000100
width = 16
```

- `operation`は`read`または`write`。
- `width`は`8`、`16`、`32`。
- writeは`value`必須、readでは`value`を指定しない。
- 各transactionは前のtransactionが完了した後に開始する。
- transactionをbyte accessへ分割したり結合したりしてはならない。

## 5. 期待結果

`expected.transactions`は`transactions`と同じ件数・順序でなければならない。

```toml
[[expected.transactions]]
result = "fault"

[[expected.transactions]]
result = "ok"
value = 0x00001234
```

- `result`は`ok`または`fault`。
- 成功readは`value`必須。
- writeとfault readでは`value`を指定しない。

## 6. deviceの期待状態

```toml
[[expected.devices]]
id = "reporter"
state = { status = 0, case_id = 0x11223344, detail = 0x55667788, reserved = 0 }
```

指定deviceの全state fieldをtransaction列の完了直後に比較する。faultしたtransactionによる部分的または一時的な変更が残ってはならない。

Test RAMはCPU caseと同じ`expected.memory` arrayで指定rangeを比較する。未指定rangeは比較しない。

## 7. 厳密性

runnerは未知field、未知device、範囲外整数、重複device ID、空transaction列、transaction/result件数不一致をsuite errorとして拒否する。
