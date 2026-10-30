# Kagura規範仕様

KaguraはCPU ISAとbus契約だけを定義する汎用runtime規格である。\
仕様書versionは[`../../README.md`](../../README.md)の記載に従う。

- [CPU ISA](./cpu.md)
- [Bus契約](./bus.md)
- [適合性](./conformance.md)
- [適合試験仕様](./conformance/README.md)

## 適用範囲

CPU ISAとbus契約は次を定義する。

- instruction encoding と architectural state transition
- fault の発生条件
- CPU と bus の access contract

Kaguraを利用する各machineまたはsoftware layerは、次の契約を定義する。

- source program の表記と変換方法
- 具体的な address map、memory 容量、初期配置
- software 実行上の規約と package 形式
- 製品固有の device と host service

CPU ISAとbus契約への適合は、これらの製品固有の選択から独立して判定する。\
適合試験用のaddress map、test device、case fileは[Conformance Test Specification](./conformance/README.md)が定義し、runner adapterが異なる実装へ共通の試験を適用するために使用する。
