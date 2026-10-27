# 交互式技术选型记录

日期：2026-10-27。依据：[抽取计划](../plans/maafw-screencap-extraction-plan.md)。

本记录是项目自身的技术选型与决策说明，归档于仓库 `docs/`。本机环境、绝对路径及消费者检查明细另存于同级参考目录的 `local/observations/local-environment-and-consumer-observations-2026-10-27.md`；来源审计、补丁、映射和对照材料按基线/运行批次外置。已确认的技术路线仍需完成所属阶段的原型和验收；选型确认不表示平台支持或验收已经通过。

本轮首版前置技术选型状态：已完成，启动 P0 所需的用户决策已齐备；已覆盖 P0/P1 主技术路线、实际消费者、Python 分阶段路线及首版/后续分发政策。工程结构按用户对单库的倾向和平台隔离要求落实；后续架构与文档评审的优化已按 R8 收口；其余路线按各轮明确答复确认。已同步产品抽取计划、AGENTS.md 和 nightly 工具链配置；具体 SDK 接口、参数和打包细节按产品计划第 17.1 节在对应阶段细化。P0 部分原型及构建入口已经实现，实际状态见[开发指南](../guides/development.md)；完整三平台原生验收仍待完成。

| 项目 | 收口结论 |
| --- | --- |
| 工程 | 单一捕获 crate + xtask，模块/依赖/原生构建按目标平台隔离 |
| Rust 与分配 | 最新 nightly + 标准库实验 fallible API，内部窄封装，不维护 stable 双实现；稳定并验收后迁回 stable |
| Windows | windows-rs 专用 crate + windows-bindgen 私有投影；`x86_64-pc-windows-msvc` |
| Linux | pipewire + wayland-client 高层绑定，显式启用 wayland-backend client_system |
| macOS | objc2 框架绑定直接接入，按需使用 block2/dispatch2 |
| U/P 适配 | C++ + OpenCV，通过必要的私有窄 C 边界接入 |
| 原生构建 | CMake 按平台选生成器；首版 Windows 仅实现已安装 VS2026 + MSBuild，VS2022/MSVC-Ninja 后续扩展 |
| OpenCV 准备 | 最新正式版源码构建为默认，核验模块与优化配置并按身份/配置缓存 |
| 验收环境 | GitHub 托管 runner 优先，先探测真实桌面、权限、GPU/原生取帧与回调退出能力 |
| 实际消费者 | Python 优先，首个项目为使用 opencv-python 的 nightreign-overlay-helper 游戏悬浮窗 |
| Python binding | 首版 ctypes + 公共 C ABI，后续迁移至 PyO3 + maturin 并继续经公共 C ABI 接入 |
| 发布渠道 | 首版仅 GitHub Release SDK/配套 Rust 与 Python 资产；后续版本再启用语言官方/权威公开渠道 |
| 参考资料布局 | 保留三个同级目录；deps 管缓存，reference 按 baselines/patches/runs/local 归档，项目技术决策留在 docs |

## 已确认需求与后续实施决策

后续独立决策使用四位编号维护：[ADR 0001：GDI 全虚拟桌面](0001-gdi-virtual-desktop-capture.md)已纳入产品扩展规划，P2 后在 P3 实施，不替代 R1–R8，也不扩大 U/P 迁移范围。
[ADR 0002：原生接口现代化](0002-native-interface-modernization.md)将旧兼容用法迁移及新接口接入纳入 P3，策略/消费随 P4/P5 验收；详细清单与能力边界独立维护，不提前改动 U/P。

