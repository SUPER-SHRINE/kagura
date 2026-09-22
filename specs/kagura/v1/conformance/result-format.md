# Kagura適合試験結果形式

## 1. encoding規則

共通結果はUTF-8のJSON Linesとし、1 caseにつき1 objectを1行で出力する。field名とenum valueはcase TOMLと同様にcase-sensitiveとする。

## 2. case結果

成功例:

```json
{"format":"kagura-conformance-result","version":1,"suite":"cpu","case":"KAGURA-V1-ADD-001","level":2,"result":"pass"}
```

失敗例:

```json
{"format":"kagura-conformance-result","version":1,"suite":"cpu","case":"KAGURA-V1-ADD-001","level":2,"result":"fail","mismatches":[{"path":"state.registers.r3","expected":3,"actual":4}]}
```

validなcaseを実行した結果の必須field:

| field | 意味 |
| --- | --- |
| `format` | literal値 `kagura-conformance-result` |
| `version` | 整数 `1` |
| `suite` | case結果では`cpu`または`bus`、coverage validation errorでは`coverage` |
| `case` | case TOMLの`id` |
| `level` | CPU caseではcase TOMLの`level`、Bus caseでは`null` |
| `result` | `pass`、`fail`、`error`、`unsupported` |

`fail`では`mismatches`を1件以上含める。各mismatchは比較対象を示す`path`、`expected`、`actual`を持つ。

`error`ではcaseまたはrunnerの問題を説明する`message`を含める。`unsupported`ではrunnerが提供する最大levelを`runner_level`に含める。

case TOMLから`id`を取得する前にerrorとなった場合は、`case`を`null`とし、入力を識別するUTF-8 pathを`source`へ入れる。

```json
{"format":"kagura-conformance-result","version":1,"suite":"cpu","case":null,"source":"cases/cpu/broken/case.toml","level":null,"result":"error","message":"missing required field: id"}
```

coverage validation errorでは`suite`を`coverage`、`case`と`level`を`null`とする。`source`には可能なら問題を含むcoverage fragmentのrelative pathを入れる。coverage errorの詳細規則は[Coverage Manifest Format](./coverage-format.md)に従う。

runnerは追加fieldを出力してもよい。結果consumerは未知の追加fieldを無視しなければならない。

## 3. path表現

代表的なmismatch path:

```text
termination.kind
termination.fault
termination.faulting_pc
termination.address
termination.width
termination.operation
termination.retired
reporter.status
reporter.case_id
reporter.detail
state.pc
state.registers.r0
state.memory[0]
devices.reporter.case_id
bus.length
bus[0].operation
bus[0].address
bus[0].width
bus[0].result
bus[0].value
transactions[0].result
```

配列indexは0-basedとする。

## 4. process終了status

CLI runnerは次のprocess exit statusを使用する。

| status | 意味 |
| ---: | --- |
| `0` | 全caseがpass |
| `1` | 1件以上のcaseがfail |
| `2` | suite error、runner error、未対応case |

platformがこの数値を表現できない場合、最も近い同等の成功・失敗通知を使用してよい。JSON Linesの`result`が規範的な個別結果である。
