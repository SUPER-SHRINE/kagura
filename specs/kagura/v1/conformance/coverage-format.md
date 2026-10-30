# Kagura適合性coverage manifest形式

## 1. 目的

coverage manifestは、Kaguraの規範要件とconformance caseの対応を記録し、suiteが主張する網羅性を検証できるようにする規範metadataである。

coverage validationはcaseの実行結果を検査しない。\
次の2つは独立した条件である。

- coverage manifestが本書の整合性要件を満たすこと
- manifestが参照する全caseを候補実装がpassすること

Kaguraへの完全適合を主張するsuiteは、両方を満たさなければならない。

## 2. manifestの探索

conformance rootの`coverage` directory直下にある、file名が`.toml`で終わる通常fileをcoverage fragmentとする。\
subdirectoryは再帰的に探索しない。\
README、Markdown、その他の補助fileはfragmentとして解釈しない。

validatorはfragmentをcoverage directoryからのUTF-8 relative file名のcode point順に読み込む。\
fragmentは1件以上存在しなければならない。

各fragmentの`[[requirements]]` arrayを記載順に連結し、1つのlogical manifestとして検証する。\
fragment境界はrequirement ID、case参照、適合範囲のnamespaceを作らない。

coverage formatのversionは親であるKagura conformance specificationに従う。\
fragment内に独立したversion fieldを記載しない。

## 3. fragmentのencodingと構造

