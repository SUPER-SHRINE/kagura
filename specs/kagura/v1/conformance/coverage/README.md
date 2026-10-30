# Coverage対応表

このdirectoryはKagura適合caseの追跡可能性を示すindexである。\
TOML fragmentは[Coverage manifest形式](../coverage-format.md)で定義する規範metadataである。\
caseの配置と規範入力は[適合試験仕様](../README.md)に従う。

## Logical manifestの構成

Coverage manifestはsuiteとreview対象ごとに次のfileへ分割する。

- [`common-arithmetic.toml`](./common-arithmetic.toml): reset、fetch、予約opcode、ADD、NAND、MUL、SHIFT
- [`compare-load.toml`](./compare-load.toml): CMP、LDB、LDH、LDW
- [`store-jump.toml`](./store-jump.toml): STB、STH、STW、JZ
- [`bus.toml`](./bus.toml): Bus Contractの直接transaction

4つのfileを統合するlogical manifestの構成、探索、schema、参照、validationの規則は[Coverage manifest形式](../coverage-format.md)で定める。\
requirementとcaseの対応は[case参照](../coverage-format.md#7-case参照)に従う。
