# Kagura support 0.2.0

対応するKagura仕様書versionは`1.1.0`です。

## 主な変更

- ローカルで閲覧できる仕様サイトを追加し、GitHub Pages向けの公開構成を整備
- CPU、bus、適合試験の規範記述を明確化し、数値形式と計算式を追加
- CPU fault時にregisterとPCを保持し、fault前に成功したbus accessの副作用を維持
- `expected.bus`の省略時はbus traceを比較せず、空配列指定時はaccess 0件との完全一致を検査
- Reporterのwrite eventと終了条件を定義し、値0、不正値、同一status、reservedのhost初期値を扱う規則を明確化
- CPU caseとBus caseのdevice snapshotで、指定deviceの全state fieldを必須化
- Test RAMのunaligned accessを必須とするcaseを追加（case 181件、従来180件）
- support workspaceのテストを80件に拡充（従来69件）
