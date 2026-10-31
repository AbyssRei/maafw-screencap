# 开发、构建与验证

日期：2026-10-30。状态：P0 准备阶段已按计划第 16 节验收，P1 实施中，首先迁移窗口 GDI 与显式局部对照。下节列出已可运行的工具与原型；完整上游对照、SDK 和产品桌面验收仍随 P1–P5 实施。

## 本次连续三轮无输入复测

已新增补齐 15 组旧 FramePool 原生配置的三轮、45 次执行，原始 EXE/DLL 哈希未变，正式期间四路输入事件为零，系统最后输入计时与光标不变，最大观察间隔 79.72ms，监听已确认注销。初版九模式与正式版的四个遗漏上下文、两个不重绘对照分别保留身份；其中 10 组三轮通过，初版四个 pseudo 模式及正式版 alpha=0 对照均三轮 exit 5，仅原已知内部颜色断言失败，布局、原生 owner/关闭、持帧及 apartment 证据通过。它们与下述 36 份失败日志范围分开封存，不能简单相加为全部曾构建程序的覆盖。

随后十个旧 Rust 入口也已连续三轮执行完毕，30 次调用全部通过。四路输入正向控制均来自实际事件，正式期间输入事件为零、系统输入计时与光标不变，最大采样间隔 78.95ms，监听正常注销；59 份冻结输入及 156 份重建源码保持哈希一致。四个 caller apartment、原生 FFI、pseudo helper、GDI/读回与 target-loss 回归、缺失 factory 负向控制、完整 FramePool 生命周期及 pseudo wrapper 均保持原测试选择；独立校验核对 60 个完整生命周期、240 行原/实际窗口状态、确切测试数量与名称、像素断言成功及 dispatcher shutdown。Rust 程序来自原源码/历史 nightly/lockfile重建，使用当前安装的 linker，不能宣称二进制与旧程序逐字节相同；原 DLL 和原像素/状态输入保持确切身份。此批单独封存；更广的旧原生入口身份核查仍在进行，完整 P1 未验收。

2026-10-31，按用户最新要求，**当前本机结论以各原程序最新一次排除输入干扰的连续三轮有效结果为准**。36 份候选历史失败日志对应的程序/配置已全部完成有效三轮，另保留七种原 FramePool 模式；三个有效批次共 132 次调用，按原日志配置计为 **5 份通过、31 份失败，待补为零**。原程序失败包括像素/资源断言、配置/加载、前台控制前提及旧测试假设，不能将 31 份全部解释为库捕获缺陷。该清单穷举的是确切 36 份候选历史失败日志，不是每个曾构建的可执行文件。

首批 13 组修复前程序/配置及七种原 FramePool 模式，每轮 21 次调用，共 63 次；44 份程序/DLL 哈希前后不变，断言、捕获流程和等待均未修改。三轮连续约 197 秒，测试前通过用户真实鼠标及键盘操作验证 Raw Input 与低级鼠标/键盘 hook 四条通道；正式期间事件数为零，系统最后输入计时和光标始终不变，亮屏、active/unlocked、Default 桌面及无 LockApp 条件成立，最大采样间隔约 80.35ms。原生 hook、Raw Input、电源通知均确认注销，临时保持亮屏请求已释放；不记录按键内容。

任一正式输入事件、监听错误、观察间隔超过 500ms 或桌面前提失效，整批作废并保留日志；三个有效批次均无此情况。每个原测试须三轮全部通过才记该配置通过。旧执行保持其原始状态及材料，用于追溯，不覆盖本批当前结论，也不由今天的结果倒推历史失败原因。

| 首批原程序/场景 | 三轮结果 | 当前本机结论 |
| --- | --- | --- |
| 原 WGC/COM 原型全套 | 每轮 12 项通过，hardware/WARP 各完成 20 轮 | 本批通过；旧卸载访问违规记录保留为历史 |
| 原 ScreenDC 两项黑图测试 | 两项每轮均通过，各保留原 20 轮断言 | 本批通过 |
| 较早原 PrintWindow 13 项测试；共享夹具提取后的 inactive/stop 40 轮测试 | 两组每轮均通过 | 本批通过 |
| 原 Desktop Duplication 初始颜色测试 | 失败／通过／失败 | 本批失败，存在轮次差异 |
| 原 FramePool 四种 pseudo 模式 | 四项每轮均失败，exit 5 | 本批失败 |
| 较后原 PrintWindow 21 项测试 | 19 过/2 败；20 过/1 败；20 过/1 败 | 本批失败，pseudo 失败范围随轮次变化 |
| 原 ScreenDC 角部像素及窗口高度夹具 | 两项每轮均失败 | 原夹具本批失败 |
| 原构建缺符号、前台控制前提、Custom 类型常量及 GDI 24-bit 成功假设 | 各自旧测试每轮仍失败 | 按各自构建、环境或测试代码边界记录，不冒充捕获通过 |

首批其余 FramePool plain/opaque 正向控制每轮通过，zero-repaint 负向控制每轮仍不能满足已知颜色断言。完整逐调用结果、四通道输入日志、程序身份、三轮比较及有效性判定均外置保存。

用户随后要求优先补测剩余旧程序，现已顺序完成两批共 23 份配置、69 次调用，每份均连续三轮；执行原程序和断言，保留原缺变量、混合路径及缺输出目录配置。两批分别核对 19/63 份冻结输入，合计 82 份不同输入哈希不变；每次所选测试数量均匹配原日志。第二个原生监听器从第一批开始前一直注册到两批结束，独立复核其完整约 362.15 秒时间线，包括两批衔接期间，四通道原始/低级输入事件为零，系统最后输入计时与光标不变，亮屏、active/unlocked、Default 桌面及无 LockApp 条件成立，最大采样间隔 79.87ms。两批监听的 hook、Raw Input、电源通知和保持亮屏请求均确认释放。

| 补测原程序/配置 | 三轮结果 | 当前本机结论 |
| --- | --- | --- |
| 原日志 5：ScreenDC null/销毁/零客户区的资源拒绝测试 | 每轮 1 项通过 | 本批通过；历史失败保留 |
| 原日志 4：早期 ScreenDC 可见像素程序 | 4 过/1 败；4 过/1 败；5 过/0 败 | 本批失败，第三轮通过不覆盖前两轮 |
| 原日志 3：早期 PrintWindow 13 项程序 | 11 过/2 败；11 过/2 败；12 过/1 败 | 本批失败，子目标资源断言每轮失败，pseudo 失败范围变化 |
| 原日志 16：PrintWindow 27 项程序 | 26 过/1 败；25 过/2 败；25 过/2 败 | 本批失败，前台控制每轮不可用，另外的状态/像素失败随轮次变化 |
| 其余 19 份补测配置 | 每份三轮均非零退出 | 保留逐用例的像素、状态、加载、路径及前台控制失败 |

补测共一份通过、22 份失败；连同首批形成 36/36 份有效复测覆盖。原日志 29 的 method=8 每轮记录 source_passed=false、Rust_known_pixels=true，原日志 30 的 method=4 两项每轮均为 false，原程序仍判失败。原日志 1/33 缺少所需绝对适配路径，9 缺输出路径，18/31/32 在 DLL 加载阶段失败，11/14/16 有明确前台控制前提错误，均按实际边界保留；它们不算捕获通过。当前无输入的轮次差异不能倒推旧执行的失败原因。以上只覆盖这些确切旧程序和本机条件；其他平台及完整 P1/SDK 验收仍未完成。

