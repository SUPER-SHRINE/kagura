# Kagura

KaguraはCPU ISAとbus契約に焦点を当てた汎用VM基盤の規格です。

`specs/`はこのリポジトリの本体であり、Kaguraの適合要件を定義する唯一の規範です。\
`support/`にある参照実装とツールは、仕様の理解と検証を支援する非規範の付属物です。

規範資料は次を管理します。

- Kagura CPU ISAとbus契約
- Kagura適合試験protocol
- 実装に依存しない適合試験case
- 規範要件とcaseを対応付けるcoverage manifest

## 内容

| パス | 役割 |
| --- | --- |
| [Kagura規範仕様](./specs/kagura/v1/README.md) | CPU ISAとbus contract |
| [適合試験仕様](./specs/kagura/v1/conformance/README.md) | 適合試験仕様、coverage、test case |
| [`support`](./support/README.md) | 非規範のRust参照実装、conformance runner、開発ツール |

適合試験の規範入力とrunnerによる取り扱いは、[適合試験仕様の「正とする情報」](./specs/kagura/v1/conformance/README.md#正とする情報)で定義します。

## 規範範囲

`support/`の挙動と文書はKagura VM規格の一部ではありません。\
実装と仕様が食い違う場合は`specs/`を正とします。\
`support/`は仕様versionとは独立して更新でき、製品固有のmachine構成や統合規約は各製品側で管理します。

## 参照実装

Rust参照実装と付属ツールは、次のコマンドで検証できます。

```text
cargo test --manifest-path support/Cargo.toml --workspace
```

このテストでは、同じリポジトリにある規範適合試験suiteを直接使用します。

## バージョン

- 仕様書のversionは[`specs/README.md`](./specs/README.md)の記載だけを正とします
- Git tagは`support/`全体のversionとし、SemVerの数値をそのまま使い、`v` prefixは付けません
- 仕様書とsupportのversionは独立して更新し、supportのrelease noteに対応する仕様書versionを記載します

例えば、support versionが`0.1.0`の場合、Git tagも`0.1.0`とします。

## Release

annotation付きtagをpushすると、GitHub Actionsがreleaseを作成します。\
tagのannotationがrelease noteの本文になるため、対応する仕様書versionを必ず記載してください。

```bash
git tag -a 0.1.0
git push origin 0.1.0
```

workflowはtagとsupport versionの一致を検証し、test後にWindows x86_64、Linux x86_64、macOS arm64向けのarchiveとchecksumを作成します。\
GitHub Releaseはdraftとして作成されるため、release noteとassetを確認してからGitHub上で公開します。\
軽量tagはreleaseに使用できません。

## ライセンス

このリポジトリは[MIT License](./LICENSE.md)で公開します。

## ローカルで仕様を読む

サイトはリポジトリのルートで次のコマンドを実行すると、生成して閲覧できます。

```powershell
python -m venv target/website/venv
target/website/venv/Scripts/python -m pip install -r website/requirements.txt
pwsh -NoProfile -File tools/build-docs.ps1 -Serve
```

初回はサイト生成用Python環境を作成し、依存関係をインストールしてください。\
サイトは `http://localhost:8012` で開きます。
