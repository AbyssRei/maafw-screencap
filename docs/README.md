# 文档导航与维护约定

日期：2026-10-29。P0 准备阶段已验收，P1 实施中：独立 xtask、外部来源校验、分配原型、显式 Windows GDI、COM apartment、D3D/DXGI/WinRT 读回及受控窗口 FramePool 捕获/事件原型已实现，Actions 原生矩阵已定义；Linux PipeWire 自有 C 源夹具的 20 次流生命周期、取帧、独立持帧和完整退出已通过原生 Actions，早期 SPA 夹具退出失败证据保留外置；macOS 主线程 ScreenCaptureKit 原型已通过 20 次受控窗口取帧、独立持帧、完成块释放与晚到结果原生验证；Wlr 真实 headless/Pixman compositor 与 system libwayland 原型已通过 20 次取帧/SHM/fd 生命周期、队列隔离及正常退出的原生 Actions 验证；Desktop Duplication 的 20 次硬件采集/释放原型已通过本机及原生 Actions 验证；结果按平台记录，P0 已通过计划第 16 节的准备与最小原生可行性门禁；完整 U 行为对照仍由 P1 完成。默认 binary 仍为 Hello World；内部 library 开始窗口 GDI 与 PrintWindow 迁移及显式局部对照，完整 U、公共 SDK 和绑定尚未完成。实际命令及原型边界见[开发指南](guides/development.md)。

## 阅读顺序与权威来源

P1 另已加入独立 ScreenDC 内部实现和默认路径的 ForegroundUtils/HwndUtils 迁移，正式入口为 `cargo xtask upstream-parity screen-dc`。原头文件脚本、真实资源故障及本机局部像素/清理检查已有证据；亮屏且自有测试窗口置顶的新批次通过 20 轮严格源码/Rust 对照。此前黑图/灰图、夹具资源和记录目录失败均保留；相同受控场景随后通过两个精确提交的原生 Actions；遮挡、移动、实际显示深度与完整后端/阶段验收仍未完成，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

FramePool 也已开始独立内部迁移与显式 `upstream-parity frame-pool` 局部对照：本机四个 Rust 调用环境匹配原图，缓存克隆与关闭后持帧已有实际证据，另有 20 次 Rust 完整取帧/关闭生命周期与 dispatcher 完成退出。伪最小化包装也已迁移，但本机原源码最小化场景的彩色图样检查返回黑图，匹配的 Rust 字节/状态仅记诊断；完整源码重复/故障生命周期、原生错误/重建、包装验收和 manager 仍待完成；不构成公共 SDK 或阶段验收。


DesktopDup/Window 也已有独立内部迁移及 `upstream-parity desktop-dup` 入口。本机三个原源码进程（含空 HWND 的 adapter 0/output 0）、20 次混合 Rust 生命周期及独立转换/异常边界检查通过受控目标图样、帧归还与持帧；原映射 Mat 生命周期缺陷由明确计量的安全复制适配，完整原生故障、多输出/旋转和 manager 仍待验收。

截图 manager 已开始私有迁移，`upstream-parity manager` 对照原源码函数体的 122 个脚本，并通过本机 20 次真实 GDI manager 生命周期。它保留预热/测速、毫秒截断、候选与图像释放及 U 的调用线程 DPI 副作用；完整原生混合方法、包装和 framework 转发仍待验收，不构成公共 SDK。

PipeWire 已开始消费端算法迁移，`upstream-model pipewire` 在 Windows 工具链上比较原源码的等待、回放、停用、转换与所有权边界；本机 74 个案例/118 次操作匹配。受控原生批次中，完整原 C++、Rust daemon、Rust supplied-fd、Rust OpenCV 各通过 20 生命周期/60 张关闭后持帧；七格式/28 组有效 POD 与精确转换字节对照也通过 Actions。真实 MemFd helper 的 60 次复制/所有权与六类拒绝，以及原 C++/Rust 同一对象各 40 个 session/120 张持帧也已通过原生对照。测试区分已 join 的内核退出任务与活跃线程，严格保留 fd/任务身份检查；旧即时数量断言和后续编译失败不撤销。实际 Portal 授权、通用 fd 失败/重试、GPU 同步、完整格式/故障及 P1 仍待完成；真实 kernel DMA helper 和独立生产器/回调流检查已加入，首个 helper 编译失败保留，修正后原生编译与负向控制通过，真实 kernel helper 已通过，实际 NULL-data DMA 流已通过原生对照，旧 const/API 编译失败保留，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

`upstream-context wlr` 的原源码入口已通过 20 次原生生命周期/60 张持帧及递归 roundtrip 检查；私有日志、名称提供者及被动 manager 夹具边界保留。首个私有 Rust Wlr 捕获切片与对应 native 探针已加入；首轮编译失败修正后，20 生命周期/60 组 raw+BGR 与实际转换异常检查已通过原生运行；该批次仍保留一项 Clippy 失败，修正待复验，不代表完整后端已验收。操作与边界见[开发指南](guides/development.md#当前-p1-迁移与对照)。

`upstream-context macos` 已实现未改写的 Objective-C++ ScreenCaptureKit 原源码入口，首轮 24 项原生检查通过，60 张窗口图像与预热图已独立复核。窗口路径核验已知内部像素和关闭后持帧，首个 display 路径仅观察布局/所有权；原生资源计数不当作配平断言，Mach port 增长的因果仍待隔离。它不代表 Rust macOS 后端已迁移，详见[开发指南](guides/development.md#当前-p1-迁移与对照)。

| 文档 | 职责及当前状态 |
| --- | --- |
| [实施计划](plans/maafw-screencap-extraction-plan.md) | 项目范围、U/P/E 顺序、依赖政策、参考资料布局、后端要求、验收与发布门禁；计划已确认，实施待完成 |
| [技术选型与决策](decisions/interactive-technology-selection.md) | R1–R8 的理由、约束及替代关系；选型已确认，原型/验收按条目记录 |
| [ADR 0001：GDI 全虚拟桌面](decisions/0001-gdi-virtual-desktop-capture.md) | 纳入独立产品截图方式的范围与理由；P2 后 P3 实施，尚未实现或验收，不替代 R1–R8 |
| [ADR 0002：原生接口现代化](decisions/0002-native-interface-modernization.md) | 旧兼容用法迁移及有效接口升级的范围与理由；P2 后实施，尚未实现或验收 |
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
