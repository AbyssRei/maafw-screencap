# 开发、构建与验证

日期：2026-10-29。状态：P0 准备阶段已按计划第 16 节验收，P1 实施中，首先迁移窗口 GDI 与显式局部对照。下节列出已可运行的工具与原型；完整上游对照、SDK 和产品桌面验收仍随 P1–P5 实施。

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

Windows 默认 workspace 测试包含 8 项 PipeWire 消费/CPU 行所有权测试（另有一项显式原源码 oracle 默认忽略）、4 项分配原型与 26 项工具测试，Unix 另运行一项 symlink 越界测试。选择 `linux-pipewire` 后原生 driver 取帧测试仍须显式启用。Windows GDI 桌面原型仅在 `prototype-windows` 下编译，默认忽略，须显式串行执行：

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
cargo xtask upstream-parity screen-dc
cargo xtask upstream-parity screen-dc --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity frame-pool
cargo xtask upstream-parity frame-pool --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity desktop-dup
cargo xtask upstream-parity desktop-dup --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-parity manager
cargo xtask upstream-parity manager --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-model pipewire --offline-inputs runs/<prepared-native-run>/manifest.json
cargo xtask upstream-context pipewire # native Linux x64 original-source context
cargo xtask upstream-context frame-pool
cargo xtask upstream-context frame-pool --offline-inputs runs/<prepared-native-run>/manifest.json
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

随后完整 ScreenDC 组件批次在 GDI 回归之后复现两项黑图失败：源码/Rust 均为黑图，原源码在单次呈现屏障后仍为黑图；日志和四张原始图像均已归档，前述单独运行的通过不能覆盖该失败。当前夹具改为复用已有 WM_PAINT 绘图和专用 UI owner/message loop，在该 owner 上查询统计和关闭，再 join 退出；ScreenDC 的原生验收目标不编译 PrintWindow 后端或伪最小化。捕获调用及逐像素断言不变，必须重新核对这一夹具前提，不能将单次 DwmFlush 当作已充分证明的就绪保证。

专用 UI owner 的后续完整后端顺序检查也出现同样两项黑图失败，尽管绘图消息、owner 身份和无绘图错误已核对；不能声称该夹具改动修复黑图。只读桌面证据显示已超过关屏时限、前台窗口为空，熄屏目前仅为待用户确认的原因推断，未据此重分类断言失败。独立 callback-module 保活至 join 退出、原窗口 helper 与伪最小化脚本检查通过。PrintWindow 单独过滤的回归又记录了冷进程计数与原弹窗黑图失败；其条件编译的 PrintWindow 分支与此前源码等价，实际失败仍保留，不能记为回归通过。

ScreenDC 的真实资源故障入口以隔离命名空间编译原函数体，只在 DC/位图/转换边界注入，helper 和窗口查询仍为原生调用。13 组普通/组合故障核对实际到达的故障、调用/错误顺序、主错误、四项清理错误、回收前残留计数和 CPU owner 释放；夹具回收不计为产品清理成功。另有短读、元数据失败、24 位 DDB 的三项安全分类，以及 null/已销毁/零宽客户区拒绝检查。零宽窗口的前台 helper 在冷进程单独初始化 7 个后续保留的 GDI 对象，独立无捕获控制已记录，测量前显式建立该夹具前提。完整原生故障入口四项测试已通过；它们不能替代仍失败的严格可见像素检查，实际非默认显示深度也未验收。

精确提交的原生 Actions 又记录了不同的可见内容失败：第一轮源码/Rust 图样正确，第二轮前台为 true，但源码首先读到统一灰色 `[12,12,12]`，Rust 随后正确，原源码呈现前/后控制同轮也为灰色。不能用本机熄屏推断解释这一 runner 结果。当前仅增加一个待验证的夹具控制：自有 ScreenDC 测试窗口在隐藏时设置 `DWMWA_TRANSITIONS_FORCEDISABLED`，成功后才首次显示；保持 PrintWindow 的既有模式、捕获等待及逐像素断言，既不改全局动画设置，也不重试失败。私有 C 接口原有布局/统计前缀保持，尾部反映该设置。设置、module 保活、四项原生故障和 PrintWindow 两项前缀/退出检查已通过；动画是否为根因及这个控制能否稳定可见像素，仍需新原生证据，所有旧失败保留。

