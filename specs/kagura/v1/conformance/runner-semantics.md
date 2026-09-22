# Kagura適合試験runner semantics

## 1. runnerの責務

runnerはcase TOMLの意味を候補runtimeへ適用するreference-neutralなadapterである。runtime内部APIは自由だが、本書のsetup、停止、観測、比較順序を変更してはならない。

CPU runnerとBus runnerは別実装でもよい。CPU runnerはcandidate CPUへreference test devicesを接続する。Bus runnerはcandidate busへTest RAM、Reporter、High Test ROMをmapし、transactionを直接適用する。

## 2. suiteとcoverageのvalidation

runnerは候補runtimeを起動する前に、[Coverage Manifest Format](./coverage-format.md)に従って全coverage fragment、全CPU case、全Bus caseをlogical suiteとして検証しなければならない。

coverage validationにはcase IDの結合suite内一意性、requirementとcaseの双方向参照、spec文書・heading anchorの存在確認を含む。coverageが不正な場合はsuite errorとし、適合判定のために候補runtimeを実行してはならない。

## 3. caseのvalidation

runnerはCPUを起動する前に次を完了しなければならない。

1. TOML syntax、format、version、field、値域を検証する。
2. case IDがcoverage validationで収集したIDと一致することを検証する。
3. binary pathを安全に解決する。
4. binary、memory patch、期待memory rangeがTest RAM内に収まることを検証する。
5. caseが要求するlevelをrunnerが提供できることを検証する。

case自体が不正な場合はtest failureではなくsuite errorとする。候補runtimeを実行してはならない。

## 4. setup順序

runnerはcaseごとに新しいmachine stateを作り、次の順序でsetupする。

1. Test RAMを全byte `0`で初期化する。
2. Reporterをresetする。
3. binaryを`0x00000000`からTest RAMへcopyする。
4. `initial.memory` patchを適用する。
5. `initial.devices`を適用する。
6. CPUをresetする。
7. Level 2以上なら`initial.pc`と`initial.registers`を注入する。
8. retired countを`0`にする。
9. bus traceを空にする。

case間でCPU、RAM、Reporter、trace、fault stateを共有してはならない。

## 5. step実行loop

各step試行についてrunnerは次を行う。

1. 候補CPUへ1命令の実行を要求する。
2. CPUがfaultした場合、命令をretired countへ加えずfault terminationを記録する。
3. 成功した場合、retired countを1増やす。
4. 成功命令がReporter terminationを成立させた場合、それを記録する。
5. `until = "steps"`のretired countへ達した場合、steps terminationを記録する。
6. terminationがなければ次のstepへ進む。

同じstepで成立し得る優先順位は`fault`、`reporter`、`steps`とする。faultしたwriteはReporter terminationを成立させない。

step試行数が`max_steps`へ達しても指定stop conditionが成立しなければ、step limit failureとする。

## 6. 観測時点

termination直後、次の命令をfetchする前に観測する。

- 終了状態の詳細
- retired数
- PCとregister
- 指定memory range
- Reporter状態
- bus trace

観測操作自体によってbus traceまたはdevice stateを変化させてはならない。候補runtimeの通常read APIに副作用がある場合、adapterはsnapshotまたは専用inspection APIを用いなければならない。

## 7. 比較

runnerは次の順序で比較する。

1. actual terminationが`execution.until`と一致するか。
2. `expected.termination`の全指定fieldが一致するか。
3. `expected.state`の全指定fieldが一致するか。
4. `expected.bus`が指定されていれば完全一致するか。

integerは符号なしのbit patternとして比較する。32-bit値は`0..0xFFFFFFFF`の範囲で完全一致させる。memoryはbyte列として完全一致させる。

TOMLに記載されていない観測値はpass/fail判定へ使用してはならない。ただしfailure diagnosticへ補助情報として出力してもよい。

1件でも不一致があればcaseはfailとする。runnerは最初の不一致で停止しても、全不一致を収集してもよいが、共通結果の`result`は同じでなければならない。

## 8. fault時のsemantics

fault caseではCPUがhostへ返した直後のstateを比較する。faultした命令によるregister、PC、deviceへの部分的な変更を、adapterがrollbackして隠してはならない。

候補runtimeがfaultを例外、result value、callbackなどのどの形式で表現してもよい。adapterはそれをKagura fault名とmetadataへ損失なく変換しなければならない。

## 9. 独立性

- runnerは規範入力を別形式から再生成または変換してはならない。
- runnerは期待値を候補runtimeの計算結果から生成してはならない。
- binary loadやinitial state注入をguest命令として実行してはならない。
- runner固有の最適化によってbus accessを追加、削除、分割してはならない。

## 10. suite結果

suiteはcoverage manifestがvalidで、かつ全caseがvalidかつpassした場合だけpassとする。

- case failが1件以上あればsuiteはfailとする。
- suite errorが1件以上あればsuiteはerrorとする。
- runnerが対応しないlevelのcaseはskipではなくunsupportedとし、完全適合判定には使用できない。

## 11. Bus caseの実行

Bus runnerはcaseごとに新しいmachine stateを作り、device reset、`initial.devices` setupの順に実行する。その後、`transactions`を記載順に候補busへ1回ずつ渡す。

各transactionの成功/faultとread valueを`expected.transactions`へ比較し、最後に`expected.devices`をsnapshot比較する。faultしたtransactionをrunnerがrollbackして結果を作ってはならない。候補bus/deviceが返した直後のstateを観測する。
