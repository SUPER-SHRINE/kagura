# Linuxのセットアップ

Kagura CLIは、リリースページに掲載されたLinux x86_64向けアーカイブから利用できます。

## ダウンロードと展開

1. [KaguraのReleases](https://github.com/SUPER-SHRINE/kagura/releases)を開き、利用するリリースの`kagura-VERSION-x86_64-unknown-linux-gnu.tar.gz`を`~/Downloads`へダウンロード
   - 例: support 0.2.0では`kagura-0.2.0-x86_64-unknown-linux-gnu.tar.gz`
2. 利用者が書き込めるディレクトリへアーカイブを展開

```sh
mkdir -p "$HOME/.local/opt"
tar -xzf "$HOME/Downloads/kagura-0.2.0-x86_64-unknown-linux-gnu.tar.gz" -C "$HOME/.local/opt"
cd "$HOME/.local/opt/kagura-0.2.0-x86_64-unknown-linux-gnu"
./kagura --version
./kagura help
```

コマンド例のバージョンはsupport 0.2.0です。\
別のリリースを選んだ場合は、ファイル名と展開後のディレクトリ名をそのバージョンに合わせてください。

## PATHへ追加する場合

展開した場所から`./kagura`を実行できます。\
毎回どのディレクトリからでも`kagura`と入力する場合は、展開先の絶対パスをPATHへ追加してください。

Bashでは`~/.bashrc`、Zshでは`~/.zshrc`を開き、次の行を追加します。

```sh
export PATH="$PATH:$HOME/.local/opt/kagura-0.2.0-x86_64-unknown-linux-gnu"
```

設定を反映するには、ターミナルを開き直してください。

## 適合試験を実行

端末の作業ディレクトリに関係なく実行できるよう、実行ファイルとsuiteの場所を指定します。

```sh
"$HOME/.local/opt/kagura-0.2.0-x86_64-unknown-linux-gnu/kagura" conformance \
  "$HOME/.local/opt/kagura-0.2.0-x86_64-unknown-linux-gnu/specs/kagura/v1/conformance"
```

仕様は[適合試験](../specs/kagura/v1/conformance/README.md)を参照してください。
