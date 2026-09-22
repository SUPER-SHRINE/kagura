# Kagura付属実装

このディレクトリの実装とツールは、Kagura仕様の理解・検証を支援する非規範の付属物です。
Kaguraの適合要件は [`../specs`](../specs/README.md) のみが定義します。実装と仕様が食い違う
場合は仕様を正とします。

対応する仕様書versionは[`../specs/README.md`](../specs/README.md)を参照します。supportのversionと
仕様書versionは独立しており、対応関係は各support releaseのrelease noteに記録します。

## 内容

| パス | 役割 |
| --- | --- |
| [`reference/rust`](./reference/rust) | Rust参照実装と試験用device/machine |
| [`conformance/runner`](./conformance/runner) | 規範conformance inputを実行する参照runner |
| [`tools/assembly`](./tools/assembly) | 非規範のassembly構文とassembler library |
| [`tools/cpu-tester`](./tools/cpu-tester) | CPUの対話的・手動検証用ツール |
| [`tools/kagura-cli`](./tools/kagura-cli) | 付属ツールをまとめるCLI |

## テスト

リポジトリルートから実行します。

```text
cargo test --manifest-path support/Cargo.toml --workspace
```

conformance runnerは[現行の適合試験suite](../specs/kagura/v1/conformance/README.md)にある
`case.toml`と`program.bin`を直接読みます。
assemblerから規範入力を再生成しません。
