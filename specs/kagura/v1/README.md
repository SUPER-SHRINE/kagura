# Kagura規範仕様

KaguraはCPU ISAとbus contractだけを定義する汎用runtime規格である。仕様書versionは
[`../../README.md`](../../README.md)の記載に従う。

- [CPU ISA](./cpu.md)
- [Bus契約](./bus.md)
- [適合性](./conformance.md)
- [適合試験仕様](./conformance/README.md)

## 適用範囲

Kagura が規定するもの:

- instruction encoding と architectural state transition
- fault の発生条件
- CPU と bus の access contract

Kagura が規定しないもの:

- source program の表記と変換方法
- 具体的な address map、memory 容量、初期配置
- software 実行上の規約と package 形式
- 製品固有の device と host service

これらは Kagura を利用する各 machine または software layer が定義します。Kagura の仕様と test case は、その具体的な選択へ依存してはいけません。

適合試験で使用する address map、test device、case file は Kagura runtime の要件ではない。これらは [Conformance Test Specification](./conformance/README.md) が、異なる実装へ同じ試験を適用するために定義する。