后续结果未证实仅禁动画有效：用户确认并唤醒本机熄屏后，禁动画的完整 backend 入口仍为 16 项通过、2 项灰图失败；相应提交的 Actions 也仍失败。只读诊断又在一轮源码捕获前观察到客户区中心由另一窗口占据，随后目标抬升才取得正确像素。[IsWindowVisible](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-iswindowvisible) 仅表示可见样式，不保证无遮挡；[WindowFromPoint](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-windowfrompoint) 的点窗口/root 与源码捕获前后的前台、cloaked、坐标和样式一起保留。原 helper 共享的 5 秒冷却不保证为每个新窗口再次请求前台，不能用 Rust 返回后的前台值推断原源码取帧瞬间的可见性。

新的夹具前提是在自有 ScreenDC root 上额外请求 `WS_EX_TOPMOST`，不授予前台权限；原 helper 仍可按原行为将它取消置顶。只改变夹具，保留原捕获、等待及严格像素断言，原 PrintWindow 模式与 C 接口前缀保持。一轮亮屏本机完整 backend 入口 19 项通过，包括 20 轮配对逐像素/清理/关闭后持帧，以及独立的 20 轮源码呈现前后控制；观察到各轮中心点 root 属于目标。相同受控场景随后通过两个精确提交的原生 Actions，证据及提交身份外置保存；这仍只证明该受控场景，显式遮挡/移动/深度与完整 P1 仍待验证。首次运行因未创建外部图像记录目录中断，属于保留的 harness I/O 失败；补建目录后的结果单独记录，不能覆盖之前的像素失败。


DesktopDup 和 DesktopDupWindow 已有私有迁移，入口为 `cargo xtask upstream-parity desktop-dup`，可复用明确的 `--offline-inputs`。独立 features 为 `windows-desktop-dup`、`windows-desktop-dup-window`；后者复用底层并启用原前台/rcWork helper。它保留默认 hardware device、CreateDXGIFactory、显示器枚举/主输出回退、DuplicateOutput、每次 2000ms Acquire、最多三次底右 alpha 预热后另取最终帧、access-lost 部分清理及释放顺序。鼠标更新和实际桌面更新时间分别记原生证据，不增加 U 的新鲜度过滤。

原 `screencap_impl` 的 Mat 借用 staging 映射，却在返回前 Unmap/ReleaseFrame；[Microsoft 的 Unmap 契约](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-id3d11devicecontext-unmap) 明确该调用会使映射指针失效。Rust 在映射有效期内复制定义的 BGRA 像素行，仍在原生映射、原纹理、帧和 desktop resource 释放之后做 BGR 转换。该全输出 CPU 复制明确计量，不能称为消除复制。Window 仍先转换全屏、再查客户区/输出并裁剪；为不暴露 ROI 间的其他桌面像素，另做目标 BGR 紧密复制，这会改变原 ROI 对全图存储的保留和分配点，属于明确的安全适配。

对照 executable 在独立进程调用未修改的原 full/window 类；自有 UI owner 在目标外以 50ms 定时器交替绘制一个独立标记窗口，提供预热和最终 Acquire 所需的桌面更新，不重试捕获、不改变目标图样。原源码两种方式和 Rust 20 次混合生命周期（每种十次）已通过本机严格目标图样、ReleaseFrame HRESULT、目标独立存储及所有原生/窗口关闭后的 CPU 持帧检查；实际监视器、marker 活动及 frame/mouse 时间证据外置。另一个独立原源码进程与精确 Rust 用例验证空 HWND：它选择 adapter 0/output 0，不能假定该输出就是 OS 主显示器。两侧先独立查询该输出是否连接并包含测试窗口；不满足前提记为 missing_environment，不改选其他输出。本机该场景的严格目标图样、归还 HRESULT 与关闭后持帧已通过。

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

