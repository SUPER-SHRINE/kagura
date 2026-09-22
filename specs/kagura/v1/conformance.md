# Kagura適合性

Kagura準拠CPUは[CPU ISA](./cpu.md)のinstruction、architectural state、fault semanticsを実装し、[Bus Contract](./bus.md)に従ってmemory accessを行う。

同じ初期 architectural state と同じ bus 応答に対し、observable な state transition と fault が仕様に一致することを conformance の基準とする。製品固有の machine 構成や software integration は Kagura conformance の条件ではない。

## 適合試験仕様

Kaguraの適合試験を異なるruntime実装へ共通に適用するため、[Kagura適合試験仕様](./conformance/README.md)を定義する。

このtest specificationは次を規定する。

- test binaryを実行する固定machineとtest device
- case TOMLの形式
- coverage manifestの形式と整合性検証
- caseのsetup、実行、観測、比較規則
- runnerが出力する共通結果形式

test specificationはKaguraのCPUまたはbus contractを拡張しない。Kagura適合実装は、test machineやcase TOMLを製品機能として実装する必要はない。適合試験を実行するadapterだけがtest specificationを解釈すればよい。
