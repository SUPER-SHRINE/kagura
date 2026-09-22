# Kagura

Kagura は CPU ISA と bus contract に焦点を当てた、汎用 VM 基盤の規格です。

`specs/` がこのリポジトリの本体であり、Kagura の適合要件を定義する唯一の規範です。
`support/` の参照実装とツールは仕様の理解・検証を支援する非規範の付属物です。

規範資料は次を管理します。

- Kagura CPU ISA と bus contract
- Kagura適合試験protocol
- 実装非依存の conformance test case
- 規範要件と test case を対応付ける coverage manifest

## 内容

| パス | 役割 |
| --- | --- |
| [Kagura規範仕様](./specs/kagura/v1/README.md) | CPU ISAとbus contract |
| [適合試験仕様](./specs/kagura/v1/conformance/README.md) | 適合試験仕様、coverage、test case |
| [`support`](./support/README.md) | 非規範のRust参照実装、conformance runner、開発ツール |

規範的な CPU test input は `program.bin` と `case.toml` の組です。Bus test input は `case.toml` です。runner はこれらの規範入力を直接使用し、実行時に別形式から再生成してはいけません。

## 規範範囲

`support/` の挙動と文書は Kagura VM 規格の一部ではありません。実装と仕様が食い違う場合は
`specs/` を正とします。`support/` は仕様versionとは独立して更新でき、製品固有のmachine構成や
統合規約は引き続き各製品側で管理します。

## 参照実装

Rust参照実装と付属ツールは次のコマンドで検証できます。

```text
cargo test --manifest-path support/Cargo.toml --workspace
```

このテストは同じリポジトリの規範conformance suiteを直接使用します。

## バージョン

- 仕様書のversionは[`specs/README.md`](./specs/README.md)の記載だけを正とします。
- Git tagは`support/`全体のversionとし、SemVerの数値をそのまま使用します。`v` prefixは付けません。
- 仕様書とsupportのversionは独立して更新し、supportのrelease noteに対応する仕様書versionを記載します。

例えばsupport version `0.1.0`のGit tagは`0.1.0`とします。

## Release

releaseはannotation付きtagをpushするとGitHub Actionsが作成します。tagのannotationをrelease note本文として
使用するため、release noteには対応する仕様書versionを必ず記載してください。

```bash
git tag -a 0.1.0
git push origin 0.1.0
```

workflowはtagとsupport versionの一致を検証し、test後にWindows x86_64、Linux x86_64、
macOS arm64向けのarchiveとchecksumを作成します。GitHub Releaseはdraftとして作成されるため、
release noteとassetを確認してからGitHub上で公開します。軽量tagはreleaseに使用できません。

## ライセンス

このリポジトリは[MIT License](./LICENSE.md)で公開します。
