# Kagura適合試験仕様

本仕様は、同一のKagura適合試験を異なる言語・構造のruntime実装へ適用するためのtest protocolを定義する。

適合試験はCPU suiteとBus suiteに分かれる。CPU suiteの規範的入力はraw test binaryとcase TOMLの組であり、Bus suiteの規範的入力は直接bus transactionを記述したcase TOMLである。

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

- [Coverage index](./coverage/README.md) — CPU/Bus requirementとcaseの対応

## 規範表現

本文中の「しなければならない」は適合runnerに必須の要件を表す。「してもよい」は結果の互換性を損なわない任意実装を表す。

## 適用範囲

本仕様が規定するもの:

- test用の最小address mapとdevice behavior
- binaryと初期状態の配置方法
- CPU実行の停止条件
- architectural state、memory、fault、bus traceの比較方法
- machine-readableなrunner結果
- CPU適合とBus適合の独立した判定
- 規範要件とcaseを対応付けるcoverage manifestとその整合性検証

本仕様が規定しないもの:

- Kagura製品runtimeのaddress mapまたはdevice構成
- runtime内部のAPI、class、trait、FFI
- test binaryの生成方法
- 製品への統合方法とsoftware layerの規約

Kagura適合実装は本仕様のmachineを製品機能として公開する必要はない。runner adapterが候補実装へ必要な初期状態を設定し、必要な観測値を取り出せればよい。

## CPUとBusの適合性

CPU suiteは[`cases/cpu/`](./cases/cpu/)に置き、候補CPUへConformance Test Machineを接続してISA、fault、CPUからbusへの要求を検査する。

Bus suiteは[`cases/bus/`](./cases/bus/)に置き、候補busへ規定transactionを直接適用してwidth、endianness、不可分性、mapping、fault時のdevice atomicityを検査する。

CPU suiteだけの成功をBus Contractへの適合と表示してはならない。Kaguraへの完全適合にはCPU suiteとBus suiteの両方への成功が必要である。

## 適合level

caseは必要とする観測能力を`level`で宣言する。

| level | 名前 | 必要な機能 |
| ---: | --- | --- |
| 1 | `machine` | reset、binary load、実行、Reporterまたはfaultの観測 |
| 2 | `state` | Level 1に加え、PC/registerの注入とPC/register/Test RAMの終了時観測 |
| 3 | `bus-observation` | Level 2に加え、CPU実行中bus accessの順序・width・値・成否とdevice状態の観測 |

上位levelのCPU runnerは下位levelの全caseを実行できなければならない。Level 3はCPUが正しいbus transactionを要求したことを観測するが、候補bus自体の適合を代替しない。

## 正とする情報

`program.bin`に含まれるbyte列がCPUへ渡す規範的入力である。人間向けの表現や生成用資料を同梱してもよいが、それらは参考資料であり、runnerは実行時に`program.bin`を別形式から再生成してはならない。

case TOMLは初期条件、実行条件、期待結果の規範的記述である。runner固有のtest codeとcase TOMLが矛盾する場合はcase TOMLを優先する。

`coverage/*.toml`はKaguraの規範要件とCPU/Bus caseの対応を表す規範的metadataである。完全適合を主張するsuiteは、case実行に加えて[Coverage Manifest Format](./coverage-format.md)のvalidationをpassしなければならない。

## suiteの探索

runnerは明示的に渡された`case.toml`を実行できなければならない。directoryをsuiteとして渡すinterfaceを提供する場合、そのdirectory以下にある名前が正確に`case.toml`であるfileを再帰的に収集し、suite rootからのUTF-8 relative pathをcode point順に並べて実行する。

directory名や追加の補助fileをtest caseとして暗黙に解釈してはならない。case間に実行順依存を持たせてはならない。
