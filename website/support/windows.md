# Windowsのセットアップ

Kagura CLIは、Windows 10以降のx64環境で利用できます。

## インストーラーでセットアップ

1. [KaguraのReleases](https://github.com/SUPER-SHRINE/kagura/releases)を開き、利用するリリースの`kagura-VERSION-x86_64-pc-windows-msvc-setup.exe`をダウンロード
   - 例: support 0.2.0では`kagura-0.2.0-x86_64-pc-windows-msvc-setup.exe`
2. ダウンロードしたインストーラーを実行
3. 管理者権限を許可し、インストール先を確認
   - 初期値は`C:\Program Files\SUPER-SHRINE\Kagura`
   - インストールは全ユーザー向け
4. PATHへの追加を選択
   - システムPATHへの追加は初期状態で選択済み
   - 追加しない場合はチェックを外す
5. インストール後、新しいターミナルを開いて動作を確認

PATHへの追加を選んだ場合は、次のコマンドを実行します。

```powershell
kagura --version
kagura help
```

PATHへ追加したときは、インストール前から開いているターミナルやアプリケーションを再起動してください。\
PATHへ追加しなかった場合は、実行ファイルの場所を指定します。

```powershell
& 'C:\Program Files\SUPER-SHRINE\Kagura\kagura.exe' --version
& 'C:\Program Files\SUPER-SHRINE\Kagura\kagura.exe' help
```

### ZIPから使う

インストーラーを使わない場合は、同じReleasesから`kagura-VERSION-x86_64-pc-windows-msvc.zip`をダウンロードして展開します。

展開したフォルダーで次を実行してください。

```powershell
.\kagura.exe --version
.\kagura.exe help
```

ZIP版は展開した場所から実行できます。\
PATHへ登録する場合は、展開先のフォルダーを使用者がPATHへ追加してください。

### 更新とアンインストール

新しいバージョンへ更新するときは、対象バージョンのインストーラーを実行してください。\
前回のインストール先とPATH追加の選択を引き継ぎます。\
更新では旧版の削除が完了してから新版を配置するため、旧版の管理対象ファイルも整理され、利用者が追加したファイルは保持されます。\
旧版の削除に失敗した場合や再起動が必要な場合は更新を中止するので、削除を完了してから再実行してください。

Windowsの「インストールされているアプリ」からKaguraをアンインストールできます。\
インストーラーがシステムPATHへ追加した項目はアンインストール時に削除され、ほかのPATH項目は保持されます。

## 適合試験を実行

インストーラー版には仕様書と適合試験suiteが含まれます。\
PATHへ追加した場合は、suiteの場所を明示して実行します。

```powershell
kagura conformance "C:\Program Files\SUPER-SHRINE\Kagura\specs\kagura\v1\conformance"
```

PATHへ追加しなかった場合は、実行ファイルとsuiteの場所を指定します。

```powershell
& 'C:\Program Files\SUPER-SHRINE\Kagura\kagura.exe' conformance 'C:\Program Files\SUPER-SHRINE\Kagura\specs\kagura\v1\conformance'
```

ZIP版では展開したディレクトリへ移動してから、次のように実行します。

```powershell
.\kagura.exe conformance .\specs\kagura\v1\conformance
```

仕様は[Kagura適合試験](../specs/kagura/v1/conformance/README.md)を参照してください。
