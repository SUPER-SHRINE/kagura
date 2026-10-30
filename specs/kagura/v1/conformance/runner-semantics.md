# Kagura適合試験runner semantics

## 1. runnerの責務

runnerはcase TOMLの意味を候補runtimeに適用するreference-neutralなadapterである。\
runtime内部APIは自由だが、本書で定めるsetup、停止、観測、比較の順序を変更してはならない。

CPU runnerとBus runnerは別々に実装してもよい。\
CPU runnerはcandidate CPUにreference test devicesを接続する。\
Bus runnerはcandidate busにTest RAM、Reporter、High Test ROMをmapし、transactionを直接適用する。

## 2. suiteとcoverageのvalidation

runnerは候補runtimeの起動前に、[Coverage Manifest Format](./coverage-format.md)に従って全coverage fragment、全CPU case、全Bus caseをlogical suiteとして検証しなければならない。

## 3. caseのvalidation

runnerはCPUを起動する前に次を完了しなければならない。

1. TOML syntax、format、version、field、値域を検証する
2. case IDがcoverage validationで収集したIDと一致することを検証する
3. binary pathを安全に解決する
4. binaryが[CPU caseのload条件](./case-format.md#3-binaryのload)を満たし、memory patchと期待memory rangeがTest RAM内に収まることを検証する
5. caseが要求するlevelをrunnerが提供できることを検証する

case自体が不正な場合はtest failureではなくsuite errorとする。\
候補runtimeを実行してはならない。

## 4. setup順序

runnerはcaseごとに新しいmachine stateを作り、次の順序でsetupする。

1. Test RAMを全byte `0`で初期化する
2. Reporterをresetする
3. [CPU caseのbinary load](./case-format.md#3-binaryのload)に従い、binaryを指定先のTest RAMまたはHigh Test ROMへ配置する
4. `initial.memory` patchを適用する
5. `initial.devices`を適用する
6. CPUをresetする
7. [CPU caseの初期状態](./case-format.md#4-初期状態)に従い、Level 2以上で指定された`initial.pc`と`initial.registers`を注入する
8. retired countを`0`にする
9. bus traceを空にする

case間でCPU、RAM、Reporter、trace、fault stateを共有してはならない。

## 5. step実行loop

[machineの実行境界](./machine.md#8-実行境界)で定義する各step試行について、runnerは次を行う。

1. 候補CPUへ1命令の実行を要求する
2. CPUがfaultした場合、命令をretired countへ加えずfault terminationを記録する
3. 成功した場合、retired countを1増やす
4. 成功命令が[Test Reporterのtermination条件](./machine.md#5-test-reporter)を成立させた場合、それを記録する
5. `until = "steps"`のretired countへ達した場合、steps terminationを記録する
6. terminationがなければ次のstepへ進む

同じstepで複数の終了条件が成立し得る場合、優先順位は`fault`、`reporter`、`steps`とする。\
faultしたwriteはReporter terminationを成立させない。

step試行数が`max_steps`に達しても指定stop conditionが成立しなければ、step limit failureとする。

## 6. 観測時点

termination直後、次の命令をfetchする前に次を観測する。

- 終了状態の詳細
- retired数
- PCとregister
- 指定memory range
- Reporter状態
- bus trace

観測操作自体によってbus traceまたはdevice stateを変化させてはならない。\
候補runtimeの通常read APIに副作用がある場合、adapterはsnapshotまたは専用inspection APIを用いなければならない。

## 7. 比較

runnerは次の順序で比較する。

1. actual terminationが`execution.until`と一致するか
2. `expected.termination`の全指定fieldが一致するか
3. `expected.state`の全指定fieldが一致するか
4. `expected.bus`が指定されていれば完全一致するか

device状態は[CPU caseのdevice状態](./case-format.md#9-device状態)に従い、指定された`expected.devices`の全state fieldを比較する。

integerは符号なしのbit patternとして比較する。\
32-bit値には[case形式の共通値域](./case-format.md#1-fileの組)を適用し、完全一致させる。\
memoryはbyte列として完全一致させる。

CPU caseのbus traceの`value`とBus caseの成功readの`value`は、実測値を指定widthの符号なし値として32-bitへゼロ拡張し、期待値と完全一致で比較する。\
期待値はcaseに記載された32-bit値をそのまま使い、widthに合わせた切り詰めは行わない。\
例えばwidth `8`のreadが`0x34`を返す場合、実測値は`0x00000034`となり、期待値`0x1234`とは不一致である。

TOMLに記載されていない観測値をpass/fail判定に使用してはならない。\
ただし、failure diagnosticの補助情報として出力してもよい。

1件でも不一致があればcaseはfailとする。\
runnerは最初の不一致で停止しても、全不一致を収集してもよい。\
ただし、共通結果の`result`はどちらの場合も同じでなければならない。

## 8. fault時のsemantics

fault caseではCPUがhostへ返した直後のstateを比較する。\
adapterはfaultした命令によるregister、PC、deviceへの部分的な変更をrollbackして隠してはならない。

候補runtimeはfaultを例外、result value、callbackなど任意の形式で表現してもよい。\
adapterはその形式をKagura fault名とmetadataへ損失なく変換しなければならない。

## 9. 独立性

- 規範入力の取り扱いは[適合試験仕様の「正とする情報」](./README.md#正とする情報)に従い、別形式への変換によって入力を置き換えてはならない
- runnerは期待値を候補runtimeの計算結果から生成してはならない
- binary loadやinitial state注入をguest命令として実行してはならない
- runner固有の最適化によってbus accessを追加、削除、分割してはならない

## 10. suite結果

suite結果の集約とCLI終了statusは[結果形式](./result-format.md#4-process終了status)に従う。\
runnerが対応しないlevelのcaseはskipではなく`unsupported`と記録し、完全適合判定には使用できない。

## 11. Bus caseの実行

Bus runnerはcaseごとに新しいmachine stateを作り、[Bus caseの初期状態](./bus-case-format.md#3-deviceの初期状態)に従ってTest RAMとdeviceを初期化する。\
`initial.memory`のpatchを適用した後、`initial.devices`を設定し、`transactions`を記載順に候補busへ1回ずつ渡す。

各transactionの成功またはfaultとread valueを`expected.transactions`と比較する。\
transaction列の完了直後、[Bus caseの期待状態](./bus-case-format.md#6-deviceの期待状態)に従って`expected.memory`と`expected.devices`を比較する。\
runnerはfaultしたtransactionをrollbackして結果を作ってはならない。\
候補busまたはdeviceが返した直後のstateを観測する。