fragmentのTOML表記と文字encodingは[case形式の共通規則](./case-format.md#1-fileの組)に従う。\
top-levelには1件以上の`requirements`だけを含めなければならない。\
未知のtop-level fieldはsuite errorとする。

```toml
[[requirements]]
id = "CPU-ADD-SEXT"
spec = "cpu.md#3-算術"
statement = "ADD uses a sign-extended 16-bit immediate."
cases = [
  "KAGURA-V1-ADD-IMM-MAX-POS",
  "KAGURA-V1-ADD-IMM-MIN-NEG",
  "KAGURA-V1-ADD-IMM-NEG-ONE",
]
```

各requirementは次のfieldを正確に1回ずつ持たなければならない。\
未知のfield、欠落field、重複fieldはsuite errorとする。

| field | 型 | 意味 |
| --- | --- | --- |
| `id` | string | logical manifest内で一意なrequirement identifier |
| `spec` | string | requirementの根拠となるKagura規範文書の節 |
| `statement` | string | caseが検査する規範要件の簡潔な記述 |
| `cases` | array of string | requirementを検査するcase IDの集合 |

TOML commentとfield間の空白は意味を持たない。\
requirementおよび`cases`の記載順はvalidation結果または適合判定に影響してはならない。

## 4. 要件identifier

`id`はASCIIの大文字英字で始まり、その後はASCIIの大文字英字、数字、hyphenだけで構成しなければならない。

```text
[A-Z][A-Z0-9-]*
```

空文字、末尾のhyphen、連続するhyphenは使用してはならない。\
logical manifest内で同じ`id`を複数回定義してはならない。

IDは追跡に使う安定した識別子である。\
statementの表現変更だけを理由にIDを変更すべきではない。\
異なる規範要件へ意味を変更する場合は、新しいIDを使用しなければならない。

## 5. 仕様参照

`spec`は現行仕様のrootを基準とし、次の形式でなければならない。

```text
relative-markdown-path#heading-anchor
```

規則:

- pathとheading anchorの両方を省略してはならない
- absolute path、backslash、`.`または`..` path component、queryは使用してはならない
- pathは現行仕様のroot内にある既存のregular Markdown fileへ解決されなければならない
- fragmentに含めてよい`#`は1つだけとし、anchorは参照先文書のATX headingから生成される識別子と一致しなければならない
- 参照先はKaguraの規範要件を記述する節に限り、現行仕様のroot内の規範文書ではないconformance directory配下やconformance case自身、一時的な計画文書を根拠としてはならず、symlinkまたはjunctionを含む場合も解決後の実体は現行仕様のroot内に留まらなければならない

heading anchorは次の順で生成する。\
ATX headingの先頭にある`#`列と、その後に続く必須空白を除いたheading textをtrimする。\
heading textの末尾に空白と1個以上の`#`が続く場合、そのclosing sequenceをheading textから除き、末尾を再度trimする。\
残ったheading textでは、ASCII英字を小文字化し、各空白文字を1個ずつhyphenへ変換する。\
ASCII英数字、hyphen、underscore以外のASCII punctuationを除去する。\
同じ文書内で同じanchorが複数生成される場合、その文書をcoverageから参照してはならない。

## 6. 要件文

`statement`はtrim後に空でないUTF-8 stringでなければならない。\
参照した規範要件と、列挙したcaseが観測するbehaviorを説明しなければならない。

validatorはstatementの自然言語上の正しさや、caseがstatementを十分に証明するかを自動判定しない。\
この意味的対応はsuite reviewで確認しなければならない。\
validatorが保証するのは構造と参照の完全性だけである。

## 7. case参照

`cases`は1件以上のcase IDを含まなければならない。\
同一requirementの`cases`内で同じcase IDを重複させてはならない。

case IDはCPU suiteとBus suiteを合わせたcase discovery結果に正確に一致しなければならない。\
validatorは[CPU Case TOML Format](./case-format.md)と[Bus Case TOML Format](./bus-case-format.md)に従ってcase IDを取得する。

1つのcaseを複数のrequirementから参照してよい。\
1つのrequirementを複数のcaseで検査してよい。\
caseとrequirementの一対一対応は要求しない。

## 8. logical manifestの整合性

validatorは全fragmentと全CPU/Bus caseを収集した後、少なくとも次を検証しなければならない。

1. 全requirement IDがlogical manifest内で一意である
2. 全CPU/Bus case IDが結合したsuite内で一意である
3. 各requirementが1件以上のcaseを参照する
4. 各CPU/Bus caseが1件以上のrequirementから参照される
5. 参照された全case IDが存在する
6. 各requirement内のcase IDが重複していない
7. 全`spec`参照の文書とheading anchorが存在する

未知のcaseを参照することと、存在するcaseがどのrequirementからも参照されないことは別々のerrorである。\
どちらも拒否しなければならない。

## 9. validation順序と候補実装の分離

runnerは候補CPUまたは候補busを実行する前にcoverage validationを完了しなければならない。

1. caseをdiscoverし、各case TOMLの構文とIDを検証する
2. coverage fragmentをdiscoverしてparseする
3. requirement、case、spec referenceの整合性を検証する
4. 全validationが成功した場合だけ候補実装へcaseを適用する

coverage validationが失敗したsuiteから一部のvalid caseだけを実行し、その結果を完全適合の根拠としてはならない。\
diagnostic目的で実行を継続する非適合modeを提供してもよいが、その結果をsuite passとして表示してはならない。

## 10. errorと結果

coverage不整合はcase failureではなくsuite errorである。\
suiteへの集約とCLI終了statusは[結果形式](./result-format.md#4-process終了status)に従う。

JSON Linesを出力するrunnerは、各coverage errorを[Result Format](./result-format.md)のerror objectとして出力する。\
`suite`は`coverage`、`case`と`level`は`null`とする。\
可能な場合、問題を含むfragmentのconformance rootからのrelative pathを`source`に入れる。

```json
{"format":"kagura-conformance-result","version":1,"suite":"coverage","case":null,"source":"coverage/compare-load.toml","level":null,"result":"error","message":"duplicate requirement id: CPU-CMP-EQUALITY"}
```

validatorは可能な限り複数の独立したerrorを収集してよい。\
同じ原因から派生するdiagnosticを重複して出力する必要はない。

coverage validation成功時の専用result objectは要求しない。\
coverageがvalidであることは、suite errorを出さずcase実行へ進んだことで表現する。
