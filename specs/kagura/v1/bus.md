# Kagura Bus

本書は[Kagura CPU](./cpu.md)が使用するbus accessを定義する。\
address mapはmachineが定義する。

## アクセス

- addressは32-bitのbyte addressとする
- 8-bit、16-bit、32-bitの読み込みと書き込みを提供する
- 複数byteの値はlittle-endianとし、幅が`w` bitの値`v`では先頭addressから`i` byte目が`(v >> (8 * i)) & 0xFF`に対応する（`0 <= i < w / 8`）
- 未割り当てaddressまたはdeviceが受理しないaccessはfaultを返す
- 各accessは分割できない1回の操作とし、16-bitまたは32-bit accessを複数のbyte accessとして扱わない
- 成功したwriteは全体をcommitする
- faultしたreadまたはwriteはdeviceに副作用を残さない

CPUはbus faultを`BUS_FAULT`として扱う。
