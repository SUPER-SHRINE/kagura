# 仕様一覧

## 仕様バージョン

現行仕様のversionは`1.0.0`です。この記載をKagura仕様書versionの唯一の正とし、個別文書には
versionを重複して記載しません。

仕様書versionはSemantic Versioningに従い、`v` prefixを付けません。

- MAJOR: 既存の適合実装と互換性のない規範変更
- MINOR: 後方互換性を維持した規範要件または機能の追加
- PATCH: 意味を変えない明確化、誤記、適合試験metadataの訂正

## 文書

- [Kagura規範仕様](./kagura/v1/README.md): CPU ISAとbus contract
- [適合試験仕様](./kagura/v1/conformance/README.md): 適合試験protocol、format、coverage、test case

この索引は Kagura VM 基盤の規範仕様と適合試験だけを対象とします。