| 项目 | 当前状态 | 处理时机 |
| --- | --- | --- |
| 实际消费者 | 用户已明确 Python 游戏悬浮窗 nightreign-overlay-helper，已有 opencv-python | 按 R5-01 的线程、像素、帧寿命和冻结包约束实施并验收 R6 已选路线 |
| 发布渠道 | 用户已补充首版仅 GitHub Release SDK，后续公开多渠道 | 按 R5-02 分阶段启用；首次发布和首次启用新渠道前提醒准备相应凭据 |
| GDI 全虚拟桌面 | 独立产品截图方式已纳入规划，尚未实现或验收 | 按 ADR 0001 在 P2 验收后的 P3 实施；具体方法/目标和 ABI 值届时定案，P4/P5 验证策略及消费 |
| 原生接口现代化 | 已按 ADR 0002 纳入实施，尚未实现或验收 | P3 迁移 DIB、DXGI、FramePool、PipeWire 并接入 ExtImage；P4/P5 完成能力、策略及真实消费验收，逐项冻结最低运行要求及显式兼容条件 |
| 平台 API/feature、生成参数及窄桥接 | 待原型证据 | P0/P1 验证并细化，不将每个局部实现参数变成用户前置审批 |
| SDK ABI/安全 API、版本/错误/回调具体机制 | 尚未最终定案 | P2 验收后在 P3 落实 |
| Python 接入与轮包工具、C++ 包装、Rust 可选图像适配及资产布局 | Python 首版 ctypes、后续 PyO3 + maturin 已定；具体参数与其他工具细化 | 按 R6 及 P3 契约落实声明、视图、初始安装包和迁移验收，其他绑定在交付前定案 |
| 异步流、共享转换及性能优化 | 未采纳，条件评估 | P6 根据完整链路测量及实际需求决定 |

消费者、Python 分阶段路线与分发需求已明确，具体参数按原型和 P3 契约细化，不提前实施或冻结 P2 之后的 SDK 设计。首次发版（含各 binding、预发布）及首次启用新发布渠道前先提醒用户配置 GitHub 仓库所需密钥/相应发布配置，按产品计划第 15.6 节执行。
当前没有待用户回答的前置选型问题。后续若原型证据表明已选路线无法满足验收，或出现影响范围、兼容性与发布方式的新取舍，再就受影响事项继续交互选型。

## 沿用的计划约束

- 顺序为 P0/P1 完整迁移 → P2 应用固定 PR #1587 → P3–P6 SDK 与扩展。
- Rust 为主体；默认最新 nightly 工具链，实验分配 API 集中在必要内部封装，不维护额外 stable 实现；相关 API 稳定并通过迁移验收后转回 stable。最终公共动态边界为 C ABI。
- 首批平台为 Windows x64、Linux x64、macOS arm64；本地只验证本机平台，其他平台由原生 Actions runner 验证，不使用 WSL 完成 Linux 验收。
- 最终基础核心和基础绑定无强制 OpenCV 依赖；U/P 使用显式启用的验收适配兑现上游转换和输出行为。
- 项目自身的技术选型和架构决策说明存入 `docs/`；参考资料、补丁、映射、来源/许可核验、实际构建与对照报告外置。本记录不固定未来依赖版本。

## R1-01 工程拆分与平台隔离

状态：按用户对“单一捕获库 + xtask”的倾向和明确的平台隔离要求收口；P0 平台隔离原型待验收。

采用一个捕获 crate，平台后端作为内部模块；xtask 独立承担准备、对照和资产任务。最终 SDK 公共接口在 P2 验收后设计，不因工程拆分提前冻结。

平台隔离采用三层措施：

1. 模块使用 `#[cfg(target_os = "windows")]`、`linux`、`macos`；后端可选 feature 与目标 OS 同时约束。使用 `#[cfg]` 移除不适用的模块，不以普通 `if cfg!(...)` 代替。
2. 平台运行依赖声明在 Cargo 的 `[target.'cfg(target_os = "...")'.dependencies]` 中，避免其他平台的运行依赖进入当前目标的编译/链接图。
3. `build.rs` 根据 `CARGO_CFG_TARGET_OS`、`TARGET` 等目标信息执行探测、生成及原生编译/链接；构建脚本本身在宿主上编译运行，构建工具依赖与产品运行依赖分别管理。

平台选择由编译目标自动决定，默认对应当前系统。同一平台内的后端开关仍遵循 AUTO/ON/OFF：xtask 在 Cargo 构建前探测并选择 feature，`build.rs` 复验；源码消费者显式选择所需 feature。AUTO 缺依赖禁用并报告，显式 ON 缺依赖失败。