同源功能代码的原生 [Actions 41100061053](https://github.com/AbyssRei/maafw-screencap/actions/runs/41100061053) 已结束，提交为 `afda7b8c567260c8b8ab747d10a656408a9d9d65`：Linux 成功，Windows 与 macOS 失败。两份官方组件 ZIP 的 SHA-256 已核对；下载后的 ScreenDC 与 DesktopDupWindow 各 38 场景、152 次取帧、380 份快照，以及 DesktopDupWindow 全屏与工作区记录均通过独立 `--check-only` 复算。官方完整 Windows 工程归档也已核对外层 SHA-256，逐项验证同一 F 的 76 份 Git blob、七个组件登记的 7295 份不同输入/日志，以及每份完整产品快照的 382 个 Git 文件；七份原生 CMake 来源路径均指向该固定 F，不能据此接受未覆盖场景。macOS 组件 100 项检查中 88 项通过、12 项失败；同路径阶段记录又观察到三次内容查询均有回调，一次样本提交在该记录时段内未观察到回调，独立阶段分析通过不等于原取帧通过。这些远端结果不计入本机补测。

后续 [Actions 41129758262](https://github.com/AbyssRei/maafw-screencap/actions/runs/41129758262) 的 macOS job 已在取帧前停止：Apple 正式 Xcode 更新到 `27.2 (27B52)`，实际 runner 的三个 `27.2` 路径均为 `27B48` RC，未安装正式 build。P0/P1 两个入口均明确记录 `missing_environment`，各只有发布查询通过、工具链选择及准备缺环境三项结果，没有新增捕获通过或失败。官方 ZIP、job 发布的完整 tar 摘要、两份工程记录各 393/395 个登记输入与日志，以及两份完整产品快照各 383 个 Git 文件均已独立核对；归档批次锁文件单独保留。已冻结确切版本的 GitHub 镜像清单，与实际安装观察一致。这不撤销较早 Xcode 27.1 批次的原生结果，也不把上一个 macOS 批次的十二项失败转记到新批次；正式 build 可用后仍须重新完成对应原生验证。

## 修正 fork 来源的实施交接

2026-10-30 用户确认[ADR 0003](../decisions/0003-corrected-fork-extraction-baseline.md)：以修正 fork 的确切提交作为完整提取来源 F，PR #1587 的修复进入 P1；P2 改为来源冻结与完整提取验收，不再重复打旧补丁。当前 F 锁定至评审后继 `4140ac9674a9fde174212c895c8c37ff3acf1ef8`；早期 a434521e/ee2569a5 及各自证据独立保留。7dc130bb 修正全屏候选复核、精确断言和 CI，3fa6dc4f 隔离 CI 配置以保持上游可合并；4140ac96 前置测试 Logger include，修复根工程构建的头文件边界，生产 helper 字节不变。来源准备与 Rust/C++ helper 同步已实现；下文既有 U/未打补丁来源结果保持实际历史含义，不重命名为 F。完整新版原生/调用链验收仍未完成。

主会话实施顺序：

1. fork 已修正并正常推送：普通窗口按 rcWork 校正，仅 WS_POPUP 客户区屏幕矩形精确等于 rcMonitor 时豁免，删除外窗口 ±8px 条件，保留失败、调整优先级和 flags。新版正式 CTest 2/2 通过，包含 58 个真实头文件脚本案例和 3 个真实 Win32 几何及屏幕像素场景；首次屏幕像素见证失败仍保留。这不等于[计划 §14.3](../plans/maafw-screencap-extraction-plan.md#143-三阶段验收与-pr-专项矩阵)的完整截图验收。
2. `cargo xtask reference prepare-fork --revision client-confirmed-4140ac96 --head 4140ac9674a9fde174212c895c8c37ff3acf1ef8` 准备真实 commit/tree、七条提交及精确六文件累计差异（helper、三份测试、根 CMake 与 test workflow）。v2 策略显式收录 CI 输入，v1 四文件及其原来源清单仍可验证。父修订的来源清单与结果保持原字节。旧 `reference prepare` 和固定常量继续校核历史 U/PR；新记录保存在独立 `baselines/f/`、修订级补丁目录，由索引的 `selected_baseline` 指向。新原生批次只消费该明确选择的 F，缺失时拒绝，不回退旧来源。`reference verify` 仍检查历史记录；来源准备本身不重新登记或改写旧结果。
3. `geometry.rs` 与 WindowApi 脚本已同步，61 个客户区/任务栏/边界/失败及查询间 resize/move/style 变化案例的完整 C++/Rust 调用序列匹配。候选豁免只复核一次，使用新几何回落并传播复核错误；不是原子窗口快照。旧 55 案例结果保留其原身份；私有适配边界由六槽改为八槽以传递实际 rcWork/rcMonitor，并在调用前校核布局，旧适配器明确拒绝。本机显式重放归档 OpenCV/MaaUtils/Boost 输入，从不可变 F 快照重建的适配器另通过 20 轮普通受控窗口 ScreenDC 对照，40 份实际 BGR 文件逐字节匹配已知图样，关闭后持帧断言通过；这组普通窗口证据只覆盖其受控场景；完整任务栏/DPI/多输出及 manager/factory 调用链仍待验收，新的原生依赖在线验收另行记录；保留转换、等待和清理时点，无变化的其他平台旧证据须核对来源及范围后才能引用。
4. 本地历史按用户要求分轮整理并保留旧身份：首次 F 重建准备先于 helper 提取，旧初始快照的 32 项 workspace 测试与 55 组 C++/Rust 对照仍对应其原源码。评审后的 v2 来源准备位于 `89e72e5b`，首次 helper 提取 `e7da2b85` 同步单次复核和 61 案例；原生门禁 `03d64f47` 严格要求五项测试、61/122 摘要及复核标记。评审重建初始快照通过 33 项 workspace 测试及 61 组 helper 对照。两轮逐快照核对无关 blobs，最终整树按预期核验；双仓库 refs/worktrees、bundles、旧新映射、原有未提交工作及各次失败均外置保全。规范分支为 `codex/p1-reviewed-native-gates`，旧分支保留，没有强推；原生依赖明确重放归档，不将这些结果称为新一轮原生依赖在线验收。当前 4140ac96 仅为同一生产 helper 的测试构建后继，另存不可变来源，不重写历史结果。

P1 尚未完整验收；P2 来源冻结和 P3 SDK/原生现代化门禁不因该修订提前通过。原 FramePool 包装已知颜色失败、WGC 卸载异常及其他历史失败继续独立保留。ee2569a5 曾通过 build/test 并转为 Ready for review；3fa6dc4f 的根 CI 在像素断言前编译失败，原失败保留；4140ac96 的 test/build 已按该确切 head 通过，Windows 根工程几何 CTest 2/2 亦实际执行；PR 保持 Ready；这不是合并或本项目全平台验收。

## 当前仓库与本机命令

PNG 修复随后已在 [Actions 41184885199](https://github.com/AbyssRei/maafw-screencap/actions/runs/41184885199) 的确切 `b9e9f42` Linux PipeWire 组件通过，154 项组件检查全部通过。官方轻量 ZIP 摘要匹配，下载后复核 40,043 个实际登记路径和全部检查日志；原始图像、身份、元数据、实际虚接口、重入与新增编码/缩放的六项数据校验均通过，144 组、288 张实际捕获/持帧及 576 次缓存解码另行复算，真实 PNG 探针记录 libpng 1.6.59/zlib 1.3.2。轻量包包含 487 个未提供的输入或一字节占位，未用它们冒充完整源码/依赖归档；完整来源审计仍待完成，该结果也不覆盖后续新 raw-type 矩阵、其他组件、macOS 或完整 P1。原 41174221177 的 PNG 夹具失败保持失败。

Custom 现另有实际原始像素类型矩阵：显式适配器根据编译头文件查询 13 种深度及 128 种通道，共 1,664 种规范类型，涵盖 16F/16BF、Bool 和扩展整数类型；这些值仅用于私有对照，不是公开 SDK 格式。原 `MaaImageBufferSetRawData` 函数体保持不变，安全验收桥先检查类型、保留深度和可读 span，再在同一原/Rust manager 回调位置调用真实 setter。Windows 归档依赖开发验证的 Debug、Release 各通过 4,992 组、9,984 次真实调用；0/1/255 返回值、输入覆盖后的 clone、false 输出保留、9,984 个 Mat 在 input/context 结束后持有和模块卸载后的独立 CPU 字节读取均核对，两个 profile 的 9,662,208 字节捕获和持帧流逐字节一致。

六类短 span、保留深度、额外类型位及溢出几何输入均在调用原 setter 前被拒绝，原 helper 调用计数和已有输出不变；七组篡改记录被独立 `upstream-trace custom-raw <外部 raw 目录> [--check-only]` 拒绝。实际 OpenCV 类型头随原生批次归档，零测试/缺失完整计数不能通过入口校验；六项既有 Custom native/data 回归、workspace 测试和 Clippy 通过。首个 standalone 命令因父输出目录未创建在实际捕获前失败，记录保持失败，补齐外部目录后使用相同程序执行；新增的 span 拒绝矩阵另有后续独立执行。上述本机结果仍是 Windows 开发对照，不表示所有编码格式、类型标志组合、颜色转换或完整 Custom/P1 已验收，核心截图算法未变。

随后 [Actions 41186951837](https://github.com/AbyssRei/maafw-screencap/actions/runs/41186951837) 的确切 `2730b87` Linux PipeWire 组件通过全部 156 项检查，包含新增 raw 原生与实际记录入口。官方轻量 ZIP 的 SHA-256 与登记摘要一致，下载后独立复核 39,965 个实际登记路径及全部检查日志；原图像、编码/缩放、身份、metadata、实际虚接口、重入与 raw 七项数据校验均通过。Linux 的 1,664 类型、4,992 案例、9,984 次原/Rust helper 调用及 9,984 个保留 owner 均核对；类型目录、逐例记录、六类拒绝记录、作用域/释放计数和两个各 9,662,208 字节的实际捕获/持帧流与 Windows Release 逐字节相同。轻量包有 491 个未提供或占位输入；其后官方完整工程 ZIP/tar 均核对摘要，已逐项验证该组件全部 40,456 个登记输入/日志路径、同一固定 F 的 76 份源码 blob 和全部 402 个产品 Git 文件，另严格核对单独归档的 Cargo.lock。两个实际生成的 Custom helper 与已审阅 Windows 输入仅有 CRLF/LF 差异，源码文本对照只规范换行，各自原始字节摘要仍严格匹配。首轮校验误将额外锁文件计为 Git 文件、第二轮误要求跨平台生成文本原始字节相同，失败日志及原脚本均保留；修正校验后通过，没有更改 native 输出。该完整来源审计只覆盖此组件，其他组件、平台和完整 P1 仍未由它验收。同一 run 的 macOS 两个入口日志明确停在正式 Xcode `27.2 (27B52)` 缺失前提，未新增实际取帧结果；不替用 RC 或覆盖早期失败。

PNG 参照依赖现由 `scripts/prepare-png-codec.ps1` 显式准备：在线批次每次查询正式 libpng/zlib，登记版本、确切 Git 来源、规范源归档与许可，构建静态库并运行六个已知 RGBA 像素的真实编解码探针；Windows 及 Unix 的 imgcodecs 准备使用相同依赖，并检查 OpenCV 确实选择这些库，禁止静默回落到其他 PNG/zlib。libpng 使用官方主页的当前公开版而非没有 Release 条目的 GitHub 镜像；本次为 libpng 1.6.59、zlib 1.3.2。离线批次必须包含登记的 release/page/resolution/source 输入；缓存复用仍进行在线更新检查。

该准备路径已通过 Windows 开发验证：在线、缓存复用和离线重建均完成实际 PNG 编解码，OpenCV 保留 IPP/CPU 优化并选中确切库，实际 Custom 的 144 组编码/缩放、96 组原始回调以及身份、元数据、虚接口和重入回归均通过，输出另经独立校验。路径校验的首个 CRLF 错误、首个旧头文件恢复失败及诊断脚本的测试入口/负向夹具错误均保留记录。原 [Actions 41174221177](https://github.com/AbyssRei/maafw-screencap/actions/runs/41174221177) 的 Linux Custom 在 PNG 夹具创建时失败；原准备确实关闭了 PNG。新的 Linux 原生运行尚待验证，本机开发通过不覆盖该失败，也不替代新的正式工具链/全平台或 P1 验收。核心依赖树和截图算法未因这次准备修正而改变。

离线 meojson 恢复已补齐确切字节校验：一份 Windows 正式归档的 19 个头文件受 Git `core.autocrlf=true` 影响保存为 CRLF，而实际消费记录为 LF。逐项与 Git blob、原消费文件及归档比较后，确认仅 CRLF→LF 即能恢复全部登记哈希；恢复器仅在该转换结果精确匹配已登记 SHA-256 时写入，其他差异仍拒绝。新的源归档显式固定 `core.autocrlf=false`，可另存逐头文件恢复记录。旧归档、原始失败及恢复前字节保持不变；19 份旧归档和 19 份规范归档的实际恢复、错误摘要拒绝均已验证，这不是新的截图验收。

Custom 现沿同一私有 manager 增加原 `request_uuid` 与 `get_features` 迁移，不执行输入动作，也不建立公共 provider ABI。Windows 显式归档重建已验证原函数体及真实 MaaUtils StringBuffer：64 组原版/Rust 用例保留空、ASCII、NUL/非 UTF-8、长字节串和完整 64 位能力值，另观察借用回调表增删、缺失 manager、真实 C++ 异常及之后的独立查询。132 份完整 UUID 输出、20 次 Rust 原生栈缓冲作用域和 64 对持有结果已由 `upstream-trace custom-identity` 独立复算；Rust CPU 字节在模块卸载后另在线程读取。原 96 组截图/192 份像素回归仍通过。首个探针与 MaaUtils 归档有一份 header 不同，来源关联失败保留；后续显式重建从已验证归档新解出全部 50 份 header。首个新在线批次因运行时文档改变触发 source-stability 失败，固定整树后的 Rust 批次十二项检查通过，但当时 VS2026 稳定版已更新至 18.11.3，已安装的 18.11.2 因此在原生构建前记录缺环境；SDK 10.0.28000.2957 已就绪。Actions 41135367620 的 Linux UUID/features 用例已通过，下载后的 132 份实际输出亦独立复算通过；新 macOS 入口仍缺正式 Xcode 27.2/27B52。

Custom 的 `get_info` 现也迁移至同一私有 Rust manager。Rust 保留默认对象创建、当前借用回调、非零返回值、缺失/false 默认、独立对象获取及最后覆盖 `type=custom` 的顺序；显式验收适配器使用真实 StringBuffer 和 meojson parser/object，不给核心添加 JSON/OpenCV 依赖。原版接受尾随注释、NUL 后终止、非 UTF-8 字符串及 `1e1000`，拒绝重复字段；默认 serde 诊断曾在九类输入上不匹配，原记录保留，未用它替换源解析语义。Windows 归档依赖重建通过 37 类载荷、592 组源码/Rust 对照及 1,193 份实际输出字节复算，覆盖 null/table/回调变化、真实 C++ 异常与之后的独立查询。152 次原生 StringBuffer 作用域、113 次 parse、597 次 type 替换和全部实际对象释放均核对；解析/对象复制位于临时 buffer 内，type 替换位于其释放后，反向调用会被适配器拒绝。592 对原生对象在 callback/context 销毁后继续读取，独立 CPU 字节另在模块卸载后跨线程读取。opaque 对象 head 的额外记账分配显式保留；它不是原生对象移动/并发契约。新 `upstream-trace custom-metadata <外部 metadata 目录> [--check-only]` 按原始字节验证 JSON，不经另一解析器损失数字/编码信息。最新本机/Linux 在线局部门禁见下文；framework/loader、通用故障/重入、编码/resize/全部类型及 P1 继续未完成，原生捕获接口的新入口见下文。

后续 Actions 41141355002 的 Linux 元数据原生用例与下载后的 1,193 份实际字节复算通过，但整个 PipeWire 组件命令在摘要校核时失败：外层和 Custom 两次 meojson 准备写入同名 `meojson-source-archive.log`，覆盖了先登记的日志。原失败和两份登记摘要保留；嵌套准备现显式使用 `custom-meojson` 步骤前缀，实际最新正式版的两次准备已验证所有输入/日志摘要不变，后续 Actions 41147180960 的 Linux 133 项组件检查全部通过。它不把原命令改记为通过；macOS 该批仍在取帧前缺正式 Xcode 27.2/27B52。

Custom 另新增 null 回调表的私有创建拒绝和真实原生接口消费：完整原 API/manager 两份 cpp、原公共头和基类按固定 F 编译，仅覆盖日志；真实 `MaaCustomControlUnitCreate/Destroy` 和 C++ 虚表消费捕获、连接、身份、能力及元数据，同一 Rust manager 经窄测试桥接入。原输入方法仅为编译完整参考类保留；Rust 测试类未调用的输入/app/shell 槽返回 false，不构成输入迁移或完整 framework 接口承诺。Windows 归档输入的二十轮生命周期及 1,308 次源码/Rust 虚接口查询、四组实际基类/混入类型、七类真实回调异常及后续独立调用、二十次实际析构后的 Rust 状态通知均通过。原生调用方的 56 次 Mat 替换与 56 次 UUID 替换均在其临时 buffer 存活时完成；作用域外调用被拒绝，metadata 继续保持原解析/替换顺序。最初字节正确但延后原生调用方替换的桥已保存为诊断快照，不作所有权时序验收。二十对真实 Mat、字符串和对象持有至 unit/context/回调状态结束后，独立 CPU 像素在模块卸载后跨线程读取。opaque head 增加记账分配，独立 Rust UUID 表示增加字节复制；没有第二份整帧复制。`upstream-trace custom-native-api <外部 native-api 目录> [--check-only]` 独立复算实际输出、布局、调用顺序及作用域/owner 记录，并复核同批原生图像 ABI；六种篡改记录均被拒绝。后续本机/Linux 原生入口见下文；macOS、通用故障/重入和 framework loader 继续待验证，控制计数不是一般堆泄漏证明，完整 P1/SDK 未验收。

Custom 另新增同一存活对象的同步回调重入入口：connect、connected、inactive、features、UUID、screencap、get_info 七种外层回调各嵌套七种真实虚接口操作，分别执行正常内层调用与回调内临时替换原表项后抛出真实 C++ 异常，再恢复同一借用表并继续外层操作。Windows 显式归档依赖下二十轮生命周期，Debug/Release 各通过 7,840 次源码/Rust 虚接口查询与 1,960 次嵌套异常；内外层 UUID、图像和 JSON 使用不同内容，240 个原生值跨过 unit/context 销毁仍保持，独立 CPU 字节在模块卸载后跨线程读取。

首个实际对照中，原 C++ 的 UUID→UUID 成功，Rust 验收桥因要求 active scopes=1 而返回错误 1；原失败、当时 Rust 程序与原 DLL 均保留。仅修正私有验收桥：图像作用域使用 owner-thread 栈，字符串作用域另绑定真实 buffer 和单调身份，metadata 记录本操作结束后应恢复的外层身份。它允许内层完成时外层 buffer 仍存活，保持每个操作自己的 parse/as_object 发生在 buffer 内、实际 Mat/UUID 发布发生在其 buffer 结束前、type 替换发生在自己的 buffer 结束后。七项控制仍拒绝作用域外发布、跨当前 buffer 解析、内层存活时提前替换外层值及内层自身提前替换；两层分别结束后的正确替换与字节也须通过。未修改原源码或核心 manager 的行为。

`upstream-trace custom-reentry <外部 reentry 目录> [--check-only]` 独立复算 4,728 份实际数据、回调顺序/深度、完整能力位、Mat 布局/owner、内外层输出、scope 控制与释放记录；七种宽度、深度、控制、owner、像素、持有值和析构篡改均被拒绝。既有图像 96 例、身份 64 例、metadata 592 例及真实 API 1,308 次查询回归通过。Windows 与 Unix 原生脚本均接入新精确测试、完整摘要/零测试拒绝及独立记录复算；正式在线与 Linux 后续结果见下文。这只是同 owner channel、一次嵌套的来源行为对照，不定义公共 SDK 允许的重入、并发使用或活动销毁，通用分配失败、framework loader 和完整 P1 仍未验收。

确切 `561238723b40620244c5674de937d1222de0e048` 的本机正式在线批次随后通过全部 27 项原生检查。独立核对 7,663 项输入/日志、全部 394 个产品 Git 文件、十份实际原 API/头文件与固定 F 的字节，并保存正式测试程序、DLL 和锁文件；全部 Custom 实际输出与此前归档输入下的最终 Debug/Release 记录一致。VS2026、SDK 和 MSVC 身份与下述更新后工具链相同。[Actions 41167609520](https://github.com/AbyssRei/maafw-screencap/actions/runs/41167609520) 的 Linux 组件通过 135 项检查，下载后的五个独立记录入口均通过；4,728 份重入数据及图像、身份、metadata、实际 API 全部输出与本机一致，十份原 API/头文件与固定 F 一致。官方轻量包摘要与 37,897 份可用登记文件亦已核对；其中另有 472 份登记输入未包含在轻量包内，不把它描述为完整依赖/产品快照审计。该批 macOS 仍在取帧前缺正式 Xcode 27.2/27B52。

Custom 新增独立编码/缩放回调入口：原 `MaaImageBufferResize`、encoded getter/setter 函数体与固定 F 字节一致，实际 MaaUtils ImageBuffer 与 PNG codec 保持在 callback 原调用点；Rust 沿同一 manager 接收其真实 Mat。144 组组合覆盖灰度/BGR/BGRA PNG、无效编码、非空地址的零长度、NULL 拒绝、0/1/255 回调值、原尺寸、2× 上/下采样、单边自动尺寸、双零及负尺寸。Windows 显式归档输入下，Debug 与 Release 各运行二十个生命周期、288 次源码/Rust 调用，均通过；每组另有 576 次独立缓存解码检查具体像素，24 次负尺寸 C++ 异常被测试边界容纳，288 个真实 Mat 保持到其 callback/context 结束后，独立 CPU 字节另在线程读取并跨过模块卸载。缓存输入覆盖后，原字节仍保持；false 保留原 caller 输出，成功空图仍覆盖它。诊断编码复制和 cache decode 是额外测试工作，不称为消除的捕获开销。

`upstream-trace custom-images <外部 images 目录> [--check-only]` 独立复算全部 2,025 份数据、布局/owner、PNG chunk CRC、图样、缓存及持帧记录，Debug/Release 数据完全一致。首个独立校验器错误地把所有空 Mat 头都当作默认值：实际 `clear` 对已装入原始图的 Mat 调用 `release`，保留其类型/通道/步长，读取跨度和 owner 已归零；无效 imdecode 则是默认空头。失败的校验器、程序与原数据均保留；现按这两类来源分别严格校验，并要求源码/Rust 完整头一致。九组像素、持帧、cache decode、PNG CRC/尺寸、空头、owner、调用行及作用域计数篡改均拒绝；Windows/Unix 的真实入口拒绝八组零测试/缺摘要变体。原图像、身份、metadata、实际虚接口和重入回归输出与此前正式批次一致。后续确切 `59f5323` 的本机在线批次通过全部 29 项原生检查，独立校核 9,694 份登记输入/日志、397 个产品 Git 文件、十份原 API/头文件及新增原 helper 函数体；所有实际输出与归档输入下的最终数据一致，正式程序/DLL/锁文件已另存。其他平台、其余 codec/raw 类型、任意缩放与通用故障仍待验证；不是公共 SDK 或完整 P1 验收。

用户更新后，VS2026 `18.11.3 / 18.11.12328.437` 与 SDK `10.0.28000.2957` 的新正式在线批次通过全部 25 项原生检查；实际参考编译器为 MSVC `19.51.36260.0`。准备、编译及验证期间产品源码保持不变，独立核对了 2,930 项输入/日志、固定 F 的 76 份来源及全部 392 个产品 Git 文件。图像、UUID、元数据和实际 C 创建/销毁/虚表门禁均通过，测试二进制与锁文件另存。

[Actions 41147180960](https://github.com/AbyssRei/maafw-screencap/actions/runs/41147180960) 的 Linux 133 项组件检查及下载后 33,283 份可用登记文件校核亦通过；两平台各十份原 Custom API/manager/公共头与 F 原字节一致，真实编译命令消费这些来源，612 份 API 数据及 1,193 份元数据字节跨平台一致。轻量 Linux 包不含完整依赖与产品快照，不将其描述为完整源码归档审计。

随后 [Actions 41164682813](https://github.com/AbyssRei/maafw-screencap/actions/runs/41164682813) 的 Linux 完整官方归档已补齐独立审计：70,787 份登记输入/日志、19 份完整产品快照（各 392 个 Git 文件）、F 全部 744 份原文件与来源归档、76 项显式来源及真实编译关联均核对。该批 Wlr/PipeWire 组件分别通过 131/133 项检查。其源码为 `7cf8896`，尚未包含后续重入切片；完整归档的结果不转移到其他批次。随后 `5612387` / Actions 41167609520 的完整官方 Linux 包也已核对 ZIP、完成 job 报出的内层 TAR 摘要、75,550 份登记输入/日志、F 全部 744 份文件/原归档与 76 项来源，以及 19 份各 394 个 Git 文件的产品快照；其中一个为按原测试主动改成不可编译的消费者，两个预期失败的配置记录仍保留失败状态。三份实际编译关联及十份 Custom 原 API/头文件均匹配固定 F，7,196 份实际 Custom 输入/数据另解出。这补齐该确切新批次的完整来源审计；原生语义仍按各组件及独立记录校核范围记录。

该批 Windows Custom 在编译前失败：复用的 PrintWindow 离线清单没有新增 `meojson-source.tar`。现与 manager 一样为 Custom 单独在线准备并刷新 runner 工具链，保留离线入口的完整输入要求；后续 Actions 41164682813 与 41167609520 的 Custom 步骤均已通过，不把原准备失败归为采集断言，也不改写原结果。完整 Windows job 与其余组件各按实际结果记录，该批历史四项 FramePool pseudo 失败与 macOS 正式 Xcode 缺环境继续保留；完整 P1 仍未验收。

Custom 的私有迁移入口为 `upstream-parity custom`，首批正式编排限 Windows x64/VS2026，不要求可见桌面。核心 `custom-capture` feature 只编入通用回调算法，无原生运行依赖；原生验收显式编译固定原 manager 的构造、connect、connected、screencap、inactive 函数体、完整原 callback table 和原 raw setter，并使用真实 MaaUtils ImageBuffer/vtable 与 OpenCV core/imgproc/imgcodecs。局部本机对照已运行 96 个案例/192 次源码与 Rust 截图调用，覆盖缺失 table/回调、0/1/2/255 返回值、三种实际头文件定义的 8-bit 类型、成功空图、失败保留旧输出、raw setter 克隆、修改借用表和 C++ 异常后独立调用；36 个 Mat head 保持到 context/input 销毁后，最终 head/context/adapter scope 配平。独立 `upstream-custom` 逐字节复算全部 192 份输出及原生 ABI/资源记录。首个正式在线批次的 22 项原生检查全部通过，但命令最后因归档验证器漏识别新组件前缀失败，原日志保留。补齐前缀和既有 scope/parent/phase 测试后，`cargo xtask reference verify-run runs/<run-id>/manifest.json` 已只读校核该批次完整输入；它不重新截图、不登记或改写状态，不能把原命令失败变为通过。修正后的 876cfbb Actions 入口已通过 18 项离线原生检查，下载后的 192 份实际输出、ABI 与 owner 记录亦独立校核通过；该早期图像批次本身不验证 manager 虚表/create-destroy、UUID/info/features 或其他平台；后续切片见上文，encoded/resize/全部类型和完整 P1 仍未验收。OpenCV 5 的类型常量由实际编译头读取，不使用旧主版本的位打包假设。

默认入口仍是 Rust 2024 Hello World binary。内部 library 已实现首个窗口 GDI 迁移与显式局部对照入口，尚无完整 U、公共 SDK 或 binding；独立 workspace xtask 与 P0 原型继续可用。`.github/workflows/p0.yml` 定义三个原生 runner 的工具验证和现有原型；Linux 已验证私有 PipeWire 流；macOS 已验证受控窗口 ScreenCaptureKit 主线程原型。P0 汇总退出审阅结合固定来源/许可范围、职责与用例定义、本机最新工具链原生验证和三平台证据，已确认准备门禁通过；完整 U 对照仍待 P1。单次原型批次的 incomplete 和待办保留该任务的局部/历史范围，不独立表达整个阶段结论。命令从仓库根执行，本机只验证宿主 OS，不以 WSL 代替 Linux 验收。

先更新 nightly，再改写 CARGO_HOME；若已经改写，使用 `rustup update nightly --no-self-update`。随后将可清理缓存及构建输出放入同级 deps：

```powershell
rustup update nightly --no-self-update
$env:CARGO_HOME = "$PWD/../maafw-screencap-deps/cargo-home"
$env:CARGO_TARGET_DIR = "$PWD/../maafw-screencap-deps/target"
cargo build --release
cargo run
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Windows 默认 workspace 测试覆盖分配原型、通用回调、PipeWire 消费/CPU 行所有权、各平台 manager 逻辑及工具记录校核，实际数量与忽略项以当次日志为准；Unix 另有 symlink 边界检查。这些测试不执行真实桌面取帧。选择 `linux-pipewire` 后，原生 driver 取帧测试仍须显式启用。Windows GDI 桌面原型仅在 `prototype-windows` 下编译，默认忽略，须显式串行执行：

```powershell
cargo test --release --test p0-interop --features prototype-windows-com -- --include-ignored --test-threads=1
cargo clippy --workspace --all-targets --features prototype-windows-com -- -D warnings
```

`prototype-windows` 在自有小窗口绘制确定颜色，以 GetDC → compatible bitmap → BitBlt → GetBitmapBits 实际取帧，校验 BGR 字节，并在释放原生资源后验证独立像素及跨线程释放。`prototype-windows-com` 叠加私有 rich 投影与三项测试：平衡 WinRT 初始化计数、宿主 STA 拒绝 MTA 后保持 apartment，以及显式 WARP 设备的 D3D11 → DXGI → WinRT 对象身份往返与 staging 像素读回。后者校验非对称两行 BGRA，按实际 RowPitch 只复制目标像素，释放所有原生对象后跨线程持帧，并核查线程 DPI 上下文。WARP 是明确选择的软件设备，不是硬件失败后的回退。

第四项 COM 测试创建使用上游 `Create` 入口的 FramePool，核查未启动会话时的“原生 HRESULT 成功 + 空帧”与 rich 投影空错误的差异，并验证未启动会话的事件注册、显式注销 HRESULT、Close 和闭包持有状态释放。该用例未启动捕获，没有在途回调，不能作为事件送达或回调排空证据。维护输入还显式选择窄投影需要的工厂接口壳及 delegate 类型，避免生成代码缺少依赖；不修改生成器输出，也不调用 P3 的 CreateFreeThreaded。

另有两个显式桌面用例，分别固定硬件与 WARP 设备（不互相回退），各执行 20 次 FramePool 创建/抓帧/关闭。它们在自有 MTA 线程创建当前线程 DispatcherQueue，沿上游 `Create` 入口捕获自有窗口，验证真实 FrameArrived 的线程、GPU staging 读回与已知内部像素、原生资源释放后的跨线程 CPU 持帧、显式注销和队列关闭完成后的回调状态释放。使用 `windows-future` 只为原生 IAsyncAction 类型，按原生消息循环等待，不引入通用异步运行时。目标尺寸取原生捕获项，避免将窗口创建尺寸直接当成物理像素。

本机连续 MTA 退出/重建原型曾在生成的进程级工厂缓存中复现访问违规；当前改为在所属通道显式取得工厂，以生成的窄 ABI 调用并在 apartment 退出前释放。没有通过永久初始化 MTA 来掩盖计数/寿命问题。失败及修复后的结果分别归档。

同一 feature 还包含显式 Desktop Duplication 原型，已通过本机硬件及原生 Actions 验证。自有窗口从首次呈现起使用本轮已知颜色，在默认硬件设备的实际 adapter 中选择包含目标区域的输出，以旧 `DuplicateOutput` 入口执行 20 次设备/输出/采集生命周期；没有切换到 P3 的 DuplicateOutput1。每次有限等待、按原生 RowPitch 仅复制自有窗口内部像素，在 Unmap 前取得独立 CPU 存储，检查成功 ReleaseFrame 后再次释放得到 INVALID_CALL，并验证第一帧跨后续原生生命周期保留及最终跨线程消费。记录 adapter、目标尺寸、pitch、原生 LastPresentTime 与 AccumulatedFrames；不改变线程 DPI 上下文。初次夹具先呈现另一颜色导致旧帧断言失败，证据保留；修正首次绘制颜色后通过，没有按像素失败重试。该原型明确要求未旋转输出，未覆盖多 adapter/输出切换、ACCESS_LOST、完整超时与格式矩阵，也不是固定上游 DesktopDup 的行为对照。

这些原型不提供产品截图 API，不构成六种 Windows 方法、完整 Desktop Duplication/FramePool 异常/重建/权限矩阵、混合 DPI 或多显示器验收；P3 的 DIB 现代化与全虚拟桌面路径均未提前实施。四项基础 COM 测试不需要桌面交互，默认在选择该 feature 后执行；GDI、两个实际 WGC 和 Desktop Duplication 用例仍须显式启用 ignored 测试。具体本机和 runner 结果分别以外部运行记录为准，缺硬件/桌面或捕获能力不能记为捕获通过。

## 当前 P1 迁移与对照

DesktopDupWindow 新增独立原版进程与 Rust 的 38 场景原生几何矩阵，由 `upstream-parity desktop-dup` 显式编排。本机原版与 Rust 各完成 76 次截图，包含四边 ±1/±7/±8/±9、精确全屏、带边框 popup、普通窗口、平移和过大窗口；`upstream-trace desktop-geometry <外部 window-geometry 目录> [--check-only]` 独立复核完整客户区、外框、WINDOWPOS、380 个快照、全幅已知图样和一致的 16×16 更新标记、152 份关闭后持帧文件及 Rust 图像 owner 归零。原版 ROI 实际保留整屏 BGR Mat 行距，文件按目标行导出；Rust 显式安全复制取得紧密目标行距，两种布局分别记录，不因导出文件紧密就声称原生 ROI 紧密。每侧先调用其真实 helper 并等待异步调整完成，截图期间不得重复调整；不改变原版预热、等待、整屏转换和裁剪时点。该结果限本机单输出、96 DPI、底部任务栏的调整完成后捕获，不代替首次调整仍在途的截图、多 DPI/任务栏/输出、完整故障矩阵或 P1。原生依赖明确重放归档 OpenCV/MaaUtils/Boost；新增 Actions 编排的原生结果待运行。

ScreenDC 新增显式原生几何矩阵，由 `upstream-parity screen-dc` 在独立进程中编排；本机 38 个场景、双方 76 个原生窗口及 152 次实际截图已通过，380 份原生几何快照和全部 BGR 文件亦由 `cargo xtask upstream-trace screen-geometry <外部目录> [--check-only]` 独立复算。覆盖客户区四边 ±1/±7/±8/±9、精确全屏、客户区匹配但外框不同、普通窗口的工作区/任务栏范围、评审中的平移和过大窗口。每侧先直接调用其真实几何 helper，等待自有窗口的异步位置消息完成，再沿未修改的 ScreenDC 完整路径连续取两帧；断言完整客户区及外框、原生 WINDOWPOS 消息、全幅已知像素、重复捕获不再校正、GDI 清理及窗口关闭后的独立 CPU owner。它验证校正完成后的捕获，不代替首次异步校正尚在途时的截图行为或完整调用链。

该自有窗口夹具在实际 UI owner 上设置及查询几何，复制请求后才投递，关闭时 join；不改变被测 helper、捕获或等待逻辑。首次带边框窗口有 38 个角部像素不符，原失败、原始文件、源码和 DLL 独立保留；另设 `DWMWCP_DONOTROUND` 后全幅断言通过，不忽略边缘像素。随后普通窗口受原生最大 tracking size 约束，未达到请求的客户区高度，夹具前提失败亦保留；现由自有 WindowProc 显式允许该次请求的最大尺寸，记录实际 tracking 尺寸及方角属性 HRESULT。当前批次使用单显示器、底部任务栏和 96 DPI；其他任务栏位置、自动隐藏、多显示器负原点、不同 DPI、最大化/最小化、DesktopDupWindow/manager 转发和 P1 完整验收仍待完成。原生依赖明确重放已归档 OpenCV/MaaUtils/Boost，不据此声称新一轮原生依赖在线验收。新增 CI 编排尚待运行。

macOS 新增显式原生样本入口 `upstream-trace macos-sample-matrix`，随 `upstream-context macos` 编排；原生 Actions 的实际测试与记录校核已通过，下载后六百四十份像素文件也已独立逐字节复算。验收桥接以 `CVPixelBufferCreate` 请求行对齐，读取实际步长与可读跨度，初始化目标像素和全部填充，再创建真实 ready `CMSampleBuffer`；四种奇数宽度各运行二十轮，实际步长分别为 64、192、384、256 字节，分别调用独立编译的未改写原 `image_from_sample_buffer` 和 Rust `with_sample`。每轮保存原生输入、实际覆写后的输入、双方转换与原格式输出；原生引用关闭后，二百四十个 CPU owner 留到另一个已 join 的线程读取和释放。校核器复算完整像素、布局、填充、原生 owner/写锁与 Rust 读锁记录，实际原生编译记录确认原函数独立编译且未注入 dispatch 观察器。本机仅执行工具检查与下载记录复算；原生 fixture 引用释放不等于证明系统分配器立即回收。该批 macOS 组件八十四项检查中八十项通过，既有同路径 display-patch、host-premise 原 oracle 及其配对校核四项继续失败；新样本结果不覆盖或重标这些失败。该用例不请求截图权限、不初始化 AppKit，也不代替桌面取帧、完整格式/错误矩阵或 P1 验收。

Linux 旧 Wlr/KWin 兼容构造路径已开始私有 Rust 迁移，保留输入配置值及截图 path/fd/node 的原始转发；它没有实现输入控制或 Portal 授权。显式构造对照编译完整原 `WlRootsControlUnitCompat.cpp`、`KWinControlUnitCompat.cpp`，消费真实 C++ 原接口、配置和 manager 查询函数，覆盖 120 组参数、空指针/空字符串/NUL、原始字节、MaaBool、整数边界和 host 调用顺序。Portal 调用是明确的脚本边界：原 Open 失败保留的六个 helper 残留先记录，再由夹具回收，不称为产品清理成功。`upstream-config linux-compat` 生成原记录，`upstream-config linux-compat-records` 复核 Rust UUID、元数据和事件；Windows 本机 120 组完整原函数/Rust 对照已通过（38 组构造成功、六组 Open 失败残留），独立校核也拒绝双侧相同的错误元数据；随后 Linux 原生 PipeWire 113 项组件检查全部通过，其中完整 120 组兼容构造记录已下载并独立复算。此项不连接输入或截图后端，不据此接受真实 Portal fd 所有权或完整兼容入口截图转发；原生接入仍须沿保留这些配置的同一捕获工厂完成，不能静默改成 Input=None。

Wlr 另新增真实协议失败入口，随 `upstream-context wlr` 原生运行；当前 Actions 结果待验证。源码与 Rust 各执行 20 次独立连接，每次依次收到 Buffer 前的 Failed、Ready、实际覆写同一 SHM 后的 Failed、再次 Ready，以及等待时服务端主动断开该 client。它检查失败时保留调用方输出、失败路径不调用 Rust 转换器、相同布局复用同一 `wl_buffer`、断连后原 connected 标志不自动清除、独立成功图像持有到全部原生资源关闭，以及 fd/任务退出。源调用方在后续成功写入前显式分离 Mat，避免把原版可写输出别名误称为独立帧；这项宿主操作不是后端修复。`cargo xtask upstream-protocol wlr-failures <外部目录> [--check-only]` 校核实际请求、120 份 SHM 输入及双方即时/持帧等共 440 份像素文件。刻意断连后的 Rust 原生清理错误与主要失败分开记录，fd/任务数量回落不证明所有协议销毁请求成功。该已声明的负向场景不重试失败断言；任意断连时点、活跃快照下显式 client close、完整故障矩阵及 P1 仍未验收。

Wlr 新增显式 registry/SHM 验收入口，由 `upstream-context wlr` 编排；原生 Actions 的 117 项组件检查已通过，下载后的 160 次实际取帧、560 份像素文件和完整请求/退出记录亦已独立复算。源码与 Rust 各自独立的真实 libwayland-server 进程从两个输出开始，在同一连接内执行 20 轮公告新输出、保留旧截图对象、移除新输出及再次构造，共各 80 次取帧。不同输出使用不同像素，客户端快照 ID/公告版本须与服务端实际请求相符；另保存关闭后的 BGR 持帧、实际 SHM 输入、fd/任务与原生服务端对象退出记录。`cargo xtask upstream-protocol wlr-registry <外部目录> [--check-only]` 独立复算这些记录。它保持原版“最后公告输出”、构造时快照及忽略 `global_remove` 的行为；此夹具没有 compositor 或宿主桌面，尚不代表多显示器桌面、活跃快照下 client close、完整故障矩阵或 P1 验收。

可选外层 Windows 截图委托现有私有 Rust 状态迁移及 `cargo xtask upstream-delegate win32 <外部记录目录> [--check-only]`，由 `upstream-parity manager` 显式编排。它仅覆盖固定 `GamepadControlUnitMgr` 的截图初始化/转发/停用、完整 `Win32ControlUnitLoader` 与 MaaUtils `LibraryHolder` 源函数体，不实现手柄或输入操作。真实原接口声明、Boost function/shared owner/mutex 与 OpenCV Mat 在 C++ 模型中消费；该模型的 DLL/日志/路径和底层截图结果仍是明确脚本边界。274 个案例、548 个阶段（含 8 个可选方式未选择阶段）已在本机匹配，覆盖可选条件、64 位选择器、两个输入选择器 None、加载/符号/构造/连接失败及异常、直接输出修改、空图、停用和关闭后持帧；实际双侧记录另校验像素、owner/event 配平与失效视图未调用。

另已实现独立的真实 DLL 消费切片，由 `cargo xtask upstream-delegate win32-native <外部记录目录> [--check-only]` 复算实际记录。私有平台 loader 按实际 Boost Windows 顺序尝试固定名 `.dll`、`lib` 前缀及无扩展名，使用原生 flags=0，不修改进程搜索路径。标准 Arc/Weak 在同一缓存互斥锁下共享模块，最终 FreeLibrary 完成后才允许新加载；unit 先销毁，模块后释放。模块可跨线程引用不表示 unit、回调或 COM 对象可以迁移；本切片仍限定调用线程。当前路径域为显式绝对运行目录，仅规范分隔符；完整 C++ filesystem 别名/大小写/组件比较并未验收。

本机实际通过 80 轮双委托共享加载生命周期和 160 张模块/窗口关闭后持帧。三个文件名路径另分别由使用真实 Boost DLL、原 LibraryHolder、loader 和外层函数体的独立 executable 取帧；GDI-only 源 DLL 编译原 manager 函数体和未改写 GDI，输入 None、其他工厂拒绝及日志/目录注入为夹具边界。另一真实 DLL 使用原 Win32ControlUnitAPI 虚表作为验收 facade，实际回调 40 个 Rust manager/GDI 对象，见证 120 次原转换点、线程和完整析构。另验证缺失模块/两个导出、共享模块时不同目录拒绝、真实 C++ create/connect 异常及返回 false/异常后的输出修改；实际 165 份像素和 1093 条原生事件由 xtask 独立校核，仍不代表六工厂、通用共享加载失败或完整 framework/P1 验收。

原 DLL 静态链接 OpenCV 时，返回 Mat 的 allocator/vtable 可属于该 DLL；验收桥接在持有模块期间复制到独立 CPU 存储，并先销毁 DLL Mat。源 provider 增加一次边界复制；Rust facade 先复制借用图像到 provider Mat，再复制到桥接，增加两次边界复制，不将这些工作计为消除。图像仅保留桥接模块，原 provider 可完整卸载。C++ 异常在窄桥接内转成状态；编译使用 [/EHs](https://learn.microsoft.com/en-us/cpp/build/reference/eh-exception-handling-model)，避免 /EHc 将原 extern-C 工厂假定为不会抛异常。原生销毁消费不明时固定错误槽报告失败并保留模块；实现还必须断开所有 host/回调借用，仅保留模块不足以保证借用有效。此故障及分配耗尽仍待原生验证，不声称清理成功。

`upstream-parity manager` 现编排独立的真实 DLL 测试、非零测试守卫、像素/模块/owner 记录校核和失败归档；该新增 CI 路径尚待原生 Actions 验证。显式离线 manager 输入须完整包含 22 份独立模型/原生 oracle、工具链与来源记录（含完整原 Create/Destroy 函数体和独立 runtime executable），缺失或重复时提前拒绝。上述本机原生构建明确重放已归档的 OpenCV/Boost/MaaUtils/VS/SDK 输入，Rust 依赖另按在线批次更新；不以此重放声称新一轮原生依赖在线验收。

后续工厂补齐已移除源码 DLL 中其余方法的拒绝占位，改为共同编译原版六种方法及 helper，GDI 的 80 轮/160 张持帧回归通过。验收 Rust facade 也按原 API 接受空 HWND、零/组合/未知位选择器，将构造与连接校验分开；实际 manager 的构造/测速异常继续经 C++ 异常桥接返回，不压成普通 false。另有 `cargo xtask upstream-delegate win32-factory <外部记录目录> [--check-only]` 校核 36 组真实原接口构造/销毁、72 条事件、18 个 Rust context、模块卸载及无隐式连接/取帧；此入口不构造连接时才创建的 backend，不据此接受其余五种方法的真实取帧。完整六工厂运行、异常矩阵和 framework 接入继续待验收。

真实工厂运行另由 `cargo xtask upstream-delegate win32-runtime <mask> <外部案例目录> [--check-only]` 校核，mask 为六种单方法位及 63。每项源码与 Rust 在独立进程/自有 UI owner、可见非最小化窗口和 MTA/current-thread dispatcher 上分别执行 20 轮，保存各 60 张图及窗口/模块关闭后的持帧；本机七项共 280 次生命周期、840 次实际输出、1680 份即时/持帧文件已通过独立像素、原虚表调用、选择器、调用线程、unit/module/CPU owner 及退出记录校核。对应原生 Windows CI 的 manager 37 项组件检查也已通过，七项共 1680 份实际图像文件及关闭事件已下载并独立复算；其他组件和原 FramePool pseudo 失败仍单列保留。混合候选保留真实测速，因此两方赢家可不同，但都必须属于请求 mask。新增通用 renderer 仅供显式验收，基础产品不编入该 UI host 或 OpenCV；原 FramePool 测试与新入口共享同一测试 host，捕获期间不泵消息，仅在 shutdown 确认 dispatcher 完成。

DesktopDup 原方法按 HWND 选择显示器后返回整屏，不能误当作窗口图；runtime 保留完整输出，并用各方独立查询的 monitor/client 几何核对 96×64 自有窗口 ROI。其余屏幕像素仅校核完整可读跨度和持帧字节身份，不声称存在确定的宿主桌面彩色 oracle；DesktopDupWindow 仍验证完整窗口图。首版观察器曾误限 DesktopDup 为窗口尺寸，并将派生类识别为基类，造成两项失败，旧产物保留；修正观察器后仅重新执行受影响的 DesktopDup、Window 与混合项，其余通过记录按明确窗口范围复算，未重试捕获以掩盖失败。完整宿主/格式/故障、伪最小化、原生资源及 P1 验收仍未完成；既有四项 FramePool 包装彩色图样失败不变。

原源码重连先替换 loader；加载/创建早期失败时，旧的 no-op shared view 可能仍存在，而它不保活实际 unit。模型观察该状态，跳过对已销毁对象的虚调用；Rust 保留对应部分状态和原释放顺序，但显式拒绝失效视图。该安全适配依据 [C++ 对象寿命规则](https://eel.is/c++draft/basic.life)与 [Rust 原始指针访问条件](https://doc.rust-lang.org/std/ptr/index.html#safety)，不是把未定义调用当作成功对照。新 loader 构造失败仍保留旧 owner，新 view 构造失败仍保留已创建的新 loader；这两项为 Rust 控制，原生分配耗尽/完整控制块故障仍待验证。调用方线程约束单独保留，不声明原生对象可跨线程。

显式 `upstream-context pipewire` 和 `upstream-context macos` 新增同一 Custom 回调切片的原生验证：独立在线解析 MaaUtils 与 OpenCV，构建真实 core/imgproc/imgcodecs，重用 Windows 的原函数体、ImageBuffer/raw setter、Rust 测试和 192 份输出校核。自有回调不要求桌面、授权 UI 或输入设备；Custom-only 直接构建/运行图及 Clippy 与平台捕获分开检查。默认 core/imgproc 准备选项保持不变，Custom 的 imgcodecs 选项进入独立缓存身份，外部 codec 发现仍关闭；不据此接受 encoded/resize。源码、许可、构建记录和失败/图像资料仍外置，轻量产物保留实际输出与必要源码记录。Windows 主机只检查脚本语法、真实既有原生日志及零测试/未关闭 owner/依赖泄漏拒绝条件；新 Unix 原生入口已通过 Actions：macOS 组件 67 项、Linux PipeWire 组件 110 项检查全部通过；下载后的两平台各 192 份 Custom 输出及 ABI/owner 记录也已独立复算。

Wlr 另接入独立的原生 Wayland IPC/SHM 协议夹具；它没有 compositor、宿主桌面或输入设备，不能替代原有真实 wlroots/Pixman 取帧。`upstream-context wlr` 会显式构建它并分别运行 screencopy v1/v2/v3，各做 20 轮源码/Rust 生命周期，覆盖六格式、7×5 奇数尺寸、packed/额外 13 字节行填充和 YInvert 共 24 场景。每版本保留 24 份真实 producer SHM 输入、双方各 480 张 BGR 与 Rust 原字节，并检查重用后的持帧和 fd/任务/回调及服务端对象退出。`cargo xtask upstream-protocol wlr <外部矩阵目录> [--check-only]` 独立复算实际输入、4320 份输出与完整元数据。原生 Actions 的 112 项组件检查已通过，下载后的 72 份真实 SHM 输入、4320 份输出、双方案例表及服务端退出记录也已独立复算，11379 份可用产物引用哈希匹配；完整格式/失败/client-close/P1 仍未验收。原版隐含 packed Mat step 与忽略 YInvert 的行为继续保留：无填充且未翻转是正确目标像素，其余作为旧行为字节对照，不能声称正确目标捕获。RGB888 验收已补真实 OpenCV `copyTo`，Rust 前置 CPU 安全复制仍另行记账；核心默认无 OpenCV 依赖。

新增 `cargo xtask upstream-alpha frame-pool <外部诊断目录> [--check-only]` 校核实际透明度/内容诊断记录。`upstream-parity frame-pool` 另编译原生 `mfs_framepool_alpha_reference`：在各自独立的同一窗口上执行 alpha=255/0/255/0/255，分别使用每次新建与持续 FramePool 会话，切换两套已知图样并以独立原 PrintWindow 作当前内容见证。调色只在自有 UI owner 上同步更新，默认夹具图样不变；原捕获/helper 函数和等待均未改写，每步只调用一次原 FramePool。记录真实捕获前后窗口状态、20 份图像、重绘计数、持帧及窗口/dispatcher 退出。维护版本机取样及逐字节校核通过：新会话 alpha=0 两次全黑，持续会话一次全黑、一次约 2 秒后返回上一场景，其余六次 FramePool 及全部十次 PrintWindow 匹配当前图样。旧场景结果与原等待/缓存回放相符，但未截获缓存分支，不能宣称样本新鲜度或所有平台因果。黑图/旧图在此入口仅为诊断分类；四项原包装彩色图样失败及 P1 非通过门禁继续保留。原生 Actions 的新诊断也已取样并独立核对下载后的 20 份图像：六项 alpha=255 和全部 PrintWindow 匹配当前图样，四项 alpha=0 全黑，没有观察到旧场景回放。该组件合计 35 项通过、四项原包装彩色图样仍失败；不能把诊断通过当作后端或 P1 验收。

另一次独立本机 CDB 观察在原 `FramePoolScreencap.cpp` 的缓存 clone 行设置 PDB 源码断点，保留反汇编、调用栈、可执行文件/PDB 和实际图像。持续会话的 alpha=0 步骤命中该行一次，约 2 秒后返回上一图样；独立 PrintWindow 同时匹配当前图样，逐字节检查确认这次旧图来自原缓存分支。原源码、等待和每步一次捕获均未改写，调试器会影响执行时序，不能由此证明透明度成因或普通运行耗时。同次新建会话最后一个 alpha=255 步骤无帧且无缓存，正向控制失败、进程退出 5；九张 FramePool 与十张 PrintWindow、窗口退出和 dispatcher 完成均已保存并独立核对。整次实验仍记失败，不重试捕获，不改动正常诊断或原包装的验收条件。

后续原生 Actions 41100061053 的原版透明度序列出现另一项正向控制失败：持续会话最后恢复 alpha=255 后约 2 秒返回全黑，其余五项不透明 FramePool 图和全部十项 PrintWindow 均匹配当前图样。下载后的二十份实际 BGR 已独立逐字节复算，前后窗口记录仍为 alpha=255；记录确认持有十张结果、两个窗口关闭及 dispatcher 完成，但未建立这次黑图的原因。该原程序退出 5，后续 Rust FramePool 与四种 pseudo 检查没有执行，不能沿用较早另一批的四项失败来描述本批实际覆盖范围，也不能以资源退出成功接受此次像素控制。

窗口 GDI 的内部实现位于 `src/backends/windows/gdi.rs`，通过 `windows-gdi` 编译能力启用；没有对外发布 Rust/C SDK 接口。它保持 GetDC、兼容 DC/位图、BitBlt、GetBitmapBits 及转换前后的资源顺序。转换由调用点的私有适配边界执行；默认产品不链接 OpenCV。受检算术、初始化的 fallible CPU 分配、原生 32 位布局与完整读取校验属于显式记录的安全适配，没有提前迁至 P3 的 DIB 路径。

```text
cargo xtask upstream-parity gdi
cargo xtask upstream-parity gdi --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity print-window
cargo xtask upstream-parity print-window --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity screen-dc
cargo xtask upstream-parity screen-dc --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity frame-pool
cargo xtask upstream-parity frame-pool --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity desktop-dup
cargo xtask upstream-parity desktop-dup --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity manager
cargo xtask upstream-parity manager --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity custom
cargo xtask upstream-parity custom --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-custom <external-custom-artifacts> --check-only
cargo xtask upstream-trace custom-identity <external-custom-artifacts>/identity --check-only
cargo xtask upstream-model pipewire --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-context pipewire # native Linux x64 original-source context
cargo xtask upstream-context frame-pool
cargo xtask upstream-context frame-pool --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-context macos # native macOS arm64 original Objective-C++ context
```

`upstream-context frame-pool` 是独立的上游调用环境观察，不执行 Rust 对照。它把固定原 `FramePoolScreencap` 编译为独立 executable，分别在未初始化 COM、显式 MTA、MTA 加当前线程 dispatcher、STA 四个新进程中各调用一次原捕获，不在捕获内部添加 pump、等待或重试。自有绘图窗口有独立 UI owner/message loop；记录调用前后及原对象销毁后的 COM type/qualifier、原始 BGR、已知内部像素、耗时与窗口 owner。仅配平显式成功取得的初始化引用，dispatcher 在真实 shutdown 完成后退出。每个模式独立保存成功/失败，后续模式继续执行；完整原生、依赖、来源及许可材料进入外部 P1 incomplete 批次。

首轮本机四个独立进程都返回 96×64 BGR，已知内部像素正确，四张完整原图的哈希一致；未初始化模式最初为 `CO_E_NOTINITIALIZED`，调用后为 implicit MTA，原对象销毁后仍保留。显式 MTA/STA 的自有初始化引用释放后恢复未初始化。这个结果记录了 C++/WinRT 激活路径的宿主副作用，不能据此要求 U 提前采用 P3 的固定 MTA worker，也不表示 dispatcher、权限、等待/缓存、格式、缩放、尺寸变化或 Rust FramePool 的完整矩阵已验证。先前 WGC 工厂缓存的 DLL 卸载访问违规仍保留；独立进程观察不撤销该失败。

内部 `windows-frame-pool` 实现现位于 `src/backends/windows/frame_pool.rs`，另有平台私有 WGC/工厂封装；默认产品仍不链接 OpenCV。它保留硬件 `D3D11CreateDeviceAndSwapChain` 的 flags=0、R8G8B8A8 swapchain、原 `Create`/BGRA/单缓冲、旧帧 drain/Close、每 2 ms 轮询至 2000 ms、无帧时克隆缓存，以及 Unmap 前转换并克隆新缓存。`uninit` 保留缓存/device/context/swapchain；尺寸变化重建、原返回失败和未被局部 catch 捕获的 WinRT 异常分别保留部分状态，不额外回滚。owner 不声明 Send/Sync，不建立库内 MTA worker。

`upstream-parity frame-pool` 显式编译独立原 executable 与最小绘图/转换适配，不把原 C++/WinRT 工厂缓存放入可卸载的验收 DLL。它从原图记录启动未初始化/MTA/MTA 加当前线程 dispatcher/STA 四个独立 Rust 进程，核对逐字节像素、转换/克隆次数、独立 cache/result 所有者、关闭原生对象和窗口后的持帧、completion event 释放、调用方 COM/DPI，并保留 GDI 回归。四种本机上下文均通过局部检查；第二次调用实际观察到 2 秒等待后的缓存克隆。dispatcher 仅在捕获和原生对象关闭后的夹具退出阶段 pump，等待真实 ShutdownQueueAsync 完成，捕获内不添加 pump；自有 COM 引用配平后恢复未初始化。另有 20 次 Rust 窗口/device/session/pool 创建、实际捕获、幂等 uninit 与析构，逐张比对原图并一直持有全部 CPU 结果；核对精确 image-owner 数、completion event 和关闭错误，全部窗口 join 和 dispatcher 完成后再检查像素。这是相同原图的 Rust 重复生命周期检查，尚非 C++ 重复/故障全过程对照，也未测量全部 GPU/系统资源泄漏。完整原生故障/异常/清理、尺寸/DPI/边框/从属窗口/权限、伪最小化包装和 manager 尚未验收。

另一个真实 FramePool 控制在 cache clone 的 C++ 边界抛出异常，核对临时结果 owner 释放、原生关闭结果，并在错误返回后观察到 staging 可再次成功 Map；精确 Map/Unmap 调用拦截仍是独立待验项。显式返回的清理错误进入固定证据槽；Drop 的失败采用固定原子记录，避免在主错误展开时格式化、分配或因日志 I/O 再次 panic。这些局部控制不替代完整失败矩阵。

原客户区 ROI 为零面积时，OpenCV 路径返回 empty Mat 并以空克隆替换缓存；内部实现单独返回 EmptySample 并清空旧缓存，不能与无帧超时混同。几何单元检查覆盖零面积，真实零客户区原生矩阵仍待补齐。初始化的三次 Size 查询保持各自 catch 边界：创建 pool 参数中的失败按原逻辑返回 false，首尾两次异常保留逃出局部 catch 的部分状态；再次 CreateForWindow 前也按 put_abi 释放旧 item。这些错误路径仍需原生故障对照，不能仅凭审阅认定通过。

Rust 端另有真实目标销毁控制：先取帧并核对独立夹具内部像素，再在 UI owner 上销毁目标并完成 join；下一次捕获在窗口有效性检查立即失败，不继续轮询、转换或替换旧缓存。显式 session/pool 关闭通过，关闭后 CPU 图像仍有效。此项保留上游失败时的部分状态；完整原 C++ 目标销毁/故障全过程对照仍待完成。

`frame_pool_pseudo.rs` 已迁移原 FramePool 伪最小化包装，复用同一个 helper，并在 inner 的 session/pool/device/cache 析构前完整销毁 helper，保留 stop/join/恢复的先后顺序；`windows-frame-pool` 无须启用 PrintWindow。独立原 executable 另有五个包装场景：普通可见，以及普通/layered 窗口的最小化后 stop/inactive。参考文件记录图像和四个时点的原生样式、alpha、iconic、前台及客户区尺寸。Rust 对照执行 20 次包装生命周期，比较图像字节、样式/透明度/最小化及尺寸、独立 image owner 和关闭后像素；前台位单独记录，最小化夹具要求目标不是前台，不将跨进程的前台权限当作等价保证。

首个本机包装批次保留四项原源码彩色图样失败：最小化包装得到 96×64 全黑 BGR，普通可见场景正确；其 20 次 Rust 字节/状态对照匹配仍只是诊断，不能覆盖已知图样失败。独立夹具控制在同一 UI owner 上以 alpha=0/255 各测试有/无强制重绘：零透明度两种情况均黑，重绘后观察到 WM_PAINT；不透明两种情况均取得已知图样。该证据仅支持本机的透明度相关原因，不推导所有系统的 WGC 行为，也不修改上游 alpha=0、原捕获顺序或像素断言。正式编排保留 source check 的 failed 和批次非通过，并继续独立回归及字节/状态诊断。完整包装、前台恢复及 P1 门禁仍未关闭。

共享夹具准备/状态函数拆为独立 TU 后，PrintWindow 普通窗口回归又在 inactive 第 15 轮出现严格像素失败：源码图样正确，Rust 有 1725 个字节不同，部分区域为黑色。完整断言数组已原样提取为外部 BGR 文件；未重跑捕获。函数体与拆分前逐字一致，只证明这两个函数的逻辑未改，不足以排除回归或解释该次失败；完整 PrintWindow 验收仍未通过。

安全适配明确计入差异：验证 [CopyResource 所需描述符关系](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-copyresource)、ContentSize、ROI、RowPitch 和整数范围，仅复制初始化的目标 BGRA 行供私有转换器使用，避免将未定义的映射 padding 构造成 Rust slice。转换及 cache clone 仍在 Unmap 前；这新增一次目标 BGRA CPU 复制，不能描述为消除工作。工厂使用显式 owner，保留 SDK 的 implicit MTA 和任意激活 HRESULT 失败后的 DLL 层级尝试，避免投影的全局裸指针缓存；相应寿命和失败矩阵仍需扩充。border 请求保留 Completed handler + manual-reset event 的原 5 秒等待，超时不取消原生操作，晚到 delegate 只持有独立状态，不访问捕获对象。SDK 工厂/等待算法的 Microsoft 版权与 [MIT 文本](../../licenses/cppwinrt-MIT.txt)随产品保留，所用头文件身份与副本进入外部批次。此局部实现和检查不接受完整后端或 P1。

本机首个局部对照已通过；三平台 CI 与本机结果分开记录。此入口显式使用 Windows x64 桌面，并复用独立原生准备后编译原始固定 GDI 类和 C++/OpenCV 转换适配。Rust 私有模块内的测试核对非对称图案、奇数宽度、逐像素 BGR、跨后续捕获/窗口销毁的持帧、转换时点和原生资源计数；另覆盖 null/已销毁/空/最小化窗口与被封住的转换异常。故障对照以隔离命名空间编译原始函数体，通过真实 GDI 对象记录创建、复制、转换及清理的原生边界响应；14 组正常/故障/组合场景分别核对主错误、清理错误、调用顺序及 CPU owner 释放。夹具先记录捕获实现遗留的资源，再单独回收，不能据此声称产品在原生清理失败后已成功关闭。结果、源码快照、依赖与未覆盖范围写入外部 P1 组件批次。原生 Actions 的 Windows 任务执行这个入口，其他平台继续各自的原型与构建检查。

U-WIN-GDI-02 的原生失败/清理与窗口状态场景已纳入对照；U-WIN-GDI-01 仍缺真实非默认显示深度，管理器/转发与完整 P1 验收尚待完成。原生 24 位 DDB 在当前 32 位设备上可能被 SelectObject 拒绝，该观察不等于实际 24 位显示器取帧。短读探针定义原参考输入的全部字节后观察其非零即成功分支，Rust 的安全拒绝单独分类。完整目录中的其他后端继续按 P1 迁移。

PrintWindow 的内部迁移位于 `src/backends/windows/print_window.rs`，由独立的 `windows-print-window` feature 启用。它与 GDI 共享 DC/位图清理和私有转换类型，声明生成分别选择所需函数；PrintWindow 单独启用不编入 GetBitmapBits 路径。保留主窗口捕获、root-owner/last-active popup 选择、各自 BGRA2BGR、原生清理后 BGR ROI 合成的顺序；弹窗原生可选失败返回主图，转换/分配异常仍使整个操作失败。

原源码在位图仍选入 DC 时调用 GetDIBits，违反其[原生前提](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdibits)。Rust 暂时移出位图再读回，成功后在原转换点前选回，最终清理顺序保持；原生对象查询见证读回和转换时的真实选择状态。此为显式安全适配，未改为 P3 的 CreateDIBSection。矩形交集另外直接对照已编译 OpenCV 的 400 组边界组合：仅拒绝原点差的 int 溢出，保留合法偏移的空交集/裁剪结果。完整扫描行数、DWORD 对齐的初始化缓冲及几何受检算术分别验证；读回存储使用 Vec<u32>，字节视图借用同一所有者，不额外复制整帧。

`print-window` 对照同时回归 GDI，显式编译原始 PrintWindow 类及自有窗口类，使用 WM_PAINT/WM_PRINTCLIENT 绘制不同主图/弹窗图样。夹具在捕获前调用 GdiFlush/DwmFlush；这不证明此后每次 PrintWindow 都得到已绘制的像素。记录实际绘制消息而不假设 `PW_RENDERFULLCONTENT` 只走一种消息路径。负原点、部分重叠、边界外、隐藏/无弹窗及零宽弹窗各执行二十次夹具生命周期，逐像素检查与关闭后持帧；这仍是局部对照。八组查询对照还核对正常/root 查询为空、popup 为空或等于 root、无效/隐藏 popup、任一 ClientToScreen 失败的顺序、短路与主图返回。查询探针只替换查询边界，捕获/转换/清理仍执行原函数体；C/Rust 布局与表函数签名在加载后核验。

真实子窗口夹具在更大的 root 内创建 37×23 子窗口，弹窗仍由 root 拥有，偏移以子窗口为基准。无弹窗、正/负偏移、边界外、隐藏和零宽弹窗六个场景各执行二十次生命周期；核对 root 与目标不同、子窗口尺寸/逐像素图样、转换/合成次数、清理和销毁后的持帧。root 的尺寸与图样分区不同，不能以捕获 root 代替子窗口。隐藏设置还须核验 IsWindowVisible 的实际结果。

窗口夹具独立持有其回调代码所在模块，不依赖适配器借用或 guard 的 Drop 一定执行。专有 UI 线程注册/创建窗口并持续运行 GetMessage 循环，统计命令也在该线程读取绘制状态；捕获继续在调用线程执行。创建时核对 UI owner、原生 HWND owner 与调用线程身份，初始消息队列有明确排空上限。关闭命令在 UI owner 上销毁窗口和注销类，完成 join 后再释放 context 与模块。先释放适配器、部分创建失败和正常关闭分别验证；遗忘 guard 会保留模块、context 与 worker，失败关闭不自动重试。

29 组原生故障场景另外对照主图/弹窗的客户区、DC、位图、选择、PrintWindow、读回、转换、各清理调用及组合故障。共享探针分别保留两个 HWND 的资源；先记录实现的清理尝试和遗留对象，再由夹具单独回收，不能把夹具回收当作产品成功清理。读回的临时移出/选回有独立调用边界，生产路径仍转发原来的 SelectObject 调用，避免与最终恢复混淆。六组安全适配场景另行分类：原参考不具备新增的读回选择调用，正扫描行短读的参考输入全量定义后再观察其非零即成功分支。原始事件仍保存 GetLastError，成功调用的未定义错误值不作为等价错误契约；失败的注入错误值、主错误和清理错误均逐项核对。

另一个独立 TU 只在原参考的 cvtColor 边界注入主图/弹窗转换异常，原生捕获、读取和清理仍执行原函数体。对照要求转换异常退出整次操作，记录转换时及展开后的真实 GDI 数量。导出的图像 owner 数量与原 C++ 内部 Mat 分开观察：异常模式只为 BGR 输出设置局部分配观察器，直接记录主图 BGR 分配、释放与字节数，输入 BGRA 和其他 OpenCV 分配不计入它；不替换进程默认分配器，不导出引用观察器的 Mat。异常遗留分配会使检查失败，并保留其模块供晚到释放。Rust 主图 owner 在弹窗异常后也须释放，不能把这类异常降为可选弹窗失败。完整前台/后台、伪最小化及窗口状态覆盖尚未完成，U-WIN-PRINT-01/02/03 均未宣称完整验收。

已有正式批次出现原参考弹窗位图全黑、Rust 后续捕获图样正确的逐像素失败；临时移出位图、重复读取和原始位图读取都不能恢复该次像素。额外裁剪查询和后续未复现结果尚未建立因果关系，失败与诊断证据保留外置，PrintWindow 的局部验收仍未关闭。诊断 TU 与未插桩的参考 TU 分开，诊断读取返回第一次得到的图像，不以再次读取修补验收输出；额外裁剪/flush 对照已撤回，普通夹具不携带未经证实的修复。

伪最小化 helper 已迁入 `src/platform/windows/pseudo_minimize.rs`，PrintWindow 包装保留构造时启动、截图前同步 ensure、`inactive` 与析构停用的原顺序。标准 Arc 保活监视状态，调用方 owner 不宣称 Send/Sync；监视线程只执行原有窗口查询与属性恢复，保持 100 ms 轮询，stop 完成 join 后再恢复属性并按原条件重新最小化。原生调用和等待期间不持有状态锁。固定大小证据按操作分别保存首末失败及次数，并保留全局首末错误，避免早期失败遮蔽样式和透明度两个恢复错误，ShowWindow 的零返回单独记录为先前可见性，不误称操作失败。部分应用失败后不新增回滚；`inactive` 恢复后不重新最小化，目标消失后不自动重启监视线程。

22 组原 C++ 函数体脚本对照覆盖原样式/透明度、各失败短路、监视恢复/目标消失、重复 start/stop 与 inactive，逐项比较调用、100 ms 等待请求和状态快照。脚本明确替换线程调度，不能证明真实线程退出；单独的真实线程同步测试检查 join 完成前不恢复属性。旧同线程夹具在 layered 还原后的待绘制阶段捕获到旧标题内容；重绘/flush 控制及原失败均保留外置。相同原始 helper/capture 面对独立运行消息循环的 UI owner 时已通过已知图样对照，夹具的激活锚点也须由同一 UI owner 设置，不能从捕获线程调用 SetActiveWindow 代替。未修改产品 helper 的等待、转换或 ShowWindow 调用。普通/layered 窗口分别覆盖 stop/inactive 各 20 次生命周期，目标销毁各 20 次，逐像素、样式/透明度、真实线程退出和关闭后持帧均单独验证。前台恢复在独立的精确测试入口执行；若原生 SetForegroundWindow 返回零且其他进程保留前台，编排记录 missing_environment，继续独立检查并使批次保持非通过，不使用输入注入或绕过权限。该情况不能记作恢复成功。此前弹窗黑图的独立 A/B/A 控制仍可能不复现，不能据此宣称其根因已修复；完整 helper/wrapper 和 PrintWindow 验收仍未完成。

ScreenDC 的内部实现位于 `src/backends/windows/screen_dc.rs`，独立 `windows-screen-dc` feature 不要求编入 PrintWindow。它保留前台/几何 helper 失败后继续捕获、原 `rcWork` 校正、未使用但仍须执行的窗口矩形查询、屏幕 DC 客户区坐标、GetBitmapBits 与清理前转换；null HWND 不表示全桌面。窗口 GDI 与 ScreenDC 共用原读回代码，PrintWindow 的 GetDIBits 路径仍独立。所有原生资源在同步调用线程创建、使用和销毁，没有新增 Send/Sync 保证。

`screen-dc` 显式编译原 ScreenDC 类及原窗口 helper 头文件，保留 GDI 回归。脚本对照检查 11 个连续冷却步骤、2 个直接置顶调用、16 个几何、3 个尺寸、4 个样式与 4 个缩放案例；溢出检查不执行原 C++ 未定义算术。另有脚本化捕获/清理边界测试，不能作为真实原生故障验收。本机 20 轮呈现就绪后的源码/Rust 客户区逐像素、资源计数及窗口关闭后 CPU 持帧已通过；完整正式批次和其他平台结果仍以外部记录为准。

首次 ScreenDC 桌面检查暴露两个夹具问题，原失败保留：冷进程创建窗口初始化 9 个后续保留的 GDI 对象，独立无捕获控制确认捕获基线须在窗口创建后；后续第 9 轮原源码读到其他桌面内容，仅原源码的 20 轮呈现前/后控制又观察到 2 轮呈现前非夹具像素，呈现后均正确。夹具以 GdiFlush/DwmFlush 建立一次就绪前提，捕获实现、等待和像素断言不变，不重试失败。前台请求被拒绝仍可得到可见像素，不等于前台恢复成功。遮挡、真实原生失败清理、异步移动/缩放、非默认深度和完整 helper/manager 验收仍待完成，不能声明 ScreenDC 或 P1 已验收。

完整 ScreenDC 本机批次曾有两项全黑失败，原日志和图像外置保全。该次桌面已熄屏，后续用户确认记录和当时约 27 分钟无输入、前台窗口为空的观测仍可核对，因此它不满足可见桌面的测试前提，不再列为未解决程序缺陷。此次按用户要求恢复失败批次的修复前源码、原始原生适配器、归档依赖及同一 Rust nightly，核对 142 份来源输入后重建；两项原始测试在原生显示电源为 On、会话 active/unlocked、输入计时未变且光标固定的条件下各完成 20 轮，原像素断言与程序均未修改。该复测不依赖后续 WM_PAINT、UI owner 或置顶修订；旧执行的失败不改写为通过，环境误报的有效归因已纠正。后续专用 UI owner/message loop 的职责仍是同步绘图、统计、关闭和 join，不能把单次 DwmFlush 当作一般就绪证明。

专用 UI owner 的后续完整后端顺序检查也曾出现黑图；其绘图消息和 owner 身份不构成可见桌面的证明，不能声称该夹具改动修复了熄屏。旧文档“待用户确认”已失效，用户已确认当时显示器关闭。此次额外的熄屏对照又观察到输入计时变化、显示电源恢复及 LockApp 前台覆盖，故该组不用于简单归因，也不因电源恢复为 On 就声称窗口无遮挡；它和上述已核实的历史环境误报分开。其他开发中间版本的计数、像素和状态失败仍按各自来源核对，不以 ScreenDC 的环境结论覆盖。

本地历史复审现已对 36 份候选失败日志逐项校验哈希并另存备份，重放十三组确切修复前源码/配置和七种原 FramePool executable 模式。原 WGC 程序在 20 轮硬件生命周期后，于 WARP 第 2 轮再次发生 `0xC0000005`，Application Error 再次指向 `GraphicsCapture.dll_unloaded`；原 FramePool 四种 pseudo 模式也再次退出 5。这些执行全程亮屏、会话 active/unlocked、输入桌面 Default、无 LockApp 前台、输入计时和光标不变，不能归因于此次用户操作或熄屏；确切非法访问及四种 pseudo 的根因仍未据此确定。两份原 ScreenDC 几何源码/原 DLL（各校验 379 份输入）也分别复现带边框 popup 角部像素失败、普通窗口实际客户区高度 1061 而请求 1080 的夹具前提失败。现有方角/最大 tracking size 夹具修订和后续通过记录保持独立，不将旧执行改写为通过。

从本任务的原始修改记录另恢复了两份早期程序，并以各自当时的 Rust nightly、归档依赖和原断言执行。Desktop Duplication 原程序在原第 48 行再次读到夹具初始 `[51,34,17]`，此次首轮预期 `[65,34,17]`，历史第 7 轮预期为 `[71,34,17]`；固定输入和光标下也可复现，支持初始绘制与后续 repaint 使用不同颜色的夹具呈现假设问题，但未据此确定确切 producer 呈现时间。GDI 原测试按原输入字节与实际修改 diff 重建，未格式化其旧源文件；在原第 153 行再次因原生 `SelectObject` 拒绝 24-bit DDB 而触发“一律成功”的旧断言，其余六项通过。后续修订允许并记录这种原生拒绝，实际非默认显示深度捕获仍未验收。两组执行均有独立亮屏/Default/无输入监测，原失败继续保留，不能归因于此次用户操作。

同次复审还区分了捕获前错误：首个 PrintWindow 批次的原 CMake cache 将两项参数拼成一个 BOOL 值，Print 适配未启用；原 DLL 缺少相应符号，原测试再次在调用前失败。旧 manager 的两个 GDI probe 同样请求了其原 DLL 不含的 Screen 符号；其余五项通过不代表这两个 probe 执行成功。Custom 原测试的 OpenCV 类型常量硬编码为 16/24，而同一原头文件编译所得为 64/96；恢复原 Rust/原生源码和当时 nightly 后再次在 callback/图像调用前失败，属于已修正的测试假设。前台控制用例则再次明确报告 `MFS_MISSING_ENVIRONMENT:foreground_control`，不证明用户切窗或 helper 恢复成功。另一份原 PrintWindow 程序此次 13 项全过，历史原因仍未确定；后续原程序的 popup 和两项 pseudo 失败仍复现。共享夹具提取后曾在 inactive 第 15 轮出现局部黑图的原 PrintWindow 程序，另按 155 份源码哈希、原生配置及当时 nightly 重建，此次 stop/inactive 共 40 轮通过；该旧失败的原因仍未确定，不因未复现就删除。完整逐日志表、原输入身份、原始失败备份和复测电源/输入记录均外置；当时尚未精确重建的 23 份开发中间版本随后已恢复并完成有效连续三轮，结果与确切覆盖范围见本文开头；完整 P1 仍未完成。

ScreenDC 的真实资源故障入口以隔离命名空间编译原函数体，只在 DC/位图/转换边界注入，helper 和窗口查询仍为原生调用。13 组普通/组合故障核对实际到达的故障、调用/错误顺序、主错误、四项清理错误、回收前残留计数和 CPU owner 释放；夹具回收不计为产品清理成功。另有短读、元数据失败、24 位 DDB 的三项安全分类，以及 null/已销毁/零宽客户区拒绝检查。零宽窗口的前台 helper 在冷进程单独初始化 7 个后续保留的 GDI 对象，独立无捕获控制已记录，测量前显式建立该夹具前提。完整原生故障入口四项测试已通过；它们不能替代各自环境下的严格可见像素检查，实际非默认显示深度也未验收。

精确提交的原生 Actions 又记录了不同的可见内容失败：第一轮源码/Rust 图样正确，第二轮前台为 true，但源码首先读到统一灰色 `[12,12,12]`，Rust 随后正确，原源码呈现前/后控制同轮也为灰色。不能用本机熄屏推断解释这一 runner 结果。当前仅增加一个待验证的夹具控制：自有 ScreenDC 测试窗口在隐藏时设置 `DWMWA_TRANSITIONS_FORCEDISABLED`，成功后才首次显示；保持 PrintWindow 的既有模式、捕获等待及逐像素断言，既不改全局动画设置，也不重试失败。私有 C 接口原有布局/统计前缀保持，尾部反映该设置。设置、module 保活、四项原生故障和 PrintWindow 两项前缀/退出检查已通过；动画是否为根因及这个控制能否稳定可见像素，仍需新原生证据，所有旧失败保留。

后续结果未证实仅禁动画有效：用户确认并唤醒本机熄屏后，禁动画的完整 backend 入口仍为 16 项通过、2 项灰图失败；相应提交的 Actions 也仍失败。只读诊断又在一轮源码捕获前观察到客户区中心由另一窗口占据，随后目标抬升才取得正确像素。[IsWindowVisible](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindowvisible) 仅表示可见样式，不保证无遮挡；[WindowFromPoint](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-windowfrompoint) 的点窗口/root 与源码捕获前后的前台、cloaked、坐标和样式一起保留。原 helper 共享的 5 秒冷却不保证为每个新窗口再次请求前台，不能用 Rust 返回后的前台值推断原源码取帧瞬间的可见性。

新的夹具前提是在自有 ScreenDC root 上额外请求 `WS_EX_TOPMOST`，不授予前台权限；原 helper 仍可按原行为将它取消置顶。只改变夹具，保留原捕获、等待及严格像素断言，原 PrintWindow 模式与 C 接口前缀保持。一轮亮屏本机完整 backend 入口 19 项通过，包括 20 轮配对逐像素/清理/关闭后持帧，以及独立的 20 轮源码呈现前后控制；观察到各轮中心点 root 属于目标。相同受控场景随后通过两个精确提交的原生 Actions，证据及提交身份外置保存；这仍只证明该受控场景，显式遮挡/移动/深度与完整 P1 仍待验证。首次运行因未创建外部图像记录目录中断，属于保留的 harness I/O 失败；补建目录后的结果单独记录，不能覆盖之前的像素失败。


DesktopDup 和 DesktopDupWindow 已有私有迁移，入口为 `cargo xtask upstream-parity desktop-dup`，可复用明确的 `--offline-inputs`。独立 features 为 `windows-desktop-dup`、`windows-desktop-dup-window`；后者复用底层并启用原前台/rcWork helper。它保留默认 hardware device、CreateDXGIFactory、显示器枚举/主输出回退、DuplicateOutput、每次 2000ms Acquire、最多三次底右 alpha 预热后另取最终帧、access-lost 部分清理及释放顺序。鼠标更新和实际桌面更新时间分别记原生证据，不增加 U 的新鲜度过滤。

原 `screencap_impl` 的 Mat 借用 staging 映射，却在返回前 Unmap/ReleaseFrame；[Microsoft 的 Unmap 契约](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-unmap) 明确该调用会使映射指针失效。Rust 在映射有效期内复制定义的 BGRA 像素行，仍在原生映射、原纹理、帧和 desktop resource 释放之后做 BGR 转换。该全输出 CPU 复制明确计量，不能称为消除复制。Window 仍先转换全屏、再查客户区/输出并裁剪；为不暴露 ROI 间的其他桌面像素，另做目标 BGR 紧密复制，这会改变原 ROI 对全图存储的保留和分配点，属于明确的安全适配。

对照 executable 在独立进程调用未修改的原 full/window 类；自有 UI owner 在目标外以 50ms 定时器交替绘制一个独立标记窗口，提供预热和最终 Acquire 所需的桌面更新，不重试捕获、不改变目标图样。原源码两种方式和 Rust 20 次混合生命周期（每种十次）已通过本机严格目标图样、ReleaseFrame HRESULT、目标独立存储及所有原生/窗口关闭后的 CPU 持帧检查；实际监视器、marker 活动及 frame/mouse 时间证据外置。另一个独立原源码进程与精确 Rust 用例验证空 HWND：它选择 adapter 0/output 0，不能假定该输出就是 OS 主显示器。两侧先独立查询该输出是否连接并包含测试窗口；不满足前提记为 missing_environment，不改选其他输出。本机该场景的严格目标图样、归还 HRESULT 与关闭后持帧已通过。

F 的精确全屏 popup 新增 `window-fullscreen` 原版进程及独立 Rust 精确测试：客户区四边等于真实 rcMonitor，且 rcWork 必须不同；自有 UI owner 改在客户区左上角绘制 16×16 已知双色标记，首次完整重绘后每 50ms 只更新标记，关闭先取消定时器再销毁窗口并 join。既有外部标记场景保持原方式。原版一次、Rust 两次本机取帧及关闭后持帧已通过，逐像素验证完整目标和一致的标记颜色；真实几何消息计数保持不变，Rust 图像 owner 最终配平。实际环境为 1920×1080、工作区 1920×1032、96 DPI。`upstream-trace desktop-fullscreen <外部 window-fullscreen 目录> [--check-only]` 重检两侧原生几何、工作区差异、三次完整已知像素、持帧及 Rust owner 完成记录；两侧即时/持帧文件同时注入相同错误像素仍被独立已知图样校核拒绝。

另加入普通 `WS_OVERLAPPEDWINDOW` 的 `window-workarea` 原版进程和独立 Rust 用例：初始客户区等于 rcMonitor，双方各自使用固定来源的 helper 完成一次 rcWork 调整，确认异步窗口几何已生效并重新绘制，再开始原版一次、Rust 两次 DesktopDupWindow 捕获。`upstream-trace desktop-workarea <外部 window-workarea 目录> [--check-only]` 要求完整前后客户区/外矩形、WINDOWPOS 参数/flags、一次 helper 调整及捕获期间不重复调整，并复算完整已知图样/标记、关闭后持帧和 owner 完成记录。本机 1920×1080 → 1920×1032 的三次实际捕获及关闭后持帧通过；七份实际图像、几何与字节另经独立校核，双方全部即时/持帧文件同时注入同一错误像素，以及双方记录同时注入同一错误 WINDOWPOS 均被拒绝。全屏原用例回归、默认和 DD feature Clippy 通过，Actions 原生验证尚待执行；原生依赖为归档 OpenCV/MaaUtils/Boost 重放。它覆盖调整完成后的普通窗口，不声称覆盖首次截图触发调整的竞态、完整边界/DPI/输出矩阵或 P1。

本轮先用 3fa6dc4f、再从最终 4140ac96 的不可变来源分别重建原生适配器与原版程序，并执行 61 组 helper/122 条完整对照、38 场景 ScreenDC/152 次实际采集及上述 DesktopDupWindow 全屏对照；4140ac96 还通过既有 ScreenDC 十二项检查与 DesktopDup 三种原版模式、二十轮 Rust 混合生命周期、空 HWND 和转换/异常边界回归。首次较早来源的全屏原版图有 4096 个非标记像素保留初始 96×64 客户区旧图样，原图、日志及编译输入独立保留；夹具首次开启标记时增加完整重绘，没有放宽像素断言。一次手工 Rust 运行因漏设适配库变量在加载前失败，没有取帧，日志亦保留。此项只覆盖精确 popup 单例；DesktopDupWindow 全部偏移/边框/任务栏位置/DPI/多输出及 manager/factory 转发仍待完成，新 Actions 验证尚待执行。原生依赖明确重放归档 OpenCV/MaaUtils/Boost，不将本机成功作为新一轮在线原生依赖验收；较早来源及其失败、成功记录均不重标为当前 F。

独立转换边界用例先以诊断取帧取得真实 staging/COM 引用，随后保留原预热标记进行捕获；这次额外取帧不计为原源码调用次数对照。在正常转换和受控 C++ 异常的两个入口，真实 ReleaseFrame 均返回 DXGI_ERROR_INVALID_CALL，确认当时没有仍被获取的帧；staging 再次 Map 成功，输入 CPU 指针独立且首行相同，诊断映射在调用 C++ 转换前结束。受控 cv::Exception 保持实际捕获帧的成功归还、精确图像 owner 计数及之前图像的关闭后像素。本机正式批次已通过这些检查；再次 Map 成功不等于精确拦截 Unmap 次数。

完整动态桌面不做全帧逐字节对照；原源码偶然返回正确像素也不能证明其越界生命周期有效。全量原生故障、精确调用/预热脚本、原源码重复生命周期、多适配器/显示器查找失败回退/旋转/access loss/格式、移动/遮挡/DPI、精确 Unmap 拦截和 manager 链仍未验收。该迁移不启用 P3 的新工厂或 DuplicateOutput1。

首次 DesktopDup 原生 Actions 在 Rust 第 8 轮通过当轮像素/归还检查后，失败于夹具的 `marker[1] > 1` 断言，未完成剩余生命周期。该计数包含首次同步绘制，旧断言实际额外要求一次定时回调；原生取帧也可消费初始图像、指针或其他窗口更新，不能从取帧次数推得额外回调。夹具现记录实际绘制计数，保留首次绘制、标记存在与绘制错误检查，保持每次原生 Acquire/Release、严格目标像素和持帧断言；不增加捕获等待或重试。原始日志及失败批次保留；修正后的本机正式批次通过 25 项检查，显式补齐首次绘制检查后的三个重建原源码模式、20 次 Rust 生命周期及 Clippy 也通过。修正后的原生 Actions 也通过全部 25 项；旧失败仍保持失败。

另一轮 FramePool 原生 Actions 在原源码 MTA 场景返回图像并完成关闭后持帧，但已归档的 96×64 BGR 图有 2048 个错误像素：右下 `(32,32)` 至 `(95,63)` 的第三色块全黑；其余三个调用环境的原图符合图样。这是原源码像素断言失败，不能归为环境缺失，也不能称为整张黑图。具体绘制、透明度、调用环境或其他原因尚未确定；此次审阅没有重新取帧，其他批次通过不撤销该失败。

后续另一批原生 Actions 的原图也已审阅：四个原源码最小化包装场景均为全零 BGR，普通可见场景匹配图样。它证实该批次实际返回黑图，仍不证明所有平台环境下的透明度或绘制原因；四项已知颜色失败保持失败，Rust 字节/状态匹配只作诊断。

截图 manager 的私有迁移入口为 `cargo xtask upstream-parity manager`，可显式复用 `--offline-inputs`。`windows-manager` feature 组合六种已有 Windows 捕获能力；默认产品构建仍不编译验收适配器。固定 `Win32ControlUnitMgr` 的截图函数体、原 enum 及所选 MaaUtils `duration_since` 在原生构建目录提取，仅归一化换行。声明 shim 与 input=None stub 明确排除输入实现；不是完整原控制单元类的编译。原后台键守护在未配置按键时不启动线程，inactive 空集直接返回；该边界审计仍不验证输入对象的分配/锁/通知故障。

Rust 保留连接时先清除旧捕获 owner、再设置调用线程 PMv2 DPI、随后验证 HWND 的顺序；成功设置后按 U 保留该上下文，P3 的宿主上下文恢复尚未引入。None 允许连接但没有截图 owner，未知位独占选择失败。先构造全部选中候选，再依次预热与独立测速；预热失败仍测速，成功测量按毫秒截断，严格小于才替换优胜者。预热/测速临时图像在原条件结束处释放，计时包含测速图像销毁；未选中 owner 随后释放，胜出 owner 保留。`Some(empty Mat)` 是成功空结果，调用失败保持此前输出，后续截图不因 connected 查询失效而提前截断，也不静默换法。

122 个原源码脚本在真实 MSVC `unordered_map`/`shared_ptr` 和 OpenCV Mat 分配/释放上观察顺序，平台、后端、时钟显式脚本化；涵盖全部 64 种方法位组合、异常/失败、毫秒截断同耗时、临时析构成本、输出保留及重新连接。本机已通过；这不是跨工具链的 unordered_map 顺序保证。Rust 用有界唯一 owner 槽位转移胜出后端，不复制 C++ 容器/控制块分配；原生回调/worker 状态仍由原后端独立保活，不据串行调用声明 Send/Sync。原分配故障的完整等价性仍待验收。

独立 GDI-only 原源码函数体入口实际调用原 GDI 类与调用线程 DPI API，其他原生方法显式不可用；仅添加线程/DPI/次数探针，并抑制 manager 日志。20 次配对源码/Rust manager 生命周期已通过本机严格奇数宽 BGR、两次连接取图加一次交付、GDI 资源及原生/窗口/模块关闭后的持帧。夹具以物理尺寸创建，分别从 unaware 调用者进入原源码/Rust manager；测试在外部恢复上下文。单方法探针和脚本时钟不证明真实混合方法的速度选择；完整六方法/包装副作用、胜出/淘汰原生错误清理、兼容模式 DPI、framework 转发和全平台 P1 仍待完成。

首次 manager 正式批次的 GDI 回归有两项因验收符号缺失而未执行：Rust 编译全部能力后，旧通用探针提前请求了该次 GDI-only 模块未选中的 ScreenDC 符号。该失败批次保留。GDI 与 ScreenDC 现使用显式探针入口，目标符号在各自开始原生故障状态前核验；修正后的 GDI 七项、ScreenDC 四项及 manager 两个入口已分别通过本机检查。后续正式 manager 批次的 20 项检查全部通过，包含 122 个脚本和 20 次真实 GDI 生命周期；记录仍为 P1/incomplete。正式 manager 批次串行运行独立回归/脚本/生命周期，保留每项失败而不中断其他独立证据，整批仍以任一失败为失败。

修正标记计数后的 DesktopDup 原生 Actions 已通过全部 25 项，实际 20 轮标记绘制计数包含 1；此前失败保留。该轮 PrintWindow 30 项和 ScreenDC 19 项通过，FramePool 仍为 32 项通过、四项最小化原源码颜色失败。补充诊断 ZIP 已核对发布摘要，所含记录引用已校验；它不代替完整工程归档，也不撤销其他失败。

`upstream-model pipewire` 是当前 Windows 原生 C++/OpenCV 工具链上的可移植消费端对照，单独标记为 P1/incomplete，不是 Linux 实际捕获。私有 `linux-pipewire` 能力现有消费端算法及首个待原生验证的 daemon/SHM driver 切片。原源码的等待 helper/锁 guard、格式表、connected/inactive/wait_for_frame_locked/screencap 函数体在构建目录提取，仅归一化换行；loop、clock 和格式编号是明确的脚本边界，Mat 所有者及转换使用真实 OpenCV。这不是完整原 Linux 类或 SPA 数值 ABI 的编译验证。

本机 74 个案例、118 次操作已匹配原源码：每次激活并请求新帧、整秒截断与亚秒余量至少等待一秒、伪唤醒、负等待返回后的谓词复查、前三次超时回放与第四次拒绝、空缓存、停用/再激活、锁外转换/异常和 BGR 共享 owner 转移。原生 set_active 返回值按 U 忽略；源码错误回调不主动 signal，消费端也未提前加入 P3 的终止唤醒/绝对等待。额外的时钟/计数溢出检查独立验证，不执行原 C++ 未定义溢出。原生建连、init/deinit、POD、process、SHM/DMA-BUF、缓存写入别名、线程迁移、关闭/注销和 Linux manager 仍待迁移与原生 Actions 验收；受控 daemon/给定 socket 的 P1 原生证据见下文，完整后端仍未完成。

`upstream-context pipewire` 已接入原生 Linux x64 源码环境诊断，完整原 cpp 与 Rust driver 已在 Actions 编译、链接并取得初批正确像素，生命周期资源断言仍有失败，完整入口未通过。它直接编译固定完整 `PipeWireScreencap.h/.cpp`，仅抽取原 screenshot base 排除输入/ConfigBase/meojson，并明确使用私有 namespace 与日志抑制 overlay。同批解析最新正式 OpenCV 与 DRM 头文件来源，保留 IPP/CPU 优化，记录 LLVM、系统 SDK 包、编译命令、原生链接与离线输入。默认 P0 生产端仍为原七色条；此入口选择独立 `--asymmetric` 二维图样、96 字节行距及初始化填充，不在该图样上执行 P0 的 RGB 条断言。

新的原生 Rust 切片包含 daemon 建连、先启动 loop 再 connect、固定 4096 字节 POD builder、稳定事件/状态地址、真实 loop 锁和返回值保留的相对等待、逐次归还前驱 buffer、只按请求复制最新 SHM、停止线程后注销并释放对象。完整原 cpp 现在不改内容地 include 到测试翻译单元，以调用其 static 格式 helper；另用原 buffer POD 表达式保存三个协商 POD，与 Rust 逐字节比较。独立 Rust 用例要求 20 次生命周期/60 张 raw RGB、inactive、返回计数、fd/线程归零和原样持帧；RawAudit 只保留 RGB，不执行 OpenCV 转换，因此没有宣布原生 BGR 对照通过。另已加入显式 C++/OpenCV 共享验收适配器、私有 C 头与实际 ABI 消费/异常测试，并安排独立 20 次生命周期/60 张 BGR 持帧验证，受控 RGB 原生结果已通过。实际转换仍位于 native 解锁之后；每周期在转换入口等待真实 process 回调推进一次，额外等待只作解锁诊断，不是源码耗时对照。Mat owner 借用 loader 生命周期、清除 native cache 后仍持有真实 BGR 存储，不添加第二次全帧 CPU 复制；未声明 opaque Mat/loader 可跨线程。只有 Linux 测试使用 libc loader 声明，默认核心仍无 OpenCV 依赖。该切片已在原生 Linux 编译，四项行所有权测试通过；首次 Clippy 因当前 nightly 弃用 fetch_update 失败，现已改为 try_update，保留原内存序和受检计数。受控取帧结果见下文；实际 Portal 授权、无既有指针的映射、DMA-BUF、完整格式/故障/共享缓存矩阵仍待迁移。

CPU 存储的四项 host 测试已通过，验证 stride/跨度/溢出、目标行与填充隔离、空指针拒绝、独占存储复用及清除缓存后的跨线程持帧。负 stride、非零 chunk offset、损坏标志和短跨度显式拒绝，未静默提前套用 P3 的 offset 语义。已有持帧者时分配新 CPU owner，保留单次行复制；这是修复原 Mat 缓存可写别名的安全适配，需独立核验原生行为和成本。原生实现保留每构造一次 raw pw_init、析构无 deinit 的 U 策略；测试配平仅发生在完整 native teardown 之后，禁止与高层 binding 的 OnceLock 初始化便利构造函数混用。

首批 Linux 源码 context 在参考程序 CMake 配置时失败：OpenCV 5 的 core/imgproc 静态包导出尚未构建的 libtiff archive；完整原源码编译、fd 错误观察和截图都没有执行。该失败保留外置，现已明确关闭此 raw-pixel 诊断不使用的图像编解码依赖，同时继续要求 IPP/CPU 优化；后续原生批次已通过完整原 cpp 配置/编译/链接，随后在归档并不存在的 libdrm COPYING 时失败，仍没有执行截图。现按该 tag 实际 meson/header 许可声明及完整头文件/源码归档，后续运行待验。回调首个故障、buffer 归还错误和 core disconnect 清理错误分别记录。Linux 轻量日志/原图诊断在完整工程归档前上传，便于定位真实失败。

源码入口安排 20 次独立 fd/init/三次截图/inactive/close 生命周期，按原 API 检查 60 张已知 BGR 和 fd/线程数，原对象关闭后继续持帧并跨线程读取；每张可解释布局的原图在颜色断言前保存。图样携带生产端序号并使各行内容随序号变化，持帧需保持原序号和像素；重复/倒退仅记诊断，不把它推断为原 API 的回放状态。转换库初始化在捕获资源基线之外，原构造函数的 `pw_init` 由测试在原对象完全销毁后额外 `pw_deinit`；这不证明原析构本身配平初始化。另在独立进程观察真实 `connect_fd` 早期错误的描述符状态，只在原生对象清理后按 socket cookie 回收仍匹配的 socket，不预设所有错误都会消费 fd。入口核对实际头文件/运行时版本及动态符号所在库。原 API 不暴露捕获时间/回放状态，耗时只记诊断。它仍不是 Rust 原生 driver、完整 C++/Rust 对照、framework/input、格式/故障/DMA-BUF 矩阵或 P1 验收。

首轮实际 Linux 执行已通过三个协商 POD 的 Rust/原源码字节比较，并观察到 connect_fd 早期失败 errno95 后输入 socket 仍打开；这不等于一般产品 fd 失败所有权已解决。随后原 C++ 与 Rust 都在协商时超时：私有 daemon 不含 session manager，P0 曾显式创建 link，完整 U 的 AUTOCONNECT 请求则没有被外部路由。生产端以零帧在正常 SIGTERM 清理后返回1，两个捕获失败及停源失败均保留。现增加独立自有路由监督器，仅核对指定 mfs-p0-fixture ID 和完整节点信息中的上游 media.name/node.target/autoconnect，管理这对 port0 的非 linger link；不选择宿主/默认来源、不重试失败 link，也不改产品属性/等待。监督器的 native 结果待验，P0 原有显式 link 路径不变。

包含路由监督器的首次原生批次已取得原 C++ 六张正确 BGR、Rust raw 三张正确 RGB；两项分别在第二/第一生命周期关闭后多出一个线程，严格资源断言失败。独立 Rust OpenCV 探针通过 20 生命周期、60 张已知 BGR 的原生关闭后持帧、20 次转换入口回调推进、buffer 归还与 owner 释放检查；私有 ABI/真实异常测试也通过。监督器 23 次 link 创建、绑定、销毁配平，生产端和 daemon 正常退出。这些局部证据不撤销两个资源失败，也不构成完整原源码对照或 P1 验收。新增线程身份、名称、状态和 thread-loop/data-loop join 日志诊断；计数首次不符后才记录延后观察，原失败值固定保留，不以稍后恢复改变结果。诊断尚待新原生批次验证。ready 文件在完整关闭临时文件后原子发布，以免父进程在写入期间归档。

另有独立 `--joined-task-observation` 原生控制进程，不创建捕获对象：完成一万次真实 `std::thread::join`，逐次检查刚 join 的 Linux TID 是否仍被 `/proc/self/task` 枚举，记录前 20 个残留观察及首次观察后的状态。该控制仅报告实际观察次数，不预设必须出现或不出现；它不能替代 capture 的注销/join/资源断言，也不能撤销失败。该控制已在原生 Actions 观察到 join 后短暂可枚举的 TID。

另在独立未运行的 main-loop 上显式拦截所选 fd 的 `add_io`，观察真实 `pw_context_connect_fd` 后期 ENOMEM、一次拦截和完整 teardown 前后的 fd 状态；其余原生方法和 data 不变，退出前恢复方法表。这仅为测试隔离进程的错误边界探针，尚待原生编译/执行，不是产品连接替代实现，也不证明多线程宿主中按 socket cookie 清理或所有失败分支安全。

独立原生格式 oracle 已在 Actions 通过：直接使用固定原源码的七项 SPA/通道/转换表，生成 28 个普通值/Choice(None)、有/无 Linear modifier 的有效 POD，同时保存原解析结果、带 11 字节初始化行填充的输入和真实 OpenCV BGR。Rust 按原 POD 解析并取得独立目标行，覆写原输入、清除 cache 后逐字节比对 24 次转换和四次 BGR owner 复用。该矩阵没有 native buffer、回调或真实流格式切换，不能代替完整格式协商、损坏 POD、DMA-BUF 和端到端生命周期验收。

后续线程诊断确认：原 C++ 的多余 TID 在读取其详细信息时已消失；Rust OpenCV 的 MaaScreencap 任务带 PF_EXITING、vsize/RSS 为零、fd 表为空，1ms 观察即消失。该批 raw Rust 的 20 生命周期/60 张 RGB 及关闭后持帧检查通过；原 C++ 和 OpenCV 的即时 proc 数量断言仍保留为失败。后期 add_io 探针已实测返回 ENOMEM、恰好一次拦截，fd 在完整 teardown 后仍打开。早期和后期观察都不等于一般 fd 所有权契约已完成。

据此修正测试的 proc 退出时点前提，已在受控原生批次通过：关闭前记录活跃 MaaScreencap 的 TID/starttime；原生 stop/join、对象销毁及严格 fd 检查之后，仅允许同身份任务已经消失或带 PF_EXITING 且无地址空间/RSS 时进入最长 100ms 的内核任务移除观察。原 baseline TID 必须全部保留，最终集合须完全恢复；未知任务、TID 替换、仍有用户态地址空间、fd 不符或逾期均失败。C++/Rust 均实际观察到约 1.1–1.3ms 的内核任务移除。测试没有等待仍活跃的 callback 完成；旧批次的即时数量断言不改判，捕获实现、等待和销毁顺序保持原样。

私有 Rust driver 的 supplied-fd 建连切片已通过受控成功路径验证：与 daemon 方式共用原顺序的 loop/context/core/listener/stream 建立，只在非空 core 后将原 fd 数值置为 -1，close 不恢复它，后续 init 沿用原源码的 daemon 选择。该入口是显式 unsafe 的 U 适配，要求调用方保证独占 socket、移交前解除原 owning handle、完成原生清理后才处理失败残留；没有推导空 core 的一般所有权，也没有添加泛用 fd 清理或重试。独立进程用例通过 20 次私有 socket、60 张 RGB、归还/持帧及已消费 fd 在原生关闭后为 EBADF；它验证 fd 传输成功路径，重新初始化、实际 Portal 授权、通用失败/重试和安全宿主接口仍未完成。

同一最新受控 RGB 批次中，完整原 C++、Rust daemon、Rust supplied-fd、Rust OpenCV 各通过 20 生命周期、60 张关闭后持帧，原图独立校验共 240 张；路由 80 次创建、绑定、销毁配平且自有进程正常退出。真实七格式/28 个有效 POD 和转换字节、私有 ABI/异常测试继续通过。源码与 Rust 分别采集生产端的不同序号，按各自序号核对已知图样，未声称同一时刻采样或源码延迟对照。owned mapping、DMA-BUF、完整流格式/故障/缓存矩阵、mobility、Linux manager 与其他平台完整迁移仍未完成；这些局部通过不放行整个 P1。

无 data 指针的 MemFd 映射已迁移，原 helper/手动描述符路径已有原生 Actions 对照，实际 NULL-data 流仍未验收。私有 rustix owner 保持原 `PROT_READ`、`MAP_PRIVATE`、零映射偏移及 maxsize/缺省 chunk.size 长度选择，只借用 native fd；目标行复制后先 unmap，再通知成功和归还 buffer，失败/展开同样释放映射。新增 fstat 验证实际文件覆盖目标行可读跨度；这项安全检查增加一次系统调用，不将其称为性能优化。非零 owned mapoffset 仍显式拒绝并留待完整来源/偏移审阅，不提前改为 P3 偏移处理。已有 SHM 指针仍由 native lease 保活；DMA-BUF 同步和完整清理失败矩阵仍未验收。

显式 MemFd helper 入口编译固定原 `ScopedBufferMap`，生成 20 个真实 memfd、60 组 padded/maxsize-zero/无 chunk 的 RGB/BGR 参考，检查原生只读/private VMA、unmap、fd 保留与覆盖/关闭后的持帧。Rust 对照 native `spa_data` 布局、60 次复制映射及 60 次单独权限观察映射，复制入口返回后要求 VMA 消失，关闭 fd 后保留 60 张 RGB/BGR；另检查负/过宽 fd、非零 mapoffset、短文件、空跨度和真实 mmap 权限失败。额外 60 次观察映射仅属于测试；不声称与源码映射次数或耗时相同，也不将手动 SPA 描述符当作真实 PipeWire callback/queue 验收。映射与 unmap 错误分别保留，未用后者覆盖首次复制错误。

首个 owned MemFd 原生批次在 Linux 测试编译时因 rustix 权限错误常量拼写失败，尚未运行上述 helper 或任一取帧 probe；失败日志与诊断归档保留。常量修正后的后续原生批次通过上述 helper、六类拒绝及现有四条受控取帧路径；mmap 权限失败实际为 errno 13。Rust 的 60 次复制映射和 60 次额外权限观察映射各自 unmap 配平，未消费 native fd，关闭后保留 60 张 raw RGB 与 60 张 BGR。原 helper 的 60 张 RGB/BGR 和 60 份带填充输入另有独立像素/填充复算；本机格式/Clippy 不代替这些 Linux 结果，也不撤销此前失败。

同一对象生命周期的两个独立入口已通过原生 Actions：完整原 C++ 的 `--reinit` 与 Rust 的 `reinitializes_after_consumed_fd_and_retains_both_sessions_cpu_images`，由 `upstream-context pipewire` 分开执行。每侧 20 个对象各经历给定 fd 与后续 daemon 两个成功 session，保留 120 张原 BGR/独立 raw RGB；检查首次 init 前重复 close 不消费 fd、已连接 init、重复 inactive/close、关闭后拒绝且保留原输出、已消费 socket 关闭、缓存清空及资源基线。Rust 另直接检查已连接 init 不替换 native 对象、关闭后原生对象为空；callback 状态由同一 Rust 对象保活至最终析构。该批次的现有取帧/MemFd/reinit 共 600 张保存图像逐像素独立复算，路由 create/bound/destroy 各 160 次且自有进程正常退出。这仅对照 U 在成功 core 后将存储 fd 清为 -1 的行为；通用失败输入所有权、Portal、失败重试与产品 SDK 的来源恢复政策仍未验证或定案。

完整 `init` 的失败/显式重试对照已通过受控原生 Actions。独立 `--fd-retry` 与 Rust `retries_proven_live_fd_after_native_add_io_failure_and_retains_both_sessions` 以私有 ELF interposer 在自有未运行 loop 上仅拒绝目标 socket 的一次 `add_io`，其余方法、callback data 和实际 native connect/失败清理保持，返回前恢复原表。每侧 20 次真实 ENOMEM、完整清理后的原 SO_COOKIE、仅保留该 socket 的 fd/任务基线通过后，才显式提交同一已证实存活的 socket；后续各 40 个成功 session 保留 120 张图像。控制计数为两次 connect_fd、一次拒绝，首次 native errno 仍保留。只有这两个隔离消费进程加载 interposer；Rust 通过 Cargo JSON 找到已编译 libtest 后直接运行，Cargo/rustc、生产端、路由和 daemon 不加载它。该测试不自动关闭未知残留 fd，不改变产品重试行为，也不能推导全部 native 失败或 Portal 的所有权。

该批次的独立 joined-task 观察在第 542 轮因诊断 I/O 失败中止，不能记作整批通过：PF_EXITING 线程的 status 文件打开后无内容，`output << input.rdbuf()` 在零字节提取时设置输出 failbit。原始部分文件与失败日志保留；其余取帧/资源检查和两侧新 fd 用例均通过，840 张保存图像与 60 份带填充输入也已独立复算。诊断代码现按固定块显式 read/write，分别记录 open/read/close errno 和读取字节数，真实输出失败仍使检查失败。后续 Linux 批次的 65 项检查全部通过，包括空/正常/缺失文件、实际 `/dev/full` ENOSPC 和 10,000 次 join 观察；其 840 张图像也再次独立复算。没有调整捕获、等待、任务身份判断、资源计数或其通过条件。

后续又保留一次原版 reinit 的 `/proc` task/stat 读取失败：组件 75 项通过、一项失败。旧 ifstream 读取未记录 errno，不能据此证明 kernel retirement 因果或截图/资源泄漏。现仅在原 good-or-eof 拒绝之前补存 TID、errno、stream flags 和读取字节数，不把不可读 task 当作消失，不重试捕获，也不调整 FD/task 身份或原通过条件；新增诊断待原生验证。

DMA-BUF 私有迁移已接入回调的单 plane packed 路径。首个原生批次在 C++ kernel fixture 编译时触发 int 到 Linux u64 字段的列表初始化窄化错误，未运行 DMA 或源码捕获；现改为字段赋值，后续原生 Linux 编译、Clippy 与原/Rust 负向控制通过。正向 kernel helper 两项因 runner 无权打开已存在的设备而返回 missing_environment/77，其余 69 检查通过，840 张既有图像独立复核正确。准备脚本补齐 GitHub 托管临时 VM 上既有 udmabuf 的测试 owner 后，后续原生批次确认真实 udmabuf 导出及两侧 helper 正向通过；960 张保存图像和 120 份带填充输入已独立复核。该批次的 73 检查仍有两项生产器构建/设置失败：timer API 需要可变 timespec 指针，夹具原传入 const 对象；现按 API 修正，不用 cast 绕过限定，实际流仍待原生验证。新 owner 保持无指针分支的 `PROT_READ`/`MAP_SHARED`、零偏移和 stride×height 映射长度，记录一次 READ START；仅在 START 成功后执行 READ END，再 unmap，之后才返回 buffer。已有指针按 U 跳过映射/同步；START 失败仍按源码保留指针并继续复制，错误独立保存，不把这种 U 结果称为 SDK 一致性保证。P3 的失败拒绝与 EINTR/EAGAIN 重试没有提前引入。额外 fstat 可读跨度检查增加一次系统调用；非零 owned mapoffset、多 plane 和未知布局明确拒绝，完整 modifier/格式/故障矩阵仍未完成。

原 helper 与 Rust 的 ENOTTY 控制使用明确无效的普通 MemFd，仅验证 START 失败／已有指针选择，不计为 DMA 成功。正向 helper 必须由 `/dev/udmabuf` 或 `/dev/dma_heap/system` 实际导出 DMA-BUF：每侧 20 个 fd/60 次复制，生产端 WRITE START/END，原 READ helper、Rust READ/SHARED/END/unmap、关闭后 RGB/BGR 持帧和 C ioctl ABI分别检查。Rust 额外 60 次权限观察映射独立计量。该夹具不证明实际 PipeWire DMA 回调或 GPU 完成同步。仅在 `RUNNER_ENVIRONMENT=github-hosted` 的临时 Actions Linux VM 上尝试加载已安装 udmabuf 模块并设置新测试设备 owner；不下载驱动，不对本地/自托管主机自动提权。两种 exporter 都不可用时返回 77，报告 `missing_environment`，保留其他独立结果且整批不通过。

另已接入独立 CPU 写入的 kernel DMA 生产器和实际流验证入口；首个实际生产器构建的 timespec const/API 错误已修正，新的原生构建/运行尚未验收；生产器编译单独记账，缺少 exporter 不能掩盖其构建结果。普通 RGB 生产器与路由先完整停止，随后在同一私有 daemon 上创建新的 DMA 生产器与精确路由；注册/移除等待只属于夹具就绪，不重试捕获。生产端通过 ALLOC_BUFFERS 提供真实 DMA-BUF fd，完成 WRITE START/copy/END 后才归还，不设置 MAPPABLE；捕获端保留 U 的 MAP_BUFFERS。原源码与 Rust 各要求 20 个流生命周期/60 张已知图像，Rust 还要求真实回调的 60 次 NULL-data owning map、READ START/END/unmap、无 borrowed bypass、queue 与资源配平，关闭后仍保持独立 CPU 图像。原源码输出为 BGR，Rust 该入口保留 RGB；各自按生产序号核对，不声称同一时刻逐帧对照。原源码回调指针没有被单独插桩，GPU 异步完成与 Portal 授权仍未覆盖。

`cargo xtask upstream-context wlr` 是原生 Linux x64 原源码入口，原 C++ 编译和受控运行已验证。它直接编译固定的 WlrScreencap、WaylandClient、MemfdBuffer、WaylandHelper 与原有生成 C 协议文件；截图 base 从原声明抽取，日志、私有 namespace 和外部 UUID 名称提供者有明确适配边界。名称提供者用 getrandom 生成 UUID 形状的随机文件名，不验证原 Boost UUID 工具；真实 memfd 创建/截断/映射、Wayland buffer、native 连接和 OpenCV 转换保持原调用。该入口现也运行下述 Rust 捕获切片，完整框架/P1 对照仍未完成。

自有 compositor 的显式 source-client 模式仅额外注册原客户端强制要求的 virtual-keyboard/virtual-pointer manager；不创建输入设备或执行输入动作。参考进程要求 20 次连接/三次截图/关闭生命周期、60 张完整已知 BGR 图像、fd/任务身份配平及关闭后独立持帧。链接器 wrapper 只记录并立即转发真实 wl_display_roundtrip，要求观察到原 Buffer 回调内的递归 roundtrip；不提前把它改为排队或移到回调外。完整 source 失败/格式/客户提前关闭矩阵仍待实现；Rust 递归执行通道的原生验证见下文。独立轻量诊断 artifact 在通用工程归档校验前上传，完整工程包仍为来源/依赖重建依据。

Linux 两个原源码入口共用 [OpenCV 准备函数](../../scripts/prepare-opencv-reference.ps1)：每次重新查询正式发布及 tag，核验源码，保留 core/imgproc、CPU/IPP/HAL 优化和原 codec 发现排除规则。源码/编译器/选项/构建环境构成共同缓存身份；后端上下文仍按批次单独归档。共用准备不引入核心 OpenCV 依赖；共用准备已通过下述原生批次。

后续原生 Linux 批次确认 DMA 实际流通过：PipeWire 组件 76 项检查全部通过，原 C++ 与 Rust 各 20 个流生命周期/60 张关闭后持帧，Rust 记录 60 次 NULL-data owning map、READ START/END/unmap 和缓冲归还。独立复算包括 1080 张 PipeWire 保存图像和 120 份带填充输入；这不证明 GPU 异步完成、Portal 授权或完整 P1。此前权限缺失及生产器 C 编译失败保留。相同批次的 Wlr 原源码组件 82 项检查通过，60 张保存 BGR 也独立复核；真实递归深度为二，roundtrip 总调用数只作观察，不作延迟/CPU 优化结论。

私有 `linux-wlr` 切片已开始真实 Rust 迁移。首轮原生编译暴露两个 manager 代理类型推断、调用端多传一个 ABI 参数及异常 Result 保活借用错误，四个 Rust 入口均未运行；83 个其他组件检查通过。按原 C 头的八参数 ABI 修正后，使用合法两通道 Mat 触发真实转换异常。随后原生批次的布局、20 生命周期/60 组 raw+BGR 与异常释放/后续捕获均通过；87 项检查仍有一项 Clippy question_mark 失败，等价 `?` 修正后，后续批次 87 项 scoped 原生检查全部通过。它使用维护中的 Wayland 绑定及 ObjectData，按原 advertised version 绑定并保留最后 output、GlobalRemove no-op、被动 manager 前提、每次新 frame、Buffer 内创建/复制、原 roundtrip 点、Flags 忽略和转换后的 frame 销毁。高层 dispatcher 持有非递归锁，故仅在活跃 ObjectData/TLS 回调内通过一个头文件检查的 C 队列 getter，调用原生 roundtrip_queue；不跨递归持有 Rust 可变状态借用或状态锁。这个窄缺口由 build.rs 按实际 TARGET/feature 在验证配置后编入 OUT_DIR，Wlr 未启用或其他 OS 不探测/链接该接口；需要选中 libwayland 的真实 getter，不静默降级。

原源码 Mat 不传 stride，Rust 继续其 packed-step 解释和未处理的反转 flag，并记录实际 native stride。独立 raw CPU 副本在 native lease/映射有效时产生，随后执行真实私有 C++/OpenCV 转换，frame 代理仍存活到转换结束；这增加复制成本，不称作消除工作。BGR 可复用独立 CPU owner。普通 create_buffer 失败继续原等待且不新增错误唤醒；Rust 自身回调 panic 以独立标量故障保存，不保证捕获原生/绑定的 aborting OOM。尺寸/跨度/窄化、超出绑定的协议版本、NUL 路径检查及 RAII 是明确安全适配；原 memfd FD0/失败部分状态、完整原错误/共享 client/early-close、所有格式/padding/flags、外国队列和 native mobility 尚未完成对照。该切片不最终确定 Frame/Session/ABI 或 P3 调度。

源码入口先做独立原 C++ 捕获，再以单独进程运行 Rust 布局控制、20 生命周期/60 张 raw+BGR 和实际转换异常/后续捕获检查；每个进程单独记录 native fd/task 基线。探针覆盖逐次实际 OpenCV 转换时 frame 尚存活、返回后已销毁，空闲 SHM 覆写不改变已持有图像，关闭后的 CPU 持帧及所有原生/Mat owner。该切片已通过上述真实 Linux 执行及独立 60 raw/60 Rust BGR/60 原 BGR 像素复算；组件的 Clippy 失败仍保留。后续复查另将 Buffer 字段改为 map 先于 file，使显式 proxy/pool 释放后的析构按原实现先 unmap 再 close；这一修正已通过后续 87 项原生检查及独立像素复算。Windows 本机的编译/格式/Clippy 只能证明对应宿主路径。

Wlr 原版入口另增加三个独立原生进程探针，最新组件 90 项检查全部通过：两个 capture 共享 client，在调用方及第一个 source 释放后继续取帧，20 轮保留 60 张图；无旧 capture/proxy 所有者时，同路径连接不重开、关闭后重开、别名改路径、缺失路径失败及随后恢复各 20 轮；FD 0 的 MemfdBuffer 原析构残留观察。最后一项仅夹具在观察后显式 munmap/close 并恢复自身 stdin，不算产品清理成功。新增 60 张共享 client、20 张重连图以及既有 60 组 raw/Rust BGR/原版 BGR 已分别独立复算匹配。源 FD 0 残留得到实际确认；原 Rust RAII 会释放 FD 0；现在 Buffer 对成功创建的 FD 0 分支显式保留映射和描述符，普通分支仍在 proxy/pool 结束后按 unmap→close 释放。新增 20 轮真实取帧探针，分别观察残留、覆盖闲置映射后验证独立 raw/BGR owner、夹具回收和 stdin 恢复，已通过原生运行，最新 91 项检查全过，20 组新增 raw/BGR/原版图已独立复算；失败构造和 partial-state 的 FD 0 对照仍未完成。这些探针用于确定 Rust 后续共享/重连/残留迁移，尚不证明对应 Rust 行为已实现或通过。

`cargo xtask upstream-context macos` 在原生 macOS arm64 显式构建固定的完整 ScreenCaptureKitScreencap Objective-C++ 源码，首轮 24 项原生检查已通过。它仅抽取原 screenshot base 声明，并使用私有 namespace/logging 排除无关 framework/input；原 MRC、内容枚举/窗口或首 display 选择、config、回调内锁定/转换/释放和五秒 semaphore 等待均未改写。最新正式 Xcode 与 OpenCV 从源码准备，后者保留 arm64 NEON/CPU/HAL 默认优化并记录实际 build information；Intel IPP 对该架构不适用，Linux x64 仍使用原有 IPP 选项。核心 crate 不依赖 OpenCV。

该入口的 AppKit 自有窗口在进程主线程创建、绘制和关闭，原同步捕获在逐次 join 的 caller 上执行，主线程只泵夹具事件。一个预热和 20 次窗口/source 生命周期要求 60 张 47×31 BGR，逐像素检查去掉两像素边缘后的 43×27 内部区域，并在 source/窗口关闭和另一读取线程上验证保活；完整保存行包含未断言的边缘。缺失 window 返回 nullopt，另一个进程以 window_id=0 观察首个 display 的实际非空图像、布局及 source 关闭后 owner；该 display 检查不称作已知桌面像素验收。保存的 60 张窗口图和预热图已独立复算，首 display 为 1024×768 BGR，布局/长度匹配。资源 TSV 只观察 FD/thread/Mach-port 数量，不声称配平：Mach port names 从 260 增至 381，仍需控制实验定位原因；FD 8→9 是基线后打开 TSV 所引入的夹具差异，现改为打开并检查记录文件后再采样基线，原生复验确认基线及每轮 FD 均为 9、线程基线/末轮均为 7，port names 仍从 262 增至 380；原 semaphore 残留与 port 增长因果、原版超时/错误对照及完整 display/格式/manager 仍待完成；下述初始 Rust 迁移尚待原生验收。八秒夹具 watchdog 只结束独立失败进程，不 detach/cancel 原生调用。缺少 Screen Recording 权限返回 missing_environment，不修改 TCC、不弹授权。构建、两种运行及轻量诊断 artifact 分别记录，阶段仍为 P1/incomplete。

私有 `macos-screen-capture-kit` feature 已开始迁移同一原版一次性捕获，独立于 AppKit 原型 feature。首轮原生编译/Clippy 和首 display 布局/CPU owner 探针通过，后续宿主修正后的 40 项原生检查已全部通过。它使用 objc2/block2/dispatch2，保留首个精确 window ID 或首枚举 display、BGRA/sRGB/无光标配置，以及提交内容枚举之后开始的原生五秒 semaphore 等待。实际调用方转换仍在 sample callback 的锁内完成；超时不取消后续转换，回调保活输入、转换器和晚到结果。受检布局/锁定、额外独立 CPU 行复制、fallible owner 和异常边界是明确的安全适配，复制成本没有被消除。

另新增首 display 的受控内容入口：原版 `display-patch` 与 Rust 精确测试各捕获三次默认 `window_id=0` 的完整显示器输出。独立 AppKit 宿主按实际首个 SCDisplay ID 匹配 NSScreen，将 47×31 不透明绿色窗口置于其内部；原生 SCDisplay/SCWindow 的完整矩形、配置尺寸、窗口 ID、PID 与可见性固定检查区域。原配置继续使用 display.width/height；[Apple 将这两个属性定义为 point 尺寸](https://developer.apple.com/documentation/screencapturekit/scdisplay)，本入口不将其重解释为物理像素或引入 P3 缩放政策。源/Rust 各三张即时 BGR 与关闭后持帧、Rust 原格式 CPU 副本共十八份输出由 `cargo xtask upstream-trace macos-display-patch <外部目录> [--check-only]` 校核：已知绿色内部区域、Rust 全图 BGRA→BGR 字节、完整持帧、原生几何身份、回调/锁/输入释放及显式 semaphore 夹具回收记录。Rust 在场景窗口实际关闭确认之后跨线程读/释放 CPU owners；桌面其他区域不预设固定像素，也不跨不同取帧时点比较全桌面字节。首批原生 macOS 记录中 C++/Rust 编译、Clippy、原有 display 布局用例及新宿主窗口正常关闭均通过，但新增原版首 display 的第一次取帧约 5.06 秒返回 nullopt，尚未取得任何新图像，Rust 新用例因此未执行；组件 70 项中 68 通过、2 失败，失败继续保留，不能称为几何/像素断言失败或通过。后续诊断编排保留原失败，独立执行已知窗口区域的 Rust 观察，保存其 None/错误/超时返回及同一次调用的晚到 owner 记录；完整校核仍要求原 oracle 全部成功，不以单侧成功或晚到结果接受失败调用。该诊断编排的后续原生批次已实际执行：71 项组件检查中 69 通过、2 失败；原版再次约 5.12 秒返回 nullopt，独立 Rust 三次取帧及窗口关闭后持帧均通过，完整校核明确因原 oracle 失败而拒绝成对验收。下载后的 26 份相关实际文件均核对登记哈希，另以独立字节检查复核十二份 Rust 即时/持帧文件、绿色区域、全图 BGRA→BGR、调用返回和 owner/锁记录；这些仅证明 Rust 侧观察，不能推断原版失败已修复。Rust 本次没有失败返回，十五秒晚到观察分支仅通过原生编译，尚无该分支的运行证据；原五秒等待、选源与捕获行为未改变，消费进程额外内容查询及初始化历史的影响仍待对照；旧 display 单次布局观察及零/多显示器、混合缩放、完整资源/后端和 P1 仍分别待验收。

另保留独立的初始化历史诊断：原版 `display-patch-host-premise` 在新进程中省去消费端附加的 SCShareableContent 几何查询，直接执行未改写的默认 display 捕获；独立宿主的实际几何只作为单显示器的预期条件，输出明确标为 `source-expected-geometry.tsv`，不声称实测了消费端枚举到的 display 身份。对应 Rust 探针和 `cargo xtask upstream-trace macos-display-host-premise <外部目录> [--check-only]` 仍检查双方各三张已知区域图、十八份即时/持帧、完整转换和释放记录；该报告始终标为诊断，不能替换或接受原查询用例。首次原生对照已完成：75 项组件检查中 71 通过、4 失败；带查询原版约 5.11 秒返回 nullopt，省去查询的独立原版约 5.15 秒仍返回 nullopt，两个独立 Rust 用例各三次取帧通过。下载文件中 53 份相关原生输入、二十四份 Rust 图像/持帧、完整转换与释放记录另经独立校核，两对失败记录也被本机校核器严格拒绝。附加查询不足以单独解释失败；进程间系统服务状态与执行顺序尚未隔离，根因仍待原生分支证据。

单独的 `mfs_macos_observed_context` 验收可执行程序现加入 semaphore 透传诊断，记录真实 create/wait/signal 返回、线程与原错误日志。固定上游源码仍作为独立翻译单元，强制包含的观察头只用于这个可执行程序；未观察的原 oracle 仍独立编译并先执行。若原调用已失败，夹具再观察同一次调用的 signal 至多十五秒，不重新取帧、不延长原五秒等待，也不宣称排空了原生 owners 或回收原残留引用。观察 IO 会影响时序，输出目录、状态及图像与原 oracle 分开，不能供成对验收。原生批次现已完成：78 项组件检查中 74 通过、4 失败；未观察原版两例仍约 5.06/5.15 秒返回 nullopt，但单独观察程序两例各三次捕获通过，六次真实 wait 返回均为 0，十二份即时/持帧文件及等待、signal 和线程记录经独立校核。两个 Rust 用例也各三次捕获通过，二十四份图像/持帧另经完整转换和所有权复核；未观察原版的失败成对记录继续被严格拒绝。观察程序没有复现失败，其非零 wait 返回及十五秒晚到观察分支尚无运行证据；日志时序、二进制身份或其他差异的具体因果仍待进一步对照，不能把观察程序成功当作原失败已修复。

新增的 `rust-macos-display-distinct-host` 是另一项独立对照：场景宿主改用同一原版可执行文件的字节相同副本，消费端继续使用原路径和未加观察的 `display-patch`。准备阶段核验两份文件的 SHA256 与执行权限；该例存档两份实际二进制、运行路径与宿主 PID，`cargo xtask upstream-trace macos-display-distinct-host <外部目录> [--check-only]` 重新计算实际文件哈希并执行相同的几何、十八份图像/持帧、转换与释放检查。该例不运行 semaphore 观察器，报告保持诊断范围，不能替换原同路径用例或证明根因。原生批次已执行 82 项检查，78 通过、4 失败；新用例的未观察原版与 Rust 各三次取帧、十八份图像/持帧及严格记录校核均通过，旧的同路径原版及其成对检查仍失败。下载后另核对 106 份相关原生文件，独立复算新用例像素、完整转换、持帧、宿主 PID/权限与两份二进制的逐字节一致性；本机记录校核再次通过新用例并拒绝旧失败对。macOS 诊断上传已补齐实际编译命令、原来源身份与范围说明，本批已据此独立复核五份固定上游文件及两种编译目标的区别。此结果建立了不同宿主路径下的成功对照，尚不确定路径、文件身份、系统服务或调度的具体因果，完整 P1 仍未验收。

另加入 `display-observer-same-path`：场景与消费进程都使用同一路径的 `mfs_macos_observed_context`，场景保留既有 Foundation loop，消费端仍调用 `display-patch`，原生 create/wait/signal 仅由独立观察程序透传记录。`upstream-trace macos-display-observer-same-path <外部目录> [--check-only]` 要求两份归档二进制、相同路径/权限/实际 PID、原生几何与窗口关闭确认、三次已知图样和六份即时/持帧，以及完整的三组真实成功等待/信号、调用线程连续性和时钟顺序。该控制不运行 Rust，不生成或替代未观察原 oracle；报告固定为诊断且不接受成对验收。首轮原生组件 99 项中 87 通过、12 失败：同路径观察消费端等待约 5.05 秒返回原生非零值 49，后续十五秒观察内仍无 signal；场景正常退出，新增消费及严格记录校核失败，旧十项失败保留。此前不同场景路径的观察成功不能推广到同路径，具体因果仍未确定。

为区分内容查询和截图回调，独立观察目标另加入编译期 selector 别名及窄 Objective-C category 转发：保持原 filter/config、回调结果、native 执行通道和五秒等待，记录 submission/callback/原处理器返回，以及线程、时钟、原 NSError code、首 display 和配置尺寸；不改写未观察的原源码字节或在运行时替换系统方法。转发 block 仅捕获原 completion 和值，不额外保活 filter/config/sample，依赖 [Clang Blocks ABI](https://clang.llvm.org/docs/Block-ABI-Apple.html) 的 block 捕获复制/释放；[内容查询](https://developer.apple.com/documentation/screencapturekit/scshareablecontent/getwithcompletionhandler%28_%3A%29?language=objc)和[截图完成](https://developer.apple.com/documentation/screencapturekit/scscreenshotmanager/capturesamplebuffer%28contentfilter%3Aconfiguration%3Acompletionhandler%3A%29?language=objc)仍由原生接口回调。`upstream-trace macos-capture-stages <外部观察目录> [--check-only]` 可分析失败日志中的未观察回调/未返回处理器，固定声明 `source_capture_accepted=false`；它不以晚到结果接受失败调用，也不证明取消、排空或完整 P1。该补充已在原生 Actions 编译并执行：100 项组件检查中 88 通过、12 失败，新增日志分析通过，原有失败全部保留。同路径消费端三次内容查询均有成功回调及原处理器返回，最后一次处理中已提交 1024×768 的原生截图；五秒等待返回 49，之后十五秒观察仍没有 sample callback、signal 或 source error。从截图提交到观察结束约 20.12 秒；这定位到该次记录中未观察到截图完成回调，不证明永久不完成或确定根因。下载后核对 1927 份可用登记哈希、五份固定 F 源文件、实际二进制/路径/权限/PID 和全部原生阶段记录，严格捕获门禁继续失败；完整工程归档现已核对该次实际父来源 manifest、76 份 F 源码 blob、五份编译来源及全部 1963 份登记组件输入/日志哈希；捕获失败判定不变。额外 block 层和日志可能改变观察时序，旧记录保持原身份。

新增 `rust-macos-display-foundation-consumer` 宿主控制：场景仍由同路径、同字节的原版程序以 AppKit 创建，只有原版消费进程不调用 NSApplication 初始化，继续在主线程运行 Foundation default run loop。原捕获类、消费端原生几何查询、默认首 display、五秒等待和 Rust 测试宿主均不变；原同路径 AppKit 及其他诊断仍分别执行。`upstream-trace macos-display-foundation-consumer` 额外要求实际同路径/二进制/权限/PID、捕获前后 NSApp 均为 nil、原 oracle 成功及原有十八份图像/持帧校核；失败不能以 Rust 单侧结果或宿主条件替代。首轮原生批次 88 项检查中 82 通过、6 失败：新增原版约 5.02 秒后返回 nullopt，捕获后 NSApp 已非 nil，原 oracle 及其成对记录检查均失败；旧的两个同路径原版及其记录检查继续失败。跳过夹具显式初始化没有建立无 NSApplication 的捕获前提，也没有解决超时，不能据此确定内部初始化位置或根因。

`rust-macos-display-appkit-consumer` 进一步隔离主线程事件处理：只在该原版消费模式启用 `updateWindows`、带五毫秒 deadline 的 `nextEventMatchingMask` 和有事件时的 `sendEvent`；场景、其他原版模式及 Rust 宿主保持原执行方式。[Apple 事件循环说明](https://developer.apple.com/library/archive/documentation/General/Conceptual/MOSXAppProgrammingGuide/CoreAppDesign/CoreAppDesign.html)列出窗口更新、取事件和分发事件，而原夹具只运行 Foundation run loop。捕获后保存主线程、NSApplication、poll/event/update 计数与原捕获状态；失败保留主错误和记录错误。`upstream-trace macos-display-appkit-consumer` 校核这些状态以及同路径二进制/PID、六次已知像素捕获和十八份持帧/转换记录；零事件明确保持零，不把轮询称为实际事件分发。首轮原生组件 92 项检查中 84 通过、8 失败；新增原版约 5.15 秒返回 nullopt，主线程/NSApplication 前提保持，实际 poll/update 各 920 次、event 为 0，新增原 oracle 及其记录校核失败，原有六项失败仍保留。该显式事件处理控制并非完整 `NSApplication.run`，也没有解决超时；不更改后端默认行为、原五秒等待或旧失败的验收状态。

新增独立生产端控制 `rust-macos-display-appkit-scene`：同路径、同字节的场景进程使用 `display-scene-appkit`，仅它启用上述 AppKit 事件处理；原版消费端仍使用未观察的 `display-patch` 和既有 Foundation loop，Rust 宿主不变。窗口实际关闭后、发布停止确认之前保存生产端的 NSApplication、主线程和 poll/event/update 状态，以 `window_closed` 表示生产者完成，不能替代消费端的 `source_capture_passed`。`upstream-trace macos-display-appkit-scene` 要求生产者状态、同路径/二进制/权限/PID，以及原 oracle 成功、双方六次已知区域捕获、十八份图像/持帧与完整转换/所有权校核；原有控制及失败分别保留。首轮原生组件 96 项中 86 通过、10 失败：生产端确在主线程轮询/更新各 959 次，取出并分发 2 个事件，窗口关闭确认通过；原版消费端仍约 5.14 秒返回 nullopt，新原 oracle 及成对校核失败，原有八项失败仍保留。独立 Rust 三次取帧及十二份持帧/转换输出通过；下载后另复核 1908 份可用输入/日志哈希、两份实际二进制、路径/PID、原消费模式及全部十二份 Rust 图像和回调/锁/释放记录，失败对继续被校核器拒绝。生产端显式事件处理没有解决该次超时；它不是完整 `NSApplication.run`、根因证明或 P1 验收。

原 MRC 函数没有释放 semaphore 的创建引用，P1 显式保留该残留；仅验收夹具在调用方及原生块所有者都结束后回收，不能称作产品清理成功。入口现在编排独立进程场景，先用未改写的原版捕获同一窗口，再分别执行 Rust 精确测试：布局/尺寸、20 轮及 60 张 raw/BGR 保活图、真实转换异常后的 Unlock/下一次取帧、缺失窗口、五秒超时及六秒夹具转换延迟、首 display 布局。窗口关闭确认先于跨线程 CPU Mat 读/释放；FD 比较在显式夹具回收之后，线程/Mach-port 仅作观察。私有 C++/OpenCV 适配头、原生布局查询和真实消费验证 ABI；默认产品仍不依赖 OpenCV。首轮组件为 37 通过/6 失败：三个窗口进程在 CGS_REQUIRE_INIT 断言处 SIGABRT，对应 probe 也失败；四个原版同窗口 oracle 与场景关闭确认通过，Rust 窗口 raw 数量为零。失败记录保留。新增测试专用 main-thread host，在主线程初始化/泵 AppKit，另一个已 join 的线程调用标准 libtest runner，保留精确选择、ignored、报告和失败退出；它匹配原版夹具宿主前提，不修改核心初始化政策，修正后的原生宿主已通过。随后新增宿主的原生批次因 NSDefaultRunLoopMode 类型误用（将非空 NSString 引用当作 Option）而编译失败，保留 33 通过/11 失败记录，没有宿主运行或 Rust 窗口像素证据；现按实际绑定直接使用引用，修正后已通过原生编译/运行，旧失败仍保留。超时探针另用显式闸门保活晚到工作直到调用方 owner 释放，并检查实际转换成功，避免只观察回调结束。最新 60 组 Rust raw/BGR 已独立复算，完整 BGR 字节匹配同窗口原版，四项精确原生测试及场景关闭确认通过；资源基线/每轮 FD 均为 8、线程观察均为 8，显式夹具回收后 port names 239→240，不作 semaphore 因果或产品资源配平结论。新增 capture-only 直接 Cargo 构建及 normal/build 依赖树检查（不选 prototype/AppKit crate、临时清除并恢复 P0 配置变量）已通过，原生组件由 40 项增至 42 项并再次独立复核图像，构建/依赖隔离不表示无需宿主初始化即可捕获；完整原版错误/空图/锁定、格式/几何/权限、display 像素和 manager 矩阵仍待完成。

私有 `macos-manager` 截图链的最新原生组件 49 项全部通过，包含 Clippy、capture-only/manager-only 直接构建及依赖隔离。320 组原版脚本对照和双方各 20 轮真实转发通过；双方各 60 张独立留存 BGR 已逐字节复核，Rust 启动前登记的 61 个原版 oracle 哈希在结束后均未变化。此前 48 通过、一项 Clippy 失败仍保留；首版夹具覆盖原图的批次不作为两侧图像存档验收。它复用同一 ScreenCaptureKit 实现，不引入 SDK Session 或输入控制。连接先清 connected，成功构造新 owner 后才替换旧 owner；OS 不可用或构造错误保留旧 owner，截图入口不额外检查 connected。None/未知方法仍可连接成功；nullopt/错误保持调用方原输出，Some(empty Mat) 成功并替换输出；UUID 为 window ID 十进制，inactive 恒真且不调用截图后端。四项 portable Rust 控制已在 Windows 通过，只证明该算法。验收抽取构造/连接/状态/UUID/截图/inactive 六段未改写原函数体，以原 MaaDef 类型/常量、明确的 availability/backend/Input=None 脚本和真实 cv::Mat 编排对照；构造故障是脚本后端异常，不称作实际 allocator/OOM 验证。同窗口原版及 Rust 真实生命周期、原版连接不截图和 source owner 释放、失败不换输出实际指针/owner、确认关闭窗口后跨线程持帧均通过。get_info 投影、完整 API/宿主/失败矩阵与后端/P1 仍未验收。

私有 Wlr 现已分离共享 client 的 registry/连接状态与每个 capture 的回调、缓冲区和结果；capture 复制原版 manager/SHM/output 快照并保活同一个标准 Arc client。可变 close/reconnect 只在旧 capture/proxy owner 全部结束、取得唯一 client 访问时执行；保留活跃旧快照的断连/重连仍待原版证据和安全寿命处理。新增两个独立进程测试，各 20 轮，分别检查调用方及第一个 capture 释放后继续取帧、最终 client 释放，以及同路径不重新绑定、显式关闭后重开、别名路径变化、缺失路径失败和后续显式连接；要求独立 raw/BGR 对照原图、分离的 native buffer、回调 owner 配平、关闭后持帧和精确 FD/task 基线，最新 93 项原生组件全部通过，新增 60 组共享及 20 组重连 raw/BGR/source 图和 40 行资源记录均已独立复核。此前成功 FD-zero 路径的 91 项组件及独立图像复核已有通过证据；构造失败/部分状态、早关、格式与完整后端仍未验收。

Linux manager 的私有 Input=None 编排已开始迁移，六项 portable 测试通过；它们只验证算法，不证明原生 Linux manager 取帧。连接先释放 capture 再释放 input，并保留已有 Wayland client；首次连接失败保留的 client 在下一次 manager connect 中不会自动重连，因此 manager connected 与真实可取帧状态可能不同。构造先于虚 init，失败后保留已选 owner，截图不增加 connected 门禁，也不回滚后端在失败时对输出的写入；inactive 按 capture、input 顺序调用。None 仍构造真实无截图方法，PipeWire 仅拒绝 fd<0 且 node=0，U 的 ExtImage 构造仍拒绝。UUID 保留原 Config.h 中 std::string 字段的原始字节，不加 filesystem 引号/转义；或使用有符号 fd/node，再附 Input=None 后缀。独立 C++/OpenCV 程序已编译九段未改写原源码区域（包括构造/析构、connect、UUID、转发及创建函数），使用真实 shared_ptr、原 Config.h 字段和 Mat，client/backend/init/构造边界显式脚本化。此前 Windows/Linux 的 960 组报告虽然匹配，shim 却将原 std::string socket 字段误写成 filesystem::path，不能作为 UUID 语义通过证据。现直接从原 Config.h 提取成员声明，保留真实类型/默认值并排除 JSON 宏；Rust 已去除错误的引号/转义，修正后 Windows 和原生 Linux 各 960 组对照通过；Linux 包含 240 个非 UTF-8 socket 用例，原始 string UUID 与两侧报告均已独立核验。旧报告及投影错误说明均保留。异常构造仍是脚本故障，不是 allocator/OOM 证据；真实 Wlr/PipeWire 工厂、配置/metadata/API 边界仍待接入和验收。

Linux manager 已接入同一私有 Wlr/PipeWire 实现的原生工厂，新增 Wlr 原版及 Rust 独立进程对照：各 20 轮、60 张留存图，含 inactive、复用 client 的 manager reconnect，以及失败 client 保留后 connected/截图结果分离。原图在 Rust 执行前记录哈希，之后检查未被覆盖；要求真实像素、关闭后 CPU 持帧及精确 FD/task/回调 owner 基线。None/未实现方法和非负 PipeWire fd 的 unsafe 接管边界有独立控制；普通安全入口拒绝未证明所有权的 fd。当前仅有本机编译隔离检查，新增 Linux 原生编译、Clippy、取帧及故障结果仍待 Actions；PipeWire manager 实际流转发和完整配置/API 仍待后续验证。此接入不增加自动重连、SDK FD 契约或输入控制。

PipeWire manager 的实际流探针已加入 upstream-context pipewire：原版精确 manager 函数体分别连接真实 PipeWire 源，Rust 通过同一 native factory/Consumer 在原转换点使用 C++/OpenCV。daemon 模式要求 20 个 manager、40 次重建后端和 120 张图；supplied-fd 模式要求 20 个独占 socket、60 张图，且不复用已被消费的配置 fd。Rust 同时留存 raw RGB 与对应 BGR 并逐字节检查通道顺序；双方按生产序号独立核验像素，不把不同取样时刻的图强行逐字匹配。原版与 Rust 的全局 pw_init 均只在 manager/native 销毁后由夹具配平；真实截图、不依赖 connected 的转发、inactive 和默认等待保持原实现。新增编译/资源/图像结果仍待原生 Actions，未接受完整 manager/P1。

Linux manager 首轮原生 Wlr 批次的原版 20 轮/60 张图及资源检查通过，留存像素已独立复算；Rust 测试因显式卸载 adapter 时仍有已取空 Option<Image> 的析构借用而报 E0505，九项相关 Rust 入口未执行。夹具现显式释放该输出槽后再卸载，保留所有 Mat owner 归零断言；修正后的原生编译/取帧仍待验证，旧失败不撤销。

PipeWire manager 首轮批次保留 81 项通过和 5 项失败：新的常量 chunks_exact Clippy 门禁、两个 Rust 预热在真实取帧前因 maxsize 写成行宽 6 而非完整 12 字节触发 ShortSpan，以及两个原版夹具浅拷贝/复用 caller Mat 后的持帧变化。现修正预热大小和固定数组分块，并把原版输出 owner 移入 held，与 Rust take() 的调用前提一致；新增真实 OpenCV 控制要求浅 alias 被下一次 cvtColor 改写，而 move 后的 owner 保持像素。此控制已在本机 Windows 编译运行通过，Linux 和 manager 实际修正仍待原生验证。旧记录不撤销；所有既有 1080 张流/映射图和 120 个带 padding 输入在该失败批次仍独立核验正确，不把它们当新增 manager 通过。

修正后原生 Linux manager 批次的 PipeWire 85 项、Wlr 96 项组件检查均通过。新增 PipeWire 540 张图、180 组同样本 RGB/BGR 与 80 行资源记录，以及 Wlr 60 组 raw/BGR/原版图、40 行资源记录均已独立复核；Wlr 的 60 个原图事前哈希未改变。此结论只覆盖受控 Input=None 转发与记录中的重连/停用/所有权前提，完整配置、metadata/API 与 P1 仍未验收，旧失败保留。

按计划 §14.2，独立原版/Rust PipeWire manager 长流入口各先完成三次预热，再持续至少 120 秒；逐帧保存完整 RGB/BGR payload 和调用耗时/producer marker，每秒记录 fd、线程数与 RSS，并保存关闭前后的精确 task IDs。只保留最早三帧和最后一帧的 CPU owner，关闭后复验，Rust 同时核对原 Consumer 的 replay 决定和 native copy/buffer 计数；原版未公开的 replay/缓冲计数明确为未知，不把 marker 变化伪装成 native 新鲜度。实际 v1 记录格式与逐字节验证在独立 xtask 中实现：`cargo xtask upstream-trace pipewire <source|rust> <外部长流目录> [--check-only]`；默认只创建新的 trace-validation.json，`--check-only` 复算并核对已有报告而不写入，报告含 p50/p95/p99 调用延迟及资源范围。入口已通过原生 Actions：双方各 3000 次 steady 调用、至少 120 秒、120 条资源记录和关闭后四张持帧，9000 个像素文件已独立复算。相同提交的 PipeWire 87 项与 Wlr 96 项组件全部通过。此前另一个批次虽通过双方长流，但独立原版 DMA 流在 stop/join 后读取 proc task stat 时收到 ESRCH，仍为失败且未改观察器。这是自有 CPU RGB/Input=None 场景的证据，不证明 GPU 完成、一般内存增长上界或性能达标。

新增 `cargo xtask upstream-config linux <原源码可执行文件> <新的外部目录>`，在单独进程观察 Linux 创建/销毁、配置默认值、UUID、features、未连接截图失败输出保留及 get_info。显式 PipeWire 源码准备入口查询最新正式 meojson，核对 tag/commit、缓存清洁度和 include 哈希，归档源码/MIT 许可，并构建独立 `mfs_linux_config_model`；它保留完整原 Config/UnitBase/manager 类声明和原 API/被查询函数体，使用真实公共 C++ handle、虚表及 OpenCV。日志/警告包装与空 Wayland 类型由显式 overlay 提供，排除的 native connect/输入虚函数一旦误用即抛出异常；外层 C++ 观察器捕获，不宣称原 C 接口具有异常屏障。

该源码配置入口在 Windows VS2026/MSBuild 上先通过 604 案例（125 个成功创建/479 个拒绝），扩展后通过 946 案例（426 个成功创建/520 个拒绝）；成功案例逐字节保存 UUID/完整 metadata，并将原 get_info 回喂原 create，要求状态/字节完全一致。覆盖必需字段、十字段类型/整数边界和词法、全局重复键、未知巨大数字、256 层未知容器、非 UTF-8 原始字符串、C 字符串 NUL 截断、控制字节及 Unicode。最初预期拒绝尾随注释的语料检查失败，实际源码在根对象后的 slash 使 skip_whitespace 返回 false，因而接受 slash 后任意尾部；已保留旧记录，修正语料并扩展 255 种尾随字节观察。BMP 转义插入 surrogate pair 之间也被当前 meojson 接受；新增原始字符串全部 256 字节、36 组完整 metadata/UUID 组合及 50 组 surrogate 原始间隔/尾部长度，核验其 packed-string 跳过边界。参考目标明确断言当前默认 x64 16 字节 parser trait；依赖升级改变此前提时必须重新对照，不默默选择旧依赖。

私有 Rust Config 与同一个 manager 已接入字节配置、完整十字段 get_info 和 UUID。它以安全 Rust 读取字节，使用 fallible Vec/HashSet/显式容器栈，验证未知字段中的语法与重复键，不构造通用递归 JSON 树；保留原词法、非 UTF-8 字符串、NUL、slash 及上述已观测 Unicode 兼容行为。所有 946 个源码创建/拒绝结果和 426 组 UUID/metadata 字节已在 Windows 主机实际比对通过，原始准备/语料的 4132 个记录哈希保持不变；额外 manager 控制验证创建/查询不连接、未连接截图不修改输出、停用/销毁不调用未创建后端。原生 PipeWire manager 检查通过 JSON 创建，并另显式运行保存原版观察值的 Rust 对照；相应原生配置结果见下文。输入配置继续保留，同一 manager 的截图相关宿主初始化边界见下段，不实现输入控制；无公共 C 导出，完整 framework/API/P1 未验收。这些是 legacy 兼容事实，不是新 SDK 解析契约；核心不引入 meojson/OpenCV/serde_json 运行依赖，相关 MIT 许可保存在 THIRD_PARTY_NOTICES。

同一个私有 Linux manager 现保留非 None 输入相关的原截图初始化顺序：截图或输入选择 Wlr 时才创建共享 client，先构造截图再检查输入 selector，随后依次初始化截图和宿主提供的输入 owner；输入构造/初始化失败后仍保留截图转发与 inactive，原 get_features 从现存输入 owner 查询并掩码原两项标志，不依赖 connected。`Factory` 的输入边界只包含构造、init、features 和 inactive，不包含输入动作。默认 native factory 仍只提供 None；非 None 的已知 selector 明确返回 HostInputRequired，不能静默改配置，也不表示完整兼容入口已经接通。新增原函数体模型编译未改写 get_features 和 create_input，输入类由标明的宿主脚本提供；本机 2304 案例对照覆盖六种输入值、四种截图值、共享 client、两次连接、构造/初始化异常与 false、持存截图和 features，未执行原生输入或后端捕获。manager 控制与精确源模型入口加入 PipeWire 编排；原生结果按实际批次记录，不据此接受真实 Portal 授权或完整兼容入口。
原生接入现新增内部 `WithInput` 组合和带类型的 `InputProvider`/`InputOwner`：只替换输入初始化边界，client 与截图构造、原生 init、取帧、转换及关闭仍使用同一个 NativeFactory/Backend。输入 owner 自持真实资源，可长于临时 provider；inactive 从现存 owner 执行，保留原截图再输入的顺序。`HostConnection` 分别保留 init 的 true/false/错误、原生失败和最终 close 观察，宿主初始化返回错误仍可检查已保留的截图对象。默认 NativeFactory 仍只提供 None；带宿主接入不改写原配置。非负 fd 的源入口继续要求显式 unsafe 独占 offer，权限仅在该次构造栈内；宿主构造失败时未消费 fd 的一般所有权仍需单独验证。

该适配首轮 Linux 原生测试编译因 Portal 关联类型误写为 Handle 而失败；修正为 Helper 后已进入原生执行，但 Clippy 拒绝通用 manager 比 Backend/Converter/Attempt 更宽的可见范围及未使用的 HostedFactory 重导出。现将 manager 限于 Linux 内部、从实际使用处引用组合别名，保持原类型和行为，不放宽 warnings。完整修正后的原生门禁仍待验证，旧编译/Clippy 失败分别保留。

新的显式 PipeWire `--manager-host`/Rust 精确测试各执行二十个真实 private-daemon/supplied-fd 生命周期，宿主 init true/false/受控错误 97 分别七/七/六次；错误码 97 是夹具定义的宿主边界控制。输入动作与实际 DBus/desktop Portal 授权均不执行。Rust 经已迁移的 KWin 配置构造器交接一个真实 socket，helper 在原生连接前结束；source 侧为明确的受控 host offer，不能据此声称完整原 KWin C ABI 已通过。宿主 owner 另持真实 socketpair，要求捕获 fd 在其析构前已关闭，随后两个宿主 fd 仅各关闭一次。每个场景捕获三张真实 BGR，包括 init false/错误后的调用，保留 raw/BGR 至原生和宿主销毁之后。后续原生批次的这两个入口和实际记录校核已通过；双方四十次生命周期、一百二十次取帧及下载后的 360 份即时/持帧 RGB/BGR 文件又由独立 xtask 复算通过，精确 task IDs、宿主主错误与清理码、关闭顺序均核对。该批共 120 项组件检查中 119 项通过，一项 Clippy 失败，整批仍失败；可见范围修正后的后续批次已通过全部 122 项原生 PipeWire 源码组件检查，下载后的两侧宿主矩阵再次独立复算通过。

`cargo xtask upstream-trace pipewire-host <外部 matrix> [--check-only]` 只复核实际记录，不取帧。v1 数据含 source/rust 的二十二列 cases、基线和每次关闭后的 task IDs、即时与持帧像素；独立核对宿主主错误/清理码、features 保留见证、helper 先结束且 offer 仍打开、native-before-host 关闭、资源数量及精确任务集合，逐字节验证 360 份已知 RGB/BGR 与持帧文件，并拒绝缺失/多余像素文件。上述通过仅覆盖实际宿主边界夹具，生成器、portable 控制或空记录不能充当原生取帧验收。

另新增独立的 pre-init 宿主构造失败控制与 `cargo xtask upstream-trace pipewire-host-offer <外部 matrix> [--check-only]` 实际记录校核。它已通过上述 122 项原生组件批次，下载后的四十轮记录和 458 份相关实际文件身份又独立核对通过。原 C++ manager/PipeWire 构造与销毁及同一 Rust 实现各执行二十次：从真实 socketpair 的宿主端 dup 一个独占 offer，先关闭原宿主 fd，再令输入构造边界返回受控错误 98，阻止进入原生 init。要求截图对象仍在、无输入 owner/features，未初始化截图拒绝并保留调用方输出；同一 SO_COOKIE 在 helper 关闭、构造错误和 manager/native 销毁后都不变，先记录未消费残留，再由夹具恰好关闭一次，并核对精确 task IDs/fd。每侧十九列记录由独立 xtask 逐项复算；不把夹具回收记为产品清理成功，不执行授权/DBus、原生 core/loop 或成功取帧，也不据此推广一般建连失败的 fd 消费保证。

完整原 KWin 兼容入口的显式 `mfs_pipewire_kwin_native_context --kwin-native` 对照目标现已通过后续 126 项原生 PipeWire 组件检查；完整源码编译、链接、真实虚表取帧和 Rust 对照均已执行。它编译未改写的完整 KWinControlUnitCompat.cpp，并使用完整原 ControlUnitAPI、manager、Config/UnitBase 声明和原截图/metadata 函数体；真实公共虚表句柄从原 Create 取得，经 connect/features/UUID/get_info/inactive/screencap 转发至原 PipeWire，最终由原 Destroy 释放。宿主 Portal C 函数仍是显式控制边界，使用真实 private-daemon socket，dup 后先关闭原 helper 描述符，以 SO_COOKIE 核验 offer 独立存活；输入构造/init 与既有夹具共用同一个 socketpair owner 控制，不提供输入动作。Rust 继续复用已迁移兼容构造器、typed host 和同一捕获后端，新增 helper 原 fd 关闭/duplicate 身份与完整 metadata/UUID/call-order 记录。`cargo xtask upstream-trace pipewire-host-api <外部 matrix> [--check-only]` 先复用原 360 份像素及资源校核，再独立核对二十个真实原版虚表句柄见证、双方各二十组完整十一字段 metadata、UUID、五步宿主调用及 fd/node 整数宽度。原目标使用真实公共 namespace/接口基类与原 C 函数；不据此接受实际 DBus 授权、输入设备、一般失败/完整 framework/ABI 或 P1。下载后的 528 份相关实际文件均已核对登记哈希，独立 xtask 再次复算通过二十个真实原版虚表句柄、二十个 Rust 构造器、四十组完整 metadata/UUID/调用记录及 360 份即时/持帧像素；归档的八份原 ABI/manager/config/base 源文件与固定上游逐字节相同，get_info 提取仅有已记录的 Windows CRLF/Linux LF 差异，函数体相同。这些是该受控宿主入口的原生证据，完整兼容失败/授权与 P1 仍未验收。

Wlr 现另接入 `mfs_wlr_native_context`：完整原 WlRootsControlUnitCompat.cpp、原公共接口/manager/config/base 声明与原 Wlr/Wayland 捕获源组成真实 C Create/Destroy 和虚接口入口。宿主输入仅控制构造/init，按原 WlrInput 构造器保留同一 client 与五个代理槽位（本夹具 seat 为空）；不执行输入动作或原输入设备初始化。Rust 经已迁移的 Wlr 兼容构造器、同一 NativeFactory/Backend 与 typed host 保留 Input=Wlr。二十轮计划分别以七/七/六次 init true/false/错误验证后续取帧、features/inactive/UUID/完整 metadata，输入 owner 析构时须只剩其持有的一个已连接 client 引用，随后全部弱 client/原代理快照失效。Rust 单独保存捕获关闭时的早期计数与最终 CPU ledger，避免把尚有输入 client 存活的早期快照误作全部回调释放证明。`cargo xtask upstream-trace wlr-host-api <外部 matrix> [--check-only]` 校核实际资源、精确 task IDs、360 份即时/持帧输出、64×48 独立图样、raw 布局和完整 metadata。该入口已通过后续 128 项原生 Wlr 组件检查；原 C++ 完整入口、Rust 精确用例及实际记录校核均已执行。下载后的 579 份相关文件均核对登记哈希，独立 xtask 再次复算通过双方各二十轮、共 120 次取帧、360 份即时/持帧像素、四十组 UUID/完整 metadata 和共享 client 析构证据；十份原 ABI/config/base/input 所有权源文件与固定上游逐字节相同，get_info 函数体亦独立匹配。每轮 Rust 回调 owner 从捕获关闭时的 created=11/dropped=5，最终回到 created=destroyed=dropped=11，未把早期快照当作最终释放；共享输入析构测试不接受显式 close/reconnect 时仍有活跃快照的另外一项缺口，完整 P1 继续未验收。

新增独立 Wlr 宿主构造失败入口：原 C++ 在复制实际共享 client/代理后、创建宿主 socketpair 前抛出受控错误 98，Rust typed host 在同一构造边界返回该错误，不返回输入 owner。两侧各二十轮经原兼容入口/同一迁移 manager 验证：连接失败且 features 为零，已创建截图仍保留同一已连接 client，每轮继续取三张已知图样，inactive/UUID/完整 metadata、关闭后的 CPU 持帧和精确 fd/task 集合均须成立。`cargo xtask upstream-trace wlr-host-construction <外部 matrix> [--check-only]` 已在 Actions 41129758262 的 131 项原生 Wlr 检查中通过；下载后的独立复算再次通过四十次构造错误、一百二十次取帧、360 份像素、原部分构造代理最终失效及 Rust 完整回调 owner 释放。官方完整 Linux ZIP/tar 摘要、该组件 8340 个登记输入与日志、383 份产品 Git 文件、固定 F 的 76 份 Git blob 及原生 CMake 来源均已独立核对。它不执行输入动作、证明一般分配耗尽恢复或覆盖活跃共享快照的显式 client 关闭；完整 P1 仍未验收。

显式 client 关闭的另一项限制仍保留：固定原 WlrScreencap 持有的 manager/shm/output 共享快照不会随 WaylandClient::close 的全局字段 reset 自动失效，而该函数随后断开 display。[Wayland 客户端 API](https://wayland.pages.freedesktop.org/wayland.freedesktop.org/docs/html/apb.html)要求先销毁代理和事件队列；[wayland-backend 的 foreign display 前提](https://docs.rs/wayland-backend/latest/wayland_backend/sys/client/struct.Backend.html#method.from_foreign_display)也要求 display 覆盖所有 Backend 克隆的寿命。这里是源码与接口前提审阅所得的风险，未据此声称复现崩溃；当前 Rust 的独占 connect/close 限制继续生效，不能直接放开借用或将构造失败矩阵当作此项验收。

首个原生 Linux 配置批次在 meojson 准备阶段失败：PowerShell 函数的 Inputs 参数遮蔽调用方同名记录列表，使 Save 对字符串调用 Add；尚未编译或执行配置 oracle/捕获。该记录保留，参数现改为 InputDirectory，并以与原生调用方相同的 inputs 列表作用域复验；原生修复结果仍待新的 Actions，不将先前 Windows 的不同列表名称包装器当作该修复的原生验证。

后续原生批次在更早的 Rust 测试目标编译处失败：JSON 创建夹具把节点 ID 放进 format 后失去原 Config 字段提供的 u32 推断，导致 E0284；该批次的 P0/P1 consumer 步骤均失败，尚未执行配置/捕获。现已显式声明 u32，修复后原生结果见下文；主机 Clippy 不编译该 Linux-only 测试体。

原 C++ 调用器另新增 `--task-retirement-read-fixture <外部目录>`，仍与取帧实现分离。普通 proc task stat 读取保持严格，并保存 errno、bad/fail/eof 和实际字节数；只有原生 close/stop/join 与所有者退出之后，才允许对已知原先活跃 MaaScreencap 的精确 ESRCH/bad/fail/非 EOF/零字节失败，再做一次新的 stat open 与任务目录 stat，二者均为 ENOENT 才证明已消失。成功读出的任务仍核验相同 id/name/starttime、PF_EXITING、vsize/RSS 为零；未知、替换、活跃、部分数据或其他错误保持失败。没有增加取帧重试或改变既有 100ms 内核退出界限。独立原生控制通过真实命名线程的 held-stat 描述符产生 ESRCH，并要求已 join、文件/目录缺失；真实活线程路径与实际耗尽当前子进程 fd 后的 EMFILE 必须拒绝，随后关闭临时 fd、恢复原 soft/hard limit，并精确核对基线 task IDs/fd。另有 24 个明确标注的合成 metadata 反向控制，不代替真实原生退出证据。实现已加入显式 PipeWire 源码入口，并已通过下文的 94 项原生组件；普通 strict read-exit 原控制及所有旧失败记录继续保留，核心捕获/等待/像素与生命周期代码未改写。

修复 u32 推断后，原生 Linux PipeWire 93 项组件检查全部通过：原版 946 案例、Rust 946 创建/拒绝与 426 组 UUID/info、八项 manager 控制、JSON 创建后的真实 daemon/supplied-fd 转发均已执行；该批次的 540 个 manager 图像和双方长流的 9000 个像素文件已独立复核。配置原始输入/输出位于 inputs/source-reference，原轻量 artifact 的 glob 未包含它们，完整官方工程 artifact 的摘要已验证，全部 852 份接受后的 UUID/info 与 4974 个登记原生输入哈希已独立校核；今后的轻量包显式包含 configuration-*/config-oracle-sources 和 meojson 身份/许可。随后新增退出观察控制的 94 项组件也全部通过：实际错误为 ESRCH=3、bad/fail=1、eof=0、bytes=0，新 stat 与目录错误均为 ENOENT=2；活路径打开成功被拒绝，实际 EMFILE=24 被拒绝，24 个合成反向控制、fd limit 恢复与最终身份均通过。旧 strict read-exit 仍严格拒绝；不将这些结果追溯改写旧失败，也不据此接受完整 P1。

后续另一次原生批次的 Rust supplied-fd SHM 测试在 cycle 3 的三张图像和原生 stop/join/destruction 后，因旧 `read_to_string` 观测器读取 task stat 收到 ESRCH 而失败；关闭计数与队列归还已记录，不据此将失败改为通过。现将 Rust 测试专用 procfs 观测器拆成打开、原始字节读取和解析阶段，保留 errno 与部分字节数；普通/live 读取继续严格拒绝。仅 post-join 路径可按上述已知所有者、精确 opened-read ESRCH/零字节与新的双 ENOENT 证据分类；成功读取仍核验相同线程代际。四项 portable 控制覆盖部分/非法 UTF-8 数据、身份替换和未证明缺失等拒绝条件。独立忽略测试 `backends::task_observer::native_fixture::validates_joined_read_esrch_without_accepting_live_or_unobservable_paths` 使用两个自有命名线程，要求 held-stat 真实 ESRCH、活线程路径拒绝、真实 EMFILE 拒绝、恢复 fd limit 和最终资源；`MFS_P1_RUST_TASK_OBSERVER_ARTIFACTS` 指定新的外部目录并逐步保留观测。显式 PipeWire 入口编排该测试，要求精确一项通过及完整摘要；后续 115 项原生组件检查已全部通过，独立核对真实 opened-read ESRCH=3/零字节、双 ENOENT=2、活路径打开拒绝、实际 EMFILE=24、fd limit 恢复、fd 基线/最终均为六及精确 task IDs。该批 supplied-fd 的二十次生命周期、六十张持帧和二十个已消费 socket 关闭也通过；旧失败仍保留。观察文件因局部与外层 finally 各登记一次而有两条同路径/同哈希引用，已分别核对同一实际文件；现去掉重复的局部登记，外层 finally 仍保留失败时的部分观测。该控制不执行捕获，不更改捕获等待或既有 100ms 退出界限。

再后续批次的 PipeWire 116 项检查中，115 项通过，原 C++ retirement-read 因果夹具在最终 fd/task 身份联合断言失败；held-stat ESRCH、双 ENOENT、活路径拒绝及真实 EMFILE 拒绝已执行。旧日志未保存最终数量和任务集合，不能证明具体失败分支、截图泄漏或内核退出竞态。现仅在该因果夹具中，为第二个已 join 的自有负向控制线程补上最多两秒的完全缺失选择，与第一个自有线程的前置选择相同；仍拒绝非预期原生错误。最终 fd/task 样本先冻结并写入 final-resources.txt，再进行原精确身份断言，后来的诊断样本不能救回失败；取帧及既有 100ms 资源门禁不变，此修复待新的原生验证。同批 Wlr 的实际 native manager 控制日志为两项通过、exit=0，但脚本仍要求一项，导致组件失败；现要求 None/unimplemented 与缺少宿主 provider 两项，不放宽断言。该批新增 2304 个源码模型及三项 PipeWire native manager 控制已通过；带类型宿主适配与真实 host capture matrix 是其后的实现，仍待原生验证。

一次后续 Windows CI 的 GDI/PrintWindow、ScreenDC、DesktopDup 与 portable PipeWire 四个入口均在 Clippy 阶段因 `windows_capture_host` 的三个 dead-code 诊断停止。Clippy 之前的 GDI/PrintWindow、ScreenDC 对照、DesktopDup 原版取帧及共同回归已实际执行并通过；各入口剩余步骤未继续执行，不能将整批描述为没有取帧或已完整通过。该 host 仅供 FramePool 和启用 FramePool 的 manager runtime 使用，现按这项实际编译能力裁剪测试模块，保留原实现及原生关闭断言；六组对应 feature 组合的本机 Clippy 已通过，不放宽 warnings 门禁。此前 FramePool 的四个原版 pseudo 已知颜色断言仍为失败，辅助模块裁剪不解决这些捕获问题。

后续 Wlr Failed/EOF 原生夹具的一次服务端进程以 exit 139 提前退出；原版客户端已执行 cycle 0 的五项请求及三份 SHM 写入，下一次连接失败，Rust 入口尚未执行。旧部分记录和失败保留，无堆栈，不能断言崩溃的精确指令。源码审阅发现请求回调内直接销毁 client，而 libwayland 分发器在回调返回后仍使用 client/connection；现将夹具主动 EOF 改到一次性 idle 回调。固定地址的 pending owner 持有 client-destroy listener，提前退出和服务端关停会在回调执行前取消；执行时先清除 idle 指针并摘除 listener，遵守 [Wayland idle 生命周期](https://wayland.freedesktop.org/docs/html/apc.html)中禁止在回调内或之后移除该 source 的要求。独立 `--disconnect-control` 以真实 socketpair/client 验证执行、提前销毁和关停取消三条路径，逐项核对已关闭原生 fd、空 client 列表和零 pending；后续原生 Wlr 123 项组件检查全部通过。三条控制的 fd 基线/最终均为十，scheduled/executed/cancelled/pending 分别为 1/1/0/0、2/1/1/0、3/1/2/0；双方各二十轮主动断连及完整 graceful stop 已核对，Failed/EOF 的四十轮、二百次操作和 440 份实际像素记录由 xtask 独立复算通过，Rust 主失败与清理错误分别保留。Failed/EOF 双方的原有像素、错误和资源断言保持，另要求各二十次 scheduled/executed、零取消/待处理及独立记录复算；这只修改验收夹具，不改变捕获实现。

Windows/macOS 的同一个私有截图 manager 已补 get_info，保持 Input=None、原字段名/顺序、HWND/u32 window_id 宽度，以及原 uint64 方法标志转 int64 的 metadata 视图。方法选择仍使用原 u64，不从日志视图推导支持能力。`tests/upstream-parity/manager-metadata` 的独立 C++ 目标只复制原字段声明与未改写 get_info 函数体，使用真实 MaaDef/最新 meojson；Windows 主机实际通过 88 个 Win32 和 66 个 macOS 字段查询/JSON 字节对照，覆盖零/高位标识与有符号边界。这不构造原生 manager、不验证虚表/C ABI 或桌面；所有输入方法保持 None。

新增 `cargo xtask upstream-metadata <win32|macos> <外部目录> [--check-only]` 校核实际 v1 cases.tsv/source-stdout.txt/rust-stdout.txt 与完整矩阵，拒绝重复字段、额外字段、改变整数宽度/词法、错行与缺失输入；保存或复核 metadata-validation.json 及原文件哈希。源与 Rust JSON 行字节精确相同，记录文件允许平台 CRLF/LF。源码模型、许可、生成字段/函数体及所有输出均在参考区；核心不增加 meojson/serde_json/OpenCV 依赖。显式 Windows manager 对照已加入两个查询测试和独立校核，在线每批查询并构建最新 meojson；显式离线对照使用该批哈希验证过的 oracle exe/声明作为参考输入，不代替产品离线源码构建。Linux PipeWire 与 macOS 源码入口也编排对应 portable/native-host 元数据查询；原生 macOS 52 项和 Linux PipeWire 95 项组件均已通过，并独立运行下载记录的 --check-only 校核；Windows 首次 manager 步骤复用了不含 oracle 的 DesktopDup 离线输入，在 16 项准备检查后失败，尚未执行查询/截图对照。CI 现为 manager 单独在线准备并解析 meojson；显式离线 manager 要求完整且唯一的 exe、四份声明/函数体及 prepared 身份，缺少时在原生构建前拒绝，不静默联网。修正后的原生 Windows manager 25 项检查已通过，154 条元数据输出已独立复核，旧失败保留。随后一轮 Windows CI 因 VS stable build 更新未就绪而在准备前失败，未执行新的截图对照；CI 现于每个在线 Windows 组件入口重做当前工具链准备；修正后的 876cfbb 原生批次已通过 PrintWindow/manager 全部检查，旧准备失败保留。

Wlr 原生名称现保留原 filesystem::path::c_str 边界：NUL 后字节仍属于 client 重连身份和 manager UUID，但 C 接口只读取首个 NUL 前的名称；空名称和首字节 NUL 仍传非空指针，不改用默认来源。新增原源码/Rust 独立进程各 20 生命周期、60 张关闭后持帧与精确资源检查，并由 Rust JSON 解码 NUL 后实际连接 manager。该 NUL 场景已通过 98 项原生组件，全部 60 组 raw/Rust BGR/原版图像及 40 行资源已独立复核。另新增无分配的原路径 component 比较，保留 dot、末尾分隔符及完整字节；841 个实际 C++ 路径比较和 20 轮真实重连/拒绝/取帧已通过 100 项原生组件，全部 20 组 raw/BGR/原版图像和 40 行资源已独立复核。通用路径与完整后端/P1 仍未验收。

Custom 上游回调的补充源码审阅已纳入外部 catalog：共 101 个源码条目的身份、精确行和路由引用通过校验。实际 raw C setter 位于 source/Common/MaaBuffer.cpp，包装 packed Mat 后调用真实 ImageBuffer::set 克隆；manager 成功返回共享该 Mat 的所有者，false 保留原输出、true+empty 则替换为空。其后的 Windows 原生链接/调用及私有 Rust 回调切片验证见本文开头；catalog 本身仍只核验源码身份和映射，不作回调捕获验收。

Linux 原型准备先查询 LLVM 维护者的最新正式版，校验官方 Linux x64 压缩包摘要，将所需编译器、链接器、资源与包内许可材料安装到 deps 独立目录，并归档原包及逐文件身份。C/C++ 和 Rust 原生链接显式选择该工具链；只有 PipeWire 启用 libclang 生成器，Wlr 不探测生成器。缓存命中仍查询更新并复验文件。LLVM 23.1.4 的 clang/LLD、PipeWire 生成与捕获、Wlr 捕获及正常退出已通过原生 Actions，工程证据已下载并核对哈希；包内许可声明链接的完整 LLVM 许可另按正式 tag 补存外部审阅记录。Windows 最新 VS/SDK 的 Actions 原生验证也已通过；本机 VS2026 18.11.2、实际编译器 19.51.36260.0 与 SDK 10.0.28000.2957 的重新构建、受控取帧、C++/Rust 边界、独立 GDI 参考和配置/独立消费者检查也已通过；实际身份与逐项结果仍按批次外置，不将旧本机结果作为新工具链验收。

macOS 原型准备查询 Apple 发布 RSS，只接受正式版标题并核对实际 `xcodebuild -version` 的版本及构建号。通过进程环境 DEVELOPER_DIR/SDKROOT 选择已安装 bundle，不修改系统 xcode-select；缺少匹配版本时保留 missing_environment。Actions 的 macOS arm64 原型使用官方 xcode-27 镜像；正式 Xcode 27.1（27A9275）与 SDK 27.0 已通过编译、链接、20 次受控窗口取帧及完成/晚到回调释放验证，高版本 RC 被排除。结果仅覆盖该原生环境的 P0 原型。

首次 LLVM 原生准备在包摘要校验后遇到 zstd 默认解码内存上限，失败结果已保留；第一次参数修复又暴露了 PowerShell 单项条件数组被展开为字符串、splat 按字符传参的问题，本机 argv 检查已复现。读取 zstd 包现在以显式字符串数组对清单与解压均传入 2048 MiB 上限，继续流式读取，修复后已通过原生验证。

Windows 环境准备可单独运行 `pwsh -NoProfile -File scripts/prepare-windows-toolchain.ps1`：查询 VS2026 stable channel 和 Microsoft SDK 正式下载页，核对安装器的 Microsoft Authenticode 签名及版本，保存外部 local 记录。默认只读，环境缺失返回 77。管理员 PowerShell 加 `-Install` 才安装：Build Tools 使用 deps 下的专用实例，SDK 为系统级安装；不请求 UAC、不重启、不关闭现有 IDE。Actions 已验证专用 Build Tools 安装、SDK 安装、后续只读复查、实际编译和最小原生/参考捕获；本机由用户手动更新 VS/SDK 后的对应编译、捕获与配置复验也已通过。安装/注册成功本身不能算 SDK 验收。

编译器/SDK 的目录名与实际补丁版本分别记录。例如本轮工具目录仍为 MSVC 14.51.36231，但真实 C++ 编译器版本为 19.51.36260.0；SDK 目录为 10.0.28000.0，安装的 servicing release 为 10.0.28000.2957。以归档的编译器检测、文件身份和正式版准备记录判断实际版本，不按目录名要求重复升级。

Windows 原生依赖缓存将选中 SDK 的安装补丁版本与根路径纳入身份，避免 SDK 原地更新头文件而目录及 kernel32.lib 不变时复用旧 OpenCV 构建。`pwsh -NoProfile -File tests/prototypes/sdk_cache.ps1` 验证同目录补丁更新会改变身份，其他 SDK 系列、重复项与枚举顺序不会影响选中身份；此便携检查不运行原生捕获。

```powershell
cargo xtask reference prepare
cargo xtask reference verify
cargo xtask reference catalog runs/<input-id>/capture-catalog.json # 校验外部职责/目标/用例定义
cargo xtask reference licenses runs/<rust-batch-id>/manifest.json # 归档该目标 registry 许可证据
cargo xtask check
cargo xtask batch --prepare-only # 只解析/归档依赖，不编译捕获 crate
cargo xtask configure --windows ON --pipewire OFF --wlr OFF --screen-capture-kit OFF
cargo xtask configuration-check # 原生配置与不可构建捕获源的独立准备检查
cargo xtask batch --desktop  # Windows 显式桌面批次；其他平台暂不加此参数
cargo xtask native --desktop # Windows x64：VS2026/MSBuild、C++/OpenCV 互操作及独立参考
cargo xtask pipewire-prototype # Linux x64 原生环境：私有 daemon/C 源和 Rust 取帧原型
cargo xtask macos-prototype    # macOS arm64：受控窗口与 objc2/ScreenCaptureKit 主线程原型
cargo xtask wlr-prototype      # Linux x64：真实 wlroots headless/Pixman 与 system libwayland 原型
```

- `reference prepare` 获取固定上游与固定 PR head，核验两个提交及唯一变更文件，外置保存完整未打补丁源码归档、相关文件发现清单、许可声明记录、累计补丁和 head 文件。它只做来源准备，不应用补丁，不构建参考程序，也不生成已验收 U/P。可用 `--root <目录>` 或 `MFS_REFERENCE_ROOT` 指定独立参考根。
- `reference verify` 校验布局版本、未知字段、必需身份、父 U、Git 对象/文件内容、完整归档与记录 SHA-256、来源清单覆盖和相对路径越界，包含 symlink/junction 的实际路径检查。外部 catalog 已记录 99 个已审阅来源、11 条截图路径及 58 个尚未执行的 U 用例；核验记录结构不等于执行对照或完成 P0。
- `reference catalog <reference-relative input> [--root <目录>]` 校验并归档外部职责、方法/目标与 U 用例定义。保留固定准备基线，同时要求已发现源码、公共控制单元头和 Portal 交接文件的覆盖；逐文件核对固定 Git blob、原始字节 SHA-256 和准确行锚点，核对报告及方法/目标/用例引用。未审阅文件必须明确标为 pending，定义不能引用它们；所有用例只能是 not_run，不能填入 passed 或完整阶段状态。校验后将输入、报告副本及工具源码快照写入新的外部批次，不修改旧基线。若已有本机批次的 Cargo.lock、工具链身份与当前完全一致且包含 vendor，关联其依赖快照供离线重建；未找到时明确记录缺口，准备入口不触发捕获库构建。`reference verify` 会重新执行已归档 catalog 的语义校验。这只证明记录结构和来源证据一致，不证明人工结论穷尽、原生对照执行、许可验收或 P0 完成。
- `check` 校验 HEAD 的 Conventional Commit 标题及 workspace 版本/许可元数据；提交前可用 `--message-file <文件>` 校验拟用消息。它不代替发布阶段的 API 兼容性与版本协商验收。
- `reference licenses <Rust batch manifest> [--root <目录>]` 从已索引的目标依赖图生成新的外部 registry 许可证据清单。核对解析节点、包身份、声明的许可表达式/文件、vendor 校验和与原始文件，复制命名为 LICENSE/COPYING/NOTICE 等的文件（包含子目录），缺少表达式或文件时分别保留待审项。新记录保存 text_documents 发现规则，避免将 copying.rs 等普通源码当作独立许可文本；显式 license-file 仍按声明收集，旧记录按其原前缀规则重检。清单覆盖该目标图中的工具、构建依赖和已选可选能力，并不等于最终产品依赖集。`reference verify` 重检清单和文件覆盖；Actions 对每个平台的实际目标图分别归档。此命令不解析 SPDX 的法律兼容性，也不替代源文件内嵌声明、原生/系统/工具链、上游/PR 和最终资产的许可审阅。

固定来源及三平台实际依赖的许可范围审阅另存外部批次，明确区分 LGPL 迁移源码、Rust 绑定/工具、原生库/协议、OpenCV 验收适配及系统 SDK。cookie-factory 发布包遗漏的 MIT 文件已从其发布 VCS 提交取回并校验；objc2 旧版声明与后补全文有上游证据，历史版权声明完整性仍待发布前收口。完整 P0 按计划第 16 节判断：要求这些来源/许可边界、最小参考/适配及原生可行性证据，不把 P1 完整 U 对照或 P3/P5 全部矩阵提前作为 P0 退出条件。

- `batch` 先更新 nightly，再更新 Cargo 解析并查询每个 registry 包的最新正式版（包括范围外主版本），归档锁文件、元数据、vendor 源码、实际源码快照和逐项日志，再执行 release build、workspace tests、fmt、Clippy 和项目约定检查。直接依赖保持全局最新版；依用户确认的绑定传递规则，已接入 PipeWire/Wayland 绑定的专属依赖链可使用已核验父约束内的最新正式版，报告同时保存全局版本、约束内版本、父声明、采用规则及升级差距。非绑定链共用版本不豁免，约束内未更新仍失败。Windows 的 `--desktop` 追加显式原型及其 Clippy。批次开始时即归档源码，结束后比较 Git HEAD、全部跟踪/未忽略源文件的路径和内容哈希；若源码变化或无法核验，明确失败并另存结束状态，防止把混合输入算作同次验收。最后将起始源码、锁文件及 vendor 复制到 deps 中独立目录，以空 Cargo home 和离线模式验证默认构建及原型编译，该消费目录没有同级 reference。失败单列且不自动重试；每个批次保留该任务未覆盖的门禁，不自动批准整个阶段；阶段结论由外部汇总退出审阅记录。

批次依赖检查以归档 rustc 的 host 为原生 target，通过 `cargo metadata --all-features --filter-platform` 检查该 target 的完整 feature 图，并保存 `dependency-target.txt`。不将 Linux 专属 helper 的 Windows 条件依赖计入当前构建；其他平台分别由对应原生 Actions 批次检查。全局最新版与绑定专属传递约束规则仍逐包执行。

`xtask` 不依赖捕获 package，默认 Cargo 构建只构建产品入口。普通构建不读取 reference，不链接 OpenCV。当前 `build.rs` 仅在目标 Windows 且显式选择原型 feature 时读取维护的过滤输入，向 OUT_DIR 生成私有 Win32 sys 声明及 COM/WinRT rich 投影；其他平台复用对应原生绑定，并已实现原型族的 AUTO/ON/OFF 配置与目标复验；当前窗口 GDI 与 PrintWindow 已开始迁移，其余后端和完整管理器调用链尚未完成。

`native` 是显式原生验收入口，当前仅实现 Windows x64 + VS2026/MSBuild，需要 Git、curl、GitHub CLI 和 PowerShell。它先完成本次 Rust 批次，再在线解析 CMake、OpenCV 和 Boost 最新正式版，以及 MaaUtils 正式版或无正式版时的维护分支。原生源码、工具归档、配置、SDK/编译器身份、日志和产物仍在 reference/deps，产品基础构建不触发这些步骤。CMake 的 ASM 编译器以显式 FILEPATH 缓存类型传入，并在重复配置后核查身份，避免缓存复用时被未定类型覆盖。OpenCV 只选择所需模块及其传递依赖，保留 CPU 分发、SIMD 与 IPP/HAL，并验证实际配置；未选择的 DNN/OCR 依赖不进入参考构建。

`pipewire-prototype` 在原生 Linux x64 上执行，自有 C 视频源、20 次流生命周期及完整退出已通过原生 Actions；早期 SPA 夹具清理断言和各次失败证据仍保留在外部批次，本机 Windows 不执行或声称 Linux 验收。入口解析 PipeWire 官方最新 release 与 Python 工具正式版，从源码构建隔离前缀，归档源码、wheels、配置和日志；创建独立 runtime/socket、无宿主设备的 daemon 和独立自有 C 视频源（RGB 行距 96 字节，目标每行 84 字节，填充显式初始化）。私有 0700 socket 显式使用无需 session manager 的访问模式，不参与宿主授权。Rust 测试执行 20 次连接/停止，每次通过移交的 fd 和明确 node ID 接入，检查 RGB 协商、受检 stride/offset、32 个真实样本、原生缓冲重复使用与逐次归还、回调线程、注销/stop 及 CPU 持帧。高层绑定用于 loop/context/stream/POD；借助窄 sys 缓冲租约保存底层 queue 返回码，panic 被封在 Rust 回调内。原生源和 daemon 由编排进程启动和依次停止，分别要求自然处理 SIGTERM 后退出 0，不接受强制结束或提前退出；已验证生产端和消费端各 640 次 dequeue/queue 配平，逐次原生缓冲复用与 CPU 像素断言通过，不访问宿主默认 remote。该入口暂不提供离线重放；Rust 门禁失败时可继续收集诊断原型结果，但最终命令仍失败，不能当成已验收。Portal 桌面来源、DMA-BUF 及完整格式/失败矩阵均未覆盖；Wlr 由独立原型验证。

`macos-prototype` 已通过原生 Actions：20 次受控窗口取帧及一次提交后放弃调用方所有权的请求，验证像素、CPU 持帧、晚到结果丢弃和原生输入释放。在 macOS arm64 记录 Xcode/SDK、编译器、部署目标与 Mach-O 依赖，从源码快照构建不使用 libtest 工作线程的显式程序；在进程主线程管理自有 AppKit 窗口和原生 run loop。沿固定上游 sample-buffer 截图入口验证 20 次受控颜色取帧、受检 BGRA 行复制、像素锁/解锁、完成块及其原生输入释放、独立 CPU 持帧与跨线程消费，并验证调用方放弃请求后晚到结果丢弃和上下文保活。回调只捕获自有状态，Rust panic 封在块内；不为原生对象声明 Send/Sync。未授权返回明确 missing_environment，仍使门禁失败，不发起授权弹窗或修改 TCC；构建、Clippy、捕获及环境缺失分别记录。正式 Xcode/SDK 选择及本节最小原型已有原生证据；原始 C++ 对照及完整目标/格式/失败矩阵仍在对应后续阶段完成。

`wlr-prototype` 已通过原生 Actions：20 次真实 compositor 取帧、SHM/fd 生命周期、独立队列和退出验证。初次自有程序链接缺少 libwayland-server 的失败记录已外置保留，修正显式直接链接后通过。入口解析正式 tag/release，检查名称与发布标记，避免将错误标为正式发布的 beta 标签当作稳定版；从源码准备 libffi、Expat、Wayland、协议 XML、libdrm、Pixman、xkbcommon 和 wlroots，并记录实际 pkg-config 版本、源码、配置、工具和链接信息。自有 C 程序启动真实 headless backend/Pixman compositor，生成不对称三色场景，不连接宿主桌面或使用输入后端。Rust 明确启用 wayland-backend 的 client_system，通过私有 socket 执行 20 次 screencopy/shm/fd 生命周期，验证 Buffer/BufferDone/Flags/Ready、像素、native ready 时间字段、read/cancel-read、独立事件队列及代理数据释放。捕获进程还核对实际加载的 libwayland-client 与准备前缀一致；只把 CPU 像素交给其他线程。原生 compositor 必须响应 SIGTERM 正常退出。该入口尚无离线重放；LLVM 工具链已按正式版准备并通过本节原生验证，完整 compositor/格式/失败矩阵与原始 C++ 对照仍在对应后续阶段完成。不是 Wlr 产品后端或 P3 ExtImage 实现。

私有 C++/OpenCV 适配器验证 C 布局和失败输出、真实外部线程回调、stop 排空及回调内拒绝、原生像素覆写/释放后的 Rust 独立持帧、跨线程释放和 Rust/C++ 异常边界。独立参考程序直接编译固定未打补丁的 `GdiScreencap.cpp`，局部对照 DLL 另可显式编译 `PrintWindowScreencap.cpp`，在原调用点执行上游转换；其构建、加载/FFI 和受控窗口实际捕获分别记账。这些参考入口不代表六种 Windows 方法或完整 U，P0 的准备验收另见计划状态。具体构建/原生测试结果以各批次记录为准。

网络不可用时，可显式选择已登记、具备所需依赖归档的原生运行记录，使用其依赖快照验证当前源码：

```text
cargo xtask native --desktop --offline-inputs runs/<prepared-run-id>/manifest.json
```

此模式恢复归档源码和 OpenCV 下载输入，以空 Cargo home、归档 vendor 和匹配的已安装 nightly 执行 Rust 离线验证，不查询在线最新版，也不自动补下载缺失依赖。系统 SDK/VS 仍需按记录准备。当前源码在执行前另存快照，原生构建及 Rust 测试从该副本执行；复验当前构建适配与重放原批次源码是不同用途，后者还须恢复该批次源码。下次在线构建仍重新检查更新。依赖归档不完整、哈希变化或本机工具链不匹配时明确失败，不计为通过。

实际记录格式由 `xtask/src/records.rs`、`reference.rs`、`batch.rs` 的版本化 serde 结构及校验逻辑维护，未知字段会失败。`manifest.json` 仅索引 baselines/patches/runs；已完成来源记录与运行记录不覆盖，运行中新失败保留在新目录。准备中断的目录保留以供检查，未索引目录不是有效基线。Cargo.lock 只约束同次验证与离线重建，不作为后续在线构建的版本约束。

Linux PipeWire 的 P0 原型日志在该步骤完成后即上传为 p0-linux-pipewire-diagnostics；P1 组件日志/图像在其源码步骤完成后即上传为 p1-linux-pipewire-diagnostics，不等待后续 Wlr 或完整工程归档。上传保留 failed/missing_environment，不改变门禁结果。

Actions 通过 `scripts/archive-evidence.ps1` 将 reference 打包为 `p0-evidence.tar`：显式枚举隐藏条目，用 PowerShell 的 .NET TarWriter 写入文件、空目录、链接及权限，再解包并执行 `reference verify` 后上传。该脚本拒绝覆盖输出，也拒绝将输出放进源 reference。下载 artifact 后先解包 tar；不要直接把展开的 reference 文件列表当成可保留上述元数据的归档。校验工具对 Git 子进程启用长路径支持，不修改宿主 Git 全局配置。实际失败状态保留在原运行记录，证据包校验通过不把失败的捕获或构建改记为通过。

Windows CI 另上传 `p1-windows-component-diagnostics`，包含当次 P1 component 的清单、原生结果、嵌套日志及参考程序原始 BGR/几何记录；捕获失败后也执行上传。它用于快速读取具体断言，不包含完整来源、依赖和归档元数据，不能代替上述完整证据包，也不改变失败状态。准备阶段未产生对应文件时明确报告无匹配文件。

`batch --prepare-only` 只更新 nightly、解析/核验最新 Cargo 依赖并归档锁文件、元数据和 vendor；不编译捕获 crate，不运行测试或离线消费，不能作为构建通过记录。

后续本机批次发现 WGC/WARP 原型的间歇性访问冲突，系统事件将故障地址归到已卸载的 `GraphicsCapture.dll`；该批次已失败，尚未进入 GDI 对照。调试器或独立压力诊断未复现不撤销原失败，具体根因和修复仍待确认。[WGC 维护者的相关调查](https://github.com/robmikh/Win32CaptureSample/issues/99)记录了内部异步工作与模块卸载的竞态，但目前不能据此断定本机根因。显式离线 GDI 对照只证明其指定输入和 GDI 组件，不替代该失败批次或 WGC 退出验收。

`configure` 为现有 P0 原型族解析 AUTO/ON/OFF，并输出外部 `build.conf` 路径与所需 Cargo features。族名为 windows、pipewire、wlr、screen-capture-kit，不代表最终 SDK 的方法/API。默认先执行上述在线准备；`--offline-inputs <Rust batch manifest>` 明确复用已归档解析，核对安装工具链及消费用 Cargo.lock，下一次在线准备仍重新检查更新。原生快照构建可用 `--consumer-lock <路径>` 指定其实际锁文件。请求、环境/原生依赖证据、源码与配置按新批次归档；ON 不满足时返回失败并保留记录，不发布可用配置。OFF 及其他平台不执行原生探测。

配置中 Windows 检查 VS2026 MSVC x64 与 SDK 库；PipeWire 检查 pkg-config 的 PipeWire/SPA、原型 v1_2_0 所需版本、clang 与 libclang 文件；Wlr 检查 system libwayland-client；macOS 检查原生 SDK/framework stub。非标准 libclang 安装通过 LIBCLANG_PATH/LLVM_CONFIG_PATH 指明。探测不代替生成器实际加载、编译/链接、运行时能力、权限和真实捕获。现有原生原型入口在依赖准备后传入配置，`build.rs` 根据 TARGET/CARGO_CFG_TARGET_OS 复验本机目标、准确 feature 集及选中依赖的相同探测证据；SDK、版本或路径漂移时要求重新准备。旧格式仅保留归档解释，新原生构建使用含证据的新配置；只向 OUT_DIR 写归一化配置/声明，声明配置、环境和发现文件的增量失效输入，不在 Cargo 启动后改变后端选择。

`configuration-check` 验证 OFF 的真实构建、配置/feature 不匹配拒绝、不可发现依赖下的 AUTO 禁用与 ON 失败；Windows/macOS 使用明确指向空目录的 SDK/开发目录，Linux 隔离 pkg-config 搜索目录。它还复制源码到 deps 独立消费者，先证明含 compile_error 的捕获入口无法构建，再以空 Cargo home、归档 vendor 和独立 target 编译运行该消费者的 xtask，证明准备入口不依赖可构建的捕获 crate。预期拒绝及其原记录保留，不能重标为原生取帧通过。每个原生 Actions 平台执行这些检查，结果按实际批次记录。

## 构建责任与启动顺序

| 入口 | 职责 | 输入/输出边界 |
| --- | --- | --- |
| xtask 准备任务 | 查询最新工具链/依赖、探测 AUTO/ON/OFF、选择原生配置和编排显式对照 | 能在捕获 crate 尚不可构建时运行；不直接或间接依赖捕获 crate |
| Cargo | 解析并编译选定 target/feature、执行标准测试和增量构建 | 产品主入口；源码消费者不依赖 reference |
| build.rs | 复验已选配置、按目标生成声明、编译必要小桥接并链接 | 消费明确输入，不负责在线更新、下载 OpenCV 或生成外部验收报告 |
| CMake | 独立参考程序、C++ 验收适配、必要原生依赖构建 | 不形成与 Cargo 重复的产品构建系统 |

准备入口建立后，顺序为“最新版本解析与探测 → 输出本次配置 → Cargo 构建 → 记录/归档结果”。缓存命中仍执行更新检查；同次对照冻结该批次解析身份，下一在线批次重新检查。离线重建使用该次快照，不执行在线更新；具体政策见计划第 5.5–5.6 节。

准备配置至少记录 target/架构、选择的后端、原生依赖来源及安装位置、SDK/编译器/运行库、生成器/profile/原生选项和影响构建的输入身份。可移植请求与本机绝对路径分开；本机配置写入明确的外部工作区，通过路径或环境传入构建，不写进产品源码。build.rs 只向 OUT_DIR 写生成结果，并为实际读取的维护输入与环境变量声明 rerun-if-changed / rerun-if-env-changed，不假设 OUT_DIR 为空。

直接 Cargo 源码消费可以显式选择 feature 并使用文档化的系统依赖发现，不强制运行 xtask；选择的后端缺依赖则失败。build.rs 不得绕过 Cargo 事后关闭已启用的依赖。

## target、feature 与验收隔离

模块使用 target_os 与后端 feature 联合 cfg，运行依赖进入 target 表，build.rs 根据 CARGO_CFG_TARGET_OS / TARGET 处理目标；build script 自身运行在宿主，不能以它的 cfg! 判断产品目标。

Cargo feature 只增加可编入能力，使用正向后端开关，不把 AUTO/ON/OFF 或互斥的 U/P/E 行为做成 feature。默认 feature 集合及源码消费者规则随原型确定；运行时方法与策略由 options 控制。workspace 使用适用的新 resolver，并显式设置成员/默认成员，避免普通产品构建无意构建工具或验收目标。

U/P/E 从各自源码身份构建。对照 feature 如确有必要只增加私有验收入口；C++ + OpenCV 适配由 xtask 显式构建，转换调用点遵守原生命周期。产品默认依赖图、普通构建及发布资产均不含 OpenCV 或验收适配。验证基础构建时移走 reference，缺失对照输入报告未验收。

P3 原生接口迁移按[现代化清单](../design/native-interface-modernization.md)逐项验证 SDK/运行时及协议能力，新增 ExtImage 输入仅在目标及选定后端需要时生成。依赖更新检查不代替调用迁移；缺少新入口时报告能力缺失，兼容调用按明确条件记录，不静默改回旧接口或切换来源。

Windows 首批为 x86_64-pc-windows-msvc，C++ 首版只实现 VS2026 + MSBuild；VS2022/Ninja 后续单独验收。具体工具链发现、缓存配置及依赖更新按计划第 5 节执行。

## 测试 target 与夹具

私有逻辑的单元测试放在所属模块的 `#[cfg(test)]` 中；`tests/` 下的集成测试只依赖公开 API。多文件集成测试使用 `tests/<name>/main.rs` 或明确的 `[[test]]` path；例如 `tests/contracts/main.rs` 对应 `cargo test --release --test contracts`。不以 `tests/unit/` 目录代替私有模块测试，也不通过公开内部实现来方便测试。

`tests/common/mod.rs` 供 Rust 集成入口共享辅助逻辑；`tests/fixtures/` 保存自有确定性输入。格式、错误、所有权等跨语言夹具可采用文档化数据文件，原生消费者实际执行验证；不会仅因位于 `tests/` 下就自动运行 C++/Python 工程。

image-contracts、upstream-parity、desktop 和 consumer 的具体入口由 xtask 编排；每个任务列明执行者、前提、实际运行的测试及退出状态。desktop 显式启用，受同一桌面焦点/窗口状态影响的场景串行运行。计划第 14 节是阶段门禁和完整夹具清单的权威来源，缺失/跳过不记为通过。

examples/ 随已实现的 API 提供原格式和可选图像适配消费者；Rust 示例参与对应 Cargo 编译检查，C/C++/Python 示例通过独立消费任务验证。当前不创建调用尚不存在的 SDK 的占位示例。

## 工程记录格式与文档

外部布局以计划第 6.2 节为准。P0 在 xtask 中随实际清单生成实现布局版本、必需字段、相对路径及输入/产物哈希校验；schema 和校验逻辑随产品源码维护，实际清单与报告保存在 reference。只在格式实际落地时添加 schema，不预建空 manifest 或空报告。

docs 的层级、权威来源和状态规则见[文档导航](../README.md)。生成器输入、脚本和自有夹具属于产品源码；参考副本、补丁、来源映射、实际验收结果和本机配置继续外置。

依据：[Cargo features](https://doc.rust-lang.org/cargo/reference/features.html)、[Build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html)、[Cargo package layout](https://doc.rust-lang.org/cargo/guide/project-layout.html)。
