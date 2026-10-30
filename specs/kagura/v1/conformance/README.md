# Kagura適合試験仕様

本仕様は、同一のKagura適合試験を異なる言語や構造のruntime実装に適用するための試験手順を定める。

適合試験はCPU suiteとBus suiteに分かれる。\
CPU suiteの規範入力はraw binaryとcase TOMLの組であり、Bus suiteの規範入力はbus transactionを直接記述したcase TOMLである。

```text
program.bin + case.toml
          |
          v
conformance runner / implementation adapter
          |
          v
Kagura CPU candidate + Conformance Test Machine
```

## 文書一覧

- [適合試験machine](./machine.md)
- [CPU case TOML形式](./case-format.md)
- [Bus case TOML形式](./bus-case-format.md)
- [Coverage manifest形式](./coverage-format.md)
- [Runnerの動作規則](./runner-semantics.md)
- [結果形式](./result-format.md)

補助index:

- [Coverage index](./coverage/README.md) — CPU/Busのrequirementとcaseの対応

## 規範表現

本文中の「しなければならない」は適合runnerに必須の要件を表す。\
「してもよい」は結果の互換性を損なわない任意実装を表す。

## 適用範囲

本仕様はrunner adapterが共通の適合試験を実行するための次の契約を定義する。

- 試験用の最小address mapとdeviceの動作
- binaryと初期状態の配置方法
- CPU実行の停止条件
- architectural state、memory、fault、bus traceの比較方法
- runnerの機械可読な結果形式
- CPU適合とBus適合の独立した判定
- 規範要件とcaseを対応付けるcoverage manifestとその整合性検証

製品machine、software layer、開発ツールは、次の具体的な選択をそれぞれ管理する。

- Kagura製品runtimeのaddress mapまたはdevice構成
- runtime内部のAPI、class、trait、FFI
- test binaryの生成方法
- 製品への統合方法とsoftware layerの規約

runner adapterは本仕様に従って候補実装へ初期状態を設定し、必要な観測値を取り出す。\
本仕様のmachineへの要件は適合試験の実行環境に適用し、製品machineの構成は各製品の契約に従う。

## CPUとBusの適合性

CPU ISAとBus契約への適合基準は[Kagura適合性](../conformance.md)で定義し、各suiteのcaseは、その規則を指定した入力と期待結果で確認する。

CPU suiteは[`cases/cpu/`](./cases/cpu/)に置く。\
候補CPUにConformance Test Machineを接続し、ISA、fault、CPUからbusへの要求を検査する。

Bus suiteは[`cases/bus/`](./cases/bus/)に置く。\
候補busに規定transactionを直接適用し、width、endianness、不可分性、mapping、fault時のdevice atomicityを検査する。

CPU suiteだけの成功をBus Contractへの適合と表示してはならない。\
Kaguraへの完全適合にはCPU suiteとBus suiteの両方への成功が必要である。

## 適合level

caseは必要とする観測能力を`level`で宣言する。

| level | 名前 | 必要な機能 |
| ---: | --- | --- |
| 1 | `machine` | reset、binary load、実行、Reporterまたはfaultの観測 |
| 2 | `state` | Level 1に加え、PC/registerの注入とPC/register/Test RAMの終了時観測 |
| 3 | `bus-observation` | Level 2に加え、CPU実行中bus accessの順序・width・値・成否とdevice状態の観測 |

上位levelのCPU runnerは下位levelの全caseを実行できなければならない。\
Level 3はCPUが正しいbus transactionを要求したことを観測するが、候補bus自体の適合を代替しない。

## 正とする情報

`program.bin`に含まれるbyte列がCPUへ渡す規範入力である。\
人間向けの表現や生成用資料を同梱してもよいが、それらは参考資料であり、runnerは実行時に`program.bin`を別形式から再生成してはならない。

case TOMLは初期条件、実行条件、期待結果を規範的に記述する。\
runner固有のtest codeとcase TOMLが矛盾する場合は、case TOMLを優先する。

`coverage/*.toml`による規範要件とCPU/Bus caseの対応、および完全適合を主張するsuiteのvalidation要件は[Coverage Manifest Format](./coverage-format.md)に従う。

## suiteの探索

runnerは明示的に渡された`case.toml`を実行できなければならない。\
directoryをsuiteとして渡すinterfaceを提供する場合、そのdirectory以下から名前が正確に`case.toml`であるfileを再帰的に収集する。\
収集したfileはsuite rootからのUTF-8 relative pathをcode point順に並べて実行する。

directory名や追加の補助fileをtest caseとして暗黙に解釈してはならない。\
case間に実行順の依存関係を持たせてはならない。
