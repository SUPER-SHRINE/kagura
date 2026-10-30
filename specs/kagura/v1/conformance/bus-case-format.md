# Kagura Bus適合case TOML形式

## 1. 目的

Bus caseはCPUを実行せず、候補busへtransactionを直接適用する。\
規範入力は`case.toml`だけであり、`program.bin`は持たない。

TOMLの適用規格、文字encoding、共通の整数値域は[CPU case形式の共通規則](./case-format.md#1-fileの組)に従う。

## 2. header情報

```toml
format = "kagura-bus-conformance-case"
version = 1
id = "KAGURA-V1-BUS-WRITE32-ATOMIC"
spec = ["bus.md#アクセス"]
```

`format`はliteral値`kagura-bus-conformance-case`とする。\
`version`、`id`、`spec`には[CPU caseの必須header](./case-format.md#2-必須header)と同じ規則を適用する。\
Bus caseに`level`または`binary`を指定してはならない。

## 3. deviceの初期状態

```toml
[[initial.devices]]
id = "reporter"
state = { status = 0, case_id = 0x11223344, detail = 0x55667788, reserved = 0 }
```

device IDとstate fieldの完全性は[machineのdevice状態](./machine.md#6-device状態)に従う。\
記載しないdeviceはreset stateから開始する。

Test RAMの初期byteはCPU caseと同じ`initial.memory`形式で指定できる。\
指定されないbyteは`0`とし、rangeはTest RAM内に収め、互いに重複させてはならない。

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

- `operation`は`read`または`write`とする
- `width`は`8`、`16`、`32`とする
- writeには`value`が必須であり、readには`value`を指定しない
- 各transactionは前のtransactionが完了した後に開始する
- transactionをbyte accessへ分割または結合してはならない

writeの`value`は共通値域の32-bit値として受理し、指定した`width`の下位bitを候補busへ渡す。\
widthを`w`、入力値を`v`とすると、書き込む値は`v mod 2^w`となる。\
例えばwidth `8`、value `0x1234`のwriteは`0x34`を書き込む。

## 5. 期待結果

`expected.transactions`は`transactions`と同じ件数・順序でなければならない。

```toml
[[expected.transactions]]
result = "fault"

[[expected.transactions]]
result = "ok"
value = 0x00001234
```

- `result`は`ok`または`fault`とする
- 成功したreadには`value`が必須
- writeとfaultしたreadには`value`を指定しない

成功readの値は[runnerのbus値比較](./runner-semantics.md#7-比較)に従って比較する。

## 6. deviceの期待状態

```toml
[[expected.devices]]
id = "reporter"
state = { status = 0, case_id = 0x11223344, detail = 0x55667788, reserved = 0 }
```

device IDとstate fieldの完全性は[machineのdevice状態](./machine.md#6-device状態)に従い、指定deviceの全state fieldをtransaction列の完了直後に比較する。\
faultしたtransactionによる部分的または一時的な変更が残ってはならない。

Test RAMの期待状態は`expected.memory` arrayで指定し、各要素の形式とrangeの条件は[CPU caseの`expected.state.memory`](./case-format.md#7-期待するアーキテクチャ状態)に従う。\
未指定rangeは比較しない。

## 7. 厳密性

runnerは未知field、未知device、範囲外整数、重複device ID、空のtransaction列、transactionとresultの件数不一致をsuite errorとして拒否する。
