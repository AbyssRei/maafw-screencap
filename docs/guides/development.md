# 开发、构建与验证

日期：2026-10-28。状态：P0 准备阶段已按计划第 16 节验收，P1 实施中，首先迁移窗口 GDI 与显式局部对照。下节列出已可运行的工具与原型；完整上游对照、SDK 和产品桌面验收仍随 P1–P5 实施。

## 当前仓库与本机命令

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

Windows 默认 workspace 测试包含 4 项分配原型与 26 项工具测试，Unix 另运行一项 symlink 越界测试。Windows GDI 桌面原型仅在 `prototype-windows` 下编译，默认忽略，须显式串行执行：

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

窗口 GDI 的内部实现位于 `src/backends/windows/gdi.rs`，通过 `windows-gdi` 编译能力启用；没有对外发布 Rust/C SDK 接口。它保持 GetDC、兼容 DC/位图、BitBlt、GetBitmapBits 及转换前后的资源顺序。转换由调用点的私有适配边界执行；默认产品不链接 OpenCV。受检算术、初始化的 fallible CPU 分配、原生 32 位布局与完整读取校验属于显式记录的安全适配，没有提前迁至 P3 的 DIB 路径。

```text
cargo xtask upstream-parity gdi
cargo xtask upstream-parity gdi --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity print-window
cargo xtask upstream-parity print-window --offline-inputs runs/<prepared-native-run>/manifest.json
```

本机首个局部对照已通过；三平台 CI 与本机结果分开记录。此入口显式使用 Windows x64 桌面，并复用独立原生准备后编译原始固定 GDI 类和 C++/OpenCV 转换适配。Rust 私有模块内的测试核对非对称图案、奇数宽度、逐像素 BGR、跨后续捕获/窗口销毁的持帧、转换时点和原生资源计数；另覆盖 null/已销毁/空/最小化窗口与被封住的转换异常。故障对照以隔离命名空间编译原始函数体，通过真实 GDI 对象记录创建、复制、转换及清理的原生边界响应；14 组正常/故障/组合场景分别核对主错误、清理错误、调用顺序及 CPU owner 释放。夹具先记录捕获实现遗留的资源，再单独回收，不能据此声称产品在原生清理失败后已成功关闭。结果、源码快照、依赖与未覆盖范围写入外部 P1 组件批次。原生 Actions 的 Windows 任务执行这个入口，其他平台继续各自的原型与构建检查。

U-WIN-GDI-02 的原生失败/清理与窗口状态场景已纳入对照；U-WIN-GDI-01 仍缺真实非默认显示深度，管理器/转发与完整 P1 验收尚待完成。原生 24 位 DDB 在当前 32 位设备上可能被 SelectObject 拒绝，该观察不等于实际 24 位显示器取帧。短读探针定义原参考输入的全部字节后观察其非零即成功分支，Rust 的安全拒绝单独分类。完整目录中的其他后端继续按 P1 迁移。

PrintWindow 的内部迁移位于 `src/backends/windows/print_window.rs`，由独立的 `windows-print-window` feature 启用。它与 GDI 共享 DC/位图清理和私有转换类型，声明生成分别选择所需函数；PrintWindow 单独启用不编入 GetBitmapBits 路径。保留主窗口捕获、root-owner/last-active popup 选择、各自 BGRA2BGR、原生清理后 BGR ROI 合成的顺序；弹窗原生可选失败返回主图，转换/分配异常仍使整个操作失败。

原源码在位图仍选入 DC 时调用 GetDIBits，违反其[原生前提](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getdibits)。Rust 暂时移出位图再读回，成功后在原转换点前选回，最终清理顺序保持；原生对象查询见证读回和转换时的真实选择状态。此为显式安全适配，未改为 P3 的 CreateDIBSection。矩形交集另外直接对照已编译 OpenCV 的 400 组边界组合：仅拒绝原点差的 int 溢出，保留合法偏移的空交集/裁剪结果。完整扫描行数、DWORD 对齐的初始化缓冲及几何受检算术分别验证；读回存储使用 Vec<u32>，字节视图借用同一所有者，不额外复制整帧。

`print-window` 对照同时回归 GDI，显式编译原始 PrintWindow 类及自有窗口类，使用 WM_PAINT/WM_PRINTCLIENT 绘制不同主图/弹窗图样。夹具在捕获前调用 GdiFlush/DwmFlush；这不证明此后每次 PrintWindow 都得到已绘制的像素。记录实际绘制消息而不假设 `PW_RENDERFULLCONTENT` 只走一种消息路径。负原点、部分重叠、边界外、隐藏/无弹窗及零宽弹窗各执行二十次夹具生命周期，逐像素检查与关闭后持帧；这仍是局部对照。八组查询对照还核对正常/root 查询为空、popup 为空或等于 root、无效/隐藏 popup、任一 ClientToScreen 失败的顺序、短路与主图返回。查询探针只替换查询边界，捕获/转换/清理仍执行原函数体；C/Rust 布局与表函数签名在加载后核验。

真实子窗口夹具在更大的 root 内创建 37×23 子窗口，弹窗仍由 root 拥有，偏移以子窗口为基准。无弹窗、正/负偏移、边界外、隐藏和零宽弹窗六个场景各执行二十次生命周期；核对 root 与目标不同、子窗口尺寸/逐像素图样、转换/合成次数、清理和销毁后的持帧。root 的尺寸与图样分区不同，不能以捕获 root 代替子窗口。隐藏设置还须核验 IsWindowVisible 的实际结果。

另一个独立 TU 只在原参考的 cvtColor 边界注入主图/弹窗转换异常，原生捕获、读取和清理仍执行原函数体。对照要求转换异常退出整次操作，记录转换时及展开后的真实 GDI 数量。导出的图像 owner 数量与原 C++ 内部 Mat 分开观察：异常模式只为 BGR 输出设置局部分配观察器，直接记录主图 BGR 分配、释放与字节数，输入 BGRA 和其他 OpenCV 分配不计入它；不替换进程默认分配器，不导出引用观察器的 Mat。异常遗留分配会使检查失败，并保留其模块供晚到释放。Rust 主图 owner 在弹窗异常后也须释放，不能把这类异常降为可选弹窗失败。原参考捕获/清理故障注入、完整窗口状态与原生失败矩阵尚未完成，U-WIN-PRINT-01/02/03 均未宣称完整验收。

已有正式批次出现原参考弹窗位图全黑、Rust 后续捕获图样正确的逐像素失败；临时移出位图、重复读取和原始位图读取都不能恢复该次像素。额外裁剪查询和后续未复现结果尚未建立因果关系，失败与诊断证据保留外置，PrintWindow 的局部验收仍未关闭。诊断 TU 与未插桩的参考 TU 分开，诊断读取返回第一次得到的图像，不以再次读取修补验收输出；额外裁剪/flush 对照已撤回，普通夹具不携带未经证实的修复。

## 已实现的 P0 工具

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

Actions 通过 `scripts/archive-evidence.ps1` 将 reference 打包为 `p0-evidence.tar`：显式枚举隐藏条目，用 PowerShell 的 .NET TarWriter 写入文件、空目录、链接及权限，再解包并执行 `reference verify` 后上传。该脚本拒绝覆盖输出，也拒绝将输出放进源 reference。下载 artifact 后先解包 tar；不要直接把展开的 reference 文件列表当成可保留上述元数据的归档。校验工具对 Git 子进程启用长路径支持，不修改宿主 Git 全局配置。实际失败状态保留在原运行记录，证据包校验通过不把失败的捕获或构建改记为通过。

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