“不包含其他平台代码”指其他平台后端不会进入本次产品编译和链接；共享逻辑正常编译。源码分发仍保留各平台源文件。Cargo 可为所有平台解析依赖并记录到同次 lockfile，这与实际编译/链接范围不同。

P0 验收：在三个原生平台分别验证模块、运行依赖、原生探测和链接隔离；验证同平台后端缺依赖时的 AUTO/ON/OFF 行为。此项尚未执行。

依据：

- [Rust 条件编译](https://doc.rust-lang.org/reference/conditional-compilation.html)
- [Cargo 平台依赖](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html#platform-specific-dependencies)
- [Cargo 构建脚本](https://doc.rust-lang.org/cargo/reference/build-scripts.html)
- [Cargo 依赖解析](https://doc.rust-lang.org/cargo/reference/resolver.html#dependency-kinds)

## R1-02 Windows 原生绑定

状态：用户已确认 A，P0 原型待验收。

采用 Microsoft windows-rs 生态的专用 crate，以及由 `windows-bindgen` 生成的项目私有绑定。按实际需要使用核心 COM/WinRT 类型，生成未覆盖的 Win32、D3D11、DXGI、WinRT 等声明；维护生成输入，生成输出放入 `OUT_DIR`，安全封装独立维护。具体专用 crate 集合和生成参数由 P0 覆盖验证确定。

选择理由：符合 Microsoft 当前面向可复用库的建议，减少 umbrella crate 的依赖重量及多版本类型协调成本。代价是需要维护生成配置与覆盖检查。

P0 验收：原生取帧、GUID/HRESULT、调用约定、COM apartment、引用计数、WinRT 接口及事件回调、注销排空、D3D/DXGI 对象与链接。发现绑定缺口时记录证据并讨论必要窄桥接，不默认为已覆盖。

依据：[Microsoft windows-rs 官方说明](https://github.com/microsoft/windows-rs)。

## R1-03 U/P OpenCV 验收适配器

状态：用户已确认 A，P0 原型待验收。

主要使用 C++ + OpenCV，沿上游 `cv::Mat`、`core`/`imgproc` 操作、参数和顺序建立对照。C++ 转换适配只参与显式启用的验收路径，捕获流程仍由 Rust 实现。

适配器通过必要的私有窄 C 边界接入，C++ 异常在边界内转换为明确结果；`cv::Mat`、STL 和适配器对象不跨公共 C ABI。不因转换后移改变失败、缓存、等待/回放、缓冲归还或帧寿命；必要时在原调用位置接入并验证。

选择理由：便于逐项追踪上游 C++ 转换语义及生命周期。代价是增加显式验收目标的原生编译和链接配置；不将该适配器变为产品共享转换库。

P0/P1 验收：独立原始输出与转换结果、stride/方向/ROI/颜色、转换失败、持帧寿命、异常隔离和上游调用顺序；Rust 产品基础构建独立于该适配器。

## R2-01 Linux 原生绑定

状态：用户已确认 A，P0 原型待验收。

优先采用 `pipewire` 和 `wayland-client` 高层绑定，并显式启用 `wayland-backend` 的 `client_system`，使 U/P 沿用系统 libwayland。必要语义未覆盖时局部使用 `pipewire-sys`、`wayland-sys` 或最小 C shim；不默认改用纯 Rust 协议实现。

Rust 管理捕获流程、缓冲和停止逻辑。保持 PipeWire thread-loop 锁与回调纪律；关联对象操作在所需锁内进行，停止 loop 时不持该锁。Wayland 保持事件队列、代理对象、fd 所有权和 read/cancel-read 配平；宏、内联及可变参数缺口才引入窄桥接。

选择理由：复用绑定的声明、对象管理和事件机制，同时通过 system backend 保持原生接入路径。代价是必须逐项核验高层封装与上游调用顺序、队列和所有权语义的对应关系。

P0/P1 验收：PipeWire 取帧、锁/通知/停止、buffer 归还和初始化配平；Wlr 协议生成、事件顺序、队列隔离、fd 移交、断连和退出；系统库缺失时的配置行为。绑定选型不代替原生行为对照。

依据：

- [PipeWire ThreadLoop](https://pipewire.pages.freedesktop.org/pipewire-rs/pipewire/thread_loop/struct.ThreadLoop.html)
- [wayland-client](https://docs.rs/wayland-client/latest/wayland_client/)
- [wayland-backend 的 system 与 Rust backend](https://docs.rs/wayland-backend/latest/wayland_backend/)

## R2-02 macOS 原生绑定

状态：用户已确认 A，P0 原型待验收。

通过 `objc2` 和 `objc2-screen-capture-kit` 及所需 CoreMedia/CoreVideo/CoreFoundation/AppKit 等框架绑定直接接入；按需使用 `block2`、`dispatch2`。仅为已证明的绑定或原生互操作缺口增加最小 Objective-C 桥接，确需 Objective-C++ 时单独说明。

Rust 管理捕获流程，明确对象 retain/release、autorelease pool、block/context 保活、回调队列与线程归属。不以绑定提供类型声明推断全部外部缓冲和退出路径已经安全。

选择理由：将原生对象与捕获状态集中在 Rust 所有权边界，减少常驻桥接源码。代价是需要核验 framework feature、运行时可用性、协议对象和回调声明的实际覆盖。

P0/P1 验收：内容查询和截图完成回调、像素锁/解锁、独立持帧、失败回滚、停止/释放后晚到回调的保活与资源接管；确认原生队列和宿主事件循环的等待关系。

依据：

- [objc2](https://docs.rs/objc2/latest/objc2/)
- [objc2-screen-capture-kit](https://docs.rs/objc2-screen-capture-kit/latest/objc2_screen_capture_kit/)

## R3-01 可恢复分配与实验 API

状态：用户已确认补充轮 A：默认 nightly，采用标准库实验分配 API，不维护额外 stable 实现；标准库分配初步原型已验证，完整 P0 原型门禁待完成。

### 背景与未采用选项

优先保留 stable 标准能力：帧和集合增长使用 `Vec::try_reserve` / `try_reserve_exact` 等 fallible 路径，写入前检查容量和长度，避免隐式再次分配。

本轮查阅的 stable 标准库文档仍将 `Box::try_new`、`Arc::try_new` 标为 nightly-only。需要可恢复分配的对象和共享回调状态仍需补充方案；工具链更新时复核，不据此锁定旧版本。

- 候选 A：标准库优先，缺口评估 `allocator-api2::boxed::Box::try_new` 与 `triomphe::Arc::try_new`，仅通过项目内部窄所有者封装使用。`triomphe` 不支持 Weak，须确认对象图无需弱引用，并核验跨线程释放、地址稳定和失败回滚；不把候选库视为已经通过审核。
- 候选 B：标准库 fallible 容器配合项目自有的最小稳定地址/共享所有者封装。减少第三方依赖，但增加 unsafe、引用计数和析构规则的维护与审查责任。

上述两条初始路线均可保持 stable 构建，但未被选中。用户先补充“可以考虑使用实验 API”，随后明确选择默认 nightly，因此采用下述标准库路线，替代原计划的 stable-only 约束。

### 已选路线：默认 nightly 与标准库实验分配 API

直接使用标准库的 fallible Box/Arc 构造，保留标准 `Arc/Weak` 等所有权机制。查阅时，当前 stable 文档仍将这些构造方法标为实验接口；当前 nightly 文档使用 `#![feature(allocator_ext)]`，不能将 allocator_api 的部分稳定化等同于所有 fallible 方法已经稳定。

具体 feature gate 以实际采用的 nightly 文档和编译验证为准；跟进工具链更新，实际工具链身份随该次构建记录保存，不作为未来版本锁定。

默认使用 nightly，fallible 所有者优先采用标准库实验 API；实验能力限制在内部所需封装。Rust 源码消费者需要 nightly，不维护额外 stable 分配实现，也不自行重写通用引用计数。标准 `Vec::try_reserve` 等已稳定能力继续优先使用。

选型理由：保留标准 Box、Arc/Weak 的所有权机制，减少第三方替代类型和双路径维护。代价是 Rust 源码构建依赖 nightly，并需随工具链核验实验 API/gate；相关 API 全部稳定且迁移验收通过后，移除 gate 并迁回 stable。

实验分配 API 不消除数据内部隐式分配、地址移动、跨线程释放和回调注销责任。所有路线都需检查帧、集合、对象/回调状态、错误通知中的分配点；不承诺恢复系统/宿主主动终止进程的 OOM，不以 `catch_unwind` 代替分配错误返回。

选型阶段同步修订了抽取计划第 5.1、8.2、15.1、17 节及相关工具链/验收表述，更新 AGENTS.md，并新增 `rust-toolchain.toml`：`channel = "nightly"`、`profile = "minimal"`、组件 `rustfmt`/`clippy`。该阶段只落实文档与配置；后续 P0 已更新 nightly，并验证 Box/Arc fallible 构造、失败回滚、Weak 和容量溢出，具体运行证据外置归档。无需通过 `RUSTC_BOOTSTRAP` 将 stable 伪装为实验工具链。

依据：

- [std Box::try_new](https://doc.rust-lang.org/std/boxed/struct.Box.html#method.try_new)
- [std Arc::try_new](https://doc.rust-lang.org/std/sync/struct.Arc.html#method.try_new)
- [allocator-api2 Box](https://docs.rs/allocator-api2/latest/allocator_api2/boxed/struct.Box.html)
- [triomphe](https://docs.rs/triomphe/latest/triomphe/)
- [nightly Box::try_new / allocator_ext](https://doc.rust-lang.org/nightly/std/boxed/struct.Box.html#method.try_new)
- [nightly Arc::try_new / allocator_ext](https://doc.rust-lang.org/nightly/std/sync/struct.Arc.html#method.try_new)
- [Rustup 工具链通道](https://rust-lang.github.io/rustup/concepts/channels.html)

## R3-02 原生验收环境

状态：用户已确认 C，目前优先使用 GitHub 托管 runner；能力探测待实施。

先探测托管 runner 的原生桌面、权限和 GPU 能力，再明确缺失场景及所需补充环境。当前没有已确认可用的自托管机器；补充环境的具体方式尚未选择。

P0 探测分别覆盖 Windows x64、Linux x64、macOS arm64 的实际 runner 架构、OS/SDK、图形适配器/驱动、桌面会话/显示来源、权限、后端可创建性、真实取帧和回调停止。Linux 同时核验实际 Wayland compositor/Wlr 协议与 PipeWire 来源；macOS 核验 ScreenCaptureKit 取帧所需会话与权限。

用受控窗口/图样确认取得的内容和来源，不只以 API 返回成功或缓冲非空判为通过。软件渲染、虚拟桌面、缺少显示器或缺少权限的条件据实记录，不据此推断物理 GPU、多显示器或权限边界场景已经通过。

输出“可构建、可加载、通过原生取帧、缺失/未验证场景”的分项结果；环境能力不足时保留未验收状态和明确的补充需求，不将跳过结果或编译成功代替桌面验收，不放宽 P0/P1 既定门禁。

依据：[GitHub 托管 runner 文档](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)。

## 线程与回调退出：沿用现有计划

沿用现有计划：P0/P1 保持上游线程、事件、等待和退出语义，标准同步原语与必要原生机制优先；新增统一调度、取消/期限及 SDK 回调契约在 P2 后设计。P0 通过可控事件和真实原生回调验证注销排空、晚到资源、部分初始化失败和清理，不以引入通用异步运行时替代这些责任。

最终 SDK/ABI 与绑定分发决策按计划阶段推进；原型实施中的局部参数根据已选路线及实际验证确定。

## R4-01 Windows ABI

状态：用户已确认 A，原生互操作与构建待验收。

首批 Windows x64 使用 `x86_64-pc-windows-msvc`，Rust、C++ 验收适配和 OpenCV 核对架构、ABI、运行库和实际链接配置。GNU/MinGW 不作为首批路线。

## R4-02 C++ 验收构建与 VS 产品线

状态：用户补充并确认首版验证仅需实现已安装 VS2026 + MSBuild，构建入口与验收待实施；此前四组合要求调整为分阶段交付。

采用 CMake，生成器按平台选择。首版 Windows 仅实现 VS2026 + MSBuild：`Visual Studio 18 2026` 生成器、x64 架构，编译生成参考程序、C++ 验收适配和对应原生依赖，不要求 Ninja。

VS2022 + MSBuild、VS2022 + MSVC-Ninja、VS2026 + MSVC-Ninja 保留为后续扩展；不作为首版 Windows 实现范围或验收门禁。后续 MSBuild 配置使用对应 VS 生成器；Ninja 使用 `Ninja` 生成器，并先初始化明确选定 VS 实例的 MSVC x64 环境、核验编译器/链接器来源。

VS 产品线、生成器与实际 MSVC 工具集分别探测和记录，不以 VS 产品名替代实际编译器身份。配置入口不提交开发机绝对路径；各组合的构建目录及缓存分开，记录 SDK、运行库、profile/构建类型和依赖选项。

本机首版验证按用户补充要求使用已安装 VS2026，仅覆盖 MSBuild 编译生成路径；VS2022/MSVC-Ninja 在后续实现后分别验收。显式请求未实现组合时报告不支持，已实现组合缺少工具时报告配置缺失。Linux/macOS 按实际原生环境选择和记录生成器。

Rust 产品仍以 Cargo 为构建入口，xtask 编排独立参考/验收任务，小型原生桥接由 build.rs 集成；本轮未创建尚无对应 C++ 工程的空构建入口或 presets。

## R4-03 U/P OpenCV 获取

状态：用户已确认 A，源码准备及构建待实施。

默认采用最新正式版 OpenCV 源码构建，明确所需模块及传递依赖，核验实际操作的 CPU baseline/dispatch、SIMD 和适用 HAL/第三方优化；按解析身份、编译器、运行库和模块/优化配置复用缓存。同次 U/P 参考与 Rust 侧验收适配采用一致的实际依赖集合。

源码构建属于显式对照/验收准备，不进入产品基础构建，不将 OpenCV 二进制或验收适配器作为首版共享转换运行库分发。

## 本机环境与消费者检查材料

本机工具发现结果、实际版本、安装目录及消费者工程只读检查明细已分离到仓库外：`maafw-screencap-reference/local/observations/local-environment-and-consumer-observations-2026-10-27.md`。这些材料用于本机准备和验收追溯，不是未来依赖锁定，也不作为产品构建输入。本记录保留可移植的选型结论、消费约束和验收要求。

依据：

- [Rust Windows MSVC target](https://doc.rust-lang.org/nightly/rustc/platform-support/windows-msvc.html)
- [CMake VS2022 generator](https://cmake.org/cmake/help/latest/generator/Visual%20Studio%2017%202022.html)
- [CMake VS2026 generator](https://cmake.org/cmake/help/latest/generator/Visual%20Studio%2018%202026.html)
- [CMake Ninja generator](https://cmake.org/cmake/help/latest/generator/Ninja.html)
- [CMake 生成器环境要求](https://cmake.org/cmake/help/latest/manual/cmake-generators.7.html)
- [MSVC 命令行开发环境](https://learn.microsoft.com/en-us/cpp/build/building-on-the-command-line?view=msvc-170)
- [OpenCV 配置、模块及 CPU 优化](https://docs.opencv.org/4.x/db/d05/tutorial_config_reference.html)

## R5-01 实际消费者与 Python 优先级

状态：用户已明确，消费者检查明细保存在外部报告。

首个消费者为 `nightreign-overlay-helper` 游戏悬浮窗工具，优先验证 Python 接入，复用已有 `opencv-python`。本机工程绝对路径由外部验收配置提供，具体检查结果见上述外部报告。

消费者适配与验收约束：

- 核验 Python、PyQt6、NumPy 与既有 cv2 环境的兼容性，区分声明范围、实际配置和通过验证的组合。
- 首版沿 ctypes + 公共 C ABI 路线，验证函数声明、数值/布局与帧所有权，不依赖借用地址在下一次捕获后继续有效。
- 后端创建、捕获与关闭在专用 Qt 工作线程执行，结果交回 GUI 线程；验证捕获期间的窗口几何变化。
- 保持 QImage 图像原点、行距、padding 和 RGB NumPy 输出语义，区分精确像素格式及 alpha，不将消费者输入统一假定为 BGR。
- 覆盖 PyInstaller onedir 冻结分发、原生 DLL 发现/收集和 Qt/OpenCV 依赖共存，并统计整个消费链路的复制与寿命。

验收目标为完整的原生取帧 → Python Frame → QImage → RGB NumPy/cv2 链路，统计复制、分配、端到端延迟和 GUI 响应；验证独立持帧、关闭后持帧、跨线程交付、混合 DPI/负坐标/移动与可写副本隔离。实际 Qt 集成属于消费者/示例层，不成为基础 SDK 的 Qt 依赖；实际消费者改动按 P3/P5 阶段安排。

## R5-02 分阶段公开分发

状态：用户已补充并确认首版仅 GitHub Release SDK，后续版本再公开多渠道；本条覆盖此前未区分版本的渠道描述。具体 namespace、包身份与上传 Workflow 尚未创建/冻结。

首版仅通过 GitHub Release 提供 SDK/配套源码和 binding 资产：包括 Rust 源码 crate、ctypes Python binding 及适用 wheel/安装包，不上传 crates.io 或 PyPI。打包、本地安装和冻结分发消费验证照常执行。

后续版本的目标仍为 Rust → crates.io，Python → PyPI，C# → nuget.org，Java → Maven Central（Maven/Gradle 消费），Node.js → npm 公共 registry。Go 使用源码仓库的模块版本 tag 发布，经模块代理获取并由 pkg.go.dev 收录；pkg.go.dev 不是独立上传二进制包的 registry。GitHub Release SDK 继续保留。

渠道按相应 binding 的交付、消费验收与版本安排启用；保持当前 Rust/C/C++/Python 首批范围，Python 实际接入优先。其他语言的具体接入工具、原生资产布局、版本/tag 映射与发布 Workflow 在其交付阶段落实。PyO3 迁移和公开包渠道启用分别验证，具体版本安排在发布准备时确定。首次发版、各新增 binding 首次发布及既有产物首次启用新渠道前，按产品计划第 15.6 节先提醒用户配置相应 GitHub 仓库密钥/发布配置。

依据：[Go 发布](https://go.dev/doc/modules/publishing)、[pkg.go.dev 收录](https://pkg.go.dev/about#adding-a-package)、[Maven Central 的 Gradle 发布说明](https://central.sonatype.org/publish/publish-portal-gradle/)。其他渠道见产品计划第 15.7 节的维护者文档。

## R6 Python binding 分阶段路线

状态：用户已确认首版验证采用 ctypes + 公共 C ABI，后续改用 PyO3 + maturin；两个阶段的实现与验收均待开展。具体公共接口、声明/视图与迁移适配仍按 P2 验收后的阶段边界确定。

| 阶段 | 技术路线与验收重点 |
| --- | --- |
| 首版验证/交付 | ctypes + 公共 C ABI；沿用当前消费者接入方式，基础绑定不新增第三方 FFI 运行依赖。核验 C 头同源声明/布局/签名、加载协商、Frame 真正保活、只读视图/可写副本、Qt 工作线程与 GUI 交付、PyInstaller 收集；CDLL 释放 GIL 不替代宿主线程调度 |
| 后续版本迁移 | PyO3 + maturin，继续通过公共 C ABI 接入同一核心，使用 Rust 实现 Python 对象/缓冲协议。与 ctypes 使用相同夹具核验像素、所有权、错误、线程和关闭行为，补充解释器/ABI/wheel、跨线程释放与冻结包兼容验收后切换主路线 |

保留 `maafw_screencap` 导入名，公共接口变化按 SemVer 声明；迁移期对照和必要兼容适配在 P3/P5 契约基础上设计。首版 SDK 中的可安装 Python 资产仅放入 GitHub Release；后续启用 PyPI 时另行完成渠道与凭据准备。

共享要求：原始捕获不强制 Qt/NumPy/OpenCV；可选薄适配复用消费者的 cv2。视图必须保活真实 Frame，写入路径使用隔离副本；不能仅凭地址包装或释放 GIL 宣称安全、端到端零复制或更高帧率。

依据：[Python ctypes](https://docs.python.org/3/library/ctypes.html#ctypes.CDLL)、[PyO3 缓冲协议](https://pyo3.rs/main/class/protocols)、[maturin bindings](https://www.maturin.rs/bindings.html)。

## R7 参考资料布局调整

保留产品、deps 和 reference 三个同级目录，详细布局见[抽取计划第 6.2 节](../plans/maafw-screencap-extraction-plan.md)。本次调整的是资料归档边界，U → P → E 阶段与首版选型沿用既有安排。

- deps 统一保存 Cargo、下载及原生依赖的可清理缓存；该次解析文件和需保留的离线重建输入进入 reference 的 `runs/<run-id>/inputs/`，不再设置职责重叠的根 `dependencies/`。
- U/P 按 `baselines/{u,p}/<baseline-id>/` 保存独立参考程序、来源/映射清单与 `audits/`；P 引用确切父 U 和累计补丁，已归档基线变更使用新 ID。
- `patches/pr-1587/` 保存累计补丁及其独立来源清单；根 `manifest.json` 只索引基线、补丁与运行批次。
- 构建工作、变体产物、日志和对照报告按 `runs/<run-id>/<platform-config>/` 归档，分别标识参考/Rust U/P/E，保持同批次依赖和输入关系，避免不同平台/重跑结果覆盖。
- 本机信息与消费者路径使用 `local/`。现有环境/消费者观察报告归入 `local/observations/`，不与实际 U/P/E 验收报告混放；项目通用技术选型说明继续存于仓库 docs。

基线/运行清单中的实际版本和快照只服务该次追溯及重建，不成为未来依赖锁定。已验收基线与发布记录保留必要快照和产物，离线重建所需材料独立于可清理缓存；其余目录按 P0 任务实际需要建立，不预建空清单或把目录约定当作验收结果。

## R8 架构边界、构建启动与文档分层

状态：用户已要求落实本轮全部架构/项目结构和 docs 分层建议；文档组织已落实，产品实现仍受 P0 → P1 → P2 → P3 门禁约束。

保留单一捕获 crate + xtask、既有三平台绑定、nightly、U/P OpenCV 对照、Python 分阶段路线和首版分发政策。优化集中在明确依赖与所有权，而非提前引入 SDK 实现或拆分多核心 crate。

| 决策 | 理由与权威说明 |
| --- | --- |
| 基础模型、会话编排、策略、内部后端和平台资源单向协作 | 减少原 core/state/policy 交叉职责；Windows 协调保持平台私有，见[架构设计](../design/architecture.md) |
| 后端私有原生租约 → 独立 CPU 样本 → 不可变 Frame | 明确释放点、跨线程边界和复制次数；发布优先转移既有存储，U/P 不移动原转换调用点 |
| 逐后端记录执行/销毁和等待关系 | 串行使用不等于可跨线程迁移，注销也不自动表示回调已排空；见[执行模型](../design/execution-model.md) |
| xtask 准备不依赖捕获 crate，build.rs 消费明确配置 | 避免准备任务启动循环，保持直接 Cargo 消费；feature 只增加能力，见[开发指南](../guides/development.md) |
| c_api 区分对外 ABI，原生 FFI 靠近后端 | 将系统调用与公共导出分开；公共头继续作为声明/常量/符号同源生成或核验的权威输入，见[C ABI 设计](../contracts/c-abi.md) |
| 单元测试随模块，集成测试有明确 Cargo target | tests/contracts/main.rs 对应 --test contracts；跨语言由 xtask 编排，自有夹具与外部参考分开，examples 随可用 API 建立 |
| docs 按职责分层且每项详细规则有唯一权威来源 | plans 管阶段门禁，decisions 管理由，design 管内部实现，contracts 管对外保证，guides 管操作；见[文档导航](../README.md) |

现有 R1–R7 标识与已确认取舍保持可引用；后续独立 ADR 使用编号文件记录提议/确认/替代关系，不机械拆分既有记录。契约文档是 P3 的设计起点，选型确认、实现完成和平台验收分别标记；不因迁移文档或建立目录宣布 SDK/后端已经交付。
