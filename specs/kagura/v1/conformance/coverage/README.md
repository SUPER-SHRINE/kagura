# Coverage対応表

このdirectoryは、Kagura適合caseの追跡可能性を示すindexである。
TOML fragmentは[Coverage manifest形式](../coverage-format.md)が定義する規範metadataである。
CPUの規範入力は[`../cases/cpu/`](../cases/cpu/)以下にある`case.toml`と`program.bin`の組であり、
Busの規範入力は[`../cases/bus/`](../cases/bus/)以下にある直接transaction caseである。

## Logical manifestの構成

Coverage manifestはsuiteとreview対象ごとに次のfileへ分割する。

- [`common-arithmetic.toml`](./common-arithmetic.toml): reset、fetch、予約opcode、ADD、NAND、MUL、SHIFT
- [`compare-load.toml`](./compare-load.toml): CMP、LDB、LDH、LDW
- [`store-jump.toml`](./store-jump.toml): STB、STH、STW、JZ
- [`bus.toml`](./bus.toml): Bus Contractの直接transaction

4つのfileは1つのlogical manifestを構成する。manifestの分割によって、別々の適合suiteが作られる
わけではなく、fragment間でrequirement IDの重複が許可されるわけでもない。探索、schema、参照、
validationの規則は[Coverage manifest形式](../coverage-format.md)で規範的に定義する。

## 統合したtest plan項目

test plan項目とcaseは1対1である必要はない。1つのcaseでoracleを弱めずに等価な観測を証明できる
場合は、意図的に観測項目を統合する。要件文とcase一覧を、その統合に関する正とする記録とする。
例を次に示す。

- `KAGURA-V1-ADD-BASIC`は、基本的なADD、zero immediate、1回の32-bit little-endian fetch、
  PCの逐次更新を検査する。
- load/storeのpositive-components caseは、2つのsource registerと正のimmediateによる寄与を検査する。
- little-endianのload/store caseは、alignmentを満たす成功caseと正確なaccess widthも検査する。
- JZのlink、target選択、immediate境界の検査は、期待状態から関連するすべての結果を観測できる
  場合にcaseを共有する。

## 解決済みの観測項目

High Test ROM caseは、`0xFFFFFFFC`におけるPCの逐次更新とJZ linkのwrapを検査する。
汎用device snapshotはCPUのload/store caseでfault後のReporter状態を検査し、Bus suiteは直接transactionを
使って同じ不可分性を検査する。このmanifestにcase形式上の未解決事項はない。
