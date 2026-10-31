# 文档导航与维护约定

2026-10-31 当前本机结论按用户要求以各原程序最新的[连续三轮无输入复测](guides/development.md#本次连续三轮无输入复测)为准。36 份候选历史失败日志对应的程序/配置已全部完成有效三轮，另保留七种原 FramePool 模式；三批共 132 次调用，按原日志配置计为 5 份通过、31 份失败，待补为零。新增 23 份的 69 次调用连续约 362 秒，原始/低级鼠标键盘事件为零、系统输入计时和光标不变，最大采样间隔 79.87ms，监听已确认注销。失败按实际像素/资源断言、加载、配置或测试假设区分，不能全部视为库缺陷。该范围是已枚举的 36 份历史失败日志，不宣称每个曾构建的可执行文件都已穷举；完整 P1 未通过。

2026-10-30 用户已确认[ADR 0003：修正 fork 提取来源](decisions/0003-corrected-fork-extraction-baseline.md)：P1 直接提取包含修正 PR #1587 语义的固定 fork，基线记为 F；P2 保留编号，改为来源冻结与完整提取验收，不再重复打补丁。fork 修复、确切来源准备、Rust/C++ helper 同步与本地历史整理已完成，正式几何/屏幕像素专项已通过；完整新版原生调用链验收仍待完成。下列既有 U/旧来源切片保留原始证据含义，不因计划调整成为 F 已验收。

日期：2026-10-30。P0 准备阶段已验收，P1 实施中：独立 xtask、外部来源校验、分配原型、显式 Windows GDI、COM apartment、D3D/DXGI/WinRT 读回及受控窗口 FramePool 捕获/事件原型已实现，Actions 原生矩阵已定义；Linux PipeWire 自有 C 源夹具的 20 次流生命周期、取帧、独立持帧和完整退出已通过原生 Actions，早期 SPA 夹具退出失败证据保留外置；macOS 主线程 ScreenCaptureKit 原型已通过 20 次受控窗口取帧、独立持帧、完成块释放与晚到结果原生验证；Wlr 真实 headless/Pixman compositor 与 system libwayland 原型已通过 20 次取帧/SHM/fd 生命周期、队列隔离及正常退出的原生 Actions 验证；Desktop Duplication 的 20 次硬件采集/释放原型已通过本机及原生 Actions 验证；结果按平台记录，P0 已通过计划第 16 节的准备与最小原生可行性门禁；完整 F 行为对照仍由 P1 完成。默认 binary 仍为 Hello World；内部 library 开始窗口 GDI 与 PrintWindow 迁移及显式局部对照，完整 F、公共 SDK 和绑定尚未完成。实际命令及原型边界见[开发指南](guides/development.md)。

后续 Actions 41129758262 的 macOS job 在原生取帧前记录正式 Xcode 27.2/27B52 缺失，runner 的 27.2 路径仍是 RC/27B48；完整归档、登记输入及产品快照已核对。这是该批环境缺失，较早的原生通过与失败各保留原范围；详见[开发指南](guides/development.md#本次连续三轮无输入复测)。

## 阅读顺序与权威来源

Custom 已新增 UUID/能力位及 get_info 的同实现迁移。Windows 归档依赖下 64 组身份查询、592 组元数据/真实 StringBuffer/meojson 对照及实际字节、作用域和 owner 校核通过；Linux 身份查询及下载后独立复算亦通过。用户更新 VS2026 18.11.3 后，本机正式在线批次的 25 项原生检查全部通过；Linux Actions 41147180960 的 133 项组件检查亦通过，实际输出与来源已独立核对。macOS 仍缺正式 Xcode 27.2/27B52。Wlr 宿主构造失败已通过 131 项原生组件及完整来源核验；各项仍不完成 P1。实际范围见[开发指南](guides/development.md#当前仓库与本机命令)。

Custom 新增真实原 C 创建/销毁与 C++ 捕获接口消费，Windows 归档输入下二十轮、1,308 次虚接口查询及 112 次 buffer 内原生调用方替换通过；后续本机正式在线及 Linux 原生批次也已通过；macOS 和 framework 转发继续待验证。较早 Linux 批次因嵌套 meojson 日志覆盖而失败，修正后最新 Linux 组件已通过；较早 Windows CI Custom 因离线清单缺少 meojson 在编译前失败，独立在线准备后，Actions 41164682813 与 41167609520 的该步骤已通过，完整 Windows job 另按实际结果记录。详细边界及保留记录见上述开发指南，不据此完成 P1。

Custom 另补同一对象同步回调重入：Windows 归档输入下 Debug/Release 的 7×7 组合通过 7,840 次真实虚接口查询、1,960 次嵌套 C++ 异常、240 个销毁后原生持有值及七项作用域控制。首个 UUID 嵌套失败源于验收桥要求恰好一层 buffer，现改为逐操作身份与嵌套作用域跟踪；核心 manager 和原源码行为未改。确切 5612387 的本机正式在线批次通过 27 项原生检查，Actions 41167609520 的 Linux 组件通过 135 项；下载后 4,728 份重入数据及其余 Custom 回归输出与本机一致，十份原 API/头文件与固定 F 一致。轻量包仍缺完整依赖与产品快照；macOS、通用故障和完整 P1 未通过，不构成公共 SDK 重入契约。

Custom 另接入真实编码/缩放回调矩阵：144 组组合、288 次源码/Rust 捕获与关闭后原生持帧在 Windows 归档输入的 Debug/Release 下通过，2,025 份实际记录一致。覆盖灰度/BGR/BGRA PNG、无效/空/NULL 输入、缓存时点、整数比 INTER_AREA 与自动尺寸；确切 59f5323 的本机在线批次随后通过全部 29 项原生检查，9,694 项登记记录、397 个产品 Git 文件、F 原函数体及实际字节均独立核对；其余 codec/raw 类型、任意缩放及其他平台仍待验收。初始空 Mat 头假设错误和后续严格校核均保留，九组篡改与八组零测试/缺摘要日志被拒绝；具体入口及边界见[开发指南](guides/development.md#当前仓库与本机命令)。

可选外层 Win32 截图委托已有私有 Rust 状态迁移和 274 个原函数体模型对照；另有真实 DLL/Boost/原接口虚表的 GDI-only 原生消费切片，本机通过 80 轮共享加载生命周期、40 个真实 Rust manager/GDI 对象和 160 张卸载后持帧。实际记录另复算像素、模块与释放证据；输入保持 None，六工厂/完整框架及 P1 尚未验收，新增原生 CI 路径仍待运行。详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

六种单方法及混合候选现另有真实工厂运行切片：本机源码/Rust 共 280 轮生命周期、840 次输出和 1680 份即时/持帧文件通过独立校核。DesktopDup 保留整屏，按实际几何核对自有窗口 ROI；其余环境、故障、资源及伪最小化门禁继续待验收。原四项 FramePool 包装失败保留；此项不接受完整 P1。

Wlr 新接入独立的 IPC/SHM 协议夹具与实际记录校核，覆盖 v1/v2/v3、六格式及行填充/YInvert；原生 Actions 的 112 项组件检查已通过；下载后的 72 份 SHM 输入、4320 份输出和全部记录也已独立复算。它补充格式与原版缺陷对照，不能代替真实 compositor/完整后端验收；范围见[开发指南](guides/development.md#当前-p1-迁移与对照)。

FramePool 新增透明度/内容原生诊断及实际记录校核；维护版本机 10 次 FramePool、10 次独立 PrintWindow 内容见证与完整字节检查通过。另一次独立调试器观察确认旧图命中原缓存 clone 分支，但该次新建会话另有无帧失败，整次实验仍失败。alpha=0 的黑图/旧场景结果仅作诊断，原包装彩色图样失败继续失败；较早 Actions 的新诊断和下载后 20 份实际图像校核通过，四项 alpha=0 均为黑图。后续 Actions 41100061053 的持续会话最后恢复不透明时却返回全黑，原诊断提前失败，后续 Rust/pseudo 检查没有执行；该批二十份实际图像亦已独立复算。入口及各批范围见[开发指南](guides/development.md#当前-p1-迁移与对照)。

Custom 的私有回调切片和真实 ImageBuffer/raw setter 原生对照已加入；本机局部 96 个案例、192 次源码/Rust 调用及独立输出记录复算通过。正式在线批次的 22 项原生检查通过，但归档器漏识别新组件前缀使命令失败；修复后的只读记录校核通过，随后 876cfbb Actions 的 18 项离线原生检查及下载后 192 份实际输出复算也通过。Linux/macOS 新入口也已通过 110/67 项原生组件及各 192 份下载后输出复算；完整 Custom/框架和 P1 未验收，默认核心无 OpenCV 依赖。当前入口与范围见[开发指南](guides/development.md#当前仓库与本机命令)。

P1 另已加入独立 ScreenDC 内部实现和默认路径的 ForegroundUtils/HwndUtils 迁移，正式入口为 `cargo xtask upstream-parity screen-dc`。原头文件脚本、真实资源故障及本机局部像素/清理检查已有证据；亮屏且自有测试窗口置顶的新批次通过 20 轮严格源码/Rust 对照。已确认熄屏的本机黑图记录经修复前源码、原适配器和当时 Rust 工具链复测，两项原始用例各 20 轮通过，现从未解决程序缺陷中移除并归为环境前提失效；原始记录外置备份，不改写成通过。灰图、其他夹具及原因未明失败分别复核；相同受控场景随后通过两个精确提交的原生 Actions；遮挡、移动、实际显示深度与完整后端/阶段验收仍未完成，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

本地历史失败已逐项登记并备份 36 份候选日志，十三组修复前源码/配置及七种原 FramePool 模式已重放。原 WGC 访问违规、四项 FramePool pseudo 和两份 ScreenDC 几何夹具失败在亮屏无输入时仍复现；构建缺符号、前台控制前提及 Custom 类型常量错误另按实际边界归类；原 Desktop Duplication 初始颜色和 GDI 24-bit 成功假设也已在无输入条件下复现。以上是历史复审观察，当前本机结论按顶部的有效连续三轮记录；其余 23 份中间版本随后已恢复原程序并完成有效三轮，新增一份资源目标拒绝用例全过，另 22 份各有失败；完整覆盖与当前结论见顶部。未复现的历史失败原因继续未决，不将全部旧失败归因于用户操作，也不据此完成 P1。

FramePool 也已开始独立内部迁移与显式 `upstream-parity frame-pool` 局部对照：本机四个 Rust 调用环境匹配原图，缓存克隆与关闭后持帧已有实际证据，另有 20 次 Rust 完整取帧/关闭生命周期与 dispatcher 完成退出。伪最小化包装也已迁移，但本机原源码最小化场景的彩色图样检查返回黑图，匹配的 Rust 字节/状态仅记诊断；完整源码重复/故障生命周期、原生错误/重建、包装验收和 manager 仍待完成；不构成公共 SDK 或阶段验收。


DesktopDup/Window 也已有独立内部迁移及 `upstream-parity desktop-dup` 入口。本机三个原源码进程（含空 HWND 的 adapter 0/output 0）、20 次混合 Rust 生命周期及独立转换/异常边界检查通过受控目标图样、帧归还与持帧；原映射 Mat 生命周期缺陷由明确计量的安全复制适配，完整原生故障、多输出/旋转和 manager 仍待验收。

截图 manager 已开始私有迁移，`upstream-parity manager` 对照原源码函数体的 122 个脚本，并通过本机 20 次真实 GDI manager 生命周期。它保留预热/测速、毫秒截断、候选与图像释放及 U 的调用线程 DPI 副作用；完整原生混合方法、包装和 framework 转发仍待验收，不构成公共 SDK。

PipeWire 已开始消费端算法迁移，`upstream-model pipewire` 在 Windows 工具链上比较原源码的等待、回放、停用、转换与所有权边界；本机 74 个案例/118 次操作匹配。受控原生批次中，完整原 C++、Rust daemon、Rust supplied-fd、Rust OpenCV 各通过 20 生命周期/60 张关闭后持帧；七格式/28 组有效 POD 与精确转换字节对照也通过 Actions。真实 MemFd helper 的 60 次复制/所有权与六类拒绝，以及原 C++/Rust 同一对象各 40 个 session/120 张持帧也已通过原生对照。测试区分已 join 的内核退出任务与活跃线程，严格保留 fd/任务身份检查；旧即时数量断言和后续编译失败不撤销。实际 Portal 授权、通用 fd 失败/重试、GPU 同步、完整格式/故障及 P1 仍待完成；真实 kernel DMA helper 和独立生产器/回调流检查已加入，首个 helper 编译失败保留，修正后原生编译与负向控制通过，真实 kernel helper 已通过，实际 NULL-data DMA 流已通过原生对照，旧 const/API 编译失败保留，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

`upstream-context wlr` 的原源码入口已通过 20 次原生生命周期/60 张持帧及递归 roundtrip 检查；私有日志、名称提供者及被动 manager 夹具边界保留。首个私有 Rust Wlr 捕获切片与对应 native 探针已加入；首轮编译失败修正后，20 生命周期/60 组 raw+BGR 与实际转换异常检查已通过原生运行；该批次保留的 Clippy 失败修正后，后续 87 项原生检查全部通过；新增共享 client/安全重连/FD 0 原版探针后，最新 90 项亦通过，新增 80 张原版图已独立复核；Rust FD 0 成功路径补齐后最新 91 项检查通过，新增 20 组图已独立复算，NUL 名称与完整后缀重连随后通过 98 项组件，新增 60 组图像与 40 行资源已独立复核；原路径 component 比较及 841 对照/20 轮连接场景随后通过 100 项原生组件，20 组图像与 40 行资源独立复核，仍不代表完整后端已验收。操作与边界见[开发指南](guides/development.md#当前-p1-迁移与对照)。

Linux manager 的 Input=None 编排、两分钟 PipeWire 长流及 Wlr 受控转发已有原生证据。配置/API 原源码与 Rust 的 946 个案例及 JSON-to-manager 取帧已通过 93 项 Linux 组件检查，完整官方存档摘要及全部 852 份 UUID/info、4974 个原生输入哈希均已独立校核；严格 proc-task 退出观察控制随后通过 94 项组件，旧 ESRCH 失败仍保留。Windows/macOS 的 get_info 已在主机通过 154 个原函数体对照，原生 macOS 与 Linux-host 编排及下载记录独立校核已通过（52/95 项组件），Windows 首次 manager 步骤因离线输入缺少 oracle 在测试前失败；改为独立在线准备后，原生 Windows manager 25 项检查及 154 条元数据输出独立复核通过。后续 Windows 批次因 VS stable 新 build 未就绪在准备阶段停止；每个在线组件入口刷新工具链的 CI 修正待验证。完整 P1 仍未验收，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

`upstream-context macos` 已实现未改写的 Objective-C++ ScreenCaptureKit 原源码入口，首轮 24 项原生检查通过，60 张窗口图像与预热图已独立复核。窗口路径核验已知内部像素和关闭后持帧，首个 display 路径仅观察布局/所有权；原生资源计数不当作配平断言，Mach port 增长的因果仍待隔离。首个私有 Rust macOS 捕获切片在测试专用 main-thread AppKit host 修正后，40 项原生检查全部通过，60 组 raw/BGR 独立匹配同窗口原版；旧初始化断言/编译失败保留，直接核心构建隔离已通过，组件增至 42 项；私有 manager 的最新 49 项组件（含 Clippy）全部通过，320 组脚本及双方 20 轮真实转发/各 60 张独立留存图已复核；完整后端未验收，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

| 文档 | 职责及当前状态 |
| --- | --- |
| [实施计划](plans/maafw-screencap-extraction-plan.md) | 项目范围、P1 提取 F / P2 冻结验收 / 产品 E 顺序、依赖政策、参考布局、后端与发布门禁；修订已确认，实施待完成 |
| [技术选型与决策](decisions/interactive-technology-selection.md) | R1–R8 的理由、约束及替代关系；选型已确认，原型/验收按条目记录 |
| [ADR 0001：GDI 全虚拟桌面](decisions/0001-gdi-virtual-desktop-capture.md) | 纳入独立产品截图方式的范围与理由；P2 后 P3 实施，尚未实现或验收，不替代 R1–R8 |
| [ADR 0002：原生接口现代化](decisions/0002-native-interface-modernization.md) | 旧兼容用法迁移及有效接口升级的范围与理由；P2 后实施，尚未实现或验收 |
| [ADR 0003：修正 fork 提取来源](decisions/0003-corrected-fork-extraction-baseline.md) | 替代旧 U → P 实施顺序，修复进入 P1 提取，P2 冻结验收 F；来源准备、helper 同步与本地历史整理已实现；评审 v2 增加一次候选复核及根 CI，完整 F 验收仍待完成 |
| [架构与项目结构](design/architecture.md) | 内部依赖、状态归属、样本交接和目标目录；方向已确认，具体 SDK 在 P3 定案 |
| [GDI 全虚拟桌面设计](design/gdi-virtual-desktop.md) | 屏幕 DC、单次 BitBlt、DIB、布局与资源流程；产品扩展规划，尚未实现或验收 |
| [原生接口现代化设计](design/native-interface-modernization.md) | DIB、DXGI、FramePool、PipeWire、ExtImage 迁移清单、能力与执行边界；已纳入实施，尚未实现或验收 |
| [执行模型与窗口协调](design/execution-model.md) | 原生执行通道、线程迁移、回调退出和窗口租约；P3 设计起点，P0/P1 保持上游行为 |
| [Frame 与内存契约](contracts/frame.md) | 像素、布局、所有权、分配和视图/转换；P3 设计起点，尚非已发布 API |
| [来源与生命周期契约](contracts/lifecycle.md) | 来源依赖、状态、代次、失败恢复和原生输入归属；P3/P4 设计起点 |
| [C ABI 与回调契约](contracts/c-abi.md) | ABI 演进、声明生成、provider、回调和语言边界；P3 定案，P5 消费验收 |
| [开发与验证指南](guides/development.md) | 当前可执行命令，以及准备/构建/测试任务的实施要求；明确区分已存在和计划入口 |

## 分层与变更规则

- plans/ 回答做什么、阶段顺序和退出条件；保留计划第 1–18 节编号，详细规则以链接指向主题文档。
- decisions/ 回答为什么选、有哪些取舍。保留 R1–R8；新的独立 ADR 使用四位编号和主题文件名，记载状态、日期、影响阶段及替代关系，不覆盖历史决定。
- design/ 回答内部如何组织与执行；contracts/ 回答调用方可依赖的保证，二者通过链接协作，不复制整段规则。
- guides/ 回答如何操作。尚未实现的命令明确标注，不与当前可用入口混列成可执行流程。
- 每项详细规则维护一份权威定义，计划只保留门禁与摘要，AGENTS.md 只提供执行摘要及链接；规则调整时同步受影响的引用和验收条件。

文档区分“提议、已确认、已实现、已验收、已替代”；这些是分别记录的事实，不以设计确认代替实现，不以编译通过代替原生桌面验收。契约形状尚未冻结的文档同时标明 P3 定案边界。

目录只在有实际内容时创建。当前 design/contracts 的内容从既有长计划提取并补充；后续后端文档足够多再建 design/backends/，真正可用的 SDK/Python 接口出现后再新增对应指南，不创建空模板或占位消费工程。通常保持两层主题路径，进一步细分应有明确的阅读需要。

仓库内使用相对 Markdown 链接。移动或重命名时同步 README、AGENTS.md、计划、选型和所有互链，检查目标文件及锚点；不保留重复旧路径副本，不提交开发机绝对路径。

## 产品文档与外部记录

产品 docs 保存可移植的计划、设计、契约、决策及使用说明。实际工具发现、消费者绝对路径、来源/许可审计、固定上游副本、补丁、职责映射、构建结果与验收报告仍保存在同级 maafw-screencap-reference，按计划第 6.2 节管理；缓存保存在 maafw-screencap-deps。

外部工程记录的 schema 与校验逻辑在实际格式落地时随 xtask 维护，实际记录不进入产品仓库。文档中的目标目录树不证明对应文件已经实现，实际文件以仓库清单为准。
