# Kagura適合性

Kagura準拠CPUは[CPU ISA](./cpu.md)のinstruction、architectural state、fault semanticsを実装し、[Bus Contract](./bus.md)に従ってmemory accessを行う。

同じ初期architectural stateと同じbus応答に対し、observableなstate transitionとfaultが仕様に一致することをconformanceの基準とする。\
製品固有のmachine構成やsoftware integrationの契約は、各製品のmachineまたはsoftware layerが管理する。

## 適合試験仕様

異なるruntime実装に共通の適合試験を適用するため、[Kagura適合試験仕様](./conformance/README.md)を定義する。

このtest specificationは次を規定する。

- test binaryを実行する固定machineとtest device
- case TOMLの形式
- coverage manifestの形式と整合性検証
- caseのsetup、実行、観測、比較規則
- runnerが出力する共通結果形式

test specificationは、CPU ISAとbus contractへの適合を検査するrunner adapterの契約を定める。\
test machineやcase TOMLへの要件は試験環境に適用し、製品machineの構成は各製品の契約に従う。