本机 74 个案例、118 次操作已匹配原源码：每次激活并请求新帧、整秒截断与亚秒余量至少等待一秒、伪唤醒、负等待返回后的谓词复查、前三次超时回放与第四次拒绝、空缓存、停用/再激活、锁外转换/异常和 BGR 共享 owner 转移。原生 set_active 返回值按 U 忽略；源码错误回调不主动 signal，消费端也未提前加入 P3 的终止唤醒/绝对等待。额外的时钟/计数溢出检查独立验证，不执行原 C++ 未定义溢出。原生建连、init/deinit、POD、process、SHM/DMA-BUF、缓存写入别名、线程迁移、关闭/注销和 Linux manager 仍待迁移与原生 Actions 验收；已有 Linux 运行结果仍仅来自 P0 原型。

`upstream-context pipewire` 已接入原生 Linux x64 源码环境诊断，首次原生编译/执行结果仍待 CI。它直接编译固定完整 `PipeWireScreencap.h/.cpp`，仅抽取原 screenshot base 排除输入/ConfigBase/meojson，并明确使用私有 namespace 与日志抑制 overlay。同批解析最新正式 OpenCV 与 DRM 头文件来源，保留 IPP/CPU 优化，记录 LLVM、系统 SDK 包、编译命令、原生链接与离线输入。默认 P0 生产端仍为原七色条；此入口选择独立 `--asymmetric` 二维图样、96 字节行距及初始化填充，不在该图样上执行 P0 的 RGB 条断言。

新的原生 Rust 切片包含 daemon 建连、先启动 loop 再 connect、固定 4096 字节 POD builder、稳定事件/状态地址、真实 loop 锁和返回值保留的相对等待、逐次归还前驱 buffer、只按请求复制最新 SHM、停止线程后注销并释放对象。完整原 cpp 现在不改内容地 include 到测试翻译单元，以调用其 static 格式 helper；另用原 buffer POD 表达式保存三个协商 POD，与 Rust 逐字节比较。独立 Rust 用例要求 20 次生命周期/60 张 raw RGB、inactive、返回计数、fd/线程归零和原样持帧；RawAudit 只保留 RGB，不执行 OpenCV 转换，因此没有宣布原生 BGR 对照通过。该切片首次 Linux 编译/运行仍待 CI；Portal fd、无既有指针的映射、DMA-BUF、完整格式/故障/共享缓存矩阵仍待迁移。

CPU 存储的四项 host 测试已通过，验证 stride/跨度/溢出、目标行与填充隔离、空指针拒绝、独占存储复用及跨关闭/跨线程持帧。负 stride、非零 chunk offset、损坏标志和短跨度显式拒绝，未静默提前套用 P3 的 offset 语义。已有持帧者时分配新 CPU owner，保留单次行复制；这是修复原 Mat 缓存可写别名的安全适配，需独立核验原生行为和成本。原生实现保留每构造一次 raw pw_init、析构无 deinit 的 U 策略；测试配平仅发生在完整 native teardown 之后，禁止与高层 binding 的 OnceLock 初始化便利构造函数混用。

源码入口安排 20 次独立 fd/init/三次截图/inactive/close 生命周期，按原 API 检查 60 张已知 BGR 和 fd/线程数，原对象关闭后继续持帧并跨线程读取；每张可解释布局的原图在颜色断言前保存。图样携带生产端序号并使各行内容随序号变化，持帧需保持原序号和像素；重复/倒退仅记诊断，不把它推断为原 API 的回放状态。转换库初始化在捕获资源基线之外，原构造函数的 `pw_init` 由测试在原对象完全销毁后额外 `pw_deinit`；这不证明原析构本身配平初始化。另在独立进程观察真实 `connect_fd` 早期错误的描述符状态，只在原生对象清理后按 socket cookie 回收仍匹配的 socket，不预设所有错误都会消费 fd。入口核对实际头文件/运行时版本及动态符号所在库。原 API 不暴露捕获时间/回放状态，耗时只记诊断。它仍不是 Rust 原生 driver、完整 C++/Rust 对照、framework/input、格式/故障/DMA-BUF 矩阵或 P1 验收。

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
