# 开发、构建与验证

日期：2026-11-04。状态：P0 准备阶段已按计划第 16 节验收，P1 实施中，已有多个后端、helper 和管理器的私有切片及显式局部对照。完整来源、平台和调用链验收仍未完成；SDK 与产品能力保持各自阶段边界。

## 逐函数对应审阅

`MaaBuffer.cpp` 新增基础字符串/图像 helper 原生消费验证：完整未修改 F 文件现在作为独立 translation unit 编译一次，实际公共 `MaaBuffer.h` 一同归档；旧五体生成片段仅保留为来源比较输入，不再 include。新增八个 StringBuffer 与九个 ImageBuffer helper 的实际调用，连同原五项操作，选定原生运行范围为 22/45；其余二十三项 list/rect 函数逐一登记为仅编译、未运行验收。没有把这些原生 API 填成 Rust 产品实现或最终 SDK ABI，Rust 函数对应并集不因此增长；较早五体审阅保留原批身份。

Windows 显式归档离线依赖下，Debug/Release 各通过八组 Custom 原生测试和八组记录校核。新增每配置二十轮、一百个独立字符串 owner、780 个独立图像 owner，覆盖嵌入 NUL、空值、非 UTF-8 字节、十三个实际深度和 1/3/4 通道。原 buffer clear/destroy、输入覆盖后完整保留 240 字符串字节和 45,120 图像字节，最终 owner 归零；释放后的 Mat 通道/type 元数据按实际来源保留，不假定 reset 为默认类型。这是 CPU helper/所有权验证，零桌面截图，也不定案 SDK 字符串编码。

首次整文件单独编译后，旧负尺寸 resize 回归实际触发 OpenCV terminate，新增 buffers/raw 尚未运行；失败程序和输出外置保留，没有补造原生退出码。目标改用 `/EHsc-`，清除 C 链接函数不抛异常的编译假设，同时保留同步 C++ 展开，使实际原函数异常进入既有私有 `guarded` 边界；两种配置的该回归和其余七组随后均通过。Debug/Release 真实 CL 命令确认该选项在 `/EHsc` 之后，原始函数体和异常行为未改；依据见 [Microsoft `/EH` 文档](https://learn.microsoft.com/en-us/cpp/build/reference/eh-exception-handling-model)。这不声称恢复 SEH/进程终止或任意分配故障。

`cargo xtask upstream-trace custom-buffers <外部 buffers 目录> [--check-only]` 核对八份实际数据文件、全部行/字节/owner 和完整目录，记录不进入产品。二十六项实际数据控制接受四项、拒绝二十二项；Windows 上执行三个平台真实 Step wrapper 的 54 个输出/退出控制及两个真实 native/checker 子进程通过。三平台独立 parity/records 入口已接入，拒绝零测试、部分/重复汇总、非零退出和损坏数据；新 Linux/macOS 原生执行仍待 Actions。本批未重新解析在线最新版依赖，也未改变预装工具年龄只作记录的 CI 规则。完整审阅由参考目录 `local/current-basic-buffer-helpers-development-path.txt` 和 `local/current-basic-buffer-helpers-source-review-path.txt` 指向，完整 P1 仍未通过。

剩余二十三个 list/rect helper 已另作固定 F 的完整顶层 Git 字节检索：744 个 blob 身份与内容均核对，五个 gitlink 仅记录身份、未把子模块内容算入检索。162 个函数标识符位点与 304 个相关类型/函数位点分布在 38 份文件；定义、声明、旧模块/语言导出、通用框架样例/测试之外，类型用于 resource/tasker/context 的 pipeline/识别结果以及 custom recognition/action agent RPC。选定 Win32/Custom/Gamepad/Linux/macOS/Dbg 与 ControlUnit 公共头的 120 个路径没有这三类类型/函数的字面使用。本轮据此保留其通用框架边界，不扩增截图 SDK 类型、Rust 对应或全量旧 API 测试；它们在验收 DLL 中仍仅编译、未运行验收。这不是任意宏/预处理展开、外部消费者或完整 P1 穷尽证明。逐文件类别、全部原始位点及源码副本由参考目录 `local/current-remaining-buffer-scope-review-path.txt` 指向；首次 reader 对 gitlink 和样例位点的错误前提分别保留。

确切 `02c7eb6` 的 Gamepad 合并对应现覆盖已有原始 CPP 索引的三十项及零函数体头文件：新增 UUID/features/info 三项 `capture_metadata`，按当前 Git 原文刷新同一 `gamepad.rs` 的全部位置；原 delegate/native/platform 三文件与旧对应字节相同。三项保留十进制 HWND、type 非 0 时 ds4、固定历史 feature 位、info 的无符号 HWND 与有符号 type/method；可失败独立 String、类型化分配错误及 JSON 文本对来源 object 的边界明确列出，不定案公共 SDK/ABI，也不声称任意 OOM 或连接后全场景由预连接矩阵验收。

另外十二项逐一登记为输入/应用范围排除，Rust 位置和执行证据均为空，不伪造实现。`ensure_foreground` 只有 `touch_down`、`key_down` 两个原始调用点，截图、停用、连接与查询不调用它；共享捕获侧 ForegroundUtils 仍按自己的来源迁移。start/stop app 是应用转发；touch/key 方法是轴、扳机、按钮控制；click/swipe/click_key 是来源的弃用 false 槽，input_text 是不支持的输入槽。它们均不因此成为 SDK 能力，析构所需宿主 reset 与截图 owner 寿命仍保留独立对应。三个外层公共 C Version/Create/Destroy 导出仍单列，未冒称 Rust SDK 导出。

只读 `reference function-map` 核对精确 F 字节、行/范围、当前 Rust Git 内容和全部声明运行组。历史 66 项原生状态仍为 60 通过/六失败；先前八项宿主模型日志及新增四项主配置查询/独立记录日志均通过、零本次新截图。汇总 72 通过/六失败是三类记录之和，不能写成新的 native 通过率。源索引三十项全覆盖也不等于预处理、宏、SDK 或编译器穷尽语义。封存记录由参考目录 `local/current-gamepad-metadata-functions-path.txt` 指向。

十六组最新清单已重新按来源路径和函数身份去重，累计 40 份来源/300 项；本次只增加三项实现对应和十二项排除。原始 44 文件索引中未进入该 Rust 对应并集的四份来源为 MaaBuffer.cpp、GamepadControlUnitAPI.cpp 及 KWin/WlRoots 兼容 API；原生保留的五项 buffer 操作和兼容创建已有各自边界审阅，不能改称完整 Rust 映射。已经出现的文件也可能保留条件分支或更广 SDK/宏范围，不能凭路径出现宣布完整 P1。去重记录在参考目录 `local/current-function-census-union.json`，旧并集已另存保全。

Gamepad 的截图相关外层在确切 `ae4e578` 补四项：原构造、析构、`connect` 和 `connected` 对应同一私有 `GamepadCapture`；已有截图/停用映射另追加外层直接转发位置，原 `CaptureDelegate`、原生 loader 和平台模块未复制或修改。四项保留标量选择器、空初态、宿主创建先于替换、连接失败保留旧捕获状态、可选截图初始化 false 仅警告、connected 短路，以及 reset→捕获 view/loader→宿主 owner 的退出顺序。reset 不得展开、原失效 view 拒绝和类型化边界错误均明确列为安全/接口边界；该对应批没有实现 ViGEm、输入、foreground、UUID/features/info 或公共 Gamepad SDK 导出。随后三项元数据的独立迁移与验证见下节，不重标该批旧映射。

新的合并对应为两份既有 F 来源、十五项，原文/blob/行号/范围摘要和实际 Rust Git 字节逐一通过只读 `reference function-map`。运行链接分开保存：原生历史批次的 66 项检查仍为 60 通过、六失败，mask `6/10/14/38/42/46` 不变；新增八项是本机 Debug/Release 完整原类/API 与 Rust 的 CPU Mat/宿主/DLL 边界模型及记录检查，全部通过、零实际截图和输入动作，不改称原生取帧。合并 reader 的 68/6 仅为两类记录的计数，不是新的 native 通过率。最终重建前后的 217 份 Gamepad 记录在每种配置均逐字节相同，539 份实现提交的 canonical tar 与全部 Git blob/mode 核对。最初外部索引 reader 的 key 错误、后续只读 SSH DNS 失败分别保存；远端身份改由 GitHub Ref API 核对，没有更改已保存结果或 Git 连接配置。选定累计增加四项至 40 份来源/285 项；其余十五项外层源码和三个 API 导出仍明确未映射。完整审阅由参考目录 `local/current-gamepad-host-functions-path.txt` 指向，不授予完整语义、SDK、截图或 P1 验收。

`source/Common/MaaBuffer.cpp` 现有独立的原生保留边界审阅：五项为 `MaaImageBufferSetRawData`、`MaaImageBufferResize`、`MaaImageBufferGetEncoded`、`MaaImageBufferGetEncodedSize` 和 `MaaImageBufferSetEncoded`。完整已登记 `custom-raw-source.inc` 与 F 原文件两段提取结果逐字节相同（显式处理 CRLF）；每个选定函数体只出现一次，实际 `bridge.cpp` 的 include、CMake 编译目标及各调用表达式也单独核对。它们是按计划保留在显式 C++/OpenCV 验收边界的原生操作，未新建 Rust 核心 codec/旧 C ABI，也没有被填进 Rust 库实现对应位置；累计 Rust 对应仍为 40 份来源、281 项。完整原文件的另 40 项函数逐项登记为本次未审阅，没有按输入排除或改称已迁移。

raw setter 保留 null handle/data 拒绝、packed `cv::Mat(height,width,type,data)`、空图拒绝、实际 `handle->set(img)` 后返回 true 的顺序；安全桥在它之前另核验 native type、受检跨度和可读长度，不增加源接口没有的 stride。resize 保留带符号尺寸和 null→0 的一次原生转发。encoded setter 保留先判 null 再判长度，因此 null+0 拒绝，非空地址+0 才 clear；非零长度先构造原始编码字节副本，再 move 到真实 buffer。两个 getter 保留借用原生 cache 的生命周期，实际桥先取 encoded 数据再取长度，在同一未修改 owner 的通道内核验并复制测试用 cache；不延长到 cache 修改之后，也不把诊断复制称为免除工作。任意指针、全部异常/分配/插值参数和完整 MaaUtils 实现均不由这些调用验收。

十二份当前相关库、生成器、C++ 桥、实际调用测试及记录校验器整文件均匹配历史原生登记输入，CRLF 差异显式记录。该完整 Custom 组件 18 项检查仍全部通过；重新以只读方式校核实际 raw 和 encoded/resize 矩阵：前者 1,664 种 native type、4,992 组、9,984 次真实调用/持帧及 9,662,208 字节，后者 144 组、288 次调用/持帧和 576 份 cache decode。两组核验前后的 9,196 份实际数据/报告文件摘要完全不变，没有重跑截图或修改历史状态。证据由参考目录 `local/current-custom-buffer-retained-bodies-path.txt` 指向；这关闭选定原生函数体与调用边界的身份核验，不代表完整 buffer/API、依赖、平台或 P1 通过。

`GamepadControlUnitMgr.cpp` 的截图委托切片补十一项：七项 `Win32ControlUnitLoader` 构造/析构/加载/卸载/创建/销毁/handle 状态、`init_win32_unit`、真实 shared_ptr no-op deleter、截图和停用；头文件作为零函数体的声明/成员寿命顺序来源一并纳入。Rust 位置仅为 `delegate.rs`、`delegate_native.rs` 和平台 `delegate.rs`，三份当前整文件均匹配已登记历史原生输入并显式核算 CRLF。实际生成的 loader/source/可选条件片段包含所选 F 字节，source/model/原生桥边界单独核对，不把完整 Gamepad 类或 ViGEm 输入写成已执行。

保留先构造新 loader 再替换旧 owner、缺模块/两个工厂导出/空工厂结果的短路、None mouse/keyboard、no-op view 和 connect 失败时先 view 后 loader 清理；截图/停用直接转发，不增加 connected/前台门禁。handle 对应是实际存储槽与 create/view/checked 的数据流，没有新造公共 Rust getter。Rust 独立 CPU 输出、失效 view 拒收、可失败路径/Arc、异常桥及销毁未证消费时保留模块均为明确安全适配；源直接 helper 误用、任意 DLL alias/析构失败/回调排空不据此验收。外层构造/析构/输入 connect/connected、foreground/input/app/UUID/features/info 等十九项及三个 Version/Create/Destroy API 导出明确未映射，未改标为已迁移；本批只覆盖实际截图路径。

只读 `reference function-map` 保留完整历史 `manager-delegate-*` 66 项检查，为 60 通过、六失败，mask `6/10/14/38/42/46` 继续失败。原 274 个脚本对照、80 个原生加载生命周期与 63 个 mask 各有原范围；不由模型/加载/销毁三项通过推断所有取帧通过。外部 reader 初版错误地要求全部 66 项通过，已在执行记录校核前拒收并保留；修正仅记录实际 60/6，没有降低 native/checker 门禁。累计选定对应为 40 份来源、281 项，没有新捕获或改写既有无输入多轮结论，完整 P1 未通过。

另从同一已封存原生批次复核九个 mask 首轮三次输出的 source/Rust，共 54 份目标 CPU 图；每份原始完整图像先匹配原生登记摘要，再按已记录几何提取实际 96×64 目标。`4/8/12` 的这些样本均为预期图样；`6/10/14` 部分样本各差 924 像素，均位于目标外三像素内；`38/42/46` 的首轮样本恰好没有差异，但其完整原生案例仍失败。没有修改输出、像素 mask 或原断言，也未证明差异像素的生产者；局部首轮图样不代替完整二十生命周期结果。

后续重新从 54 份已登记原始完整 native 文件提取目标，核对前次保存字节后，六份有差异图像均精确分为外两像素的 624 个亮组点和紧邻内一像素的 300 个暗组点，其余 5,220 点未变；六份图只有六种实际颜色/预期颜色配对。对这些配对做独立数学拟合，场景透过系数约 0.249858，亮/暗前景的推算 BGR 分别约为 `[0.128,221.538,254.751]` / `[0.128,0.247,0.132]`，最大通道残差约 0.424；矩形二加一层模型的分类错误为零，简单周期条纹模型仍有 184 点错误。这里的近似 75% 覆盖、前景颜色和几何只是保存字节的描述模型，未测得任何原生 alpha/compositor 参数，也未认定 WGC、设备或其他具体生产者；须与新的原生属性/设备对照各自比对。证据由参考目录 `local/current-native-perimeter-pattern-path.txt` 指向；没有遮罩、修改图像、隐藏指示器、重跑原生捕获或改写完整案例状态。

PipeWire 原 36 项 F helper/源类对应另有当前完整 owner 版本：八份 Rust 库源码逐一以确切 Git 字节匹配 `e987a99` 原生登记摘要，旧函数范围/函数体重新核对，并显式补入 `Capture` 的构造、初始化、捕获、停用、connected、关闭/析构及 at-rest 边界。只读 `reference function-map` 保留本次全部 201 项原生通过记录；fd41/a5 的旧映射和失败记录不改写。本批只刷新 PipeWire 源类职责，不自动刷新其他 manager/factory 的独立快照，也不增加累计来源/函数数量；仍为 38 份、270 项。

当前 whole owner 关闭配对 raw 资源与 Consumer counter reset；断连 init 在来源 stale close 前重置计数，已 connected 的早返回保留计数。独占调用完整结束后才可迁移，裸 Driver/Resources/map/lock 不外露，opaque Mat/loader 留在调用线程；只有完整私有 owner 有受限 `Send`，不增加 `Sync`、并发或 SDK 调度承诺。实际 80 组串行交接/720 份持帧及 40 次 Rust 展开控制与稳定 Box 地址、实际 loop 锁和 stop/join 顺序共同支持该边界，单独稳定地址或一次成功测试不等于一般线程安全证明。

新映射同时用实际八例 POD 证据替换旧待证假设：标准 media/raw 观察均返回 2，原 F 的 `==0` 门限进入手动循环并保留末项属性，Rust 八项结果一致。生产解析器未改，早先疑似 unsupported-standard 回落差异没有被该实际矩阵复现；不扩大为所有格式/损坏 POD 等价。八份源码是匹配原生摘要后的独立 Git supplement，轻量包仍有 609 项完整输入缺口；不由当前映射、201 项通过或有限交接推断完整来源闭合、任意 FD/DMA/组合清理故障或完整 P1。

`ScreenCaptureKitScreencap.mm/.h` 新增十项对应：四个普通函数/方法、头文件构造与默认析构、窗口/显示器各两个真实嵌套 Objective-C++ block。确切 `1469603` 的原生 Clang AST 已从官方完整工程包取回，三个 namespace 根、六项普通声明/定义和四个 block 的名称、范围、原始函数体字节均匹配固定 F；默认析构的 synthetic CompoundStmt 单独识别，未虚构 `{}` 源函数体。新 census 使用原始 F 源字节、人工分类及独立 AST 位置佐证，未改写旧 C++ grammar 的错误记录，也未宣称审阅完整 SDK/macro/inline 展开。

Rust 对应直接绑定 `screen_capture_kit.rs` 和 `pixel_buffer.rs` 的实际库函数及 RcBlock 闭包，两份整文件均匹配本次原生完整包登记字节，显式核算 CRLF。保持非零 window ID 的首个精确匹配、零 ID 的 `displays[0]`，以及 BGRA、sRGB、关闭 cursor、先提交 content 再等待原生五秒 semaphore；内部合并两个内容/样本回调路径，不复制后端。样本转换仍在 CVPixelBuffer lock 内完成，先释放 filter/config 的初始 create 所有权再 signal，复制 block 的独立引用按实际回调通道释放。源 MRC 未释放的 semaphore create +1 继续明确保留；仅测试在全部调用/回调拥有者结束后回收，不能写成产品清理成功。

Rust 独立 Arc 请求结果、晚到转换/状态、受检尺寸/布局与原生 lock 失败拒收、CPU target 行复制和 callback panic/error 边界均为明确安全适配，额外复制不冒称省掉转换。只读 `reference function-map` 保留本次完整 154 项组件检查（133 通过、21 失败）及其日志，不能由源码对应、constructor/default drop 或稳定 Arc 地址推断回调排空、严格已知像素或 SDK 关闭/线程保证。累计选定对应为 38 份来源、270 项；本批没有新捕获，不改既有无输入多轮结论，完整 P1 仍未通过。

新增 `FramePoolScreencap.h` 可用分支的内联构造对应：保留借用 HWND、空 COM/projection/device/session/texture 槽、零 descriptor/last size 和空图像；构造不提前激活、分配原生资源、调整窗口或取帧。Rust 的线程标记为额外安全限制，后续初始化/销毁仍由已审阅 CPP 方法对应承担。只读 `reference function-map` 核对本项和完整所选历史 45 项检查（39 通过、六失败）；包含构造的当前 Rust 整文件与登记原生输入一致，并显式核算 CRLF。本批没有新原生捕获或更改既有无输入多轮结论。

同批纳入 `SafeDXGI.hpp`，其内容仅为 `SafeWindows.hpp`、`d3d11.h`、`dxgi1_2.h` 三个 include，没有函数体或自定义资源 wrapper；不凭文件名虚构 RAII 职责，也不把 SDK/COM 宏与 inline 展开写成已审阅。头文件旧 SDK 分支的构造、默认析构和空捕获三项明确保持未映射，不改标为应用/input 排除，不推断已实现或验证旧 SDK 构建。累计选定对应为 36 份来源、260 项；此处只增加一项可用分支构造，不是完整头文件、后端或 P1 验收。

macOS 的普通 C++ grammar 虽已找到四个 `.mm` 方法，却不能可靠索引 Objective-C++ 消息及四个嵌套 block。新增 `cargo xtask reference macos-ast <compile_commands.json> <fixed F root> <new external output>`：仅在原生 macOS arm64 运行，从实际 CMake 数据库唯一选取 `mfs_macos_source_context` 对确切 F `.mm` 的普通编译，核对头文件/源码完整摘要及已选择的 Xcode compiler。保留 compiler、SDK、宏、include、架构和部署目标，只移除 `-c` 与 `-o`/object，增加 syntax-only/JSON AST/filter 参数；结构化调用不经过 shell，并拒收 response file、额外输出、歧义编译及需 shell 展开的命令。

该观察在原生来源准备中独立运行，原始 AST、stderr、退出码、完整命令、compiler 版本/摘要、编译数据库及两份未改来源均立即外置登记。观察失败仍保留 failed 检查并继续独立捕获，不把缺失 AST 或成功语法编译计为像素/回调排空通过。JSON AST 随实际 compiler 归档；过滤根及 block 节点数只是版本相关的语法观察，含声明/重复引用的计数不能作为完整函数 census。后续仍须逐项核对真实 token/函数体范围、默认析构及嵌套 block 的源字节与 Rust 职责；当前累计对应数量不增加。归档依赖重放通过 workspace（118 项 xtask）、Release、Clippy、参数保留/拒收及 Windows 主机拒收控制；三个真实 Step 子进程另验证成功、输出前失败和输出后失败均继续编排，失败输出完整登记。初轮外部驱动的路径拼接错误单独保留。Windows Clang 的十项合成 callable/block 语法控制仅验证语法形态，实际 Apple/F AST 执行仍待 Actions。

`PipeWireScreencap.cpp/.h` 新增 36 项对应：34 项 CPP helper/方法/谓词和两项头文件声明，合计含六项删除的复制构造/赋值及两个等待谓词。库内位置覆盖实际 consumer、native、POD、CPU buffer、resources、MemFd 和 DMA 七份 Rust；其当前字节均匹配旧 `a5` 登记原生输入，显式核算 CRLF。实际 `reference function-map` 保留完整所选 101 项历史通过检查与确切日志，未把旧正常格式通过扩大为新分支、任意故障或完整后端验收；累计 34 份固定来源、259 项。

审阅保留实际 loop 锁及 guard 退出、相对整秒等待/三秒预算、原谓词及错误后重查；捕获激活流、设置 wanted，超时先查断连、计数、空缓存，再判断大于三次，前三次回放。快照/格式在锁内取得，OpenCV 转换仍在解锁后，BGR 保留无转换入口。初始化保留 daemon/显式 unsafe FD、成功 core 后清 FD、先启动 loop 后锁内 connect 和三份 POD 顺序；后续重建按原 daemon 状态走，不新增恢复。process 排空保留最新、及时归还前项，仅 wanted 时复制，先结束 map 再清 wanted/signal/归还；core/state 回调不增加新唤醒或自动重连。

映射同时单列差异：布局/FD/fstat/mapoffset/READABLE/单 plane 验证、可失败分配、稳定 Box callback/hook、panic 边界及错误标量均为安全适配。MemFd 保留 private/offset0，DMA 保留 shared/stride×height、一次 START/END 及 START 失败仍保留已映射地址的来源行为，不提前实施 P3 重试或宣称失败同步后的 CPU 一致性。Rust 仅在独占 Arc 存储时复用，源 Mat 可能覆盖共享 BGR 输出的别名差异明确保留；独立 CPU 拷贝不冒称消除了转换/分配。stop 必须在锁外，随后按 stream hook/stream、core hook/core、context、loop 清理，再清状态；timeout counter 位于单独 Consumer，由组合 owner 配对 reset，不能把单独 `Native::close` 写成已重置整个源类状态。原每构造 raw `pw_init` 的未配对全局引用没有顺便修复，fixture 回收另作边界。

CPP grammar 对 `SPA_POD_OBJECT_FOREACH` 的一处宏循环不能直接索引；仅在独立 parser 输入中用同字节长度的循环占位，保存全部替换字节、原错误、原函数/闭包范围和摘要，实际 F 来源及可执行程序未改，不声称编译器展开或宏语义验收。头文件明确允许捕获线程与 init/close 线程不同、但由调用方串行化；当前 Rust `Resources` 的 `PhantomData<Rc<()>>` 及 `Native` 可持有的裸 lock 状态尚不满足这个迁移边界。稳定地址、调用串行及旧原生通过都不能证明 Send，本批不加 unsafe Send。新八例分支夹具的首例错误前提及独立修正见[本轮复核](#本轮原生与-ci-复核)，生产解析器未改，完整八例仍待新 CI。两次外部 reader/图校验拒收分别保留，最终只纠正真实 adapter 路径和静态函数回调的类型标签，没有放宽校验器。本批没有新捕获或改变既有本机无输入多轮结论，完整 P1 仍未通过。

`NoneScreencap.h` 的默认构造、默认虚析构和内联 `screencap` 新增三项对应。Rust 仍由原 manager owner 保存 `Unit::None`，不创建 PipeWire/Wlr 原生资源；捕获返回 false，不赋值调用方图像、不进入 converter、不轮询或标记 replay。输入构造失败后 retained None 与 connected=false 并存的状态保留；drop 仅保存私有标量 trace。Box/Arc 的可失败分配及 trace 是明确的 Rust 适配，不代表原类 ABI、任意析构故障或进程终止式 OOM 可恢复。

实际 `reference function-map` 只读核对三项、两个库内 Rust 位置族及一个旧 `a5` Linux 检查；其日志确有四项测试通过，覆盖 None 输出不变、零 converter 调用和失败输入后的保留。两份库源码、真实 Rust 测试与原 manager model 共四份来源匹配已登记原生输入，显式核算 CRLF。原 model 的 None 是脚本替身，不能据此声称执行了确切原版头文件；基类继承方法、宏/虚表及完整原生等价性仍待验收。累计 32 份固定来源、223 项，没有新编译、截图或输入监听，不改变既有本机连续多轮结论。

同批另审阅 Wlr/KWin 两个兼容创建函数：Wlr 只拒绝 null，空或首字节 NUL 仍构造；KWin 拒绝空设备或非正尺寸，保留来源方法/输入元数据与 create→open→fd→node→destroy 顺序、失败 open 的 helper 残留。它们仍是已有 Config/宿主边界，无真实 Portal 授权或输入动作。两个原版 GetVersion 返回 `MAA_VERSION`，两个 Destroy 各保留自身 null 分支并 delete 原 handle；当前没有对应 Rust 对外版本/销毁 ABI。本批单独保存这个未完成边界，不将这两份来源、四个 ABI 函数计为全文件迁移，不借用无关构造函数或排除输入标签填补对应。完整 SDK ABI 按后续阶段定案；完整 P1 仍未通过。

`MacOSControlUnitMgr.cpp/.h` 新增 22 项对应，包含默认析构声明：九项捕获/生命周期/元数据边界、十三项排除的应用/输入动作。Rust 范围明确为 `Input=None`，没有接收非 None 输入后改写为零。连接先清 connected，精确 ScreenCaptureKit selector 通过真实 macOS 14 可用性检查后才构造；新 owner 构造完成后替换旧 owner，不可用或构造失败仍保留旧对象。None/未知截图 selector 继续原日志后成功连接。截图转发只要求现存 owner，不检查 connected；None/错误保留调用方图像，engaged empty 也照常替换。inactive 仍直接返回 true，不调用后端或改变状态；UUID 保留 u32 十进制，features 在 None 输入下为零，metadata 保留 window_id 与 screenshot selector 的 signed64 解释。默认析构按源成员顺序先释放截图 owner，None 输入没有动作或输入 owner；不据此推导原生 callback 已排空。

实际 `reference function-map` 只读核对全部 22 项/排除 13 项，并绑定完整的十项既有 `bc90d12` manager 构建、依赖、控制、查询、元数据、脚本、原版/Rust 捕获及场景退出检查；这些通过状态均为历史结果。本批两份库内 Rust 与十一份真实测试/适配上下文共十三份源码匹配已登记原生输入，显式核算 CRLF，测试文件不充当库实现位置。源 CPP 唯一的 `__builtin_available(macOS 14.0, *)` 参数超出所用 C++ grammar；独立索引输入使用同长度占位并保存全部替换字节、原错误和解析输入，原函数边界/行号/摘要逐项不变，实际来源和运行时条件没有改写。这是封存 parser 的语法索引重放，未进行编译器预处理，也不授予完整 Objective-C++/宏解析或语义验收。累计 31 份固定来源、220 项；内层 ScreenCaptureKit、非 None 输入、完整框架/ABI、既有像素失败和完整 P1 仍待处理。本批没有新捕获，不改变本机无输入多轮结论。

`DesktopDupScreencap.cpp/.h` 与 `DesktopDupWindowScreencap.cpp/.h` 新增二十四项对应，包含六个清理闭包、两个构造和一个默认派生析构。保留默认硬件 device 后建原 `IDXGIFactory`、nonnull HWND 的 nearest-monitor 查找、找不到时 adapter0/output0 回退，以及使用原 device 的 `DuplicateOutput`；没有为跨 adapter 成功而换设备。每次外层捕获仍检查输出，首用至多三次仅检查末像素 alpha 的预热，此后必须另取一帧返回；三次预热都失败时仍保留初始化标记，没有增加缓存、回放或最终失败后的重试。`AcquireNextFrame` 仍等待 2000ms；access-lost 只清 duplication/staging 和 monitor，输出切换、完整 uninit 的释放顺序与残留状态分别保留，未合成一个新恢复事务。

六个闭包逐项对应枚举 adapter/output 的局部释放、primary 临时 output 的 QI 引用，以及 Unmap、raw texture Release、ReleaseFrame、desktop resource Release。成功 Acquire 才建立 ReleaseFrame 责任，成功 Map 才建立 Unmap 责任；失败时不释放别的调用拥有的帧。原版返回借用已 Unmap 内存的 Mat；Rust 在 Map 有效时复制独立 BGRA 行，仍等原清理结束后才转换 BGR，明确保留增加的整幅复制工作。窗口包装仍先完成整屏转换，再查询 client/origin/output 与检查 ROI；原共享 BGR ROI 改为隔离目标 BGR 复制，是额外的所有权安全适配，不称作消除转换/复制。后续 descriptor 兼容性、布局/stride/checked 算术、null-success 拒收和 caller-thread owner 同样保留其安全适配身份，不宣称无效 native 行为等价。

实际 `reference function-map` 只读通过四份固定 F 来源、二十四项及完整旧 `58fa6f0` 的二十三项 `desktop-dup-*` 检查身份。四份库内 Rust 与原生登记快照匹配并显式核算 checkout CRLF；生成声明入口、真实 `SystemWindow` 转发、转换/裁剪桥和生成输入另存 Git 摘要作为边界上下文。没有新编译、截图或输入监听，也不改变本机无输入多轮结论。累计二十九份来源、198 项；其余十五份原 partial 来源、生成 COM/SDK/宏展开、任意 device/access-loss、多 adapter/显示器变化、原生失败及完整 P1 仍未验收。

`CustomControlUnitMgr.cpp/.h` 新增二十四项对应，其中十项涵盖截图、连接/退出边界、UUID/features/info 与默认析构，十四项应用启动/停止、输入动作和 shell 明确排除。保留当前借用的 callback 表与 user argument、缺 connect 为 false、缺 connected/inactive 为 true、缺 features 为零，以及无 connected gate 的截图/查询。截图和 UUID 使用真实 C++ 栈缓冲：缺回调不构造，false/错误保留调用方输出，true 包括 empty 在缓冲结束前发布真实 Mat/string owner。UUID 保留完整字节与 u64 features；metadata 保留先建默认对象、在 StringBuffer 内调用实际 native JSON parser/as_object 拷贝、退出 helper 缓冲后再覆盖 type=custom 的顺序。没有给核心增加 OpenCV/JSON 依赖或冻结 provider/JSON/能力 ABI；适配中的额外 owner head、独立字节复制、分配/异常安全适配仍按原范围记录。

实际 `reference function-map` 核对二十四项（排除十四项）及完整旧 `58fa6f0` 的十八项 `custom-*` 通过身份，没有新截图。库内对应只指向 `src/backends/custom.rs`；四份真正调用 native 的 Rust 验收模块、五份 C++/声明/构建上下文单独保存边界跨度、Git 摘要和十项已恢复原生登记来源，显式核算 CRLF。初次将测试适配器写进库实现 site，被现有门禁拒绝；原数据/日志保留，新数据集改为独立边界记录，未放宽校验器。记录追到实际 ImageBuffer/StringBuffer、Mat/string/JSON owner 和 C++ 调用方发布，不以 generic trait 或 mock 代替 native 证明；适配器本身仍没有完整逐函数/ABI 验收。累计二十五份来源、174 项；任意并发改表、线程迁移、销毁/重入和全部分配故障、其余来源/宏及完整 P1 仍未验收。

`FramePoolScreencap.cpp` 新增十项函数/清理闭包对应，直接连接当前 `frame_pool.rs`、`wgc.rs` 与 `wgc_factory.rs`。保留首用硬件 device/swapchain 初始化、原 `Create`（BGRA、一个 buffer）、三次初始 Size 查询及各自 catch 边界、border/cursor/secondary 顺序、StartCapture 后最后记录尺寸；没有改成 CreateFreeThreaded。取帧仍先 Close 排空旧帧，再以两秒 steady-clock 预算按 2ms 睡眠轮询，超时按原位置 clone 缓存；当前帧只按源引用寿命释放。Map 后保持客户区、原点、DWM bounds/GetWindowRect 回退和夹取顺序，转换及缓存 clone 均先于 Unmap。Rust 的 checked ROI/stride/ContentSize 拒收与独立目标像素复制为明确的安全适配，空 ROI 清缓存并以内部 EmptySample 表达；不把无效来源行为写作等价，也不把复制或转换写作消除工作。

清理保留 session Close、pool Close、texture/item/尺寸清空顺序，uninit 仍保留 cache/device/context/swapchain；尺寸变更重建失败后的部分状态留给下次调用，没有加回滚。原 border 五秒等待用独立 event/delegate 状态适配，晚到回调只持有 event/status/error；超时不表示取消、注销或排空。实际只读 `reference function-map` 校核十项与完整 `58fa6f0` 的 `frame-pool-*` 分组，保留 39 项通过及六项失败；三份当前 Rust 与已恢复原生快照的登记身份匹配并显式核算 CRLF，显式 C++/OpenCV 转换桥源码另作摘要绑定，没有新编译或截图。累计二十三份来源、150 项。本批只覆盖 cpp 的可用实现，头文件构造/旧 SDK stub 四个条件函数体及完整 SDK inline/宏展开尚未纳入清单；原生像素、callback 排空、并发/故障与完整 P1 仍未验收。

`PseudoMinimizeHelper.cpp/.h`、`PrintWindowWithPseudoMinimizeScreencap.h` 和 `FramePoolWithPseudoMinimizeScreencap.h` 新增十九项对应，含两条 deleted 复制声明和两条 default 析构。保留原 HWND 非拥有身份、style 的零值/last-error 区分、仅原 layered 窗口查询 alpha 与 255 回退、running 置位后启动线程，以及同步 IsWindow/IsIconic 后再 apply。apply 仍按读取/添加 LAYERED|TRANSPARENT、alpha0、SW_SHOWNOACTIVATE、pseudo=true 的顺序执行；中途失败不回滚先前副作用。revert 先恢复完整原 style，再按原 layered 标志恢复 alpha，任一步失败仍继续并清 pseudo；原 color key/flags 仍不恢复，两侧均使用 key0/LWA_ALPHA。

工作线程保留 100ms 周期、目标失效时只清 pseudo 不清 running，以及只在同一 HWND 成为前台时恢复的行为；stop 先清 running、等待 join 完成，再恢复及按需 SW_MINIMIZE。inactive 只恢复，不重新最小化或停止内层捕获。两种包装保留构造顺序、捕获前 helper 调用及 helper 先于 inner 的析构边界，FramePool 显式 take/drop helper 后才释放 pool/session/device/cache。Rust 的单一 !Send/!Sync owner、私有 Arc、短 Mutex、fallible allocation、线程创建错误、poison 恢复及固定错误/worker-panic 证据属于明确的安全适配，不宣称等价于源异常、进程中止 OOM 或强制取消。没有增加画面就绪等待或跨窗口协调器。

实际 `reference function-map` 只读核对十九项、三份当前 Rust、两个已恢复原生快照中的六项登记来源身份，显式核算 CRLF；完整选定分组保留五项通过及四项 FramePool 原版 pseudo stop/inactive/layered 失败，没有重跑捕获或改写本机无输入多轮结论。累计二十二份固定来源、140 项；本批只审阅 helper/包装，未验收内层 FramePool 捕获、并发恢复、callback 排空、其余来源/宏或完整 P1。

`ForegroundUtils.h`、`HwndUtils.hpp` 的七个函数体已逐项审阅，累计十八份固定 F 来源、121 项。前台路径保留跨窗口共享的 5000ms DWORD 冷却、零哨兵、回绕减法、先更新再请求，以及 TOP/前台/TOPMOST/NOTOPMOST 调用和 5/10/5ms 等待顺序。Rust `AtomicU32` 消除原共享静态变量的数据竞争，没有把整个 native 请求序列串行化。几何路径保留默认 `GetDpiForWindow/96.0`、查询失败的零尺寸、仅检查 WS_POPUP 的辅助判据、rcMonitor 精确客户区匹配后的一次复核、变化后刷新几何、右下再左上的 rcWork 校正及原异步 flags；`SetWindowPos` 成功仍只是请求已提交，不表示 UI 队列已消费或外国窗口快照原子化。

实际只读 `reference function-map` 通过七项与十一项既有 `58fa6f0` 检查身份，四份当前 Rust 与两个原生产品快照共八项登记来源摘要匹配，显式核算 Windows checkout CRLF；没有新编译或截图。映射追到 `SystemWindow` 的真实 Win32 转发。坐标/尺寸 checked 算术属于既有安全拒收，未宣称等价于原有溢出行为；`MAA_WIN32_COMPATIBLE` 的备用缩放编译分支未实现或验证。`bgra_to_bgr` 的非空路径仍在原时点进入显式 C++/OpenCV 验收适配，Rust 桥接、native 转换与独立 owner 的源码另作摘要绑定；原工具的空 Mat 分支没有独立 Rust 工具等价性或直接空输入验收。初次外部 reader 误引用不存在的 owner 文件，在完成前停止；旧输入和错误保留，随后引用实际包含 owner 的适配文件与头，产品和原生记录未改。Pseudo-minimize、其余来源与宏、历史无输入多轮失败及完整 P1 仍保留各自未完成范围。

窗口 GDI、PrintWindow 与 ScreenDC 的六份 `.cpp/.h` 现审阅十三个函数、闭包和默认析构声明，其中三个清理闭包、三个默认析构单列。对应表追到实际 `SystemGdi` 转发，涵盖 DC/bitmap 获取与释放、`BitBlt`、`GetBitmapBits`、`PrintWindow`、`GetDIBits` 及 popup 查询。保留转换先于清理、旧选中对象恢复顺序、主图/popup 两次捕获后再查询原点和叠加、普通 popup 原生失败保留主图，以及 ScreenDC 的非致命前台/窗口调整、未消费的窗口矩形查询和空 HWND 拒绝。checked layout/完整读回拒收及 GetDIBits 的临时取消选中/恢复选中明确为既有安全适配，不宣称无效来源行为等价或提前现代化为 P3 DIB 路径。

真实只读 `reference function-map` 校核十三项与完整选定运行时分组，通过的十一项均为旧 `58fa6f0` 原生记录，没有新截图；已有本机无输入多轮和其他组件失败不改。九份当前 Rust Git 源码与两个已恢复原生产品快照的登记摘要另作核对。初次直接字节比较在 `foreground.rs` 因 Windows checkout CRLF 失败，随后仅显式核算 CRLF/LF，并分别保存实际与 canonical 摘要，恢复文件未改；文件存在不代表所有 cfg/分支已执行。调用到的 foreground/HwndUtils/pseudo-minimize 等 helper 函数体仍需各自审阅。累计十六份来源、114 项不构成完整语义、原生、宏展开或 P1 验收。

`MemfdBuffer.cpp/.h` 现新增八项对应：七项覆盖捕获构造、创建、析构、指针/长度/fd/可用性访问；单参数构造函数在固定 F 中仅供 `WlrInput` 的两份 keymap 使用，依据完整 Git 调用点列为排除，未迁移输入行为。捕获路径使用三参数构造，当前 Rust 对应 `ShmMemory` 三态、checked layout、私有访问器、失败对象的保存及代理/映射/fd 释放顺序；映射失败与截断失败的资源差别、fd=0 残留和 UUID 构造前失败均单列。无效指针/fd 不在 Rust 中伪造，原 `close(-1)` 的 errno 副作用也未宣称等价。对应源码在 `f261cbf` 与测试修正后的 `7b82fe8` 中相同；随后 mmap 原生回归已通过，见[本轮复核](#本轮原生与-ci-复核)，其他分配错误与组合失败仍待验证。

八项清单经实际 `reference function-map` 只读校验，绑定两个精确 F 文件、当前 Rust 字节范围和旧 `c564306` 的 60 项检查身份，保留 57 项通过及三项真实失败，没有新捕获。首次路径错误把不含 `runs/` 的轻量包根用于完整日志身份，旧输入与错误日志已保留；随后改为已封存的完整选择性恢复根，源字节、状态和像素未改，校验通过。此批未修改校验器或重复通用坏数据测试。连同既有 manager、Wayland/Wlr 审阅，共十份来源、101 项；旧四文件对应仍绑定修复前快照，这份补充只覆盖当前内存 owner，不扩大为完整当前后端、宏展开或 P1 验收。

P1 的职责盘点已进入函数层级。首批按固定 F 的 `Win32ControlUnitMgr.cpp/.h` 审阅 36 个函数和闭包：21 项属于捕获、元数据、生命周期及 Input=None 边界，15 项属于计划排除的输入或应用操作。构造/连接、六种候选创建、预热与计时选择、输出转发、inactive 和四个查询均列出对应 Rust 位置；嵌套的八个闭包单独登记。记录明确保留背景键盘等排除边界、默认非 `MAA_WIN32_COMPATIBLE` 的参考配置以及未完成的原生范围。63 个既有运行时检查的 55 项通过、8 项失败按实际状态引用，未改写为新执行或阶段通过。

已实现的只读入口为 `cargo xtask reference function-map <reference 相对映射文件> [--root <外部目录>]`，要求外部目录已存在。它核对独立清单逐项覆盖、F Git 文件/字节摘要、函数范围与行号、闭包父项、Rust Git 快照及原生 manifest/check/log 身份；拒绝遗漏、重复、将捕获条目改作排除、重算摘要后的源码替换、错误引用和虚构通过状态。原生检查仍失败时可完整审阅其记录。函数清单的穷尽性、人工语义说明和原生行为仍分别需要相应证据；此命令固定输出语义、原生及完整 P1 未验收，不替代全清单函数审阅和实际对照。

本批独立语法索引绑定 44 个既有 partial 来源文件和当前 Rust 快照；仅在解析输入中把独立 namespace 宏行置为空白，保持原始偏移、行与源字节摘要。其余三份来源仍有宏/Objective-C++ 语法恢复，首批函数清单只选用已人工复核且无解析错误的两个 manager 文件。解析器、wheel 与来源快照均保存在外部工程记录。初版索引的 Point 行属性访问发生 native access violation，最小探针和失败程序已保留，后续按原始字节和换行计算位置；初次摘要格式化编译错误也保留，修正后验证通过。

真实 36 项映射及 29 组损坏数据控制通过，其中包括缺失目录不创建、错误 fork 基线、重复 namespace 标记、错误字节/行、闭包父项缺失或不包围、路径逃逸、伪造语义/原生通过及原生失败状态替换。独立 runtime 分组还强制保留所有实际检查，拒绝省略失败或整个运行时清单、重叠分组与没有匹配的前缀；闭包必须有严格包围它的父项。Workspace（含 xtask 101 项）、Windows 完整 feature Clippy、Release 核心和格式检查通过；沿归档 Cargo 解析开发，官方 nightly 更新检查确认现有版本。初次镜像 DNS 失败及归档配置未传入子命令时的 registry 优先级失败均保留，显式传入配置后通过。既有本机无输入多轮结论、原生像素失败和 P1 顺序门禁保持原样。

随后核对固定 F 的 `LinuxControlUnitMgr.cpp/.h`：26 个函数中，12 项属于捕获、元数据、生命周期及输入构造边界，14 项属于排除的输入或应用操作。Rust 对应位置覆盖连接时先清理 capture/input、失败 client 保留、capture 先于 input 构造与初始化、PipeWire 的 `fd<0 && node==0` 判定、原 UUID/config 字节与宽度、无 connected 检查的输出转发，以及 client/capture/input 释放顺序。None 原生工厂和显式宿主输入构造不代表输入动作迁移，ExtImage 在 F 中仍不能创建捕获器。原 Wlr/PipeWire 和模型的 16 项既有通过检查按原 manifest/log 引用，没有新截图、完整 manager 或 P1 验收。

该头文件只有声明、没有函数体；校验器现允许它作为独立的零函数来源，并照常核对 F 字节、blob 和 namespace 标记，但整份清单仍须至少包含一个函数体。真实 Linux 26 项和旧 Windows 36 项均通过只读入口，六组损坏记录被拒绝，涵盖整份无函数体、重复/错误解析的声明头、重算摘要后的头文件替换、漏掉运行时检查和改作排除。Workspace 的 102 项 xtask 测试、Windows feature Clippy、格式与文档检查通过；仍使用前批封存的 Cargo/vendor 输入。两个 manager 家族共审阅四份来源、62 个函数/闭包；其余来源家族、宏/Objective-C++ 解析限制和既有原生失败继续待处理。

WaylandClient/WlrScreencap 的四份 `.cpp/.h` 随后补入 31 项对应记录，含九个 registry/frame 闭包、两条 deleted 复制声明及一个 default 析构。28 项归入捕获/生命周期/元数据边界，三个只供输入使用的 getter 单独排除；捕获默认仍绑定并保留原来必需的虚拟键盘/指针全局对象。记录逐项覆盖原路径比较及 NUL 语义、原 prerequisites 的 AND 判定、最后公告输出与构造快照、忽略 global_remove、回调内递归 roundtrip、六种转换、原 packed step/忽略 YInvert、Buffer 到达即 Copy、Ready/Failed 及转换结束前帧保活。deleted 声明对应 Rust Client 所有权类型和显式 Arc 分享点，属于声明审阅。

该对应记录的两份 Rust 主体与已封存 `a5cfa2e` 和 `c564306` 的 Git 内容相同，49 项既有构建/模型/原生/记录检查按实际通过状态引用。真实清单通过只读入口，15 组损坏数据被拒绝；没有新截图。记录明确保留当时待测的 mmap fd 寿命、旧缓冲清理失败叠加新分配失败、共享快照下原版立即断开，以及 Rust 保留原通道的安全适配；不会将前缀干预或源行为中的 stride/方向缺陷改作产品正确性。随后 mmap 差异已原生复现并进入修正，见下节；旧记录仍对应 `c564306`，没有直接推广到新 Rust owner。累计八份来源、93 项历史函数/闭包及声明对应已登记；其余来源家族、当前实现的对应更新、原生失败及完整 P1 继续待完成。

## 本轮原生与 CI 复核

DesktopDup 初始化现有独立失败/清理矩阵：完整未改 F 类只在 `D3D11CreateDevice` 和 `CreateDXGIFactory` 两个创建边界返回受控 HRESULT；同一 Rust 后端通过仅测试配置编入的调用方局部 hook 处理同样四种失败。设备边界使用 `E_FAIL`/`E_OUTOFMEMORY`，工厂边界在真实 hardware device/context 已创建并登记析构通知后使用 `E_FAIL`/`E_NOINTERFACE`。每侧另有二十次直接、无故障转发初始化对照，不调用截图；不是无 hook 的运行对照，也未模拟真实 OOM/物理设备故障。默认库不编入干预，两个原生调用表达式和 `init` 之外整份 Rust 捕获代码与前一提交相同，没有添加 WARP 或改变默认值。

本机明确归档离线依赖的 Debug/Release 各通过原版/Rust/记录三项：每配置共 160 次公开初始化拒绝、四十次直接初始化对照、120 次真实设备创建、四十次真实工厂创建，以及 160 次部分初始化 device/context 的真实 D3D 析构通知。公开失败返回时资源槽均为零；直接初始化对照为七位掩码中的 `7→0`。通知在 live 阶段为零，返回后首次读取及最终观察均完成；本批通知时间也均不晚于调用后的记录时钟，但不推广为任意 GPU 工作的同步排空保证。原版公开入口仍只返回 optional/bool，记录中的 HRESULT 来自受控边界；Rust 另校核实际错误名称/码。源类零取帧由未改的初始化失败早返回路径支持，Rust 还实际记录零 sample/转换；没有桌面截图或输入有效批次声明。

`cargo xtask upstream-trace desktop-initialization <外部矩阵> [--check-only]` 使用记录布局二，核对六份原始 ABI/调用/寿命文件、参数违规标记、200 行调用与 480 行寿命；旧开发布局保留原程序/校验器，不重写成新布局。41 个实际数据控制接受四项、拒绝 37 项；25 个真实 Step 输出控制、四组失败后独立继续及三个实际原版/Rust/checker 子进程通过。原类宏入口、Debug/Release CL 命令/读取日志、确切 F 字节、程序前后摘要与 635 个编译器读取路径另行复核；这些路径摘要是执行后的来源审阅，不冒称执行前依赖冻结。工厂析构通知、失败同时写出非空 out 参数、任意创建故障、成功取帧和完整 P1 均未验收，旧完整函数映射保留其快照身份。

首次外部执行遗漏 `mfs_reference_utils.dll`，两个原版进程均以 `0xc0000135` 停在加载，独立 Rust 通过、配对记录拒收；原程序/输出/退出码保持保存。补齐实际 DLL 后另建目录通过。原始布局一数据、一次 Clippy 固定 chunk lint 及外部脚本定位前提错误也各自保留；最终数据显式登记违规标记，Clippy 使用受检 `as_chunks::<5>()`，未抑制 lint 或降低断言。原版/Rust/记录的独立 CI 入口、聚合跳过和完整 initialization 目录上传已接入，托管执行待验证。默认工作区、选定 all-target Clippy、直接库检查和格式均通过；开发/来源审阅由参考目录 `local/current-desktop-initialization-development-path.txt` 和 `local/current-desktop-initialization-source-review-path.txt` 指向，不声称新在线最新依赖解析。预装工具年龄继续仅作 CI 记录。

确切 `d537d1a` 的 [Actions 41652195571](https://github.com/AbyssRei/maafw-screencap/actions/runs/41652195571) DesktopDup 组件现已独立审阅，61/61 检查通过，包含新增 adapter/首帧/记录三项。即时官方 ZIP 的 15,961,236 字节长度、完整 SHA256 与 CRC 均核对；首次超时传输及长路径解压失败保留，后以 HTTP range 恢复同一官方摘要、改用新短路径根完整解压，没有重跑捕获。全部 61 项日志及 2,033 个可用登记输入摘要一致；轻量包原始 610 项完整来源/依赖缺口仍列出，十二份选定源码仅在确切 Git 字节匹配登记摘要后另存 supplement，不冒称下载或完整闭包。

实际 pending 数据的四十个原版/Rust 生命周期侧包含 80 次公开取帧、80 个持帧 owner 和 160 份完整 192×128 BGR payload，共 3,932,160 个目标像素逐点核对通过；没有像素 mask/容差。404 行几何/gate 与 200 行独立更新 owner 记录核对：原客户区为 `[0,640,192,768]`，队列释放后恰好一次调整为 `[0,592,192,720]`，无 gate 超时，关闭后的完整持帧不变；更新窗口与前后目标均不重叠，UI 线程不同于挂起目标并共享实际 capture caller。冻结本机 checker 的八组只读验证全部通过，涵盖 pending、受控 Acquire 错误、寿命、Map、held acquisition、geometry、workarea、fullscreen，包内数据/报告未改。实际 MSVC 19.51.36260.0、SDK 26100、CMake 4.4.5 仅按预装身份记录，年龄不作门禁。该有限托管结果不替代下述本机三次零帧 `missing_environment`、历史无输入三轮结论，不接受全局 GPU/任意窗口竞争/完整输入闭包或 P1；审阅由参考目录 `local/current-desktop-pending-hosted-review-path.txt` 指向，初版外部 reader 的输入路径/清单范围/构建日志前提错误分别保留。

DesktopDupWindow 新增独立首帧异步校正测试：直接调用完整未改 F 原类和同一私有 Rust 后端，不预先调用几何 helper。自有目标 UI 队列在首帧前显式挂起，原正常异步校正请求尚未消费时，要求返回原 192×128 客户区图像；直接释放队列后只等待真实消息完成，要求恰好一次移动，再取校正后的图像并保存两类持帧。挂起目标队列时不能用同一 UI owner 的 timer 提供帧，因此另建同输出、不与校正前后目标重叠的 32×32 更新窗口，记录其实际线程、更新进度、错误与关闭。十秒夹具 gate 失败开放仅用于避免遗留挂起窗口，不构成原生取消能力。原生预热/2000ms 等待/全输出转换后裁剪流程未改；原版仅在验收发布边界 clone 目标，新增工作明确保留。

本机预定三轮均在 `SetForegroundWindow` 被拒绝时停止，尚未进入 gate 或调用任何截图；前台为实际 Codex 进程，没有伪造前台、注入输入或改变系统策略。六次测试/记录调用保留实际非零退出，输入监测整批有效、零 armed 事件，最大间隔 69.7507ms，全部 1,355 项冻结输入及副本重新摘要一致，监听已注销。三次属于 `missing_environment`，不接受首帧/像素或完整 P1；没有通过重跑消除该结果。冻结失败批已封存，由参考目录 `local/current-desktop-pending-development-path.txt` 指向。

私有 `desktop-pending.h` 三项原类 owner 创建/截图/销毁导出已有真实消费控制：独立测试和实际 Step 子命令各完成二十个空 HWND 生命周期，原类在截图入口拒绝、失败图像为空，owner 恰好消费，零帧。原版与 Rust 图像都由既有实际 Mat owner/模块保活；原生 owner 留在创建调用线程，没有新增通用 Send/Sync 或产品 C ABI。两项窗口/呈现观测函数从 ScreenDC 测试文件移至共享测试文件，函数体保持原样；ScreenDC 与 DesktopDup 独立原生配置均编译通过。

`cargo xtask upstream-trace desktop-pending <外部矩阵> [--check-only]` 复用原有精确几何/完整像素/持帧校核，另校核四十个独立更新 owner 及其与 gate 的真实线程关系。十四组记录控制接受三组、拒绝十一组：实际旧 ScreenDC 记录只读回归、实际新入口缺行拒绝，与明确使用旧像素和合成更新记录的 schema 控制分别标注；合成记录不是 DesktopDupWindow 截图证据。三十组实际 Step 输出控制含原始前台拒绝的 `missing_environment` 分类，四组独立继续及一个真实原类适配器子命令通过；首帧成功摘要在控制中明确为 mock，没有冒充原生结果。选定 release 测试程序/严格 all-target Clippy、默认 release、workspace（库 34 通过/6 忽略、interop 4、xtask 132）与格式均通过；首次测试程序缺少仅 ScreenDC 启用的 activate 方法，以及新增线程交叉校核的 signed/u64 编译错误均保留，修正只在夹具 feature 范围与受检窄化。

CI 已接入独立 adapter/capture/records 三项与确切单测试守卫、广泛入口跳过和完整 pending 目录上传；另在 DesktopDup 组件结束后即时上传 `p1-windows-desktop-dup-diagnostics`，避免等待后续长 manager 矩阵，确切 `d537d1a` 的执行及数据复核现见上文。此前 `9c1cf91` 的 Windows DesktopDup 步骤已于 2026-11-03 14:07:54 UTC 成功结束；该较早身份仍只保留原生组件步骤状态，不从单一步骤状态推算记录计数或完整 P1。

职责状态另新增参考目录 `local/current-responsibility-status-path.txt`：在保留历史 106 份来源锚点、58 个待验收案例与原映射的前提下，纠正七处把 Xcode 27.2 正式版缺失列作 CI 阻断的过时说明，补入已封存 DesktopDup 受控错误与 Mac manager 对应记录。`reference responsibilities` 对新映射只读通过；这不是全量当前源码锚点刷新或 native/whole-case 验收。预装工具年龄、RC 身份和最新查询失败继续只作参考，本机在线最新依赖策略另行适用。

DesktopDup 新增独立受控 `AcquireNextFrame` 失败矩阵：沿用完整未改 F 原类、同一私有 Rust 后端与既有原生寿命 observer，每侧二十个生命周期，各作一次真实预热截图、一次受控失败和一次真实后续截图。失败按周期选择 `DXGI_ERROR_ACCESS_LOST`、`DXGI_ERROR_WAIT_TIMEOUT`、`DXGI_ERROR_DEVICE_REMOVED`；仅显式测试作用域在真实接口边界返回该 HRESULT，保留传入的 2000ms 参数，故障处实际 driver 调用为零。预热与后续截图继续调用真实驱动；不能据此声称模拟物理设备故障或实际等待两秒失败。生产路径仍直接调用原接口，Rust 干预仅编入 prototype 测试。

四阶段记录为成功预热、失败返回、下一次成功截图和显式关闭。F 的 `ACCESS_LOST` 只释放 duplication/staging、清空 monitor，槽位由 127 变为 31，保留 device/context/factory/adapter/output 和 texture descriptor。下一操作重选 adapter/output、重建 duplication/staging，继续使用原 device/context/factory，槽位恢复 127；另外两个失败保持全部原生状态。显式关闭后七槽归零，保留原 monitor/初始化标记语义，descriptor 清零。四个旧对象分别通过真实 D3D 析构通知或较弱 DXGI 私有 owner 通知观察；恢复代次的新 duplication/staging 未另行注册，不能推广为完整恢复代次资源闭包。

预定三轮共 27 次命令全部退出 0，含新矩阵的九次原版/Rust/记录调用及十八次共享钩子的既有 Map/真实 held-acquisition 回归。独立复核重新摘要全部 1,346 项冻结输入及内容寻址副本，九组记录 checker 只读通过且文件不变；新矩阵重算 360 次公开调用、240 次实际成功截图、120 次受控失败（42 次 access-lost、78 次其他错误）、480 个旧对象、360 次 D3D 通知和 120 次 DXGI owner 释放。240 个保留 CPU owner、480 份完整即时/持帧图像的 2,949,120 像素、接口/线程/QPC、各阶段释放与保留状态均核对。四路监听 armed 区间零事件、最大间隔 77.6875ms，已注销并释放电源请求；prearm 没有实际事件，不声称新实物正控制。证据由参考目录 `local/current-desktop-acquire-errors-development-path.txt` 与 `local/current-desktop-acquire-errors-local-review-path.txt` 指向；三轮和独立复核已封存。

`cargo xtask upstream-trace desktop-acquire-errors <外部矩阵> [--check-only]` 校验上述实际记录。五十四组保存数据控制接受三组、拒绝五十一组，覆盖原生调用数/错误码/等待参数、部分清理/保留身份、通知阶段/稳定性、缺行/重复、ABI/布局/完整像素及保存报告；原矩阵未改。实际 Step 二十七组输出控制、四组失败后独立继续及一个真实 checker 子命令通过。外部复核初版误用旧报告字段、两次 mock 子进程 ANSI 渲染保留断言，以及 stride 控制的 Windows CRLF 假设错误均保留；最终仅对 mock 文本保留断言归一化 ANSI SGR，不改变整行摘要、退出码、截图或任何像素断言，也不声称原始终端转义字节保留。

本批使用显式归档的离线开发依赖，不声称新在线最新版解析。VS2026/MSBuild/SDK 28000 原生编译、所选 prototype/desktop-dup release 测试程序及严格 all-target Clippy、默认 release、workspace（库 34 通过/6 忽略、interop 4 通过、xtask 131 通过）和格式检查通过。完整 F CPP 的实际编译输入、六个冻结程序及五个 observer 导出另经来源校核。CI 已接入独立 source/Rust/records、严格单测试/完整摘要守卫及完整目录上传，广泛入口显式跳过已单列用例；实际托管执行仍待结果。预装工具年龄继续只作参考。此范围不覆盖真实显示器/分辨率切换、驱动 access/device loss、任意故障、全局 GPU、完整框架排空或完整 P1，既有失败不变。

确切 `d43c60d` 的独立 Windows FramePool Actions `41632524368` 已失败结束，组件为 106/110：新增 `frame-pool-lifetime-build`、`frame-pool-source-lifetime`、`frame-pool-rust-lifetime`、`frame-pool-lifetime-records` 全通过；与封存 `f43996a` 相比，全部原 106 项名称/状态不变，四个原版 pseudo 黑图失败保留。工具准备成功，预装工具年龄未设门禁。官方 1,822,614 字节轻量 ZIP 的完整 SHA256/CRC、全部 110 份日志及 3,312 项可用登记路径已核对，轻量包本身仍缺 660 项完整输入；十三份确切 Git 源码补件另与实际原生登记摘要绑定，显式核对 CRLF，不据此称为完整源码/依赖闭包。

新托管寿命矩阵的 127 份实际文件全部匹配原生登记，保存报告的 126 项引用亦一致。冻结 checker `--check-only` 通过且 127 文件摘要完全不变；独立再算两侧共 40 次真实取帧/生命周期、120 个对象与 D3D 析构通知、二十组显式 texture/device 引用对、四十个保留 CPU owner，以及八十份即时/持帧图像的 491,520 像素。四阶段、原生接口/线程/QPC、`uninit` 槽位 7 和独立 cache 保留均核对。此结论关闭该有限托管寿命入口，不接受 swapchain 析构、全局 GPU 资源、任意故障、完整框架回调排空、旧像素失败或完整 P1；已封存记录由参考目录 `local/current-framepool-lifetime-hosted-review-path.txt` 指向，未重复截图取得通过。

确切 `02c7eb6` 的 Windows job `125809156872` 亦已失败结束，其七组件共 401 项检查为 390 通过、11 失败。469,836,742 字节官方 ZIP 的完整摘要/CRC及全部日志摘要已独立核对；唯一新增 `manager-gamepad-metadata` 通过，原有 400 项名称均保留。与 `7b3a608` 相比，Gamepad native mask 26 从失败变通过，30 从通过变失败，其余状态不变；本批失败 mask 为 `6/10/14/30/38/42/46`，FramePool 仍为 85/89、同样四个 pseudo 失败。这里只记录实际状态变化，像素生产者及变化原因仍未确定，不替代最终本机无输入三轮、不新增像素/资源数据复算或完整输入验收。已封存焦点记录由参考目录 `local/current-02c7-windows-focused-path.txt` 指向；多次传输失败/超时及部分文件均保留，最终分段恢复通过官方完整摘要，完整 P1 仍未通过。

FramePool 新增完整未改 F 原类与同一私有 Rust 后端的成功取帧寿命矩阵。两侧各二十个自有窗口生命周期，每周期一次实际截图；在成功后对真实 device、immediate context 和 staging texture 注册既有独立 `ID3DDestructionNotifier` 观察。其私有 observer 与 DesktopDup 共用相同五项导出、具体接口 QI 和标量宽度检查，回调仍只写固定原子记录，fixture 模块保留至进程退出。未复制捕获实现，未改变产品 API、方法、等待、转换和清理流程。

四个阶段分别为成功取帧、显式 `uninit`、捕获 owner 析构、额外 texture/device 引用释放。F 的 `uninit` 不清空 device/context/swapchain，也不清除独立 CPU cache；缓存槽位实际由 127 变为 7，cache 地址保留且与返回图像不同，前两类 D3D 对象此时不得析构。奇数周期另持有 texture 和 device 两个独立引用，不假定隐式父设备关系；owner 析构后这两个对象仍须存活，按 texture→device 顺序释放后再观察最终通知。未持有额外引用的周期在 owner 析构后的两秒观测预算内须完成，最终阶段同样核对实际通知。此有限预算不构成同步析构或任意驱动行为保证；swapchain 的真实析构没有在本矩阵观察。

预定连续三轮共九次 source/Rust/records 调用全部退出 0。独立复核重新核对全部 1,527 项冻结输入及内容寻址副本，三个冻结 checker 只读复验均保持 127 份实际记录不变；重算 120 次取帧/生命周期、360 个真实对象与析构通知、六十组显式引用对、120 个保留 CPU owner，以及 240 份完整即时/持帧图像的 1,474,560 像素。原生注册方式、返回码、接口身份、线程/QPC、四阶段状态与 cache 地址逐项核验。四路监听在 armed 区间零输入、最大间隔 75.6276ms，已注销并释放电源请求；本次 prearm 未观测到实物事件，不声称新的实物正控制。记录由参考目录 `local/current-framepool-lifetime-development-path.txt` 与 `local/current-framepool-lifetime-local-review-path.txt` 指向，三轮及最终复核已封存，不得改写。

`cargo xtask upstream-trace framepool-lifetime <外部矩阵> [--check-only]` 独立验证上述记录。四十九组实际数据副本接受三组完整记录、拒绝四十六组损坏记录，覆盖 uninit 保留、owner/额外引用释放、注册/身份/时序/缓存、缺行/重复、布局/完整像素及保存报告；原矩阵未改。实际 Step 二十七组输出控制、四组独立继续执行以及一个真实冻结 checker 子命令均通过；完整整行摘要及确切单测试通过才接受，失败汇总和后续检查分别保留。

本批使用显式归档依赖。首次原生链接因继承伪最小化夹具开关却缺对应源文件而失败，首次 Rust 编译因新测试错用 `Attempt.result` 而失败；改用既有完整原生目标源清单和正确 `image` 字段后，原生/release Rust 重建、所选全部 target 严格 Clippy 均通过。外部矩阵准备的 PowerShell 引号错误也保留，发生时尚未冻结或执行截图。另一次 DesktopDup 独立构建控制在较长目录的 MSBuild 编译器探测阶段报 tlog `DirectoryNotFound`；改用较短独立目录后，同一 DesktopDup ON/FramePool OFF 配置编译通过，零截图，原失败保留，不改称 SDK 或捕获失败。最终默认 workspace 为库 34 通过/6 忽略、interop 4 通过、xtask 130 通过，格式、默认严格 Clippy 和 release 构建通过；不声称本批重新在线解析最新版。

CI 已接入独立的 FramePool lifetime 构建/source/Rust/records，广泛测试入口显式跳过该已单列用例。另修正轻量上传清单：先前 DesktopDup lifetime 的通用 `.bgr` 匹配只能覆盖像素，不能覆盖寿命表/ABI/报告；现为 DesktopDup 和 FramePool 两个 lifetime 目录各声明完整上传范围。旧运行的实际上传缺口须由其完整工程包另行核验，不能把清单修正当作旧数据已经可用。后续独立托管寿命结果见本节开头；没有全局 GPU 统计闭包、任意设备故障、完整框架回调排空或 P1 验收结论。

确切 `130aad1` 原生 Linux job `125848914800` 已成功，PipeWire 205/205、Wlr 151/151，全部 356 项名称/状态与封存 `02c7eb6` 一致。两份官方 ZIP 的完整摘要/长度/CRC、全部检查日志摘要及实际编译器/链接器身份核对，预装工具政策控制通过；这里只复核该确切提交的组件回归，不重新接受完整来源/依赖输入闭包。PipeWire 的零字节停滞与三分钟部分传输超时均保留，后续分段恢复逐项核对实际 HTTP range，最终完整官方摘要一致，旧部分文件不变。已封存焦点记录由参考目录 `local/current-130aad1-linux-focused-path.txt` 指向，完整 P1 未通过。

DesktopDup 新增独立的真实对象寿命矩阵：原版目标单独编译完整未改 F `DesktopDupScreencap.cpp`，Rust 直接调用同一私有后端；每侧二十个自有 96×64 场景生命周期，每周期只作一次公开截图。先预热成功，再观察实际 device、immediate context、duplication 和 staging texture，原 `uninit` 的七个缓存槽从 127 归零。device/context/texture 通过真实 `ID3DDestructionNotifier`，duplication 通过唯一 GUID 的 `SetPrivateDataInterface` owner；两种证据分别记录，后者只证明关联 owner 释放。奇数周期分别保留 texture 和 device 两个显式引用，不假定隐式父设备引用；释放这两个引用前不得收到其通知。最终通知的两秒预算是有限观测条件，不是 API 同步析构保证。

原生 observer 的具体接口 QI、六项标量宽度、注册方式/返回码、线程/QPC、稳定对象身份和每周期三阶段记录均独立核对；注册仅借用当前对象，临时 QI 引用在返回前释放。回调只写进程寿命的固定原子记录，无借用栈地址、原生对象访问、分配或 I/O；Rust 即使失败也保留 fixture 模块至进程退出。没有添加 `Flush`、`ClearState`、设备重置或取帧重试。持有的 CPU 图像在后端、原生资源和窗口退出后逐字节重读；不把这些私有诊断导出变成产品 ABI。

本机预定三轮共九次原版/Rust/记录调用全部退出 0；独立复核重算 120 次公开截图/生命周期、480 个对象、360 次 D3D 析构通知、120 次 DXGI owner 释放、各六十个显式 texture/device 引用控制，以及 240 份完整即时/持帧目标文件。全部 1,341 项运行输入及内容寻址副本重新摘要一致；三个冻结 checker 的只读复验各保持 127 份实际文件不变。四路 Raw Input/低级 hook 在 armed 区间零事件，最大采样间隔 79.3404ms，全部注销且电源请求归还；本次仅观察到 prearm 鼠标通道 1/3，不声称新键盘实物挑战。证据由参考目录 `local/current-desktop-lifetime-development-path.txt` 与 `local/current-desktop-lifetime-local-review-path.txt` 指向，已封存的三轮和最终复核不得改写。

`cargo xtask upstream-trace desktop-lifetime <外部矩阵> [--check-only]` 校核完整原生状态和目标像素，分别报告两种通知的意义，不声称全局 GPU 内存闭包。四十组实际数据副本控制接受三组完整记录、拒绝三十七组损坏记录；三阶段、计数、重复/缺行、接口身份、显式持有、时序、ABI、像素和保存报告均覆盖，原矩阵未改。实际 Step 的二十七组输出控制、四组独立继续执行和一个真实冻结 checker 子命令通过；不完整/重复摘要、零/多/忽略/测量测试、非零退出均拒绝，失败汇总保留。控制初版错误地把矩阵根目录无关文件视为受登记数据，失败已保留，修正仅改用保存报告字段。输出控制还暴露七项宽松匹配，现只接受完整整行及确切 libtest 前缀，不重跑截图取得绿色。

本批使用显式归档依赖、VS2026/MSBuild、SDK 28000 和记录的 nightly，不声称新在线最新版解析。首轮原生编译缺 `dxgi1_2.h`、直接 DD feature 缺共享夹具方法的两次失败均保留；补头与正确 feature gating 后重建通过，没有改后端捕获算法。CI 已接入独立 source/Rust/records 三项和构建，后续托管执行仍待结果。预装工具年龄继续只作参考；有限对象/CPU 所有权通过不覆盖任意 device/access loss、完整回调排空、历史像素失败或完整 P1。

确切 `130aad1` 的原生 Mac Actions `41615832824` 已完成：新增 `macos-live-sample-hook-control`、`rust-macos-live-sample`、`macos-live-sample-records` 全通过，组件为 136/157；全部原 154 项名称/状态及 21 个失败完全不变。官方 41,548,013 字节 ZIP 的 SHA256/CRC、11,986 项归档内容、全部 157 份日志、42 份实际登记数据、41 项报告引用及十六份确切 Git 来源补件均独立核对。冻结 checker `--check-only` 通过且 42 文件摘要未变；真实原生 Mat 类型查询为 64。

三份真实回调样本各执行一次现有 Rust 转换，完整 2,359,296 像素重算通过：原生转换前/后 BGRA 与 Rust 即时/持有 BGRA 相同，原版即时/持有 BGR 与 Rust 即时/持有 BGR 及所有 BGRA 的前三通道相同。六份 Rust CPU owner、三份原版 BGR 持帧、测试回调共享上下文归还和已 join reader 记录核对；不接受已知场景、两次独立选源或整个框架回调排空。组件仍整体失败，原像素/超时断言无豁免。直接下载三分钟零字节超时保留，后续显式任务局部代理 DNS、正常 TLS 身份校验的独立传输通过官方长度/摘要；未输出凭据或修改持久网络设置。已封存记录由参考目录 `local/current-macos-live-sample-native-path.txt` 指向，不授予完整输入闭包或 P1 验收。

新增独立 `macos-live-sample` 诊断，不替换已有原版/Rust 严格场景。确切 F 的原类仍通过单独观察模块编译，保持选源、配置、原转换和五秒等待；观察包装先转发原完成块，再于同一实际回调同步将相同 CMSampleBuffer 借给已有 Rust `pixel_buffer::with_sample` 与显式 OpenCV 适配器。预期每次取帧记录原生转换前/后 BGRA、原版及 Rust 完整 BGR、Rust 锁/复制/转换/解锁和原生线程身份，三次调用各只执行一次。新入口没有人工已知场景，因此只检验同一真实输入的转换与所有权，不能据此豁免旧场景像素失败或证明两次独立截图选源等价。

原类上下文和逃逸观察块共享一个测试回调 owner；注册成功或拒绝均按私有头约定接管/释放，完成计数在 Rust Arc 归还之后发布。原对象结束后，在已 join 的另一个线程重读并释放六份 Rust CPU owner，另保留原版三份持帧对照；模块退出与回调上下文归还分别记录，不证明整个原生框架回调排空。CPU 访问继续使用现有只读锁及配对解锁，遵循 [CoreVideo 锁契约](https://developer.apple.com/documentation/corevideo/cvpixelbufferlockbaseaddress(_:_:))，不保存样本或原生基址。默认产品后端和既有观察模式不启用新 hook。

`cargo xtask upstream-trace macos-live-sample <外部记录目录> [--check-only]` 逐字节核对三份完整原生/Rust 输入和输出、全部持帧、原生/调用/reader 线程与锁/释放记录。两项完整合成数据控制拒绝改动的 24 份像素文件，以及错误计数、线程、原生失败和 owner 重标；这不是新原生取帧。实际 Step 的二十组输出控制和四组双子命令编排控制验证零/多测试拒绝、重复摘要、退出码、缺权限分类、独立记录继续执行及环境恢复；其中只有跨线程 guard 控制使用本机真实 C++ 程序，其余捕获输出是显式 mock。

校验实际旧原生数据时另发现该批 OpenCV 的 BGR 类型标签为 64，不能把历史标签 16 写成校验器常量；现由私有头的真实原生查询提供编译时标签，再核对实际 Mat 记录。合成控制同时验证两种标签及错误配对，未将它定为 SDK 格式编码。首次外部 CLI 控制因 Windows 文本换行拒绝而停止，失败保留；这是控制生成器的问题，不改写旧原生结果。修正后的六组实际 CLI 子命令接受三项、拒绝三项，验证创建、只读、已有报告、改动持帧与改动报告；Rust 侧记录明确由旧原生数据合成，未执行新 Rust 回调或真实捕获。

此前开发批的本机 VS2026/MSBuild guard Debug/Release、Apple target Rust 类型检查与严格 Clippy、workspace 125 项 xtask、默认库 34 通过/6 忽略及 4 项 interop 均通过；使用与先前集成批一致的显式归档依赖和 nightly 身份，不声称重新查询在线最新版。首次严格 Clippy 的合成夹具 `chunks_exact` 提示已修正并单独保留失败日志。该批只完成原生测试/记录准备，当时尚未执行 Apple 捕获；后续真实结果见本节开头，不改写开发批身份。开发记录由参考目录 `local/current-macos-live-sample-development-path.txt` 指向，完整 P1 尚未验收。

确切 `02c7eb6` 的原生 Mac job 已失败结束，组件仍为 133/154；全部 154 项名称和状态与已封存 `7b3a608` 一致，21 项旧失败保留。官方 40,056,162 字节组件 ZIP 的摘要/CRC、11,941 项归档内容和每项日志摘要核对，实际工具身份和预装政策控制保留；没有工具年龄门禁失败。本轮不重审全部轻量输入缺口、源码/依赖闭包或改变像素、超时和回调结论；焦点记录由参考目录 `local/current-02c7-macos-focused-path.txt` 指向。

同一提交的原生 Linux job 成功，PipeWire 205/205、Wlr 151/151；全部 356 项名称和状态与已封存 `7b3a608` 一致。两份官方组件 ZIP 共 43,701,677 字节，摘要/CRC、44,254/9,034 项归档内容及全部检查日志摘要核对，实际编译器/链接器身份和预装政策控制保留。这里只复核新提交的组件回归，不重审完整登记输入和源码/依赖闭包，也不继承完整 P1 验收；封存记录由参考目录 `local/current-02c7-linux-focused-path.txt` 指向。预装工具不是严格最新不阻断 CI，实际能力、构建及捕获检查仍各自判定。

同一私有 `GamepadCapture` 现已迁移 `request_uuid`、`get_features`、`get_info`。保留构造时的 HWND/type/method，不读取连接状态或创建宿主/捕获资源；UUID 使用十进制 HWND，Xbox360 的值 0 以外均按来源返回 ds4 名称，先完成可失败分配和格式化再替换调用方 String。info 保留无符号 HWND、type/method 的有符号 64 位转换和原 JSON 字段；features 固定为来源的三个历史输入标志，只属于私有来源元数据，不代表 SDK 已实现输入。捕获委托和生命周期分支未改，公共 SDK/ABI 仍待后续阶段。

新的独立 C++ collector 编译整个未改 F manager/header/API，经原 C 创建/销毁及虚接口执行；Debug、Release 各 144 组、288 个查询阶段、原版/Rust 合计 1,728 次查询通过，两配置共 3,456 次。覆盖 null/大值/最大 HWND、0/1/2/有符号边界/最大 type，以及 0/1/63/64/高位/最大 method；每次用不同长度旧 UUID 验证替换，返回值在 manager 销毁后仍持有。原版和 Rust 均要求零输入 owner、零捕获 unit/模块及空事件记录；旧 72 例宿主/CPU Mat 回归在两配置也通过。本批没有实际屏幕取帧。

`cargo xtask upstream-delegate gamepad-metadata <已存在的外部记录目录> [--check-only]` 严格核验三份原始文件、全部字段/相位/顺序、来源统计和保存报告。24 项实际记录控制中两项通过、22 项缺失/损坏/多余/错误值被拒绝；字段控制先移除保存报告，避免只触发摘要差异。实际 `Step` 的十五项控制/三十个模拟子进程覆盖真实 libtest 同行格式、重复/缺失结果、零测试、非零及未知 77、坏记录和失败后继续独立检查；工作区真实 Rust→原 C++ 查询子进程也经同一个 Step 和新 checker 通过。最初控制误复制已有输出报告被 `create_new` 拒绝，保留失败后只清除新控制副本的保存输出，没有放宽写入保护。

`upstream-parity manager` 新增独立 `manager-gamepad-metadata` 进程及组件包记录路径；默认聚合跳过它，避免重复执行。在线准备编译保存原 collector，离线 manager 现要求 25 项 oracle 输入，旧数据只读入口不变。工作区测试（123 项 xtask、默认库 34 通过/6 忽略及四项互操作）、选定 Windows feature 全目标 Clippy、默认 Release、格式检查通过；依赖明确复用归档批次，不声称新在线最新解析。开发先在外部源码快照编译/验证并封存，63 组原生矩阵结束后才合入；严格补丁预检的首次 CRLF 传输错误及最初外部 CMake OpenCV 作用域链接错误分别保留。工作区复验由参考目录 `local/current-gamepad-metadata-integrated-path.txt` 指向；新托管执行待验证，函数对应计数暂留 40 份来源/285 项，完整 P1 与既有原生失败未因此验收。

确切 `7b3a608` 的 Gamepad 原生入口现编译完整未改 F manager/header/API，通过真实 Boost/LibraryHolder、原创建/销毁入口及虚接口调用实际 Win32 provider；Rust 使用同一个 `GamepadCapture`、原生委托和 manager。目录边界明确指向本批真实 DLL，输入 owner 仍是不会操作设备的显式夹具。每侧每轮二十个生命周期、六十次实际取帧，并在每个生命周期交替注入宿主 false、连接异常或构造异常；失败后两次取帧继续使用旧捕获 owner。完整宿主/原生计数记录每侧 182 行，核验 reset 时委托仍在、原生 unit 销毁及模块卸载后才释放输入 owner。`cargo xtask upstream-delegate gamepad-runtime <mask> <已存在的外部记录目录> [--check-only]` 先复算未改的窗口像素、完整跨度、持帧、选定范围和原生事件，再检查新增宿主记录；旧 `win32-runtime` 入口及历史报告保持可读。

本机全部 63 个原始非空方法组合均按预定计划连续三轮通过，共 189 次原生测试及 189 次记录校验、7,560 个实际生命周期、7,560 次受控宿主重连失败、45,360 份即时/持帧 CPU 文件。每组四路鼠标/键盘监听、可见未锁桌面前提、最大 500ms 采样间隔及注销/写入完成分别核验；本次零输入事件、最大间隔 107.2408ms，未声称新物理正向挑战。独立读回逐组核验目标已知图样、完整字节跨度、关闭后的持帧身份、原生事件和宿主计数；测试结束后再次核对全部 1,785 份冻结输入及未变的 543 文件产品树，未重跑任何已完成组。使用已更新 nightly 和明确归档依赖，不称新在线最新依赖解析。有限生命周期不等于单会话长流、任意故障或原生回调排空，完整 P1 仍未验收。最终只读汇总由参考目录 `local/current-gamepad-native-final-local-review-path.txt` 指向。

同一 `7b3a608` 的原生 Actions Windows 七组件为 389/400：manager 101/108，其 63 个原生组合中 56 个通过，`6/10/14/26/38/42/46` 七个像素失败保留；FramePool 为 85/89，四个原版 pseudo 失败保留。官方组件 ZIP 的摘要、长度、CRC、全部 400 项日志摘要、八份确切 F 来源及 72 例宿主模型记录独立核对；56 个通过组合的完整原生/像素/持帧报告只读复算，全部 63 组的宿主和原生事件记录另核验 2,520 个生命周期及 2,520 次受控宿主失败。失败组保留实际像素差异、完整跨度和持帧身份，不改称通过，也未认定像素生产者或共同原因。本机三轮通过不覆盖这些托管失败。较早 `ae4e578` 的 72 例宿主模型同样通过，但其原生失败集合为 `6/10/14/22/30/38/42/46`，两次身份和结果分别保存。

同一 head 的 Linux job 成功，PipeWire 205/205、Wlr 151/151，官方轻量包和全部 356 项日志摘要已独立核对；Mac 组件仍为 133/154，全部 154 项名称/状态与已保存的 `0fe79b2` 原生批次一致，21 项失败保留。其实际预装 Xcode 为 27.1/27A9275，Windows 原生 compiler 为 MSVC 19.51.36260.0、SDK 26100；工具年龄未阻断真实执行。这里仅补本轮组件、选择来源和记录复算，不重称轻量包已闭合全部源码/依赖输入。相关包、失败、初次 reader/传输错误及完整只读记录在 `local/current-gamepad-native-development-path.txt` 指向目录的各平台 review 中分别保留；P1 逐函数选定计数仍为 40 份来源/285 项。

Gamepad 的截图相关外层新增私有 `GamepadCapture`，通过类型化宿主 owner 复用原有 `CaptureDelegate`，没有复制捕获后端。它保留先清 connected、成功构造新宿主后才替换旧 owner、宿主连接失败时保留旧捕获委托、可选捕获初始化返回 false 仅警告、connected 短路，以及析构先 reset、再结束捕获 view/loader、最后释放宿主 owner 的顺序。截图与 inactive 不新增外层 connected 门槛；原版已失效 view 不作未定义的虚调用，Rust 仍明确拒绝。宿主只是截图相关初始化/寿命边界，不实施 ViGEm 驱动、手柄输入、前台操作或 SDK。

最终 Debug/Release 各 72 个配对场景通过，实际编译整个未改 F `GamepadControlUnitMgr.cpp`、原头文件及 `GamepadControlUnitAPI.cpp`，经真实原 C 入口构造/销毁和虚接口执行；宿主输入 owner、DLL 查找/后端结果是显式夹具。每种配置、每侧分别观察 144 次宿主创建尝试、126 次连接、71 次最终 reset、98 次 capture 转发及 98 次 inactive 转发，并核对两次返回后的持有输出、生命周期和异常/false 分离。这里的 capture 转发仅操作受控 OpenCV Mat，不是屏幕取帧。八份 F 来源逐一匹配确切 Git blob，957/958 份冻结的实际编译/生成/SDK/工作树输入及存档摘要核对；旧 Rust/C++ 委托夹具仅搬入共享位置，原断言未改，既有 274 例委托回归在两配置均通过。

`cargo xtask upstream-delegate gamepad-host <已存在的外部记录目录> [--check-only]` 独立核验 216 份原版/Rust/错误文件及完整事件、选择器、输出、宿主失败后的旧 owner 保留和销毁顺序；只读复验包含保存报告共 217 份文件且字节不变。十九次实际记录控制中两项成功、十七项损坏/缺失/多余/顺序/保存报告控制拒收，坏数据控制先移除保存报告，避免只碰到旧摘要。真实 `Step` 的十三项控制/二十六个子进程包含实际 libtest 同行日志、零测试、重复结果、非零及未知 77、记录缺失和失败后继续独立案例，均保留正确状态。首版外部 Step harness 误复用已存在的报告，被写入保护拒绝；修正为每例独立记录副本后通过，原失败保留。早期编译/重复模块/Clippy 等开发失败亦分别保留。

本批使用明确归档依赖和已更新 nightly，不称新在线最新依赖验收；工作区测试通过，包含 xtask 123 项、默认库 34 通过/6 忽略及互操作四项；另有所选 Windows 53 通过/8 忽略、默认 Release、格式和 Clippy 检查通过。忽略入口由显式调用负责。`upstream-parity manager` 新增独立 `manager-gamepad-host` 检查，即时上传保存原类来源及全部记录；新的托管执行待验证。随后独立四项来源对应将选定累计增加至 40 份来源/285 项，详见逐函数审阅；部分运行结果不代表完整逐函数覆盖或 P1 通过。历史六项 mixed mask、FramePool pseudo 和 Mac 像素/回调失败保持原状态；Actions 预装工具按实际能力和身份记录，年龄不作门禁。完整开发记录由参考目录 `local/current-gamepad-host-development-path.txt` 指向。

确切 `ab544fc` 的 [Actions 41564727121](https://github.com/AbyssRei/maafw-screencap/actions/runs/41564727121) 原生 Linux PipeWire 组件现已通过全部 205 项检查：新增 `original-kwin-portal-failure`、`rust-kwin-portal-failure` 和记录检查全部通过，与已复核 `8c1369c` 的原有 202 项逐名对照，全部状态不变。真实 C++ 原版、Rust 和实际矩阵均已执行；两侧各 100 次失败，每种 Open false/错误 91–94 各二十次，分别有一百个不同的非零 socket cookie，在失败前后保持相同。每次确认 helper 与一个未消费 fd 残留，再由夹具回收；各侧 101 组实际任务身份、fd 基线和 caller 均已独立核对。没有创建 manager、PipeWire 流或截图；不把残留或夹具回收称为产品清理，也不把受控宿主错误当作真实 DBus/桌面授权验收。

官方即时 ZIP（39,849,712 字节）的发布摘要/CRC、44,285 份文件、44,079 个可用登记路径和全部 205 份日志核对；9,401 次相同摘要重复登记明确保留，623 项完整输入仍缺。八份当前 Rust/原生夹具/编排/校验源码由确切 Git bytes 或明确 CRLF checkout 转换匹配实际登记摘要，上传的完整 KWin 原文件亦匹配固定 F；这些选取不代表全部来源闭包。冻结校验器对实际 205 份矩阵数据/报告只读通过，摘要不变。另在真实矩阵副本上执行 29 次 CLI 控制：两项成功、二十七项错误/调用顺序/残留/回收/任务/文件/报告篡改被拒收；字段和资源负向控制先移除旧报告，确保拒收来自语义或结构检查，原矩阵不变。证据由同一参考 marker 的 `hosted-review/`、`native-cli-controls/` 指向；组件通过不接受其他平台旧失败、一般宿主故障、完整兼容 ABI 或 P1，整个平台 job 的后续组件与最终状态分别记录。

新增 KWin 创建前的受控 Portal 失败矩阵：原生 C++ 入口 `--kwin-portal-failure <private socket> <全新输出目录>` 调用未修改的完整 `MaaKWinControlUnitCreate`，Rust 精确测试 `backends::linux::manager::compat::tests::portal_failures::retains_host_socket_after_each_portal_failure_before_manager_construction` 调用同一私有兼容 Config 构造器。每侧计划一百次，五种情况各二十次：宿主 Open 已取得真实 socket 后返回 false，或在 Open、fd getter、node getter、helper Destroy 依次返回受控错误 91–94。原版异常只由 C++ 调用方捕获，Rust 使用宿主 Result；不使 C++ 异常跨越 Rust ABI。每次均须没有 manager/捕获构造，保留完整调用前缀、helper、一个未消费 socket 与相同 `SO_COOKIE`，再显式关闭同一 fixture offer 并核对实际任务身份和 fd 基线。成功路径的既有 KWin 宿主夹具默认选择不变。

`cargo xtask upstream-trace kwin-portal-failure <已存在的外部矩阵> [--check-only]` 要求两侧共两百次完整记录及 204 份文件，逐项检查错误、调用顺序、真实 cookie、残留/回收区分、caller 和一百组关闭后任务列表；保存报告始终声明零截图、完整 P1 未验收。编排独立保留原版、Rust 和记录检查，失败不省略后续案例，每步 finally 保存部分证据；该矩阵不以未知退出 77 冒充缺环境。private-daemon/socket 并非 DBus/桌面授权，异常点是宿主控制条件；本项不接受真实 Portal、输入动作、一般失败、已开始原生流的清理或完整兼容 ABI。产品构造/失败清理均未改动，不把原 helper 残留改写为产品清理成功。

本机使用与上一初始化开发批次一致的显式归档依赖重放，nightly 已检查更新；123 项 xtask/工作区测试、默认 Release、严格 Clippy 和格式检查通过。Linux 目标的测试编译/Clippy 也通过，属于交叉静态检查，尚非原生执行。28 次人工 CLI 控制包含两项成功和二十六项拒收；28 项实际 `Step` 控制/56 个隔离子进程，以及实际三步编排的四种失败位置/12 次显式 mock 案例均通过，输出/错误和非通过 aggregate 保留。首版 checker 把数字任务 ID 当作字符串的编译失败及修正版本分别保存；所有控制都不算真实 Linux socket/capture 验收。随后确切 `ab544fc` 的原生 C++、Rust 和实际记录结果见本节开头，证据由 `local/current-kwin-portal-failure-development-path.txt` 指向；已有平台失败和完整 P1 门禁继续保留。

确切 `f43996a` 的独立 [Actions 41558756707](https://github.com/AbyssRei/maafw-screencap/actions/runs/41558756707) 已终态失败，组件为 102/106。新增 `frame-pool-initialization-build`、原版、Rust 和记录四项均通过，实际执行 160 次原生拒绝及 80 次设备最终释放通知、零截图；与 `8c1369c` 的旧 102 项逐名对照，状态完全不变，四个原版 pseudo stop/inactive/layered 失败仍保留。该 runner 实际使用 VS build `18.11.12328.437` 与 SDK `10.0.26100.0`，准备日志明确最新版仅作参考且退出 0；年龄未成为 CI 门禁。

官方 1,746,053 字节即时 ZIP 的发布摘要/CRC、3,292 份文件、3,185 个可用登记路径和全部 106 份日志独立核对；84 个相同摘要重复登记明确计数，625 项完整输入缺口保留。八份选定当前原生/Rust/编排/校验来源由确切 Git bytes 或明确 CRLF checkout 转换匹配实际登记摘要，另存来源补充；这不是整批完整来源闭包。新增八份初始化数据/保存报告在冻结校验器上只读通过，文件摘要不变。外部终态比较器首次触及 Windows 长路径限制的失败及修正版本另存。结果由同一 `current-framepool-init-development-path.txt` 的 `hosted-review/` 指向并封存，关闭的是新增故障路径的托管执行，原失败、全部设备故障/回调退出和完整 P1 均未验收；push 的完整三平台矩阵另行运行。

新增独立 `mfs_framepool_init_reference` 与私有 Rust 单测 `preserves_failed_initialization_state_and_releases_replaced_native_devices`，检查尚未创建 item/session/pool 时的实际初始化失败。每侧 20 个对象，各固定四次调用：有效但隐藏的 HWND 连续两次在真实 `CreateForWindow` 返回 `E_INVALIDARG (-2147024809)`，原类保留 device/context/swapchain 三个槽；销毁窗口后，同一对象使用原标量 HWND，连续两次在真实 `D3D11CreateDeviceAndSwapChain` 返回 `DXGI_ERROR_INVALID_CALL (-2005270527)`，三个槽为空。第二次隐藏调用及首次销毁后调用须在返回前释放旧设备。原类/头文件/函数体未修改，测试通过类型化成员指针观察原字段；Rust 使用同一私有 `FramePoolCapture`，转换、clone、缓存回放均不得到达，产品 `frame_pool.rs` 没有改动。

私有 fixture 的八个 C 导出实际加载、调用并核验六项标量宽度。`ID3D11Device::SetPrivateDataInterface` 持有原生 `IUnknown` sentinel，不反向持有设备；其最终 `Release` 写入进程寿命的原子记录，包含附着/最终线程与 QPC。Rust 将 fixture DLL 保留至进程退出，失败路径也不卸载可能仍被原生回调使用的代码。终止通知可以来自另一原生线程，不以调用线程相同为先验条件；本机三轮实际均在各自调用线程终止。本测试没有创建捕获 session，因此不证明 WGC callback 排空、任意设备故障、驱动资源总量或完整失败恢复。

本机 VS2026/MSBuild 编译原版与 fixture 后，冻结 1,477 份实际程序、CL/link 读取、SDK、生成声明和工作树输入，再自动执行三轮原版 → Rust → 独立记录检查，共九次进程调用。480 次真实初始化拒绝及 240 次设备终止通知全部通过，零截图；四路 Raw Input/低级 hook 鼠标键盘监听为零输入事件，最大采样间隔 78.5914 ms，小于 500 ms，桌面前提和注销完成，未声称本次注册有新的物理输入正控制。全部冻结现场/存档摘要、527 份工作树快照、八份读取日志和两份确切 F blob 已独立核对，三组只读复验前后的 24 份数据/报告均不变。

`cargo xtask upstream-trace framepool-init <已存在的外部矩阵> [--check-only]` 核验两侧 160 次调用、80 次终止通知、原版实际 HRESULT 日志、ABI、线程/时钟/槽状态和七份完整输入；报告明确零截图且 `full_P1_accepted=false`。基于实际矩阵的三十次 CLI 控制中两项成功、二十八项缺行/多行/重复、状态/错误码、迟到/缺失通知、ABI/文件/保存报告篡改等被拒收。真实 `Step`/`Invoke-FrameCheck` 的二十八项控制、五十六个隔离子进程核验完整汇总、零测试、重复输出、非零/未知 77 退出及失败后继续独立案例。首版门禁误拒真实串行 libtest 同行前缀、误收重复结果行，现均修正；失败版本和修正后记录分别保留。外部 dumpbin reader 首次误将 DLL 标题当作导出，修正后八项实际导出已核对，原失败也保留。

本批是显式归档依赖离线重放，nightly 更新后的 122 项 xtask/工作区测试、所选 Windows 42 通过/25 忽略、默认 Release、Clippy 与格式检查通过；忽略的桌面测试由各自显式入口负责，不称全部原生通过。独立四调用原版 pilot 的三轮输入/原生记录也保存，未替代最终配对结果。prepare-native 将原版、Rust、记录检查分别保留，原版失败不会省略后续独立证据；即时上传新增 `frame-pool-context/initialization/**`。新托管原生执行仍待验证；CI 预装工具按实际能力与身份记录，年龄不作门禁。证据由参考目录 `local/current-framepool-init-development-path.txt` 指向；已有四项原版 pseudo 黑图、六项 mixed mask 与 Mac 失败、完整初始化/设备损失和 P1 门禁不因此改变。

同一 `0fe79b2` 的两个原生 Mac job 均已终态失败，原型步骤均通过，既有来源组件各为 133/154，21 个失败的名称和全部 154 项状态完全一致，没有因预装工具不是严格最新版停止。实际身份与结果如下；runner 名称和内核身份分别保留，不用标签推造系统版本。

| 原生运行 | 实际 Darwin 内核 | 实际 Xcode / build | 来源组件 |
| --- | --- | --- | --- |
| [手动 macos-26](https://github.com/AbyssRei/maafw-screencap/actions/runs/41549421258) | 25.6.0 arm64 | 26.6 / 17F113 | 133 通过、21 失败 |
| [push xcode-27](https://github.com/AbyssRei/maafw-screencap/actions/runs/41549403347) | 27.0.0 arm64 | 27.1 / 27A9275 | 133 通过、21 失败 |

两份官方即时 ZIP 的摘要/CRC、各 11,941 份文件、11,786 个可用登记路径和全部 154 份日志均核对；每份仍有 582 项完整输入缺口。十二份捕获库、场景、观察器及编排源码分别匹配同一提交的 canonical 字节和各自的原生登记摘要。两边 same-path 观察器均完成 content 转发、提交 1024×768 sample 后，原生等待返回 49，后续十五秒无 sample callback 或 semaphore signal；不同路径及 CG 场景的四项原版/记录正控制在两边均通过。该结果不支持把问题仅归为 macOS 27，也不是只改变 OS 的单变量实验：工具、宿主和独立解析的依赖环境同时不同。它没有证明共同原因、callback 排空、像素生产者或完整 P1；来源、原断言、默认 runner 与旧失败均未改。复核由参考目录 `local/current-macos-environment-comparison-path.txt` 指向。

另对确切 `8c1369c` 的六个原版 native sample 做 56 项登记输入的只读复核：12 份转换前/后 BGRA 副本相同，每张 1024×768 图的全部 alpha 字节均为 255，转换后的每个 BGR 通道均来自同一原生样本；14 张独立 CPU 场景图全量符合 RGB 图样。原版样本在 mode0 三次的差异为 0/24/44 点，mode1 为 52/52/52 点；所有差异都位于至少一个相邻窗口 census 所记录的 `Window Server / StatusIndicator` 矩形中，最大像素边界为 `[1012,4,1020,12]`，记录的窗口边界为 `[1002,-6,28,28]`。census 和截图并非原子同步观察，不能由空间关联确认生产者、原生合成参数或另一组无回调超时的原因，也没有 mask 或改判失败。证据由参考目录 `local/current-8c-macos-sample-census-path.txt` 指向。

手动原生 workflow 现接受 `gh workflow run p0.yml --ref <分支> -f macos_runner=macos-26`，以完全相同的准备、原版/Rust 捕获和记录门禁执行另一个原生 Mac 环境；`macos_runner` 仅允许 `xcode-27` 与 `macos-26`，push/PR 和手动默认仍选前者。`framepool_only=true` 的专项选择不变。GitHub 的[标准 runner 清单](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)包含这两个 arm64 标签；实际 OS/Xcode/SDK/编译器和当前依赖仍随运行归档，预装工具年龄只作参考。新增选择不改变测试/断言/步骤、工具能力要求、资产路径或历史结论；去除新增输入并还原唯一 matrix 表达式后，整个 YAML 结构与父提交完全相等。开发时五个事件/选项组合的模型检查先通过，后续托管路由和原生结果见上表。开发证据由参考目录 `local/current-macos-runner-selection-path.txt` 指向；该额外环境的结果不能替代 macOS 27 结果或完整 P1。

确切 `8c1369c` 原生 macOS 的官方即时 ZIP（40,032,711 字节）摘要、CRC、11,941 份文件、11,786 个实际可用登记路径、全部 154 份日志与十项 AST 输出另已核对，仍保留 582 项完整输入缺口。全部状态与之前批次相同：133 通过、21 失败；相同状态不表示失败发生在相同位置。本次单独的 same-path 观察入口完成 content 回调/转发并提交 1024×768 sample 请求，原五秒等待超时，后续十五秒仍没有 sample callback 或 semaphore signal；`sample_enter` 仅为提交标记，不能写成进入 sample completion。另一个 Foundation consumer 在截图前的几何查询即超时；其他失败原 oracle 的 nullopt 与整屏像素断言各保留原边界，不统称为角落图形或同一种问题。观察器仅在 stderr 的单行输出期间持锁，完成后才调用原 completion；没有据日志认定观察器锁、Apple 服务或像素生产者为原因。参考目录 `local/current-8c-macos-native-review-path.txt` 指向该只读复核，当前失败及完整 P1 状态不变。

确切 `8c1369c` 的独立 Windows [Actions 41542384071](https://github.com/AbyssRei/maafw-screencap/actions/runs/41542384071) 已终态失败，实际组件为 98/102。四项失败仍是原版 `pseudo-stop`、`pseudo-inactive`、`pseudo-layered-stop` 和 `pseudo-layered-inactive` 的已知内部像素断言：各返回 96×64 BGR 图像，6,144 个像素全黑，程序退出 5；source/window 关闭后的 CPU 图像保留检查通过，不据此改判像素成功。其余 98 项保持原生 passed 状态，包括现有 Rust、Size、Map、查询和记录入口；这是独立新 runner 上的既有组件范围，不替代长批次的旧结果或完整 P1。

同次原版四组属性/设备诊断的 27 次固定调用全部完成：24 张符合场景，slot `3/10/11`（两次默认 WGC 存活及一次额外要求边框后的 Desktop Duplication）各有 924 个外三像素差异。三张完整 BGR 与历史六张所选异常目标逐字节相同，不仅是差异数量相同；外两像素 624 点/内一像素 300 点的既有描述因此适用于这些相同字节。实际默认 getter 在该 runner 为 true，额外 true setter 后仍为 true；本机原三轮的默认 false 结果保持自身身份。仅原版设备/swapchain 存活、所有固定销毁后观察与最终无 WGC 控制均符合场景。这是在本次有限执行中出现的对象状态与像素关联，没有确认系统指示器或其他具体生产者，没有证明原生 session/callback 排空，也不把历史六个 mixed mask 或四个伪最小化失败改判为通过。

该专项官方即时 ZIP 与 319,731,055 字节完整 ZIP 的发布摘要及 CRC 均核对；完整 tar 安全恢复 11,900 份文件，三个运行清单的全部登记输入/日志及 F 的 744 个 Git blob 核对通过，关闭的是本次专项的来源登记缺口。原始 tar 保持不变；读取校验引起 U/F 两个 Git index 的文件状态缓存刷新，前后版本另存，暂存条目完全一致，其他恢复文件未变。原版设备片段逐字节匹配 F，六份关键测试/校验器/库源码与原生 product snapshot 的 CRLF 差别明确核算；61 项 border 输入的独立 `--check-only` 验证通过且原目录 62 份数据/报告摘要不变。读取器早期的长路径、目录布局、非 native 配置清单及 canonical 路径假设拒收另行保留，不涉及原生重跑或状态放宽。来源登记闭合不代表完整系统依赖离线重建、回调正确性或 P1。

确切 `8c1369c` 的常规三平台 [Actions 41542383770](https://github.com/AbyssRei/maafw-screencap/actions/runs/41542383770) Linux job 成功：PipeWire 202/202，Wlr 151/151。实际 `linux-none-original-caller` 在 Linux 执行未改 F 头文件和精确提取基类，通过四种 Mat 形状、八次 false、十二次调用方检查、四次虚析构及四份持有调用方；没有桌面捕获。保存的原头文件/基类及六份匹配原生登记摘要的 canonical 来源补件核对，既有四项 Rust native manager controls 也实际通过；脚本化 manager 模型仍是另一范围，不改称原类执行。两个即时包的官方摘要/CRC、全部可用登记输入和 202/151 份日志核对；仍分别缺 615/617 项完整输入，未授予 Linux 全来源闭包或新增完整后端验收。以上专项与 Linux 复核由参考目录 `local/current-8c-native-framepool-and-none-review-path.txt` 指向；预装工具年龄继续只记录，真实像素/构建失败继续保留，P1 未完成。

`mfs_linux_none_reference` 新增固定 F 的真实 `NoneScreencap.h` 调用方测试，使用既有生成器提取的完整 `ScreencapBase` 声明/函数体和明确的测试命名空间；原类/函数体未改写。经真实基类虚调用执行 init、两次 false screencap、inactive 及 delete，覆盖默认空图、release 后保留类型的空图、packed BGR 和非连续 ROI。C++ 内比较 data/start/end/limit、u、flags、dims/尺寸、两项 step、type/refcount/submatrix，以及完整可见像素和 ROI 全底图；source 销毁后同一调用方再检查一次。无 daemon、输入操作或桌面取帧，成功汇总明确 `native_captures=0`。

本机 VS2026/MSBuild 的 Debug 和 Release 各通过四种形状、八次 false 调用、十二次保留检查、四次虚析构及四个 source 关闭后的调用方。六份真实 CL/link 读取日志证明使用确切 F 头文件与所提取基类，244 项编译/链接输入已外部归档；生成文件的 Windows CRLF 与 LF 原片段分别记录摘要，初版 reader 因忽略该差别的拒收也保留。该开发批次使用显式归档原生依赖，证明本机 C++ 原版 header/base 调用边界；后续 Linux 实际执行和同批 Rust native controls 的复核见本节开头。整个 manager/ABI 和完整 P1 仍待验证，既有脚本化 manager 模型未被改写成真实类的执行证明。

原生 PipeWire 来源准备现编译并归档该独立程序、原 None 头文件、生成基类和私有 Conf，单列 `linux-none-original-caller` 步骤；精确核验四种 shape、八次 false、完整头部/像素保留和销毁汇总。十三项实际 `Step` 子进程控制验证完整成功、缺项/重复/额外形状、错误计数、伪称完整验收、原生失败和缺环境，原失败退出保留。输入准备或该检查失败不被改称取帧成功，独立后续检查仍按既有规则执行；轻量 CI 包加入三个来源头文件。参考目录 `local/current-none-native-development-path.txt` 指向本批证据，Rust 函数对应计数仍为 40 份来源、281 项。

工作流的 `framepool_only` 是显式手动专项：`gh workflow run p0.yml --ref <分支> -f framepool_only=true` 在新的 Windows runner 上准备 nightly、实际可用的 VS2026/SDK、当次依赖和确切 F，然后执行既有 `cargo xtask upstream-parity frame-pool` 的 source/Rust 全部现有检查、记录验证和完整工程归档；工具年龄继续按 `preinstalled` 记录。此模式不先运行长时间的 63 组 manager 用例，因此可以较早观察尚未受该 manager 批次影响的属性/设备对照。默认 push、PR 和不勾选该参数的手动运行继续选三平台矩阵。专项的 runner/执行顺序与整批不同，其结果独立标识，不能重命名旧失败、证明排空/生产者或作为完整 P1。工作流八段实际脚本文本在十九个隔离 PowerShell 子进程中验证成功和十一种非零失败，参数/退出及归档路径保留；这些显式 mock 不安装工具、不取帧，也不代替新的托管执行。

确切 `adc14e8` 的 [Actions 41533760567](https://github.com/AbyssRei/maafw-screencap/actions/runs/41533760567) macOS job 已终态失败，组件仍为 133/154，全部 154 项状态与 `b178154` 相同，原 21 个失败保留。官方 40,055,490 字节即时 ZIP 的发布摘要、CRC 和 11,941 份文件核对；12,366 个不同登记路径中 11,786 个实际可用输入、全部 154 份日志及完整十份 AST 输出逐一匹配摘要。清单另含 9,308 次相同摘要的重复登记，已显式计数，不接受冲突摘要；580 项完整输入仍缺，未称来源闭包。实际编译器为 Xcode 27.1 路径内的 Apple Clang 21.0.0，预装工具年龄不作门禁。AST 的编译命令变换、确切 F 的 .h/.mm、三个 namespace 根和四个 block 也独立复核；没有在 Windows 重跑 macOS 捕获、解释像素生产者或扩大全 P1 验收。原始下载/两次因 reader 额外路径/唯一性假设中止的版本均保留，最终 reader 只读使用同一下载记录真实缺项及相同摘要重复项，原生状态未改。完整记录由参考目录 `local/current-adc14e8-macos-native-review-path.txt` 指向。

新增独立原版 FramePool 属性/设备诊断 `mfs_framepool_border_reference`，保留 F 类/头文件/原生函数体，在原 caller MTA/current-thread dispatcher 上观察真实 session。四组各有新窗口：原始默认 WGC、默认 WGC 后额外设置 `IsBorderRequired(true)`、仅执行原版 D3D 设备/swapchain 片段、最终无 WGC 控制。仅设备片段由确切 F 的描述符到一次 `D3D11CreateDeviceAndSwapChain` 调用提取，输出存档并独立核对；没有添加 false setter、修改授权、隐藏指示器或修改产品后端。临时 PMv2 caller DPI 正常恢复，非重叠同输出的更新 marker 保留自己的 UI owner。固定 27 次调用包含 13 项已知场景/取帧控制，销毁后的三次观察次数预先固定；没有按像素结果重试。

`cargo xtask upstream-trace framepool-border <外部观察目录> [--check-only]` 核验完整 61 份原生输入：27 份布局和完整 BGR、原生 OpenCV type/shift、FNV、属性 HRESULT/值/诊断额外 getter/setter 次数（不重计 F 自身的 false 请求）、目标/捕获/UI 线程、窗口状态、对象销毁 QPC、marker 进展/隔离/关闭和完整文件清单。目录须已存在，只读检查不创建拼错的路径；成功的记录生成另添一份 `border-validation.json`。处理组可以保留空帧或不同像素观察，但正控制不能降级；报告始终明确 producer/capture/full P1 均未验收。属性返回 false 不能证明系统边框已消失，额外设置 true 且回读 true 也不能单独证明指示器已经显示或被这次截图采到，见 [Microsoft 属性说明](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired)。C++ owner 析构不代替原生 session/callback 排空证据。

本机 VS2026/MSBuild 实际编译通过；冻结 1,513 份输入（含 CL/link 首要及依赖、确切 F 源码、SDK、生成片段、程序和监听脚本）后按用户已授权的自动启动规则连续三轮。共 81 次捕获、12 个窗口、39 项正控制，81 份完整 CPU 图像均独立匹配 96×64 已知场景；每轮实际默认 getter=false，额外 true setter 成功且 getter=true。四路 Raw Input/低级 hook 鼠标键盘监听零输入，最大采样间隔 78.573 ms，小于 500 ms，桌面前提和注销完成；未声称新的物理输入正控制。本机仍未复现历史外三像素的 924 点差异，不能据此改写 manager 六项失败、证明其生产者或宣告完整 P1。

本批使用显式归档依赖重放；更新后的 nightly、120 项 xtask/工作区测试、Release、Clippy 和格式检查通过，不视为新在线依赖验收。校验器 29 次人工数据调用包含 24 次拒收，另外真实 `Step` 的 12 项控制/24 个子进程核验成功、非零失败、缺环境、完整记录以及失败后继续独立案例；最初不递增的人工 post-call 时钟和外部 harness 的 LF/CRLF 比较失败均保留，另建修正记录。prepare-native 单独保留构建/捕获/记录步骤及失败输入、缺 API 时在捕获前明确标记 `missing_environment`，其他案例继续；即时 CI 包加入 `source-border/**`。新的托管 Windows 执行尚待验证，预装工具仍按 `preinstalled` 政策记录实际身份，工具年龄不阻断 CI。完整证据位于参考目录 `local/current-framepool-border-development-path.txt` 所指批次。

确切 `b178154` 的 [Actions 41520653914](https://github.com/AbyssRei/maafw-screencap/actions/runs/41520653914) 已验证即时上传修正：官方 40,071,377 字节 ZIP 的整体摘要及全部 CRC 通过，11,941 份文件中完整包含十项登记 AST 输出。实际源码摘要、编译命令变换、退出码、三个 namespace 根和四个 block 节点数均独立核对；组件仍为 133/154，全部状态与 `1469603` 一致、21 个失败不变。本复核只关闭新增 AST 的即时上传缺口；原生编译器身份继续按预装策略记录，版本新旧不作门禁，实际编译/链接/测试要求不降低，完整来源、严格像素及 P1 仍未验收。

确切 `1469603` macOS 的官方完整工程 ZIP（359,275,006 字节）及其中 tar 的身份/CRC/摘要现已核对，并仅取回十四项选定输入，补齐全部十项 AST 输出。编译数据库、普通未插桩入口、仅移除 `-c`/object 输出的参数变换、编译器版本、退出码、两份 F 来源、1,967,302 字节原始 AST 与节点计数分别复核；实际 Xcode 27.1 身份继续按预装策略记录，不设置严格最新版门禁。该选取不表示整个批次的来源/依赖已完整恢复。六项普通 callable 和四个嵌套 block 的原始源位置已核对，并用于上述十项 Rust 对应；原 153 项检查状态不变，新增 AST 观察通过，组件仍保留 21 个失败。

确切 `e987a99` 的 [Actions 41514949183](https://github.com/AbyssRei/maafw-screencap/actions/runs/41514949183) PipeWire 原生组件通过全部 201 项检查，原 197 项状态不变，新增两个原版 daemon/FD 交接、一个 Rust 交接及一个记录检查均通过。实际矩阵有 80 个生命周期、240 次返回图像和另加 40 次 Rust raw/converter 展开：每对象三个同时存活的调用线程依次创建/初始化、捕获/停用、捕获/异线程关闭，原生 loop 身份/启动代次不变，最终 task ID 集合及 FD 回到各自基线。独立 Python 复算全部 720 份即时/持帧 BGR/RGB、标记、已知目标像素及逐对象任务记录，归档的独立 checker 只读核对 schema1 报告亦通过；opaque Mat/loader 始终留在各自调用线程。

该次官方 39,816,782 字节 ZIP 的整体摘要/CRC、44,114 份文件和 44,113 个可用登记路径均核对通过。轻量包仍缺 609 项完整来源/依赖输入；另有十一份选定产品源码只在确切 Git 字节匹配原生登记摘要后作为独立 supplement 保存，未伪装成包内下载来源。本结论接受有限 daemon/成功唯一 FD offer 的完整 owner 串行交接回归，不接受任意并发、其他 setup/cleanup 故障、SDK Send/Sync、完整来源闭合或 P1。较早编译/lint 失败及无输入本机结论保持原身份。

确切 `1469603` 的 [Actions 41518392492](https://github.com/AbyssRei/maafw-screencap/actions/runs/41518392492) macOS 已实际通过新增 Clang AST 步骤，日志记录三个过滤 namespace 根、1,967,302 字节原始 JSON stream。组件为 133/154，旧 21 项失败名称不变；这不是截图或函数 census 验收。首次即时诊断包未收新增 `clang-ast/` 目录，现已补 `clang-ast/**` 上传范围；本次原始字节及十项输出登记仍待从完整工程包复核，不从日志长度推断函数/回调语义。

确切 `e987a992ddbee75d6571de8da1cbdf96898f1b8e` 的 [Actions 41514949183](https://github.com/AbyssRei/maafw-screencap/actions/runs/41514949183) P0 PipeWire 通过全部 27 项检查：四项 consumer、四项 row、五项实际捕获和原生 Clippy 均通过，source/daemon 正常退出。独立复核官方 90,569 字节 ZIP 的摘要/CRC、33 份文件及 32 项可用登记路径，轻量包仍缺 544 项完整输入。相较 `b0`，仅 Clippy 改为通过、外层准备失败消失，其他检查未改；本结果不接受新完整 owner 的跨线程运行，旧编译/lint 失败和完整 P1 状态保持原身份。

确切 `b0b1b84` 的 Linux libtest 已编译，四项 portable consumer、四项 row ownership 和五项真实 P0 捕获测试均通过；随后原生 Clippy 在新 `thread_mobility_tests.rs` 的两处 `chunks_exact(3)` 报 `chunks_exact_to_as_chunks`。P0 保持 26 项通过、两项失败，包括真实 Clippy 和其外层准备失败，不能把测试已编译或 P0 捕获通过写成新完整 owner 的线程交接已运行。新修正只将这段测试比较改为标准库 `as_chunks::<3>()`，并显式要求两侧 remainder 为空、像素组数量相等；不禁用 lint，不改生产代码、捕获次数、断言目标或编排门禁。实际修正后 Linux Clippy/新线程回归仍待下一次 Actions。

独立复核官方 91,321 字节轻量 ZIP 的整体摘要、CRC、全部 34 份恢复文件与 33 个可用登记路径，保存五项通过测试的日志和两个确切失败。轻量包仍缺 544 项完整输入；修复前新测试源码只在确切 Git 字节匹配 native 登记摘要后另存，不伪装成已下载的完整来源。原编译失败 `f6df469`、本批 Clippy 失败与后续结果分别保留，没有重跑本机捕获或改写历史无输入多轮结论，完整 P1 未验收。

确切 `f6df469` P0 PipeWire 原生准备为 19 项通过、两项失败：真实 Linux 编译在 `manager_native_offer_tests.rs:158` 报 `E0433`，旧 warm-init 控制使用的 `Native` 原先经 manager 的 `super::*` 引入，而 manager 现已正确改用完整 `Capture`。libtest 尚未运行，捕获和线程交接用例均未执行；不能将此编译失败写成线程、像素或 Send 运行结果。独立复核官方 57,758 字节 ZIP 的摘要/CRC/23 份恢复文件、22 个可用登记路径及两个确切失败日志，轻量包仍缺 542 项完整输入，原失败保持封存。

修正仅在 offer 测试模块显式导入 `crate::backends::linux::pipewire::native::Native`，保留它独立 warm raw-global-init/deinit 的用途；没有换成 product Capture、修改断言、放宽编排或更改生产 owner/锁行为。已检查该测试族其他 `Native` 使用和确切单行差异，本机格式、文档校验不代表 Linux 编译通过；实际重新编译、原版/Rust 线程交接仍待新 Actions。原本 116 项 xtask 等开发检查只保留其原验证范围，完整 P1 尚未通过。

PipeWire 新增私有 `native::Capture`，在同一实现中拥有原 `Native` 与 `Consumer`，并替换 manager 中分离的两个 owner。对外只给完整 init/capture/inactive/close 与标量观察，不实现 Driver、Deref，不返回裸 driver/锁/回调/映射。完整调用的真实 loop guard 在返回和展开时结束，转换在解锁后调用且不保存在 owner；稳定 Box 回调/hook 地址不随 owner 移动，缓存只有独立 CPU Arc，没有未结束 map/buffer lease。局部 unsafe Send 依这些源码不变量及固定 F 头文件允许串行异线程调用的契约建立；没有 Sync、并发调用、SDK Session/worker、绝对等待或恢复策略。底层 Native/Resources 和原生 map 保留既有线程限制，不能由新完整 owner 的 Send 推导它们也能直接移动。完整 close 同步重置 consumer counter；未连接 init 的 stale-state 清理也重置，已连接 early success 不重置，保留来源行为。原 raw global init 政策不变，测试清理单独配平。

新增原版模式直接执行未改写的 F 完整类；daemon 和成功 FD 各二十对象，三个同时存活的调用线程依次构造/init、两次 capture/inactive、一次 capture/close/destruction。以实际 TID、同一个 loop 的 start generation、四份 live task 记录及最终 baseline IDs 核对，不能因线程 ID 已复用而冒充交接。Rust 使用同样三线程/两模式，各线程自己的 Mat/loader 在交接前结束，只传完整 Capture 和独立 CPU 数据；另在第二线程故意展开一次真实 raw-copy 后的 converter，捕获展开后交给仍不同的第三线程取锁/恢复并关闭。展开是 Rust 独立安全控制，不声称对应原版转换异常；正常返回图像共 240 次，另有四十次 Rust 展开取样。资源检查在实际 join/stop/destruction 后执行；仅允许已有严格观察器识别的已知代次 kernel retirement，不为活动线程或捕获失败重试。

`cargo xtask upstream-trace pipewire-thread-mobility <外部矩阵> [--check-only]` 要求两侧/两模式共八十生命周期、每对象三个不同且不属于 baseline 的调用 TID、同一原生 loop 代次、FD/task 配平、成功 offer 已关闭、Rust 四次 raw copy/dequeue/queue 与零原生错误、四十次展开记录，以及 720 份完整 current/held BGR/RGB 数据。图样由 producer marker 解码全目标核对，来源/Rust 独立取样不强求不同时间 marker 相同。不透明 Mat/loader 的跨线程能力、任意错误/并发与最终 SDK 仍未验收。四个编排步骤独立保留原版两模式、Rust 与记录失败，零测试、忽略、非零退出和部分汇总不能通过；fixture 超时不冒称原生可取消。

本机沿明确封存 Cargo/vendor 图通过 Workspace（含 116 项 xtask）、Release、Clippy 与格式检查；新 C++ helper 仅经封存 grammar 语法解析，尚无本机 Linux 编译/执行。一个完整人工矩阵完成十六次实际 CLI 控制，十四类坏记录被拒绝；十四个真实 pwsh 子进程验证五项通过、九项拒绝和失败后继续。这些是数据/编排开发验证，不是原生交接、Send 证明或完整 P1。确切旧 native/thread-loop 来源与官方锁/停机规则另存外部记录；新的原版 C++、Linux Rust 测试及上述线程边界运行仍待 Actions。本批旧 function-map 保持其 fd41 快照，未将变更后的 owner/manager 自动重标为完整当前对应，没有运行本机桌面捕获或改写既有无输入多轮结论。

确切 `f7fc1fe` 的 Linux job 现已成功，PipeWire 为 197/197：原先三项 parser-fixture 失败对应的新检查全部通过，检查清单与旧 `1d0855a` 无增删，其余通过身份保留。八例 media/raw parser 的保存返回码均为 `2`；标准 raw 观察首属性，未改写的 F helper 经原 `==0` 前置条件进入手动回退并保留最后属性，五项接受、三项拒绝，Rust 全部相同。生产 `native_pod.rs` 仍为原摘要，先前怀疑的生产解析器缺陷未复现；旧 194/197 和全部失败继续封存，不能重命名为成功。

官方轻量包 38,342,335 字节长度、整体摘要、CRC、43,228 份恢复文件及 43,227 个可用登记路径独立核对。另逐字节解析全部八份真实 POD 的 object、媒体/格式/尺寸 key、属性顺序、类型/长度/填充和 Choice(None) child，与实际来源/Rust 记录一致；使用此前独立封存 schema2 validator 的只读报告检查通过，数据未改。轻量包仍缺 605 项完整来源/依赖输入，本批不宣称完整闭包；八例通过只关闭该局部回归，不接受上文新增线程 owner、所有 PipeWire 或完整 P1。新的线程开发验证也没有替代这批 native 结果。

确切 `1d0855a` 的原生 PipeWire 组件为 194/197：新增原版分支夹具、Rust 一致性及记录三项均失败。实际首例 raw SPA parser 返回 `2`，尺寸 8×3、首格式 BGRA 正确；夹具误要求 `code==0`，保存第一份 POD/来源行后即停止。该例原版/Rust 相同，另外七例没有执行；Rust 的八例数量断言及记录完整性拒收均保留为失败。此前提出的不支持标准格式触发回退差异没有复现，不能据此修改生产解析器。原有 28 项格式/转换的原版和 Rust 检查均通过，其他 194 项保持实际身份。

官方轻量 ZIP 的 38,330,617 字节长度、整体摘要、CRC 和全部 43,216 份恢复文件核对通过，43,215 个可用登记路径匹配；仍缺 605 项完整输入。生产 `native_pod.rs` 不在轻量包中：只在确切 Git 字节匹配原生登记摘要后，另存单文件重建及其来源说明，不伪装成已下载字节或闭合其他输入。初次 reader 错误要求轻量包含该文件而停止，其脚本、拒收与部分复核数据保留；修正后的新目录完成上述检查。当前记录仅有 raw parser 的实际返回码，media parser 的返回码由下轮新增观察取得；完整来源、八例一致性和 P1 尚未验收。

修正后的夹具增加 media 返回码/类型、raw 返回码及原版结果记录；所有八项先保存再检查整体前提。新前提要求这组有效 POD 的 SPA 成功返回值为正，保留原 F `==0` 判断及其手动回退，不把上游来源改成 `>=0`。标准 parser 观察首属性，未改写的原 helper 按实际回退顺序取最后属性；支持/不支持的判定因此分别覆盖前后重复项和 `Choice(None)`。Rust 仍逐项与原 helper 比较，并在最终严格断言前保存八项；生产 `native_pod.rs`、原 28 项入口及捕获行为均未改动。记录升级为五项 ID、十六列来源和 schema2，保存两个实际返回码序列，旧 schema1 数据与失败不改写，按其封存 validator 重放。

本机检查 nightly 未更新后，沿明确封存 Cargo/vendor 图通过 Workspace（含 114 项 xtask）、Release、Clippy 和格式检查；没有新在线依赖验收。两个明确人工矩阵完成十八次 CLI 调用，含十四类拒收，新增拒绝把 media/raw 正返回码写成零；人工 POD 只检查记录/字节范围，没有送入原生 parser。十二个实际 pwsh 子进程再次验证当前 Step 的五项通过、七项拒绝及失败后继续，不能用零测试、忽略、非零退出或诊断差异替代严格 Rust 通过。修正后的实际 C++/SPA 和 Linux Rust 编译及完整八例执行仍待新 Actions；本批没有新捕获，不改写既有本机无输入多轮、其他原生失败或完整 P1 状态。

以下三段为 `1d0855a` 前的历史准备记录；其成功返回码前提现已被上述原生首例否定，不能继续作为当前分支复现结论。

PipeWire 源码审阅发现一项待原生实证的分支差异：F 的标准解析已获得完整尺寸和非 Unknown 格式时立即返回，之后由原格式表拒绝不支持的格式；Rust 把不支持格式折叠为 Unknown 后可能进入手动回退，改选后面的重复属性。新增独立 `--format-branches` 原版探针，以真实 SPA builder 构造八份有效范围内的 POD，调用未改写的固定 F helper 与标准解析器，记录直接 BGRA、NV12、Unknown、重复属性及第二项 `Choice(None)` 的实际顺序。Rust 的 `matches_original_standard_result_and_fallback_selection` 在最后断言前保存全部八项结果；原 28 项格式/转换入口不变。生产 `native_pod.rs` 暂未修改，先等待实际 Linux 原版/Rust 结果，不把源码推断写成已复现缺陷。

`cargo xtask upstream-trace pipewire-format-branches <外部矩阵> [--check-only]` 只核对八项原生分支前提、Rust 观察、三份记录文件摘要和八份 POD 的实际字节范围，允许完整记录包含差异；`parser_parity_accepted` 按实际差异计算，诊断始终不授予捕获或 P1。编排另以单独 Rust test gate 要求全八项一致及真实一项测试通过，零测试、忽略和非零进程退出均不能通过。新记录检查不会把严格 Rust 失败变成原生捕获成功。

本机先检查 nightly 未更新，再沿封存 Cargo/vendor 图执行 Workspace（含 114 项 xtask）、Release、Clippy 与格式检查。两个明确人工矩阵的 CLI/只读控制及十二类损坏数据拒收通过，包括整体一致地改写 ID/记录后仍须拒绝旧报告；这些人工 POD 只测试范围/记录格式，未送入 native parser。十二个最终实际 pwsh 子进程验证当前 Step 的通过、拒绝、诊断差异和失败后继续，初轮外部驱动在最后分组计数处失败的代码/日志/输入另存，纠正驱动后在新目录完成控制。新回归对固定 F helper 的调用、真实枚举/布局及 C++/Linux Rust 编译执行仍待新 Actions；本批不运行截图，不改变既有无输入复测、原生失败或完整 P1 状态。

确切 `d915c42` 的官方完整 macOS ZIP 随后通过 358,744,270 字节长度、整体 SHA-256 和 CRC 校核；其中 1,819,214,336 字节 tar 恢复 30,599 份文件。十二份清单的 29,042 个唯一登记路径全部匹配，十一份普通产品快照各有 509 个 Git 文件，另单列一份故意不可构建的准备控制。76 份选定 F 来源、完整 U/F 的 741/744 个 Git blob 路径与原 tar mode、实际对象库中的 736/739 个唯一 blob body 均已核对。完整参考校验器及十六份保存报告只读通过，原恢复字节和 Git 元数据未变；原轻量审阅的 11,930 份文件、222 份 schema6 矩阵文件也全部匹配，闭合其 570 项输入缺口，未改写或替代此前封存记录。

编译上下文另核对六个固定原类实例，其中四个未加观察宏；新增别名只进入独立 native sample 原类目标，其三个 wrapper/observer 编译项另具显式观察宏。五份实际固定 F 编译来源及普通原版、同程序场景/原调用包装均保持各自身份。该批实际预装选择仍为 Xcode 27.1（27A9275）、SDK 27.0，按 CI 能力政策记录，不重标为严格最新版。此处完成的是该批完整来源、保存数据和编译上下文闭包，没有重新取帧；132/153 及全部 21 项原生失败保持不变。转换前原生像素的差异、实际生产者、历史或普通路径根因、callback 排空和完整 P1 仍各按未完成范围处理。

确切 `d915c42` 的 [Actions 41480051638](https://github.com/AbyssRei/maafw-screencap/actions/runs/41480051638) macOS job 已结束，组件 153 项中 132 项通过、21 项失败。原 148 项没有删除或状态变化；新增原生模块链接、三线程/双 translation unit 关联控制及 schema6 记录检查通过，两项严格 RGB 捕获仍失败。Objective-C++、Apple SDK 链接、selector 转发和六次真实 sample 回调均已执行；此处按实际预装工具和所需能力验证，不以严格最新版匹配作为 CI 门禁。

官方轻量 ZIP 的 39,946,845 字节长度、整体 SHA-256 和 CRC 已核对，11,929 个可用登记输入/日志、222 份新矩阵文件通过独立审阅，仍有 570 项完整编译/父来源输入缺失。六份真实 BGRA 在原 completion 前后逐字节相同，每份原版 BGR 的全部 RGB 像素均能从对应 native 行复算。六次输入分别有 4、32、52、52、52、52 个场景偏差，均位于右上角 `[1012,4,1020,12]` 范围内；本批差异已在转换前存在。十二张原版/Rust 图像及 36 份即时/持帧文件均已复算，六组成对图像中只有两组全幅相同，十四份场景 CPU 视图则全部符合三色图样。最终封存校验器只读接受保存的 schema6 报告，11,930 份轻量文件均未改变。初次审阅选中了同一开发归档内的早期校验器，因未知新增字段拒收；旧脚本/工具/错误保留，随后使用已封存的最终工具通过，没有改写原生记录或重跑捕获。额外锁和复制影响诊断时序，这些结果没有确定像素生产者、证明未观察路径或历史帧的根因、callback 排空或完整 P1；所有旧失败及新增严格失败保留。

新增独立 `mfs_macos_native_sample_scene` 和原类 object target，只有该 object 的固定 F `ScreenCaptureKitScreencap.mm` 编译时获得私有 selector 别名；普通原版、Rust 与场景查询保持原入口，不做运行时 swizzle。每次原版调用创建独立 shared owner，原 `SCShareableContent` 回调捕获该 owner，在同步转发期间用 thread-local scope 将它传至 sample 提交，再由真正 sample 回调持有；不通过可被下次调用覆盖的全局 current-call 指针关联晚到结果。记录真实首次显示器的数量/id/尺寸、原 filter/config 的存在、BGRA/sRGB/cursor 配置和实际线程。原 sample 回调前、原 completion 返回后各在成功 read lock 内验证格式/布局、复制初始化目标行并 Unlock，CPU 数据独立保存，不暴露 padding 或跨回调保留原生 sample/像素借用。

同程序新增 `records_original_native_samples_before_and_after_source_conversion`，仍用两个 fresh parent/scene 组运行 original-first、Rust-first，各六次捕获及七次 CPU-view/census。原返回和 elapsed 在额外观察等待前保存；最长两秒只等待本观察器的 post-forward CPU copy，不再次截图、不使用晚到图像替换返回，也不证明 framework block 已释放/注销。新 `cargo xtask upstream-trace macos-native-sample <外部矩阵> [--check-only]` 使用独立 schema6：六份原生输入的前后快照必须逐字节相同，并复算全部原版 BGR 像素；真实选源/配置与场景、调用/线程/模块身份也须一致。它保留完整 RGB 偏差记录和 `capture_parity_accepted=false`。额外锁、复制、getter 和等待会影响诊断时序，不能反推历史帧的像素生产者；原五秒等待、最终严格 RGB 断言、旧场景/模块/失败及完整 P1 门禁均保留。编排只在新批次临时选择观察模块，所有成功/失败路径都恢复此前存在或缺失的模块环境。

本机沿封存 Cargo/vendor 图执行 Workspace（含 112 项 xtask）、host Clippy、Release 核心和 Apple 目标 Rust 类型/Clippy；这些没有在 Windows 链接 Apple SDK 或运行 Mac 程序。VS2026/MSBuild 真实编译了共用关联模板的两个 C++ translation unit，三条线程验证旧 content 晚到、反向 sample、嵌套 scope 恢复和最终 CPU owner 释放，没有调用捕获 API。两份人工矩阵、二十二类坏记录拒收、四个旧命令拒收新 profile、三个真实历史保存报告只读、三十三次 Step 子进程 mock 及八组模块环境/失败继续控制通过。人工 native 输入与输出相同但偏离场景的用例只通过诊断，未获得捕获验收。首轮 Clippy 拒绝固定尺寸 `chunks_exact`，改用 `as_chunks`；首轮 Step 控制捕获新入口漏列 fresh missing-environment 分类，补入后在新目录通过全部控制，初版源码/失败日志保留。旧捕获/像素 helper、场景绘制和严格断言尾部另经源码比较，三份历史报告摘要不变；这些开发检查保持原范围，后续原生执行结果以上述确切批次为准。

确切 `bc90d12` 的 [Actions 41456456292](https://github.com/AbyssRei/maafw-screencap/actions/runs/41456456292) Linux job 已终态成功。当前 Wlr 151 项全部通过，包括 Clippy、真实 syscall 范围控制、memfd EMFILE 与 ftruncate EFBIG。独立读者核对双方各二十轮的错误、未覆盖调用方图像、失败资源状态、精确限制/信号恢复、正常恢复取帧与关闭后持帧；280 份实际像素、二十行 Rust memfd 范围记录、正常 producer 计数/退出均一致。单独原生控制仍实际观察零 NOFILE 下单项 poll EINVAL、真实 EMFILE、两次普通调用及故意展开后的限制恢复；这些当前记录不追写旧失败的未观测分支或 errno。

官方 Wlr 轻量 ZIP（3,885,308 字节）的整体摘要、长度和 CRC 核对，9,034 份解出文件及 9,033 个可用登记输入/日志通过；memfd schema2、ftruncate schema1 与 mmap 保存报告在当前封存校验器上只读通过，矩阵字节未变。仍有 598 项完整输入缺口；本批成功不替代之前 `d139b85` 已封存的完整来源闭包，也不把其来源身份直接推广到本批。全函数当前对应、其他平台失败和完整 P1 继续待验收；既有本机无输入多轮与历史失败不改。预装工具按实际身份和所需能力验证，严格最新版仍不成为 CI 门禁。

确切 `bc90d12` 的 [Actions 41456456292](https://github.com/AbyssRei/maafw-screencap/actions/runs/41456456292) macOS job 已终态失败；Objective-C++ 参考程序和真实 Rust 捕获均实际执行，组件为 129/148、十九项失败。与 `7b82fe8` 的旧 145 项逐名核对，没有删除或状态变化；新增的两个最高层级 RGB 用例失败，独立 schema5 记录检查通过。两臂均实际记录 ScreenSaver 1000、Maximum/requested/getter/census 2147483631，owner、窗口、七次请求及关闭一致；没有因预装工具落后于最新发布而阻断。

独立读者核对最高层级矩阵全部 204 个登记文件、十二次取帧与 36 份当前/持帧字节、原版/Rust 调用顺序和十四张正确 CPU 视图，并逐字段复算保存报告的完整像素差异。original-first 的原版三张分别偏差 0、24、44 个像素，其 Rust 三张均偏差 52；Rust-first 的双方六张均偏差 52。全部十二张中仅首张原版图像完全符合 RGB 图样，六组逐次配对只有后三组字节相同。非零偏差仍在右上角 `[1012,4,1020,12]` 的 8×8 范围内，较小图样的包围框相应收缩，未遮罩任何像素。实际 census 在首个无 StatusIndicator 的观察后，将两份 layer 2147483630 的 StatusIndicator 列在自有 layer 2147483631 窗口之前；保存列表顺序不符合简单按层级数值排序的假设。该元数据和分时 CPU 视图仍不能证明谁产生了截图中的偏差，也不把提高层级解释为已经解决问题。

官方轻量 ZIP（38,696,354 字节）的整体摘要、长度及 CRC 通过，11,701 个可用登记路径和所有日志摘要核对；十五份保存报告通过当前校验器 `--check-only`，全部 11,702 份解出文件的前后摘要不变。轻量审阅当时另缺 561 个完整输入路径，包含编译源码与父组件，未授予完整来源闭包；后续补核见下文，原轻量封存不回改。初版外部读者错误要求所有输入属于当前组件，在创建结论目录前拒绝；现按既有规则把其他组件输入计为缺失，不打开外部路径或修改原生结果，初版与修正记录均保存。审阅在同级参考区 `runs/p1-bc90-macos-maximum-native-review-*` 独立封存，未重跑捕获或改写旧批次。旧十七项失败、新两项严格 RGB 失败、回调排空和完整 P1 的边界继续保留。

该批完整官方 ZIP（355,678,373 字节）现通过整体摘要、长度及 CRC，内部 tar 为 1,634,622,464 字节。独立恢复全部 30,285 份文件，十二份 manifest 的 28,728 个登记输入/日志路径核对；本批全部 561 项轻量缺口补齐。十一份普通产品快照逐项匹配确切 `bc90d12` 的 502 个 Git 文件，另将故意不可编译的准备控制快照单列；76 项所选 F 来源、完整 U/F checkout 的 741/744 个 Git 路径和 tar 执行 mode 匹配。对 Windows 长路径下的可选 `alternates` 查询诊断另用 verbatim 路径核对实际文件不存在，并从真实 Git 对象库读取 736/739 个唯一 blob 正文，逐个重算 Git ID、匹配 checkout；没有借用其他对象来源或改写 Git 元数据。

五条真实 `ScreenCaptureKitScreencap.mm` 编译记录含四条未开启 dispatch observer 的入口，原 oracle、同程序 scene 与 original wrapper 的实际 target、MRC、Developer/SDK 路径和五项原版来源身份均一致。完整引用校验及十五份保存数据检查只读通过，最高层级矩阵的全部 204 个文件与原轻量封存逐摘要相同，30,285 份恢复字节保持不变。新完整审阅在同级参考区 `runs/p1-bc90d12-macos-maximum-full-review-*` 封存；这里补齐的是本批登记来源/输入/编译上下文，不把 Windows 读者当作新的 Mac 执行，也不证明系统像素生产者、回调排空或完整 P1。macOS 组件仍为 129/148、十九项失败，预装工具的实际身份仍不是严格最新版门禁。

macOS 现新增独立的自有窗口最高标准层级对照，分别运行 original-first、Rust-first，逐臂保留六次原取帧、七次 CPU 视图/窗口 census、持帧及整幅 RGB 断言。夹具通过 [CGWindowLevelForKey](https://developer.apple.com/documentation/coregraphics/cgwindowlevelforkey%28_%3A%29) 查询 ScreenSaver 与 Maximum，只设置自有 NSWindow，并要求 getter、实际 census、owner、关闭和查询记录一致。Apple 将这个查询定位于管理窗口的应用框架；这里用于独立验收夹具，没有成为 SDK 策略。其层级对照依据 [AppKit 的层级叠放语义](https://developer.apple.com/documentation/appkit/nswindow/level-swift.struct)，实际像素结果和历史 52 像素偏差的原因仍须各自观测，不能由层级元数据推定。

新增 `cargo xtask upstream-trace macos-maximum-level <外部矩阵> [--check-only]` 输出独立 schema5 `maximum-level-validation.json`，要求明确选择、十二字段层级/所有者记录、当前 schema2 视图/census、两臂完整原版/Rust 结果和持帧；旧命令拒绝该新 profile，原 schema/保存报告保持可读。记录通过只说明完整诊断数据可复核，原生 RGB 断言失败仍使对应命令失败。产品捕获源码、原版调用、默认 ScreenSaver 场景、RGB 绘制/等待和最终像素断言均保留；没有给出像素遮罩、原因归属、回调排空或完整 P1 验收。

提交夹具前，本机沿封存 Cargo/vendor 输入验证 Workspace（含 110 项 xtask）、Clippy、Release 核心和新 CLI；Mac 目标 Rust 测试/绑定也通过交叉类型/Clippy 检查，未在 Windows 链接 Objective-C++/Apple SDK 或执行 Mac 二进制。两份人工像素/层级矩阵、只读回放、十四类新坏记录拒收、三个旧命令拒收新 profile、三份历史只读报告、三十次实际 Step 子进程 mock 和四种编排失败继续控制通过，没有新原生截图。初次控制驱动完成断言后保留了最后一个预期 exit17，显式修正驱动成功退出并在新目录重跑，实际子进程退出码/状态未改。另一次 Git archive 模式核对捕获默认 tar umask 与 Git mode 的区别，原 tar/失败保留，显式 umask 的另一份 canonical archive 核对 501 个原 Git blob/mode 通过。官方 nightly 更新确认未变；后续实际 Objective-C++ 编译、SDK 链接和真实新场景的原生结果见上文。

确切 `71237c7` 的 [Actions 41438918273](https://github.com/AbyssRei/maafw-screencap/actions/runs/41438918273) Linux job 已终态成功。其 P0 PipeWire 原生组件的 27 项检查全部通过：实际观察 `1.6.11`、完成源码准备/构建、五项捕获测试、consumer 与行所有权控制、Clippy，以及二十次流生命周期；逐轮均有 32 次 dequeue/queue、独立 callback/owner 线程和 native buffer 重用记录。自有 source 与 daemon 的退出均为 exit0、非意外、未强制终止。这里只读检查已有原生日志，没有重复运行捕获，也没有改写旧 `d868936` 的解析失败。

官方轻量 ZIP（89,911 字节）长度、摘要和 CRC 通过，33 份归档文件及 32 个可用登记路径核对；仍有 529 项完整输入缺口，包含原始 release 响应与 `pipewire-source-identity.json`。因此本批成功只说明修正后原生构建/该受控取帧路径完成，尚不能确认 CI 是否实际遇到 null commit 并执行同 tag 查询。该分支仍保留前述本机实际准备分支控制的明确范围；完整元数据/源码闭包、P1 PipeWire 全部对照及完整 P1 继续待验收。预装工具按能力与实际身份记录，版本不是严格最新仍不构成 CI 门禁。

后续独立补核只恢复上述缺口中的一份来源身份文件：对已封存 Windows nonnull 控制输出仅作 CRLF → LF 变换，结果与 CI manifest 登记的 SHA-256 完全一致。文件明确记录 `commit_source=release.commit`、`release_commit=4287f31510771fc999f8a58a024c81ca10b8d55f`；同一原生清单也没有 tag 查询步骤或 tag 响应输入。因此这批实际 CI 走普通非空 release commit 分支，没有执行 null 后查询。这是精确登记摘要的内容重建，不是从新完整 CI 包解出，也不是把 mock 运行重标为原生控制。原始轻量审阅/封存不改；另有原始 release 等 528 项输入、实际 checkout/完整源码闭包和完整 P1 仍待核对。

确切 `d139b85` 的 [Actions 41435720011](https://github.com/AbyssRei/maafw-screencap/actions/runs/41435720011) Linux job 已终态成功，Wlr 组件列出的 151 项全部通过，包括原生 Clippy、真实 memfd 范围控制及两类资源故障。原 C++/Rust 对 memfd EMFILE、ftruncate EFBIG 各完成二十轮：失败 owner/fd、原输出、精确限制及信号恢复、正常取帧和关闭后持帧逐行核对，280 份像素与二十行 Rust syscall 范围数据独立通过。独立原生控制实际观察到零 NOFILE 下单项 poll 的 EINVAL、真实 EMFILE、两次未启用探针时正常调用及被捕获展开后的限制恢复；其故意 panic 消息属于通过的展开控制，退出为零。该独立控制不追写旧 `d868936` 未直接记录的 poll errno。新 memfd schema2、原 ftruncate schema1 与 mmap 保存报告只读检查通过，原数据未改。

检查数从 `d868936` 的 155 变为 151，差异逐项核对为五个按需 clone 步骤没有重复执行，新增一个原生范围控制；相应 tag 分页/正式 release 查询和 source archive 全部仍通过，未省去任何旧原生用例。初次外部 reader 误把查询名统一写成 `-latest`，在写结论前拒绝；实际名称为 `-tags-N`/`-releases`，旧程序和失败保留，随后按现有真实分组核对，native 状态未改。官方轻量 ZIP（3,885,312 字节）的摘要、CRC、9,034 份文件和 9,033 个登记路径通过，最初留下 597 项完整输入缺口；其独立轻量报告按原状态封存，后续完整包核对不改写它。

该批官方完整 ZIP（632,648,730 字节）现通过长度、整体 SHA-256 和 CRC；遍历 tar 的 93,495 项，选择性恢复 16,305 份 Wlr/F/直接 Rust 父输入。全部 9,630 个登记路径及 597 项轻量缺口、501 个产品 Git blob/mode 和单独 Cargo.lock、十五份实际 F 源码/头/协议文件的 canonical Git 字节与 mode、八条原版 target 编译记录均核对。新 Rust syscall 探针、未放宽的故障/恢复断言和实际 C++ 包装输入也绑定到确切 head。传输先保留原 gh 部分文件及单连接 range 部分文件，随后以分块并发恢复得到官方整体摘要匹配的完整替代；仅在完整替代再次核验后关闭确切的冗余 gh helper，原生 Actions 未取消或重跑。其父脚本因该传输被主动停止而返回非零，按传输记录保存，不当作捕获失败。此结论只覆盖所选输入，其他完整归档组件、更深依赖父图、组合错误、当前全函数对应、其他平台失败及完整 P1 仍未验收。

PipeWire 准备现处理实际下载响应的 `commit:null`：发布字段仍须含已发布状态与有效 tag；缺少 commit 时通过 [GitLab 单 tag API](https://docs.gitlab.com/api/tags/#retrieve-a-single-repository-tag) 查询同一 tag，严格校核名称、完整 Git 身份及可用时的 release/tag 一致性，统一十六进制大小写。两份原始响应和新增 `pipewire-source-identity.json` 均登记摘要；已有 checkout 的 HEAD、dirty 检查保持，不在查 tag 失败时选旧版本或缓存 commit。完整工程包的失败 P0 组件已核对 531 个登记路径、全部 516 项轻量输入缺口及另一份 500 文件的产品 Git blob/mode；首次只恢复组件前缀，reader 因缺少单独的 Rust 父 manifest 拒绝完整结论，随后从官方 tar 补回确切登记的这一项并核验摘要，原恢复文件和 native 状态未改。其父输入图未因此扩大验收。

本机直接重现旧 `commit.id` 访问异常；同一 `1.6.11` 的真实 Tag API 与 Git 远端 ref 均为 `4287f31510771fc999f8a58a024c81ca10b8d55f`，只作为本批证据，不写成未来版本约束。27 组元数据控制包含 19 组缺失、错误类型、未发布、标签/commit 不符拒收，以及待查询状态和完整身份；八次直接执行实际准备分支，只有 HTTP 传输被 mock，核对正常 release 不多查询、空/缺失 commit 的同 tag 查询、URI 编码、保存摘要及四种失败均停在源码选择前。这些检查没有执行 Linux 源码构建或取帧。资源探针修正已独立提交 `d139b85` 并启动原生 CI；此解析修正继续单独提交，不重跑或改写旧失败，也不改变预装工具的能力门禁政策。

确切 `d868936` 的 [Actions 41427067904](https://github.com/AbyssRei/maafw-screencap/actions/runs/41427067904) 原生 Wlr 编译及 Clippy 通过，组件保留 152/155、三项失败。`ftruncate` 的原 C++ 与 Rust 各完成二十轮真实 EFBIG、新待处理 SIGXFSZ 的消费、精确限制/mask/handler 恢复、失败 fd 关闭、恢复及关闭后持帧；140 份配对像素独立核对，原 schema1 报告只读检查通过。原版 `memfd` 的二十轮及 60 份像素也通过；Rust 首轮却在 Buffer 回调前以 `wl_display_roundtrip/-1` 退出，记录为 `buffer_error=0`、未构造 buffer、转换零次，producer 仅一份 offer、没有 Copy。该 Rust 入口、未完成的服务计数与配对记录三项继续失败，没有实际 EMFILE 或恢复验收；旧 mmap 的保存报告另只读通过。

针对这份实际失败，只把 libtest 的 NOFILE 压力缩到 UUID 已构造后的真实 `memfd_create` 调用，返回原 syscall 的 Result，并在继续 callback 前恢复原软/硬限制；普通库构建不包含探针，原 native 参数、调用、错误、owner、输出和恢复断言保持。归档的 `wayland-client` 在 blocking_read 中使用一个 poll 项；按 [Linux poll 契约](https://man7.org/linux/man-pages/man2/poll.2.html)，nfds 超过 NOFILE 限制会得到 EINVAL，因此整个捕获期间设零会干扰等待。这是源码/契约推断，旧批没有直接记录 poll errno；新增独立原生控制会实际检查忽略 fd 的零时限 poll、真实 EMFILE、未启用时正常调用及展开后的限制恢复，尚待 Actions，不能把推断写作旧批直接观测。

新 `memfd` 报告显式为 schema2，额外要求二十行 `rust-memfd-syscalls.tsv`，逐次匹配范围进入/实际返回、EMFILE、限制前/零/恢复后的精确值；未进入 callback、错误位置或恢复不完整均拒收。已有 `ftruncate` schema1 字节和只读回放不变；旧 memfd 人工记录仍按其归档校验器和身份保存，不充作新范围验收。Workspace 含 108 项 xtask、Windows Clippy、格式、PowerShell 语法、一份人工完整矩阵/只读检查、十二组范围损坏数据拒收和二十八个真实 Step 子进程 mock 通过。初次人工控制捕获到调用旧 `xtask.exe` 仍生成 schema1，原程序/记录保留；显式重建当前 CLI 后新目录验证通过。另在独立外部诊断 crate 用真实 Linux std/libc/rustix 类型检查单个测试模块及 Clippy 通过，没有 mock native 声明；它没有编译完整捕获 crate、链接或运行 Linux 二进制。沿归档依赖开发，nightly 检查未变，新增 native 控制与修正后捕获仍待原生执行，原版源码、mmap 和 ftruncate 断言未放宽。

本批轻量 ZIP（3,847,238 字节）、完整 ZIP（561,351,096 字节）的官方长度、摘要和 CRC 通过；完整包遍历 84,402 项，选择恢复 16,737 份 Wlr/F/直接父输入及失败 P0 PipeWire 输入。9,532 个 Wlr 唯一登记路径、596 项轻量输入缺口、500 个产品 Git blob/mode、十五份原 F blob/mode 和八条原版编译记录通过，未授予其余归档组件或 P1 验收。完整包同时取回当时的 PipeWire 响应，确认 `commit:null`、tag 为 `1.6.11`；后续有效响应不替代它。此源码身份解析缺口将独立修正，不改为预装工具版本门禁或旧依赖回退。

确切 `b563069` 的 [Actions 41424109770](https://github.com/AbyssRei/maafw-screencap/actions/runs/41424109770) Linux job 已终态失败，Wlr 组件保留 111/150、39 项失败。二十二项日志均记录新测试在 `wlr_resource_tests.rs:279` 的 `usize`/`rlim_t` E0308；Rust libtest 未能编译，mmap 及两个新资源回归均未运行，后续配对记录和未消费的服务端也保留真实失败。前述 `d868936` 在观察本批结果前已修正比较类型，其原生运行另作判定，不能据此改写本批状态。

同批原 C++ 的 `memfd`、`ftruncate` 各二十轮实际通过：真实 EMFILE/EFBIG、构造后的失败对象不留 memfd、截断失败的确切 fd 已关闭、原输出未变、原软/硬限制和线程 mask/handler 恢复、正常恢复与关闭后持帧均逐行核对。独立校验 120 份原版 sentinel、持帧与 producer 像素，两份原版服务各完整结束二十个 client、四十个 frame、二十次 Copy/Failed。轻量 ZIP（1,461,081 字节）及完整 ZIP（632,130,386 字节）通过官方长度、摘要与 CRC；完整包遍历 87,807 项，选定 10,549 份 Wlr/F/直接父输入，核对 3,875 个唯一登记路径、全部 596 项轻量输入缺口、500 个产品 Git blob/mode、十五份原 F blob/mode 和八条原版编译记录。初次 reader 对两个记录文件的全字节相等、scope 字面拼写作了错误假设，失败程序与恢复字节保留；随后按共同检查字段、扩展输入集合与实际 scope 验证，并将全部恢复文件再次只读对照原 tar，没有重捕获或改写原生状态。原版局部成功不表示 Rust、完整归档其余组件或 P1 通过。

`d868936` 的独立 P0 PipeWire 原型入口另在依赖准备时失败：curl 下载发布信息返回零，随后读取 `commit.id` 时抛出属性 `id` 不存在的错误，尚未进入源码准备、编译或取帧。轻量 ZIP 的 16 份文件及 13 项通过/1 项失败已核对，仍缺 516 项完整输入，其中包含当时的发布响应；后续单独查询含有效 commit 字段，不能替换失败响应或证明原因。其工具身份检查通过，本项没有因预装工具落后而失败，也不以缓存旧依赖绕过在线库解析。实际响应、原因与完整来源仍待该批工程包；P1 PipeWire/Wlr 是各自独立入口，继续保存真实结果。

Wlr 另新增独立 `memfd`、`ftruncate` 故障用例，每端各计划二十轮。仍使用已有“大缓冲 Failed、正常缓冲恢复”的自有 IPC 夹具，原 syscall 参数与结果不伪造：原 C++ 在 UUID 构造后的真实调用内临时把 NOFILE/FSIZE 软限制降到零，Rust 在首个捕获调用外施加自有进程限制；原硬限制不改，原软限制须足以支持未注入时的操作。检查实际 EMFILE/EFBIG、失败对象仍已构造但不留 memfd、旧输出 owner/像素不变、正常恢复、关闭后持帧和精确资源平衡。源 C++ 对 ftruncate 的确切 fd 还在任何可能复用 fd 的文件观察前验证已关闭。生产捕获代码、原 mmap 用例及其断言未改；非资源故障模式的源 wrapper 原样转发。

后续代码复查在原生执行前发现新 Linux 测试把 `usize` fd 计数直接与 `rlim_t` 比较，现已改用 `rlim_t::try_from` 检查式转换。此修正仅处理测试类型边界，不改变捕获行为、原断言含义或任何已记录原生结果；完整 Linux 编译和真实内核用例仍待验证。

按 [Linux 资源限制](https://man7.org/linux/man-pages/man2/getrlimit.2.html)与[信号等待](https://man7.org/linux/man-pages/man2/sigtimedwait.2.html)契约，FSIZE 用例仅临时阻塞测试线程的 SIGXFSZ，拒绝原先待处理的该信号，消费故障调用后新待处理的信号，再恢复掩码；不安装信号处理函数，前后处理函数、flags 和 mask 均核对。此隔离夹具假定故障区间没有外部发送 SIGXFSZ；无法恢复或前提失效时终止自有测试进程并保留失败，不能继续当作原生成功。这不是 SDK 信号管理、任意资源耗尽或一般 OOM 恢复。

已实现 `cargo xtask upstream-protocol wlr-resource-errors <memfd|ftruncate> <外部矩阵> [--check-only]`，校核每端二十行、逐次故障观察、实际 Copy 序列、恢复前后 fd、精确软/硬限制、信号状态、原输出与持帧、u32 线协议身份及源服务退出。两份人工完整矩阵和只读回读通过，十六组损坏数据被拒绝；二十个真实 Step 子进程 mock 拒绝零测试、ignored、缺摘要、十九轮和原退出 17。初次本机 controller 把两个 PowerShell 路径当作单个程序而失败，原日志保留，选定唯一程序后控制通过。Workspace（含 xtask 107 项）、Clippy、格式和两份 C++ 语法输入检查通过；这些 Windows 检查没有编译 Linux 测试体或 C++ 原生 target，真实编译/内核故障与信号恢复仍待 Actions。沿归档 Cargo 解析开发，nightly 更新确认当前版本；没有新的在线依赖验收，旧 mmap 成功及其他历史失败保持原身份。

确切 `7b82fe8` 的 [Actions 41408697160](https://github.com/AbyssRei/maafw-screencap/actions/runs/41408697160) Linux job 已终态成功，Wlr 140/140，原生 Clippy 通过；修正两处测试访问后，原先未能编译的 mmap 回归现实际执行。原 C++ 与 Rust 各二十轮均遇到真实 ENOMEM，保留 256 MiB 失败 memfd，原样恢复地址空间软/硬限制，再用不同 inode 的 140 字节缓冲完成恢复取帧；旧输出不变，关闭后持帧正确，两端服务各完整结束二十个 client、四十个 frame、二十次 Copy 和二十次 Failed。独立核对全部行、140 份实际 payload 和只读矩阵检查，原矩阵字节未改。回归函数及 C++/协议夹具与修复前 `c564306` 的 Git blob 相同，没有放宽断言或重复捕获以替代旧失败。

该批轻量 ZIP（3,724,534 字节）与完整 Linux ZIP（632,208,381 字节）的官方摘要、长度及 CRC 均通过。完整 tar 遍历 93,009 项，选择性恢复 15,905 份 Wlr/F/直接 Rust 父输入，核对 9,235 个唯一登记路径并补齐全部 592 项轻量输入缺口；496 个产品 Git blob/mode、单独 Cargo.lock、十五份原 F 文件和八条原版 target 编译记录通过，其中包含实际 `MemfdBuffer.cpp`、`WaylandClient.cpp`、`WaylandHelper.cpp` 和 Wlr 实现。其他完整工程组件、U 整树及所有更深父依赖不由这个选择性复核授予验收。

当前 Memfd 八项对应另链接 `7b82fe8` 的六十项通过检查，同时完整保留旧 `c564306` 的 57 项通过、三项失败；实际只读 `function-map` 核对来源、当前 Rust 和两批身份通过。修复前 fd 断言失败、`f261cbf` 的测试编译失败和修复后成功分别保存，不能把新成功改写为旧批通过。本项仅验证 mmap 失败 owner 寿命修复，其他 memfd/ftruncate 错误、旧缓冲关闭与新分配同时失败、原版带活代理立即断开、其余平台失败和完整 P1 仍未验收。

确切 `7b82fe8` 的 [Actions 41408697160](https://github.com/AbyssRei/maafw-screencap/actions/runs/41408697160) macOS 原生组件仍为 128/145，17 项失败逐项保留。官方轻量 ZIP 的 37,490,128 字节、SHA-256 和 CRC 已核验；200 份 witness 登记文件、36 份当前/持帧像素及十四张完整 sRGB CPU 视图经独立复核。十二张显示器 BGR 图像均在 `[1012,4,1020,12]` 范围各偏差 52 个像素，但六组原版/Rust 配对仅两组逐字节相同；更早确切 `4ebd7cb` 的六组全相同保持为独立批次，不据此推断本批或旧批像素生产者。两种调用顺序、真实程序/进程/模块身份、关闭后持帧和十四份窗口清单继续记录；诊断记录校验通过，严格整屏捕获断言仍失败，不屏蔽该区域或授予回调排空/P1 验收。

本次只读下载复核没有重拍截图，完整工程输入仍有 555 项轻量包缺口。复核程序完成全部数据检查和报告后，在最后复制自身的归档步骤触发 `SameFileError`；初始脚本与错误说明保留，仅为该步骤补同文件判断，未重跑 writer 覆盖报告。已完成的像素/身份核对和原生 17 项失败分别记录，产物及复核另行封存；预装工具版本仍按实际身份记录，不设严格最新门禁。

随后确切 `f261cbf` 的 [Actions 41405431881](https://github.com/AbyssRei/maafw-screencap/actions/runs/41405431881) Linux Wlr 组件为 107/140、33 项失败。两个旧用例的空闲 SHM 覆写仍访问已移除的 `Buffer.map`，Rust libtest 在编译时报告 E0609；因此新的失败 mmap 回归没有执行，不能据此判断 owner 修复是否通过。原 C++ allocation 二十轮及其服务端关闭通过。现仅把这两处测试访问改为显式匹配 `ShmMemory::Mapped` 后原样 `fill(0)`；持帧/像素断言、allocation 回归、生产 owner 与原 C++/协议夹具均不改。新的原生编译与回归仍待验证，本批编译失败及更早 `c564306` 的实际 fd 断言失败分别保留。工具准备通过，不把失败归因于预装工具版本。

Wlr 失败映射寿命差异已由确切 `c564306` 的 [Actions 41398197470](https://github.com/AbyssRei/maafw-screencap/actions/runs/41398197470) 原生复现：固定 F 的 C++ 用例二十轮均实际得到 MAP_FAILED/ENOMEM，保留失败 fd，恢复原限制后正常替换并取帧；Rust 第一轮的 buffer errno 也是 12，但 fd 数保持 8，未达到应有的 9，没有留下失败 memfd。旧图像字节不变，未发布 raw 或恢复图，Rust 以 101 退出。Wlr 组件保持 137/140，失败的 Rust 用例、Rust 服务端不完整计数和记录检查共三项均保留。

独立 `allocation` IPC/SHM 用例仅在自有测试进程中，把 `RLIMIT_AS` 临时下调到实际 VmSize 加 32 MiB，用 256 MiB 稀疏缓冲请求触发内核映射失败，再恢复原限制并恢复普通取帧。原版 `mmap/mmap64` 观察器逐次原参数转发、保留返回值/errno；协议端 Failed 只结束 F 原来的等待。官方轻量 ZIP 长度/摘要与 CRC、8,545 项可用登记路径、原版 60 份实际 payload 及二十组限额/fd/inode/替换记录已独立核对；Rust 的空表、首轮 fd 缺失及 105 字节旧图像也已核对，实际不完整矩阵仍被只读 checker 拒绝。完整来源/编译输入闭合另行处理，不能将轻量数据审阅等同于完整来源验收。

Rust 现用内部三态 owner 表示“不可用”“fd 已创建但未映射”“映射与 fd 均有效”，按 F 的构造顺序先保存对象，再返回原生准备错误。`mmap` 失败保留 fd 到下一次替换/关闭；`memfd_create` 失败保留空对象，`ftruncate` 失败先关闭 fd；映射成功仍先销毁协议代理/池，再 unmap、close，原 fd=0 残留边界保留。原 `check_buffer` 的对象存在性与元数据判定也保留，不以缺代理为由静默重新分配；缺失映射或代理以安全错误拒绝。捕获等待、Failed、Copy、转换和既有回归断言未放宽。旧共享/FD0 用例只调整私有字段访问，`allocation` 回归函数与 C++/协议夹具未改。

该测试及 `cargo xtask upstream-protocol wlr-allocation <外部矩阵目录> [--check-only]` 先前已通过十四组坏数据拒绝和九组 Step 编排控制；人工数据没有截图证明。本次 workspace 的 104 项 xtask 测试、Windows feature Clippy 与格式通过，沿用封存 Cargo/vendor 输入；这些本机检查不编译 Linux owner 路径。修正后的 Linux 编译、二十轮原生回归及其余 Wlr 场景仍待新 Actions。`memfd_create/ftruncate` 及组合失败的新状态目前只有源码对应，未据此声称原生穷尽或完整 P1 通过；旧失败、旧快照及对应关系保留原身份，当前 Rust 位置需要随新结果更新。

确切 `4ebd7cb` 的 [Actions 41380793132](https://github.com/AbyssRei/maafw-screencap/actions/runs/41380793132) 已原生编译并执行修正后的缓存路径：十四张完整 CPU view 均严格匹配三色图样，schema2 的 sRGB 绘制前配置、初始化、布局及十四组窗口清单核验通过，诊断记录入口通过。两种调用顺序的原版/Rust 六组配对字节全部相同，但十二张显示器图像各有 52 个偏差像素，均在 `[1012,1020)×[4,12)`；两个严格原生入口继续失败，组件保持 128/145、十七项失败。两扇 Window Server `StatusIndicator`（PID164，layer2147483630，bounds `(1002,-6,28,28)`）在十四次清单中均高于目标窗口，CPU view 没有这些像素；这是分离的视图、捕获和元数据证据，仍不证明同时的像素归因、动态新鲜度或回调排空。实际清单均提供 IsOnscreen，null 分支仅有人工控制。完整轻量 ZIP、200 份矩阵文件、36 份当前/持帧及十四份保存报告已独立核对；完整官方 ZIP/tar 的 29,935 份恢复文件、28,378 项注册路径、十一份普通 490 文件快照及一份受控快照、76 份 F 源码和 U/F 741/744 Git blobs/模式均核对通过，549 项轻量输入缺口全部闭合；十四份保存报告与完整 reference 验证只读通过，恢复字节及 Git 元数据未变，复核已封存。实际预装 Xcode 27.1/27A9275、SDK 27.0 仅作身份记录，不设最新版本门禁。`capture_parity_accepted=false`、原版/严格整屏失败和 P1 阶段状态均保持不变。

确切 `5b8ef7a` 的 [Actions 41361736663](https://github.com/AbyssRei/maafw-screencap/actions/runs/41361736663) Windows 组件保留 376/388、四项原版 FramePool pseudo 和八项 manager 失败。完整轻量 ZIP 的 31,412 项可用注册路径及 87 份保存报告只读通过；其初始 4,041 项输入缺口现已由官方完整 ZIP/tar 全部闭合。完整归档核对 61,173 份恢复文件、59,575 项注册路径、22 份普通 485 文件源码快照及一份受控快照、76 份 F 源码和 U/F 741/744 Git blobs；实际 CMake/F/工具链上下文、87 份保存报告与整个 reference 只读验证通过，恢复字节及 Git 元数据未变，复核已封存。预装 VS2026 18.11.12315.225 与 SDK 26100 是实际身份，不设最新版本门禁；Windows tar 将普通源码也标作可执行，不能声称 Unix 模式等价。独立核对 alpha 的 90 份实际/持帧像素，十组配对一致：两侧 alpha255 均为当前图样，alpha0 均为黑图，Rust 十次均有原始帧而没有回放。带独立绘制 marker 的 source-order 仍有四张边缘差异：slots9、15、16、17 的完整字节相同，均只在外侧三像素出现 924 点偏差，其余21张完整图样一致；slot17 是 C++ 对象关闭后的首次调用，不证明原生会话或回调已排空。记录检查通过只验证这些诊断事实与十四项严格正控制，不能将四张差异图改作像素通过。最初独立复核错误地要求全部25张都已知，在slot9失败；该脚本/失败说明保留，随后逐像素核对实际诊断与差异数量。托管结果不覆盖本机无输入三轮 alpha 不一致的既有结论，来源/输入闭合不改变上述捕获失败与本机权威复测结果，P1 仍未通过。

确切 `1241cb3` 的 [Actions 41374548090](https://github.com/AbyssRei/maafw-screencap/actions/runs/41374548090) 已实际编译并执行视图/窗口清单诊断，两组均完成七次 CPU 绘制和清单查询、原版/Rust 各三次捕获及关闭。清单在截图后出现 Window Server 的 `StatusIndicator`：PID174、layer2147483630、矩形 `(1002,-6,28,28)`，位于自有 layer1000 窗口上方，中心与旧差异坐标 `(1016,8)` 相同；这属于非同时的窗口元数据关联，尚不证明具体像素生成者。CPU 缓存得到三种均匀但偏移的 BGRA 值 `(0,249,0,255)`、`(255,51,4,255)`、`(0,38,255,255)`，其严格三色检查失败；组件保持 127/145，十八项失败均保留，没有生成诊断成功报告。

修正仅作用于诊断缓存：先为尚未绘制的空目标[指定 sRGB 色彩空间](https://developer.apple.com/documentation/appkit/nsbitmapimagerep/retagging%28with%3A%29)，核对 RGBA8/alpha-last/非 planar 布局和可写跨度、清零，再绘制，最后用 `getPixel` 提取原始分量；没有对完成的图像重标色彩空间。view schema2 记录绘制前配置与初始化证据，历史 schema1 仍按原规则核验。窗口 census schema2 单列 `kCGWindowListOptionOnScreenOnly` 查询来源；Apple 明确将 [IsOnscreen 列为可选键](https://developer.apple.com/documentation/coregraphics/optional-window-list-keys?language=objc)，缺失时原样保存 null，不能改写为 true；旧 schema1 的必需真值规则仍保留。三组人工 CPU 控制与三十一组坏数据拒绝通过，十三份旧报告只读通过，实际 `1241cb3` 的 CPU 颜色失败仍被拒绝。修正采用更新后的 nightly 开发验证，实际 sRGB 绘制结果已在确切 `4ebd7cb` 验证，见本节开头；捕获实现、严格整屏断言和 P1 阶段状态不变。

初始 `1241cb3` 实现了独立 `full-display-witness` 矩阵、精确 ignored 测试 `records_full_display_native_witnesses_in_both_orders` 与 `upstream-trace macos-full-display-witness <external matrix> [--check-only]`。两种调用顺序各在首帧前和六次调用后，通过请求/应答取得七组主线程证据：[AppKit 视图缓存接口](https://developer.apple.com/documentation/appkit/nsview/cachedisplay%28in%3Ato%3A%29?language=objc)只重绘自有 view，使用 NSColor 的语义 sRGB 分量输出独立、左上原点的 BGRA8；每个请求的 CG 窗口清单还记录实际顺序、ID、PID、layer、bounds 与可用名称。父进程区间必须落在对应截图调用之间，十四张 CPU view 必须逐像素匹配完整三色图样。此操作不能采样 WindowServer 合成结果，也不证明某个窗口生成了小圆点；新增绘制、查询和文件交接可能影响调度。

原有未插入观察的整屏入口与 schema3 继续独立执行。新增原生测试仍在保存完整结果后执行同一严格三色断言，像素未知时仍失败；新 schema4 记录入口可以核验十二张真实图像、持帧和所有偏差，而固定记录 `capture_parity_accepted=false`，不掩蔽任何显示器像素，也不把诊断完整性当成配对或 P1 通过。本机 workspace（xtask 99 项）、Windows 和 Mac target Clippy、二十一组坏数据拒绝、两组人工 CPU 控制及十五组实际 Step mock 通过；十三份历史原生保存报告只读复核通过，确切 `3e389c4` 的原整屏失败仍被严格拒绝，123 份矩阵文件未变。首个人工夹具的 CRLF 协议失败和首版外层脚本误用最后一个故意失败 mock 子进程退出码的结果均保留，后续分别按 LF 和新 PowerShell 进程执行验证。该初始实现使用归档开发依赖；人工控制不属于原生取帧，随后的实际执行结果和缓存修正见本节开头。

新增独立 `full-display-rgb` 验收矩阵与 `upstream-trace macos-full-display <external matrix> [--check-only]`。自有 borderless 窗口必须精确覆盖唯一显示器，首个入口明确要求 AppKit backing 与 CG point/pixel 为 1:1；整帧绘制左侧红、右上绿、右下蓝，测试 inset 为零。夹具采用 [Apple 的 NSImage 绘制 block 接口](https://developer.apple.com/documentation/appkit/nsimage/init%28size%3Aflipped%3Adrawinghandler%3A%29?language=objc)，block 只保留标量尺寸，MRC 下显式 retain/release 并由窗口保活 view/image。新私有入口与原有 small-patch/SC/CG 场景分开，固定 F 类不改；两个新进程组分别先执行原版三次再 Rust 三次，以及相反顺序。schema3 验证同程序/原生目标身份、全部十二张完整 BGR、六份 Rust BGRA 与转换/持帧、真正的 source 对象关闭及六次同步调用顺序；None、边缘坏像素、缩放/裁剪几何或不完整帧均不能通过。调用区间不伪装硬件帧时间，静态整屏不证明动态新鲜度、任意缩放、遮挡或回调排空。

本机已经通过 workspace 测试（xtask 98 项）、Windows Clippy、Mac target 的 Rust typecheck/Clippy、二十八组 CPU 损坏记录拒绝与十四组实际 Step mock 控制；旧 schema2 实际 Mac 保存报告另经当前工具只读复核不变。人工矩阵借用旧可执行文件身份而重建六像素图样与时间数据，明确为序列化/像素检查控制，不是原生截图。Cargo 对 exact-test 的 exit77 包装为 exit101 时，编排仅凭当前目录的确切 pre-capture marker、且没有 source/capture 文件，才记录 missing_environment；原始退出和失败仍保留。开发使用归档依赖，不声称新的在线解析验收。

确切 `3e389c4` 的 [Actions 41369385287](https://github.com/AbyssRei/maafw-screencap/actions/runs/41369385287) 已完成 Mac 原生编译和取帧：两个顺序各取得原版三张、Rust 三张 1024×768 图像，但十二张均未命中完整三色断言。独立复算全部 123 份整屏矩阵文件和 36 份当前/持帧，确认每张仅有 4–52 个偏差像素，全部位于 `(1012..1019, 4..11)`；其余像素、BGRA→BGR 和关闭后持帧匹配。原版先行的三对完整字节均不同，反向顺序三对相同；右上角小圆点的生成者与原因未证实。组件为 127/142，旧十二项加新增三项检查均保持失败，未生成整屏成功报告。完整官方 ZIP/tar 摘要、29,708 个恢复文件、28,151 条登记路径及十二份清单、十一份 488 文件普通快照和一份受控快照、76 份 F 与 U/F 741/744 个 blob/mode 均匹配；五条原类及同进程 wrapper 编译上下文、十三份保存报告和完整 `reference verify` 通过，547 项轻量输入缺口全部闭合，恢复字节含 Git 元数据未变，材料已封存。预装 Xcode 27.1/27A9275 与 SDK 27.0 是实际身份，不是最新版本门禁。整屏、动态场景、一般缩放、原生回调排空和完整 P1 仍未通过。

确切 `5b8ef7a` 的 [Actions 41361736663](https://github.com/AbyssRei/maafw-screencap/actions/runs/41361736663) macOS job 已结束，组件保持 127/139：五个独立原版 display 场景均在约五秒等待后返回 nullopt，五个配对检查拒绝通过；同路径 observer 的原版与记录检查也失败，共十二项。实际失败日志已逐项核对，不能从较早批次代入结果。新的同进程原版先行 CG/SC 两组及记录检查原生通过，97 份登记文件与十八份当前/持帧另经独立复核；CG 双方各三张已知图样，SC 双方各三次 None，None 不属于成功截图。完整官方 ZIP/tar 摘要、29,546 个恢复文件、27,989 条登记路径及十二份清单、十一份 485 文件普通产品快照与一份受控不可构建快照、76 份 F 和 U/F 的 741/744 个 Git blob/归档 mode 全部匹配。实际五个原类编译条目、同进程 wrapper 消费和十三组保存报告已只读核验，全部 544 项轻量输入缺口闭合，完整 `reference verify` 通过且恢复字节（含 Git index/config）不变，材料已封存。这里的工具身份按预装政策记录，不是最新版本门禁；Windows 上的只读复核不替代原生 Mac 执行。旧失败、完整显示器像素、相反顺序、原生回调排空、根因与 P1 总体验收仍未通过。

另增独立 `mfs_framepool_alpha_poll_reference` 观察目标，明确排除于默认 native 构建；生成的 F 源码只转发两个原 `TryGetNextFrame` 表达式及超时缓存 `.clone()`，三个匹配均须恰好一次。原 clock、2ms sleep、2000ms 等待、Close、catch、转换与持帧位置保持；观察 wrapper 不保留 frame、不增加原生查询。TLS 固定大小计数记录 drain/poll 的调用数与非空返回、实际缓存 clone 和这些边界的异常计数；Rust 精确 alpha 测试同时保存已有的成功调用 evidence，不改后端。生成正文、probe 头、实际 MSBuild 项目及 compile/link 输入日志在捕获前核对/归档，托管编排保留原来的未拦截配对，另跑新的 source/Rust 进程及 `upstream-alpha frame-pool-poll <observed source directory> <Rust directory> [--check-only]`。两种派生报告不会互相纳入哈希，均可独立只读复核。

该观察目标的最新连续三轮对照为 9/10、9/10、10/10，配对退出 `[1,1,0]`；18 组不透明像素全部一致，两处透明阶段差异保持失败。第 1 轮 Rust persistent/phase1 实际 drain 一帧，后续 816 次轮询全空，在 2.001320 秒后回放旧图，对应原版取得黑图；第 2 轮则是原版 persistent/phase3 drain 一帧，后续 131 次轮询全空，在 2.001496 秒后执行缓存 clone，对应 Rust 取得黑图。两侧各 29 次非空原生取帧、一次缓存 clone。这里证明的是各次被观察调用的分支；计数可能影响调度，未读取被清掉帧的内容或时间戳，也未证明旧未拦截原版分支、轮询次数差异的原因、一般帧新鲜度或历史根因，不据此改变等待/回放实现或重标任何旧失败。

四路输入为零、系统输入计时/光标固定、最大采样间隔 76.663ms，监听及 writer 完成退出；没有新的物理输入正向挑战。268 份完整 BGR/BGRA 即时/持帧、60 行计数和 1,475 项捕获前冻结输入及 488 文件源码快照经独立复核；21 组损坏记录被拒绝，两组 CPU 正向数据、九组实际 Step 控制、97 项 xtask/workspace 测试、完整 Windows features Clippy、Release/fmt 通过。最初 CPU CRLF 正向控制误将已有 CRLF 写成 CRCRLF，checker 正确拒绝；旧控制材料保留，修正仅在新的控制副本。source-only 报告遇到 sidecar 时现明确其本身不校验分支，避免沿用“未拦截”的旧描述；数据复核在副本中修正表述，原生调用和原报告/退出不改写。历史未拦截批次的 `[0,1,1]` 与全部保存报告仍只读核对一致。使用归档开发依赖，尚非新在线依赖验收；新托管入口、完整资源/回调退出及 P1 仍待验收。

FramePool 的原版 alpha/content 诊断现有 Rust 同序列对照：各自执行 fresh/persistent 两种会话的 255→0→255→0→255，保持相同 UI owner 图样、一次显式重绘、每阶段一次捕获及原调用/等待。原生 `mfs_framepool_alpha_reference` 额外在 capture/window/dispatcher 关闭后保存十张原有 CPU 持帧，不改固定 F 类；Rust 精确测试 `backends::windows::frame_pool::tests::alpha_sequence::records_original_alpha_sequence_pixels_and_held_owners` 单独启用 PrintWindow 当前内容见证，额外保存原转换边界的 CPU BGRA、回放/转换/clone/映射证据及关闭后的二十张 BGR 持帧。普通 FramePool 编排的 features 不变，该精确测试从通用聚合中排除；独立进程及 `upstream-alpha frame-pool-pair <source directory> <Rust directory> [--check-only]` 接入原生 CI，原有四项 pseudo 已知颜色失败不改变。

最新本机连续三轮中，原版/Rust 程序各自的十阶段控制均通过，配对检查结果为 `[0,1,1]`，逐轮 10/10、9/10、9/10 相同；18 组不透明图样全部严格匹配。后两轮均在 persistent 的首个 alpha0 阶段出现差异：原版黑图，Rust 在 2.001971/2.002195 秒后回放上阶段图样，转换/原生复制为零且没有伪造 raw 样本。其余透明阶段配对一致。这个不同执行时点的观测不能确定原版内部缓存分支、帧新鲜度、一般透明度原因或迁移缺陷；不追加截图重试消除失败。检查器会保存十次完整且自身有效的像素/状态观察，在任一配对差异时仍返回失败；损坏图像、布局、状态、错误或持帧直接拒绝。

该批原始/低级鼠标键盘输入为零，系统输入计时和光标不变，最大采样间隔 77.5237ms，四路监听及日志 writer 已确认注销/退出；没有新的物理输入正向挑战。独立逐字节核对 266 份 BGR/BGRA 即时/持帧、全部 1,473 项捕获前冻结输入及 486 文件产品快照，实际两个 MSBuild target 的 F 类编译输入与 Git blob 一致。二十四组损坏数据拒绝、两组 CPU 正向数据、九组实际 Step mock 控制通过；97 项 xtask 测试、workspace 测试、完整 Windows features Clippy、Release build 和 rustfmt 通过。最初三轮的配对 checker 因 C++ CRLF/Rust LF 比较失败，原执行保留；三十份实际状态标签/数值独立核对相同，修正只比较完整行，且不回改旧退出码。此前 native 配置漏开 GDI target 的构建失败也单独保留。使用归档开发依赖，不构成新在线依赖/工具链验收；新的托管执行、完整资源/回调排空、四项 pseudo 失败和完整 P1 仍待解决。批次与独立报告保存在同级参考目录，产品不依赖其路径。

确切 `88208bd` 的 [Actions 41355524703](https://github.com/AbyssRei/maafw-screencap/actions/runs/41355524703) 原生 Mac 已编译新 Objective-C++ wrapper，并通过同进程原版/Rust 的两个 mode 及记录检查。CG 场景中双方各三次得到已知图样，SC 场景中双方各三次返回 None；原版 SC 耗时约 5.034/5.067/5.141 秒，Rust 约 5.002/5.048/5.143 秒。97 份真实矩阵文件匹配登记 SHA-256，独立核对实际可执行文件长度/FNV/路径、真实父子 PID/窗口归属、原版 native caller thread、两个实际加载模块路径一致、context 清理及场景关闭。十八份当前/持帧 payload 的全图通道转换、已知目标像素和 held bytes 通过；六次 None 没有附加替代图片，也没有被计为成功截图。该结果仅覆盖原版先行的顺序；不同时间的整个显示器像素、相反顺序、实际帧时间和原生 callback/session 排空仍未证明。

该批官方轻量 ZIP 的 35,125,420 字节及完整摘要、11,166 条可用登记路径和十三份保存报告只读通过；随后完整官方 ZIP 的 351,041,182 字节及完整摘要亦通过。恢复的 29,522 份文件、27,965 条登记路径、十二份清单、十一份普通 483-file 产品快照与一份故意不可构建控制均核验；76 份固定 F 源文件、U/F 的 741/744 个 Git blobs 及归档模式通过。实际新 wrapper 的编译和原生消费、五份固定 SCK 类编译项（四份无 observer）、选定 Xcode/SDK 与 MRC 配置、十三份保存数据及完整 reference checker 通过，全部恢复字节（含 Git index/config）不变。这闭合了该确切批次的 542 项轻量缺口，没有借用旧 2c 来源；结果已封存。原生组件仍为 127/139、十二项既有失败，失败原版 AppKit 配对仍被拒绝，缺几何分支仍未原生触发；宿主关联的生产者/根因与完整 P1 仍未验收。

macOS 同程序诊断现增加同进程原版调用，以补足先前只有 Rust caller 的范围。新增私有 `same-binary-original.mm` 与现有场景模块共同编译、链接未改动的固定 F `ScreenCaptureKitScreencap.mm`。每个 mode 仍在全新测试进程内启动同可执行文件的场景子进程；同一 libtest worker 先创建原版对象并严格调用三次，再进行既有三次 Rust 调用。没有增加截图重试、改变原版五秒等待/转换位置或替换旧 oracle。原版优先的顺序明确记录，不能据此宣称相反顺序、独立调用历史或整个显示器像素已一致。

原版私有 context 在同一原生线程创建、访问和销毁，返回记录保留 None、Some(empty)、非空图像的区别，并保存真实线程/PID、耗时、编译的 Mat type/stride。非空 Mat 独立持有 CPU 像素，场景关闭和原版对象析构后再次写出 held bytes，随后销毁 context 并实际检查 owner 归零。原版模块与场景模块均保留加载引用至进程退出；对象析构和 context 归零不证明原生 callback/session 排空，不据此释放原版晚到回调的代码。

`upstream-trace macos-same-binary` 新 schema2 强制两个 mode 共六次原版、六次 Rust 调用，逐次返回状态一致，有效图片须有准确已知目标像素与相同持帧字节；CG arm 要求原版/Rust 各三张已知图样。SC 的匹配 None/empty 是失败/空值观察，不能记为成功截图。额外全图核对每份 Rust packed BGRA 到 BGR 的通道转换，不要求不受控桌面区域在两次不同时间的截图间相同。源调用、native owner、模块身份、关闭记录、失败后附加图片、schema 混用和额外 original call 都严格校核；历史 schema1 的原报告形状不变，确切 c823 的真实保存报告仍只读通过。新的 native Step 完成标记要求原版三次调用，records Step 要求完整十二次；旧日志不能误满足新门禁。

本机五份正向人工数据覆盖匹配 None、匹配 Some(empty)、有效图片、晚到不作当前结果及历史 schema1；26 项反向数据覆盖原版调用缺失、线程/PID/模块替换、None/empty/image 混淆、坏布局/持帧/目标像素、额外调用、虚报排空和全图通道错误等，新报告写出及只读回读通过。Mach-O 头、身份、返回/owner 元数据和 96×64 CPU 像素均为人工构造，不是新 Mac 程序或截图。38 项真实 Step 子进程 mock 与四项 wrapper 控制通过，保留 17/77 原退出及失败后继续独立入口。nightly 更新成功且版本未变；97 项 xtask 测试、工作区测试/Clippy、Release 构建和 Apple 目标 Rust 类型/Clippy 检查通过。这些是现有 Cargo.lock/缓存下的显式离线开发检查，新在线依赖解析仍待 CI。上述本机检查未编译 Objective-C++；其后的真实 Mac 配对及完整来源复核见本节开头，旧失败与完整 P1 仍未验收。

顺序诊断现在复用已有桌面更新夹具：每个 arm 在创建任何源对象前启动同显示器、与目标不重叠的 16×16 自有 marker，由窗口 owner 每 50ms 交替绘制，并贯穿整个 arm。目标像素、原版捕获类、调用次数/顺序和等待均不改；WGC 对象析构与第一条 post-object DD 调用之间没有新增准备、等待或 IO。新增 `frame-drivers.tsv` 记录 marker HWND/thread/PID、初末绘制数、错误、同显示器/不重叠与关闭证据；native 和 `upstream-trace framepool-order` 都严格校核。schema2 登记 56 份输入，Step 要求新的 driver/schema 完成标记及 `files=56`；历史 schema1 维持原 55 份输入和报告形状，旧日志不能满足新 Step。

本机新增有效连续三轮共 75 次调用，75 张原始 BGR 全图逐字节符合已知图样；12 个目标窗口和 12 个 marker 均关闭。四路 Raw Input/低级 hook 事件为零，最大监听间隔 83.4621ms，监听器正常注销、writer 全部落盘并 join。本批没有新的物理正向挑战。1,461 个路径在捕获前冻结，含 26 项主要单元和 968 条原生依赖路径；实际编译消费的四份必需 F C++ 源码及其余来源逐项匹配固定 Git blobs。独立复核原路径/内容寻址副本、三轮报告、75 份全图、marker 与目标 owner 关系和清理均通过。报告原字段 `owned_windows_closed=4` 每轮只计四个目标，另四个 marker 单列，不能把整批全部相关窗口写成 12 个。

本批 nightly 更新成功，仍为 `1.101.0-nightly (f9ceb4a4d 2026-10-31)`；新 C++ target 采用归档开发依赖（OpenCV 5.0.0、Boost、MaaUtils）与 VS2026/MSBuild 编译通过，不是新的在线依赖解析或最新工具链验收。97 项 xtask 测试、工作区测试/Clippy、Release 构建和格式检查通过。CPU 数据控制保持历史 schema1 只读报告兼容，通过 19 个 driver/schema 拒绝、12 个其他结构/像素/owner/时序破坏拒绝和两份人工 treatment 观察；十四个严格正向控制未放宽。14 项真实 Step 子进程 mock 保留原退出 5/17，拒绝空、不完整、旧标记及关闭 driver 的成功日志。这些 CPU/mock 控制不是截图。更新夹具的实际 hosted 执行仍待验证，marker 绘制进度也不证明每次截图的新鲜度、原生 callback/session 排空或旧外围根因；不回改旧静态失败，不接受完整 P1。

确切 `c823497` 的 [Actions 41347473179](https://github.com/AbyssRei/maafw-screencap/actions/runs/41347473179) 原生 macOS 随后通过同程序 CG/SC 两项和记录检查，组件为 127/139，既有十二项失败保留。官方轻量 ZIP 的长度 34,752,523 字节、完整 SHA-256、11,145 个可用登记路径及十三份保存报告经只读复核；独立核对 76 份矩阵文件、六次当前返回和独立 late 状态。CG 三次已知图样，十二份 raw/BGR/持帧的全图通道转换和目标像素通过；SC 三次约 5.043/5.064/5.150 秒返回 None，仍为零成功截图。该批完整输入仍缺 540 项，不能以确切 2c 的完整来源替代；缺几何分支仍未原生触发，原版失败配对继续拒绝。该结果验证 checker 修正的原生编排，不接受整体 macOS/P1。

确切 `58fa6f0` 的 [Actions 41337468981](https://github.com/AbyssRei/maafw-screencap/actions/runs/41337468981) 已终态失败，Windows 七份组件清单为 374/386，保留四项原版 FramePool pseudo、六项 manager（6/10/14/38/42/46）以及 source-order/records 两项失败。完整官方 ZIP 的 1,944,778,447 字节及完整 SHA-256 现已核验；60,797 份恢复文件、59,199 条登记路径、23 份清单、22 份普通 475-file 产品快照与一份故意不可构建控制、76 份 F 源文件和 U/F 的 741/744 个 Git blobs 均通过复核。87 项保存数据与完整 reference checker 只读通过，全部恢复字节（含 Git index/config）未改。Windows tar 也将普通 C++ 标为 0755，不据此宣称 Unix executable 等价。完整来源闭合不改变任何原生失败。

该静态顺序程序实际完成 25 次调用、四个目标窗口关闭；整体正向控制字段为零、native 退出 5、records 退出 1。完整工程包补齐轻量漏传的三十份表格/布局，全部 55 份顺序输入匹配登记 SHA-256。十二次调用是未 engaged 的 None（不是 Some(empty)），十二份为完整已知图样，唯一差异在 `slot-9/arm-1/dd-with-wgc`，此时 C++ WGC owner 仍存在；18,432 字节与旧 a5 失败 runtime 的三像素外围 ROI 完全相同，924 个外围像素改变、内部零差异。slots 10/17 是各自第一次 post-object DD 调用，均为已知图样；严格正向控制只通过 8/14，失败 slots 为 1/2/3/4/23/24。冻结的原 checker 再次只读拒绝该数据。对象存在标志与 QPC 不证明实际帧时间、原生 callback/session 排空、外围生产者、根因或完整 parity。

旧轻量上传只带回上述 25 份 BGR；现将 `frame-pool-context/source-order/**` 整个目录纳入既有 `always()` 上传，失败后仍收集原始记录。微软 [DXGI 错误定义](https://learn.microsoft.com/en-us/windows/win32/direct3ddxgi/dxgi-error)将日志中的 `0x887a0027` 定义为等待下一帧超时，[AcquireNextFrame 说明](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_2/nf-dxgi1_2-idxgioutputduplication-acquirenextframe)指出新帧依赖桌面或指针更新。静态场景未持续驱动桌面更新是新夹具的验证假设，不能用来解释旧外围根因。新上传规则仍待实际托管产物验证，整批原失败不变。

新增 `cargo xtask upstream-trace macos-same-binary <外部矩阵目录> [--check-only]`，核对同一实际 Rust libtest 可执行路径的消费进程和场景子进程。两个 arm 各用新父/子进程，子进程从 `current_exe()` 启动，在主线程加载独立 scene dylib，包装并调用现有 `source-context.mm` 的场景入口；现有原版捕获类、场景函数体和旧 oracle 均不改。mode 1 为现有 CoreGraphics 元数据场景，严格要求三次已知图样；mode 0 为现有 ScreenCaptureKit 元数据场景，固定三次调用，失败、空结果或错误均保存为观察，不算成功截图。每次调用有独立 raw owner 和当前选源记录，额外八秒只观察晚到 owner，不再次截图或以晚到结果补当前返回；原五秒等待保留。

记录包含实际可执行文件归档、长度/FNV、真实父子 PID/窗口归属、原生选源、逐次状态和独立 late 状态、可用 raw/BGR 与窗口关闭后持帧，以及子进程协作退出。FNV 只核对这组文件关系，来源完整性另由运行清单 SHA-256 负责。scene 的一次 dlopen 引用保留到该子进程退出；关闭窗口和 request 释放均不被描述为全部原生回调排空。新 checker 拒绝路径/程序身份不符、损坏持帧、失败调用附加替代图片及强制终止；两个 arm 和记录检查独立编排，前项失败仍执行后项。`77` 单独记录为场景缺少环境，原退出码不被成功日志覆盖。

本机 96 项 xtask 测试、工作区测试/Clippy/Release 构建、Apple 目标 Rust 类型/Clippy、26 份 PowerShell 解析及格式检查通过。初次三个正向、十二个反向 CPU 数据控制采用旧归档像素，但 Mach-O 头、身份和返回/晚到元数据为人工构造，不是本次 Mac 程序或截图；29 项实际 Step 子进程和四项实际 wrapper mock 控制通过，覆盖空跑、ignored、不完整日志、CG 图样不足、17/77 原退出码和失败后继续独立入口。这些检查未在 Windows 编译 Objective-C++。

随后确切 `2c824c0` 的 [Actions 41345696019](https://github.com/AbyssRei/maafw-screencap/actions/runs/41345696019) 原生 macOS 已编译 scene dylib，两个 Rust 场景测试都完成，记录检查却失败。CG 三次均返回已知图样，十二份 raw/BGR/持帧的完整跨度、逐通道转换、目标像素及持帧字节经独立核对；SC 三次分别在约 5.003/5.025/5.029 秒后返回 None，没有 payload。前两次当前及 late 记录有显示器选择，第三次没有；三次最终均未观察到 sample 回调或 completion。它们均有独立 request/semaphore owner 释放标志和正常场景退出，但这些不是原生回调排空或未创建输入的证明。75 份实际矩阵文件逐一匹配原清单，官方轻量 ZIP 完整长度/摘要和 11,144 个可用登记路径、十二组既有保存报告只读校核通过；540 项完整输入仍缺。

新 checker 的错误是把 `initial_inputs_released` 当作所有调用都必有的标志。源码中它仅在 sample 回调调用 `Inputs::release_initial` 时置一；回调未调用时，持有字段仍可通过 block 析构释放，标志零本身既不是未创建输入，也不是泄漏证明。现对失败/超时保留零或一的观察，对 Some 图像仍要求显式释放为一；非布尔值继续拒绝。新的三个正向/十四个反向 CPU 数据控制覆盖真实零标志前提、成功图像缺释放及无效标志；修正 checker 在原始矩阵上只读通过，并只在独立字节副本上生成/复核新报告，没有重跑捕获或改动旧文件。旧原生批次仍为 126/139、十三项失败，包含该记录错误和既有十二项失败；不将本机数据校核重标为旧 CI 通过。修正后的原生编排、完整来源和 P1 仍待验收，不替代旧失败 oracle，也不据同程序 SC/CG 差异推导权限或 broker 根因。

随后同一批完整工程 ZIP 在一次保留错误记录的网络中断后，复用已验证分段补齐 350,516,479 字节，官方完整 SHA-256 匹配。tar 恢复 29,476 份文件；十二份清单的 27,919 个唯一登记路径全部匹配。十一份普通快照各有 481 个 Git 文件，另区分一份不可构建控制；76 份 F 来源、完整 U/F 的 741/744 个 Git blob 和归档可执行位一致，未初始化子模块仅作声明。实际原类现在有五个编译条目，其中四个未注入观察器；新增 scene 模块的原类和 wrapper 编译均核对原 F、manual RC、实际 Xcode/SDK 及无观察器配置。轻量包全部 540 项缺口闭合，十二份既有保存报告只读通过；原 checker 仍拒绝同程序矩阵，修正 checker 对恢复矩阵只读通过。完整 reference 校验通过，所有恢复字节（含 Git index/config）不变，封存后独立读回 29,510 个文件。该闭合补充上一段的来源范围，不改判 126/139 或十三项失败，不代表在 Windows 运行 Mac 工具或完成原生回调/平台/P1 验收。

另对完整 `a5cfa2e` 包的六个失败 runtime mask（6/10/14/38/42/46）作只读像素诊断，1,488 份实际输入匹配先前完整封存的摘要。两侧各二十生命周期、三次输出共 720 张，另 720 张持帧逐字节等于各自即时文件。原版 47 张、Rust 34 张的目标 ROI 不同于自有图样；这 81 张全为同一份 96×64 像素，变化恰好是外围 924 点，内部无一处变化。外两像素为黄色混合值，内一像素是暗化值，不是直接写入字节 0 或 255 的纯黑/纯黄。80 张位于各生命周期第一次调用，一张在第二次；53 张选中 DesktopDup，28 张选中 DesktopDupWindow，选中 ScreenDC 的输出均为预期图样。按同 mask/cycle/shot 比较，原版/Rust 目标 ROI 只有 317/360 对相同，其余因是否带外围而不同；不能把共同形状称为逐次 parity 成功。完整监视器范围只核对跨度与持帧，不将环境桌面的其他像素作为固定图样。已保存独立 ROI、逐环颜色/差异和原始字节生成的对照图并封存，没有重跑截图。首次调用偏重仍不是 capture-time、freshness/cache 分支或外围生产者的证据，旧失败与本机连续三轮结论不变。

确切 `58fa6f0` 的 [Actions 41337468981](https://github.com/AbyssRei/maafw-screencap/actions/runs/41337468981) macOS job 已终态失败，组件保持 123/135，十二项显示/夹具及记录失败。其完整轻量 ZIP 长度 33,304,475 字节及官方 SHA-256、11,065 个可用登记路径和十二组保存数据复核通过，533 项完整输入尚缺。真实 schema2 的 CG 场景记录仍通过；独立 AppKit consumer 的原版保存一份几何、零张图片且未完成，Rust 保存十二份 raw/BGR payload，配对校核继续拒绝失败原版。该次依旧没有触发原版几何完全缺失分支。新轻量审阅已封存，不能用上一批完整来源替代它的缺失输入，也不以独立 Rust 成功证明相同可执行文件/宿主上下文下的 parity。Windows/Linux job 和新顺序诊断仍按各自确切结果处理。

`a5cfa2e` Windows 的完整工程 ZIP 随后补齐全部分段，并独立核对官方长度 1,954,333,565 字节及完整 SHA-256。tar 安全恢复 60,692 份文件、7,452,021,502 字节，23 份清单的 59,094 个唯一登记输入/日志路径全部匹配。二十二份普通产品快照各含 473 个 Git 跟踪文件，另单列一份不可构建捕获控制；76 份 F 来源及整个 U/F checkout 的 741/744 个 Git blob 匹配，五个未初始化子模块各保留声明范围。Windows tar 将普通 C++ 文件也记录为 0755，已与 Git mode 声明区分，不据此声称 Unix 可执行位等价。

七个组件的完整清单与先前部分包的终态清单相同；实际 CMake 的固定 F 路径、generator、Release 工具记录及原生配置另经核对。实际预装 VS2026 为 18.11.12328.437、MSVC toolset 14.51.36231、SDK 10.0.26100.0；不将 SDK 落后于最新发布设为 CI 门禁，也不从记录推导完整系统 SDK 内容已离线恢复。完整树上的 87 个保存数据入口全部只读通过，其中 57 个是实际通过的方法组合；六个失败组合仍不完整或被原校核拒绝，不能由这 57 组替代。外部复核脚本曾把身份入口误写为不存在的 `upstream-identity`，已保留错误及当时日志，改用真实的 `upstream-metadata win32-identity` 后完成数据复核，没有重跑截图或修改旧输出。完整 `reference verify` 通过，原 Git index/config 及所有恢复字节不变；来源与保存数据材料已封存。该批仍为 373/383，四项原版 FramePool pseudo 与六项 manager 失败保持失败，完整 P1 未通过。

WGC/DesktopDupWindow 顺序诊断已接入 `mfs_framepool_order_reference`、`cargo xtask upstream-trace framepool-order <外部记录目录> [--check-only]` 和 FramePool 原生编排。仅显式原生验收构建该目标；原版后端函数体不变，产品捕获算法、等待、窗口调整及边框选项均未改。它另存编译期 OpenCV 类型位布局，校核器验证实际契约，不再固定使用旧 `CV_8UC3` 数值。原程序失败仍保留，独立数据入口仍执行；55 份文件核对固定调用顺序、窗口/线程身份、C++ 对象边界、前后窗口状态、布局、摘要和完整像素。十四次独立控制要求预期图样；其余十一项处理组的外围或空结果作为观察记录，不能称为成功的 manager/Rust parity。

新集成程序使用同一归档开发依赖编译，本机连续三轮的原生和 Rust 数据入口全部通过：75 次调用、十二个窗口，全部为预期图样；正式输入为零，最大观察间隔 80.4502ms，四路注册、注销与 writer join 均复核，没有新物理正向挑战。1,454 项捕获前输入冻结这次包含 CL/link 依赖及 caret 行的主翻译单元，四份实际编译的原版 CPP 与固定 F Git blob 匹配。十二组损坏数据由实际 CLI 拒绝；两组人工 CPU 数据分别验证“仅外围三像素改变”和“空处理结果”会被如实记录，不冒充新的原生截图。八组实际 `Step` mock 子进程另验证完整成功、缺失/少调用日志和原退出 17 的保留，均零捕获。95 项 xtask 测试、Clippy、Release 构建和 native 编译通过。该开发切片不代表最新在线依赖验证，新的托管执行仍待 CI；旧失败与完整 P1 状态不变。

确切 `a5cfa2e` 的 [Actions 41321269889](https://github.com/AbyssRei/maafw-screencap/actions/runs/41321269889) 已全部终态：Linux job 成功，PipeWire 194/194、Wlr 135/135；macOS 组件保持 123/135，十二项实际显示/夹具及记录失败；Windows 七组件为 373/383，FramePool 69/73、manager 101/107，保留四项原版 pseudo 与 mask 6/10/14/38/42/46 失败。预装工具准备均通过，不以版本落后于最新发布解释这些截图失败。Windows 的七份终态清单与已取回日志共 45 份文件通过 ZIP CRC，39 份另匹配登记摘要；该部分复核已封存，完整 ZIP、来源和新失败像素仍待工程包核验，不能从旧外围图像推断本批同因。

同批完整 Linux/macOS 官方 ZIP 均核对长度及完整 SHA-256，安全恢复 84,316/29,300 份文件。Linux 十九份运行清单的 82,754 个唯一登记路径、macOS 十二份清单的 27,743 个路径全部匹配；分别核对十八/十一份各含 473 个跟踪文件的普通产品快照，并单列一份不可构建捕获控制。每个平台的 76 份 F 来源及整个 U/F checkout 的 741/744 个 Git blob、原 tar executable mode 均匹配；每个 checkout 的五个未初始化子模块仅核对声明，不能称为已恢复其内容。实际来源编译命令与匹配来源另经复核。Linux 完整包补齐 PipeWire/Wlr 轻量包缺少的 565/569 项全清单输入；macOS 补齐 531 项。Linux 的 24 个、macOS 的十二个保存数据入口只读复算全部通过。

两份完整恢复树再通过冻结校验器的 `reference verify`，Git 长路径和 filemode 差异仅用进程参数处理，原 Git index/config 与全部 tar 文件字节均未改变。最终封存后独立读回复核通过。实际预装身份为 Linux LLVM 22.1.2、macOS Xcode 27.1/27A9275 与 SDK 27.0；不将其重标为最新正式工具，也不将版本差距设为 CI 门禁。此处闭合的是这两批完整来源与保存数据，原十二项 macOS 失败及完整 P1 状态不变。较早 `5b0e624` Windows 的部分终态复核亦已封存，保持 370/383、四项 FramePool 和九项 manager 失败；未验证该部分包的完整 ZIP 摘要。

另完成原版 WGC/DesktopDupWindow 顺序实验的独立复核：四个自有静态窗口 arm 分别观察 DD 单独控制、DD 后创建 WGC、WGC 先创建，以及末尾 DD 单独控制。每个程序调用原后端 25 次，WGC 对象结束后的三次 DD 均是预定观察，不重试失败截图；每次返回后立即显式 clone CPU 图像，全部源对象、窗口及 dispatcher 结束后再导出。原 FramePool、DesktopDup 和 DesktopDupWindow 函数体未改。原 manager 的候选容器是初始化函数局部变量，只保留赢家，因此该实验不是 manager 落选对象始终存活的模型。

连续三轮共 75 次调用、十二个窗口的全部像素均匹配自有图样，没有复现托管黄色/黑色外围。正式输入为零，最大间隔 82.9276ms，四路注册、注销和 writer join 经只读复核；按用户自动开始授权，没有声称新物理正向挑战。初版数据 checker 错用旧 OpenCV 数值类型，三次失败日志保留；依据捕获前已冻结的实际 OpenCV 5.0.0 头文件确认 `CV_CN_SHIFT=5`、`CV_8UC3=64` 后，仅在既有输出上更正观察器，未重拍。独立只读复算及八组损坏数据拒绝通过。五十份产品 native 文件、907 个 CL-read 依赖路径与 960 项捕获前冻结输入已核对；另识别 caret 行中的主翻译单元，补存九项此前未冻结的源码，并核对适用的 Git/F 身份。这九项是捕获后来源审阅，不能倒称已有捕获前摘要。材料与限制已封存并读回复核。实验使用归档开发依赖，显式额外 CPU 快照不作优化或性能证据；C++ 对象寿命记录不证明系统会话/回调排空，原生状态显示的 foreground/topmost 变化保留为观察。它不接受 manager/Rust parity、不证明外围原因，也不改变旧失败或完整 P1。

macOS 新增独立 `display-scene-cg` 宿主控制：同一原版可执行路径继续承担场景与消费进程，场景改用 [CoreGraphics 活动显示器](https://developer.apple.com/documentation/coregraphics/cggetactivedisplaylist%28_%3A_%3A_%3A%29)和[自有窗口信息](https://developer.apple.com/documentation/coregraphics/cgwindowlistcopywindowinfo%28_%3A_%3A%29)发布几何，不在该场景代码路径显式调用 ScreenCaptureKit。它要求单活动显示器，记录两次几何查询、真实窗口/PID 和主线程关闭；权限仍用原 preflight 检查。消费端仍执行原 `display-patch` 和五秒等待，Rust 继续原捕获入口；`upstream-trace macos-display-cg-scene` 保留相同二进制/路径/权限、六次已知图样、十八份 payload、转换和所有权检查。确切 `5710c16` 的 [Actions 41307803980](https://github.com/AbyssRei/maafw-screencap/actions/runs/41307803980) 已通过新原版、Rust 和记录三项，下载后十八份完整像素及相同程序身份、原生几何/查询/关闭记录亦通过只读复核。官方 ZIP 长度/摘要和 11,012 个可用登记路径均匹配，530 项完整输入仍缺；组件保持 122/135 通过、十三项失败，未接受完整 macOS/P1。该项用于隔离生产者枚举行为，不替代旧失败 oracle，也不声称观察了框架内部调用或证明根因。

该批 `rust-macos-display-appkit-consumer` 在截图前因缺少原版几何文件失败：原版枚举已超时，未写出文件。现修正测试依赖，只有编排明确记录原 oracle 失败时，才允许使用自有场景几何继续取得独立 Rust 观察；已有原版几何不匹配仍立即失败，配对校核仍拒绝失败/不完整 oracle。原始执行失败不改写。`rust-display-geometry.tsv` 始终是复制的场景预期；新 `rust-display-geometry-provenance.txt` 明确来源和原版失败状态，不能将它称为 Rust 原生观察。

新 schema2 为每次调用另存 `display-call-N.rust-selection.tsv`，包含实际选中的 `SCDisplay` ID、枚举数量和宽高，断言其匹配场景/源码几何。原生对象仍只在原回调生命周期内访问，只复制标量；ID 查询和记录锁仅在 `cfg(test)` 存在。调用返回后、任何文件 IO 和显式排空前取得一次快照；失败时并发回调仍可能推进，不声称这是原子返回快照。迟到状态写入独立 `rust-late-selection.tsv`，不能补齐当前调用记录或改变原返回失败。新配对记录要求明确 schema2、正确 provenance 和三次当前选择；历史 schema1 只允许带原保存报告、不含新增字段的只读复核，并明确其未独立记录 Rust 所选显示器。工作区测试、Clippy、构建、旧产物只读回归及九组实际文件篡改拒绝控制通过；初期 schema2 正向控制使用旧像素和人工元数据，仅验证校核器，后续真实原生结果另列如下。

确切 `5b0e624` 的 [Actions 41311201962](https://github.com/AbyssRei/maafw-screencap/actions/runs/41311201962) 已通过 macOS Clippy、直接核心/manager 构建以及新 CG 场景的原版/Rust/记录入口。官方轻量 ZIP 长度与摘要、11,065 个可用登记路径和十二组只读数据检查通过，531 项完整输入仍缺。真实 schema2 包含三次所选显示器记录、十八份 CG 场景 payload 和 provenance，全部复算通过。独立 AppKit-consumer 原版先写出一份匹配几何，随后显示观察超时，未产生截图；Rust 的三次捕获、十二份 raw/BGR 及持帧、实际所选显示器、内部图样、完整转换和关闭记录通过。原版完成记录仍为 `source_capture_passed=0`，配对校核拒绝它，整体组件保持 123/135 通过、十二项失败。该次没有触发原版几何完全缺失分支，不能把 Rust 独立成功称为该分支的原生实跑验证；缺文件仍只有控制测试覆盖。

旧 `e243fee` Windows manager 的官方部分 ZIP 诊断另取回失败日志和 mask 10 完整图像；该组件 106 项中 98 项通过，mask 6/10/14/22/26/38/42/46 的原失败保留。mask 10 双方二十轮均选择 DesktopDupWindow：六十次原版调用中十次、六十次 Rust 调用中十三次包含同一额外黄/黑外围三像素，其余图像完全匹配自有图样。逐像素确认所有变化恰好覆盖这 924 个外围像素，内部不变；120 份关闭后持帧与各自即时文件相同。256 个所选文件匹配下载清单摘要及 ZIP CRC，但未验证官方完整 ZIP 摘要或完整来源。微软的 [IsBorderRequired 文档](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.graphicscapturesession.isborderrequired)说明系统捕获指示边框存在；本次像素与启用 FramePool 候选使它成为待验证解释，尚不能证明实际生产者、消退时序或根因。未屏蔽边框、放宽旧断言或重试捕获，不以这批托管观察覆盖本机连续三轮结论。

委托销毁新增独立进程控制：使用真实原工厂和虚析构，不连接、不创建后端、不截图；边界先确实销毁对象，再返回两种受控清理错误。测试核对正常路径卸载、错误路径保留 DLL、第二个错误不覆盖首错，以及借用 bridge owner 结束后的模块保留。`cargo xtask upstream-delegate win32-destroy <外部记录目录> [--check-only]` 独立检查三组对象、原生调用线程、六条创建/销毁事件和模块观察；实际原生异常、销毁状态未知、回调排空及泄漏模块的回收仍待验收。这项测试必须通过精确筛选在独立进程运行，错误路径的模块引用保留到该进程退出；manager 编排会从聚合测试中排除它，再单独执行并收集失败记录。它不改变捕获或清理实现，也不完成 P1。

CI 已按用户要求使用可用预装工具，不以严格最新版匹配作为门禁。确切 `e243fee` 的 [Actions 41294808000](https://github.com/AbyssRei/maafw-screencap/actions/runs/41294808000) macOS P0 捕获通过，P1 实际执行 131 项检查、119 项通过，包含新增原接口 manager 查询；五组原版显示/夹具场景与其数据检查，以及单独的同路径观察者与数据检查仍为十二项失败。部分原版在原五秒等待后返回空结果，另有夹具显示观察超时，不重标为工具链缺失或完整平台通过。Linux P0 的 27 项原生检查通过，但命令在随后的归档验证阶段失败：预装 LLVM 记录新增 `policy` 后，旧校验器仍只接受下载包格式；P1 PipeWire/Wlr 亦遇到该记录错误。旧失败原样保留，现增加独立的预装记录格式校验，保留旧下载格式，并继续拒绝未知字段、重复字段、缺失工具/生成库、路径越界、错误摘要和链接参数；升级查询失败或较新发布只作为参考。随后确切 `682e629` 的 [Actions 41299003920](https://github.com/AbyssRei/maafw-screencap/actions/runs/41299003920) 整个 Linux job 成功；其他平台和完整 P1 仍保留各自未完成范围。

e243fee 的官方完整 Linux ZIP 随后核对长度与 SHA-256，完整 tar 恢复 77,269 份文件，十六份运行清单的 75,709 个唯一登记路径全部匹配。十五份普通快照各含 467 个 Git 文件，另明确区分一份不可构建捕获控制及其四个辅助 Cargo 缓存文件；原生快照另存的锁文件与实际 Rust parent 一致。76 份 F 来源对照确切 Git blob，两份原生编译命令均指向固定 F 和预装 LLVM 22。修复后的校验器只读接受原 P0 PipeWire、P1 PipeWire/Wlr 清单，24 个下载数据入口全部通过；原命令在旧解析器处的失败仍不改变。该批实际原生检查为 27、195 和 140 项通过，预装 LLVM 身份是 22.1.2，不从版本记录推导另一主机可运行它或已经具备完整离线重建环境。完整恢复、来源、数据及审阅记录已封存并读回复核。

`e243fee` 的官方完整 macOS ZIP 也已核对长度与 SHA-256，恢复 29,150 份文件；十二份清单的 27,593 个唯一登记路径全部匹配，补齐该批轻量包 525 项缺失输入。十一份普通快照各有 467 个 Git 文件，另有一份不可构建控制；76 份 F 来源与原生编译消费的五份原文件摘要一致。实际四个原类编译条目分别用于源码程序、OpenCV 适配器、观察者和查询 provider，其中三个未注入观察器，原版 oracle 的条目独立核对。完整 U/F checkout 分别匹配 741/744 个 Git blob，归档权限也逐项匹配；Windows 恢复后的路径长度和四个可执行脚本权限差异，通过进程内选项处理后总校验器通过。Git 读取产生的两个 index 统计缓存已另存并恢复原始字节，封存前再次确认全部恢复文件逐字节匹配原 tar。十一组完整数据与轻量包报告相同且只读复核通过，实际工具身份为 Xcode 27.1/27A9275、SDK 27.0；旧 119/131 原生结果及十二项失败不变，不声称在 Windows 运行了这些 Mac 工具或接受完整 P1。

同一 `5b0e624` 的 Linux job 则在原版 RGBA 流第零轮关闭资源断言失败：fd 为 8，匹配基线；线程计数为 2，基线为 1。Rust RGBA 独立入口通过，下游完整格式流记录校核因原版未完成而失败。PipeWire 原日志包含 join/stop 记录，但不足以单独证明额外任务身份或完整资源退出；当前保留失败并继续复核，不归因于预装工具版本或把 Rust 结果替入原版。

该批 Linux 两份官方轻量 ZIP 的长度与摘要现已核对，PipeWire 的 43,123 个可用登记路径、Wlr 的 8,449 个路径均匹配；完整输入仍分别缺 565/569 项。原生组件保持 192/194 与 135/135 通过，先前部分传输得到的 297 个 RGBA 文件逐字节匹配完整官方包。关闭后的诊断快照显示同一 `MaaScreencap` TID/starttime 已带 PF_EXITING，但 vsize/RSS 仍非零；1/10/100ms 后仅剩基线任务。诊断发生在原失败判定后，不用这些后续样本改判原执行或补齐未完成的二十轮原版格式矩阵。

实际 host 为 Linux 7.0.0-1017-azure。对照 [Linux 7.0 退出实现](https://github.com/torvalds/linux/blob/v7.0/kernel/exit.c)、[线程 ID 唤醒实现](https://github.com/torvalds/linux/blob/v7.0/kernel/fork.c)及 [set_tid_address 接口](https://man7.org/linux/man-pages/man2/set_tid_address.2.html)，child-TID 唤醒可以早于 `task->mm` 清零；PF_EXITING 与 proc 内存清零不是同一个阶段。官方基线来源及摘要已归档，未逐块核验 Ubuntu Azure 的补丁集。现仅修订测试观察器：在原生 stop/join/销毁完成、fd 精确匹配后，仍要求已知原先活跃任务的同一 id/name/starttime 和 PF_EXITING，允许 proc 内存字段仍有值，并继续要求 100ms 内任务集合精确恢复。未知、替换、未带退出标志的任务和未证明的读取错误仍拒绝；原有 ESRCH 与 stat/目录双 ENOENT 规则保留。

本机新回归在旧判据下失败、在修订后通过；五项任务观察单测、完整工作区测试、Clippy、构建和十五个实际编排函数的 mock 子进程控制通过。C++ 控制补充三种明确标注的合成内存状态，并保持二十四项身份/读取/缺失证明反向控制，另直接拒绝仍受闸门控制的真实存活线程。独立一万次 std::thread join 观察先记录原生 before 身份，join 后立即只读一次 stat，再做枚举和日志输出；计数区分缺失、退出且内存有值、退出且内存为零及未见退出标志。该控制会改变自身的观测时机，不与旧观察器比较发生率，也不代替原版捕获清理验收；某一类计数为零时不声称该阶段已原生触发。编排要求新控制摘要和全部迭代计数，旧程序或不完整输出不得通过。

确切 `a5cfa2e` 的 [Actions 41321269889](https://github.com/AbyssRei/maafw-screencap/actions/runs/41321269889) Linux job 随后整体通过，包含真实 C++ 编译/链接、三种合成内存状态、二十四项反向控制、实际存活路径/ESRCH/EMFILE 控制及五项 Rust 观察测试。标准线程一万次 join 的分类为：打开时已缺失 9,969，读取时退出 2，PF_EXITING 且内存字段非零 4，PF_EXITING 且内存为零 25，未见退出标志 0。首二十份标量样本中有三份内存非零，全部核对相同 TID/starttime 与退出标志；第四份计入总数但不在前二十份日志中。此控制不是 PipeWire 捕获清理验收，也不能还原旧失败判定的瞬间。

新 PipeWire 194/194、Wlr 135/135 检查通过，两份官方 ZIP 的长度/摘要及完整清单内 43,092/8,449 个可用登记路径独立匹配，十七组 PipeWire、七组 Wlr 保存报告只读复核通过。七格式每侧均为 140 生命周期、420 次捕获，十四份完整调用表和完整 raw/BGR/持帧/oracle 字节均核对；原版 RGBA 单独完成二十轮、六十次捕获与持帧。完整清单仍分别缺 565/569 项输入；仅看 `native-result` 子集时是 86/90 项，不能用子集数量声称完整来源闭合。新结果不改判 `5b0e624` 原版三次调用后的失败，也不接受 Portal/GPU、完整故障矩阵或 P1。

本次同时澄清既有格式统计：CLI 的 `capture_held_files=2534` 包含十四份 BGR oracle，并非全是捕获/持帧文件。报告的原版 BGR 文件 854 份等于 840 份捕获/持帧加十四份 oracle，Rust raw/BGR 为 1,680 份；实际捕获/持帧合计 2,520 份，完整 oracle 为 28 份。旧日志和报告字节保留，不将这些重叠计数相加或据此增称捕获次数。

同一 `a5cfa2e` 的 macOS job 仍为 123/135 通过，十二项失败保留；工具准备和编译通过，失败步骤是原生 source context。官方轻量 ZIP 的长度/摘要、11,065 个可用登记输入和十二组保存报告独立核对通过，完整输入仍缺 531 项。CG schema2 的原版/Rust 各三次捕获与十八份 payload 通过；AppKit-consumer 原版仍只写出一份几何文件、没有图像，completion 为失败。其独立 Rust 三次捕获、十二份 raw/BGR/持帧文件、选源、已知像素、转换和 owner/窗口关闭记录亦单独校核，但配对 checker 仍拒绝原版失败状态。该次仍未触发原版几何文件缺失分支，不以重复结果证明系统故障原因或接受 P1。

同一 `5b0e624` Linux 官方包的十六组 PipeWire、七组 Wlr 保存报告随后通过独立只读数据校核，报告字节未变。完整格式矩阵仍以缺少调用行拒绝：原版 RGBA 只有三次调用，Rust 独立入口有六十次，不能补入原版或接受配对。外部审阅脚本最初错误要求报错文字含 RGBA，已保留该审阅失败并按实际错误和调用表修正；没有重跑原生捕获或改判其失败。

确切 `875f9b9` 的 [Actions 41306195945](https://github.com/AbyssRei/maafw-screencap/actions/runs/41306195945) 与 `5710c16` 的 [Actions 41307803980](https://github.com/AbyssRei/maafw-screencap/actions/runs/41307803980) Windows job 均已结束。各自的 `manager-delegate-destroy` 独立进程通过一个精确筛选的原生测试，三次真实工厂创建/销毁、两次对象销毁后的注入返回错误、首个错误 7101 和桥接 owner 退出后的模块保留均通过；没有截图。每份部分官方 ZIP 取回七个组件清单、该日志和五个数据/DLL 文件，ZIP CRC 与六个已登记文件的 SHA-256 均匹配，保存的清理报告再经独立 checker 校核且字节未变。完整 ZIP 摘要和完整来源未核验；不把此控制扩展为未知消费、原生析构异常或回调排空证明。

两次 Windows 组件各保持 370/383 项通过：FramePool 为 69/73，四项原版 pseudo 失败；manager 为 98/107，各九项失败。两批共同失败的 mask 为 6/10/14/22/26/38/42/46，另分别为 54 与 30。其余五组件均通过；本次仅下载清理控制的记录，未独立校核这些 runtime 失败的像素或推定成因，也不覆盖原有本机三轮结论或接受整 job/P1。初次外部路径审阅因 Windows 长路径查找失败，在原记录未改的情况下用扩展路径完成校核，初始错误单独保留。

逐提交 CI 另暴露依赖输入的串联问题：`2d2898f` 的 manager 在 libpng 查询收到 HTTP 503 后停止，后续便携 PipeWire 和 FramePool 读取最后一份清单，继而因缺少 `PNG-release.json` 失败。现把共享输入显式绑定到本 job 新产生的 PrintWindow 准备批次，核对公共来源/工具档案、实际摘要和 Rust 准备输入；捕获断言失败仍保留原退出码，不据此丢弃已经完整的依赖。该准备不完整时，其他独立组件各自在线准备，不扫描更早批次，也不把之后的 Custom/manager 清单自动替入。每个实际消费者仍核对原清单、工具链和归档输入；这是显式同 job 离线复用，不声称每次离线调用查询了最新版。

选择控制覆盖缺 PNG、失败 Rust 准备、变更摘要、路径越界、错误/历史 producer；九次实际工作流脚本块的 mock 子进程验证原失败退出 17 保留，以及四个消费者有/无共享输入时的完整参数。PrintWindow 步骤显式接管原生命令退出码；另开启 PowerShell 原生命令错误偏好，用实际退出 17 的子进程验证仍能记录完整共享输入，去掉显式处理的对照则提前停止且没有发布共享输入。所有 mock 均零捕获，不安装工具。另从先前已封存的完整原生工程包复制并核对二十五项真实档案，在新外部副本上通过选择器，原材料未改；它不构成新的依赖更新、离线编译或捕获通过。此工作流改动仍需新原生 CI 执行，历史 HTTP 和后续失败保留。

另完成原版 Query 的外部模块生命周期对照：同一完整函数体仅重命名入口，wrapper 两种模式均先加载并释放一次系统 GraphicsCapture.dll，保留模式另调用进程级 PIN。预定交替顺序连续三轮共六十次原生和六十次数据入口通过，两模式各三十次；独立复算 1,200 生命周期、3,600 次调用、3,240 张成功图像及其持有结果、360 次预期拒绝和 7,200 份 BGR（含 720 份空文件）。普通模式在入口前后观察到模块不在，PIN 模式保持映射；这里只是边界观察，不证明 native 回调已排空。此次普通模式同样全部通过，没有重现旧崩溃，因此不能据此判定 PIN 修复故障，也没有向产品加入保留模块、等待或重试。原未包装程序的卸载后执行 AV 继续保持失败身份。

该对照的正式输入为零、最大观察间隔 81.7486ms，四路注册、注销和 writer join 均独立核对，没有声称新物理输入控制。只读校验器复算原版完整查询/错误/像素/持帧，不伪造 Rust 输出；十一组损坏记录被拒绝。464 份完整产品源码、1,036 个实际 MSBuild 读取路径及 330,854,883 字节输入已归档并核对；十一份实际 F 输入包括七份选定目录文件和四份从完整 F Git/archive 另外核对的公共头，76 份选定 F 来源也全部验证。两次外部来源审阅中止分别是 `.tlog` 目录误当文件、误将十一份全部视为选定目录；原代码/日志分别保留，修正扫描与范围后完成审阅，未重跑捕获。运行、构建实验与最终审阅均已封存、读回复核；采用显式归档开发工具输入，不是在线最新工具链或完整 P1 验收。

Windows manager 的全部 63 个非空方法 mask 现均完成连续三轮，并通过逐批输入审阅和最终汇总：189 次原生入口、189 次数据入口，共 7,560 生命周期、22,680 张发布后持帧和 45,360 份即时/持帧像素文件。正式输入为零，最大观察间隔 142.0842ms，所有观察器及 writer 均确认退出；mask1 沿用已封存的有效 v5 批次，2–63 使用 v6，八份 EXE/DLL 与原始批次一致。实际测量允许两侧选中不同赢家；已知图样只验证自有目标 ROI，DesktopDup 其他显示器像素只验证跨度及持帧。Rust GDI 数量仍只作观察，不宣称零差、源码资源对照或完整 P1。

随后确切2d2898f的 Query/Map/Size 原版/Rust 回归完成三轮27次调用，输入观察有效、事件为零、最大间隔78.9086ms，监听关闭已核对。Map/Size 的18次程序/数据调用全部通过，RustQuery亦三轮通过；原版Query为`[0, 0xc0000005, 0]`，对应checker为`[0, 1, 0]`，这两项当前结果保持失败。独立复核八组完整矩阵的2,640份BGR记录（包含无图的空文件），失败组195份文件只核对摘要，不予像素验收；源码关联另重检2,473份封存文件、1,099个实际原生读取路径（含早期归档引用）、11个编译Fblob及七份冻结程序。

WER及保存的minidump将Query第二轮崩溃绑定到确切EXE：线程54012尝试执行已卸载`GraphicsCapture.dll+0x105ac`（10.0.26100.9549）代码，主线程32104在文件写入中。60条原版查询行均已落盘，持帧保存被中断，`4-recovery-held.txt`为空，checker按原规则拒绝布局；不把完成调用行当作程序通过。[相关维护者调查](https://github.com/robmikh/Win32CaptureSample/issues/99)记录过模块卸载竞态，但相似现象不能证明同一完整根因。本批没有添加等待、模块pin或自动重试，旧程序结果及P1未通过状态保留。

同一源码的 [Actions41277682533](https://github.com/AbyssRei/maafw-screencap/actions/runs/41277682533) Linux job已成功；官方两个ZIP及51,550个可用登记路径、PipeWire17组和Wlr7组下载后数据检查均独立通过，组件分别194/194和135/135。轻量包仍缺87和91个完整输入；Windowsjob及macOS正式Xcode缺失独立记录，不接受完整来源或P1。

确切 `ffcb3a098286b249deff88ba5c07cc57ceedf52e` 的 [Actions 41254680236](https://github.com/AbyssRei/maafw-screencap/actions/runs/41254680236) Linux job 已成功，PipeWire 组件全部 194 项、Wlr 组件全部 135 项通过。两个官方轻量 ZIP 的发布摘要、长度与全部 ZIP 条目校验通过；独立核对 PipeWire 的 43,177 个可用登记路径、Wlr 的 8,448 个可用登记路径，并以已登记的 Windows xtask 对实际下载 bytes/记录执行 17 和 7 组只读数据检查。范围包括真实七格式、MemFd、持续流、Custom、管理器/宿主以及 Wlr 协议、退出与通道持有记录；没有在 Windows 重跑 Linux 原生捕获。

初版只读脚本在 Windows 下未使用扩展路径，将 136 份实际存在的 PipeWire 输入误记缺失；修正版独立确认这些字节与原登记摘要一致，旧审阅及其错误数量保留在外部。轻量包实际仍分别缺少 87 和 91 个登记输入，完整来源/依赖闭合尚待工程包复核，不能以可用文件或数据检查通过代替完整来源验收。该 run 的 Windows job 继续因四项原版 FramePool pseudo 颜色断言失败，macOS 两个原生入口仍停在正式 Xcode 缺失；Linux 组件通过不接受这些平台、整个工作流或完整 P1。新源码 `2d2898f` 的 CI 与本机排队回归另行记录，不由本批替代。

同一 ffcb3a0 run 的 Windows 官方轻量 ZIP 已完整取回，95,910,014 字节及发布摘要、ZIP CRC 均独立核对。七组件共 304/308 项原生检查通过；16,283 个可用登记路径和 31 组下载后只读数据检查通过，涵盖 Custom、DesktopDup 几何、FramePool 六组、manager/delegate 及 ScreenDC 几何/异步首帧。七个运行 mask 的 checker 使用已归档工具，保留当时报告的文字身份；其 Rust GDI 前后计数仍只作观察，不宣称零差或源码资源对照。四个原版 pseudo 失败文件均为 96×64、18,432 字节全黑 BGR，原退出 5 与失败断言保持不变，不从新夹具或后续通过倒推原因。轻量包仍有 604 个唯一来源/依赖输入未提供，完整工程来源、Windows job 及 P1 继续待验收。

FramePool 的 Rust Query/Map/Size 用例现先保存本次返回的 CPU 图像；Map/Size 同时保存 cache，再按原顺序执行状态、错误和严格像素断言。原版 C++ Map/Size 亦先保存两份 Mat，再检查标记；Query 也在断言前写出图像。捕获、转换、等待、绘制及原断言条件未改，保存失败仍使测试失败，不补齐未执行的调用行。旧 Size 标记失败及其缺图记录保持原状态，不能从新保存顺序倒推历史原因。

独立 CPU 控制通过原 OpenCV Mat 和实际 Rust/native 图像 owner 检验两种合法标记、结果/cache 各自的坏标记与末像素、显式空值。Windows 归档依赖下 Debug/Release 共 32 组、16 次预期像素拒绝，64 份像素文件（含空值）均在断言后保留，两侧及两配置逐字节一致，Rust owner 最终归零。`cargo xtask upstream-trace framepool-record-control <外部矩阵> [--check-only]` 校核原始 bytes/layout、完整八行及保存报告；十组记录破坏被拒绝。实际 Step 的十六组 mock 控制通过，拒绝零测试、缺/少摘要、未释放 owner、错误截图范围及非零退出。准备脚本已接入原版、Rust 和数据三个独立步骤，轻量包收集对应目录。Rust 精确入口为 `backends::windows::frame_pool::tests::queries::preserves_image_and_cache_records_when_strict_pixels_reject_them`，显式设置全新 `MFS_FRAMEPOOL_RECORD_ARTIFACTS` 和 `MFS_GDI_PARITY_LIBRARY`。这些控制没有创建桌面窗口或截图；随后真实回归的混合结果见本节开头；托管 Windows 仍待完成。63 组合矩阵使用先前冻结程序，不替代这项验收。

Windows 原生 manager/runtime 入口现扩展至六种原方法的全部 63 个非空位组合，`upstream-parity manager` 的 CI 编排亦逐项运行 `1..63`；原来六个单方法和全选组合的限制已移除。每项仍使用独立 source/Rust 进程/自有窗口、每侧二十个完整生命周期及六十张关闭后持帧，`cargo xtask upstream-delegate win32-runtime <mask> <外部目录> [--check-only]` 逐项复算像素、实际选择、事件和持有记录。实际测速可使双方选择不同赢家，仍按各自原生选择核对窗口/完整显示器范围，不能要求两次独立墙钟测量选择相同方法，也不据此声称逐候选耗时完全相同。空 mask/未知高位不属于此取帧矩阵；既有原 manager 的 None/未知选择器语义未改。

本机全量矩阵按每个 mask 连续三轮执行，首个尝试已停止：mask=1 的三轮有效通过，含双方共 120 生命周期、360 张持有图像、720 份像素文件，正式输入为零，最大间隔 83.3351ms。mask=2 的三次程序和数据检查全部通过，但 15,916 个采样中出现一次 810.3129ms 间隔，超过 500ms 上限，因此整批三轮作废；正式输入仍为零，具体缺口原因未确定，不能归因于用户操作。原始记录与完整来源已封存，鼠标/键盘监听、通知和电源请求注销均从原日志确认。随后修订观察器并用同一 EXE/DLL 的新批次补测，旧批次保持作废；随后完整 63 组合汇总见本节开头。

观察器 v6 将日志写入移至独立线程的有界非阻塞队列，仅在前台 HWND/PID 不变时缓存进程名；桌面、会话、电源、输入计时/光标仍逐次查询，保留相同 500ms 上限并在线检测采样缺口，要求 writer join 且入队/写入/实际行数相等。单独的显式 700ms 观察线程阻塞控制被拒绝，不把短诊断当作长时稳定性证明。首次新批次在捕获前因观察目录名不一致停止，失败保留；修正后实际运行的 mask 2、3、4、5 现均完成连续三轮，原生测试和数据检查全部通过。独立只读复核四批的 480 生命周期、1,440 张发布后持帧及 2,880 份像素文件，原始/低级输入为零，输入计时/光标不变，最大间隔分别为 80.0833、78.5352、80.1721、80.5384ms，逐项确认监听注销和日志写完；完整输出摘要、十五个冻结输入及原生事件亦匹配。连同首批有效 mask 1，当时独立核对的有效前缀为 1–5；随后完整 63 组合汇总见本节开头。Rust 的 GDI 前后计数只作观察，本入口没有零差断言或原版 GDI 计数对照，不据此声称完整资源验收。

报告的 GDI 范围文字已据此更正，验证算法与冻结测试程序未改。新校验器在独立副本上通过九次真实记录校核，并拒绝十四项空/高位 mask、胜出方法、生命周期、输出范围、像素/持帧、关闭摘要、原生调用线程/输入选择器、事件、stderr 与保存报告破坏；四组合法报告除范围说明外与旧报告逐字段相同。副本采用先解除硬链接再写入的方式，原批次及正在执行的文件保持不变。这是记录校验器控制，不增加原生捕获或完整 P1 结论。

各项拥有独立的新鼠标/键盘 Raw Input、低级 hook 监听及完整时间线；输入、前提失效、监听错误或超过 500ms 缺口使当前 mask 的整批三轮作废，其他已独立完成的 mask 保留各自结论。沿用户自动开始授权，不要求新的物理输入挑战，也不继承旧注册的正向控制。静止 FramePool 场景保留原等待/缓存路径，实测一轮约 5.5 分钟；未为缩短矩阵改等待或减少生命周期。程序、来源、工具链和原生编译输入已冻结外置，使用归档依赖开发输入；新在线、托管 CI 和完整 P1 继续待验收。

macOS 私有 manager 已补 `get_features`：固定来源仅从输入 owner 汇集能力，当前 Input=None 路径在连接前后、失败重连及 inactive 后均返回零。新的原接口查询消费通过精确提取的完整 `MaaMacOSControlUnitCreate/Destroy` 函数体和真实 `MacOSControlUnitAPI` 虚表调用同一个 source/Rust manager，读取 connected、十进制窗口 UUID、能力位和有符号选择器元数据。它仍是显式查询 facade，输入和截图槽不在本项范围，不构造完整原 manager/input 类；Rust 同步回调、固定 CPU 复制及原生 meojson parse/object 表示转换均为明确记录的验收适配成本。

本机归档输入 Debug/Release 各通过 66 组 ID/方法、132 个矩阵对象和一个故障控制对象、528 份快照、2,113 次虚接口查询及 1,255 次 Rust 回调；双方在 cold、连接、重连和 inactive 四阶段逐字节一致。销毁全部对象、卸载适配库后，固定 CPU 快照移入已 join 的线程继续读取。六项真实错误边界包含 NULL 输出/context/owner、原 NULL-safe Destroy 及被 C++ 边界收住的 Rust 查询错误；另外十组篡改记录均被独立校验器拒绝。七项实际 Step mock 控制验证非零退出、零测试、缺/少摘要、错误数据和 portable 环境不能通过 native 守卫；mock 的环境记录和库名是明确夹具，不构成 macOS 运行证据。

实际入口为 `cargo xtask upstream-macos-queries <外部目录> [--check-only]`；精确测试名为 `backends::macos::manager::query_tests::consumes_original_virtual_queries_and_retains_cpu_snapshots_after_unload`，需显式设置 `MFS_MAC_MANAGER_QUERY_LIBRARY` 与全新外部 `MFS_MAC_MANAGER_QUERY_RECORDS`。Windows 使用 `prototype-windows` 测试 feature，验收适配库从 `tests/upstream-parity/macos-manager-queries` 单独 CMake 构建；默认产品不链接 OpenCV/meojson。本机两配置明确记录 `environment=0`，只覆盖受控 macOS 不可用分支，零截图，不需桌面输入监听。macOS 分支保留真实 `__builtin_available`、原 SCK 类及现有 Rust NativeManager，确切 e243fee 的原生 Actions 已实际通过 environment=1 的查询入口；下载后独立复算 528 组快照、2,113 次虚接口及持有值。官方轻量 ZIP 摘要及 10,978 个可用登记路径均核对，另有六十项完整来源输入缺失，不把查询数据复核扩大为完整源码重建、截图或平台/P1 验收。早期 Windows 归档开发依赖和原始缺环境结果保留原范围，其他原生分支/格式/故障及 framework 转发继续待完成。

Windows 原接口的四个查询槽现已接入同一私有 Rust manager：真实原 `Create/Destroy` 函数体返回原 `Win32ControlUnitAPI` 接口，由 C++ 调用方经虚表调用 `connected`、`request_uuid`、`get_features` 和 `get_info`。原版一侧编译精确提取的 manager 查询/连接函数体，Rust 一侧通过同步窄桥调用现有 manager；二者仍是显式 capture-only 验收类，Input=None，不构造完整原 manager/input 类。UUID 借用字节在下一次 context 操作前进入原生调用方的字符串；metadata 另显式解析为原 meojson 对象，该表示转换及复制是适配成本。桥在全部原生临时对象结束前复制固定 CPU 查询快照，按原头检查 33 项大小、对齐和偏移，不暴露 C++ 对象或 Rust String 借用。

最终 Debug/Release 各通过 256 个工厂对象、500 组快照、2,000 次真实虚接口查询及 80 次隐藏消息窗口生命周期；合计 4,000 次查询、1,000 组模块/窗口关闭后快照，随后移入已 join 的线程继续读取。每侧包含 88 组完整指针/选择器值，另通过 None/未知方法连接，核对真实窗口销毁后 connected 的变化、停用、相同 HWND 的 UUID/元数据及完整模块卸载；None/未知方法不创建截图 backend。两配置各 36 组工厂构造和 274 组外层委托模型回归亦通过。实际入口为 `cargo xtask upstream-delegate win32-queries <外部目录> [--check-only]`，独立复算全部查询字节、工厂/窗口/虚接口顺序、线程、context 计数和持有结果；它不执行屏幕捕获。

本批 456 份产品源码、76 个 F blob、十份实际编译读取日志及三十份编译/链接/生成读取日志的 1,019 个原生路径均核对归档，查询函数体与 F 相同。十三组篡改覆盖双方相同的 UUID/有符号元数据错误、销毁后状态复活、空记录、漏调用、错线程/工厂参数、context/host 次数、持有值、模块未卸载和缺失/额外产物，均被拒绝；首次失败的部分记录也严格拒绝。六项实际 Step mock 控制另拒绝零测试、少快照、缺摘要、非零退出和错误数据。首批在 Rust 对象创建时因提供的混合分隔符 DLL 路径被安全加载边界拒绝而失败；原版一侧已保存的 250 组查询保持部分证据，整批仍失败。独立新进程控制确认本机反斜杠形式有无 `..` 均能加载同一适配器，正斜杠和混合形式失败；不能把原因仅归为 `..`。同进程先加载模块会掩盖该差异，两个控制分别保留。规范路径的新批次通过。首次 Rust EXE 未在后续测试输入表复用调整前冻结，不声明其逐字节原程序重放；两份 C++ provider DLL 与最终 Release 程序相同。

默认 workspace 121 项通过、6 项忽略，Windows 模型 53 项通过、69 项忽略，新增查询另按精确入口实际执行；严格 Clippy、Windows manager Release 核心、格式和 PowerShell 解析通过。初次 Clippy 因输入表重复加载失败，现共用同一份定义；首版读取审计错误地将无 BOM 的生成日志按 UTF-16 解码，已分别识别编码并保留原失败。当前 106 项职责映射已刷新代码锚点及该局部原生证据，58 个原用例的完整待验收范围保留。该批沿归档的匹配 nightly/VS/SDK/OpenCV/Boost/MaaUtils/meojson 开发输入，不构成新的在线依赖、托管 CI、完整原 manager 类、混合捕获候选/故障或 P1 验收。新入口、非零测试守卫、部分输出登记与 CI 收集已接入；macOS 原生查询、全平台和完整调用链仍待完成。

本轮重新按完整职责目录盘点 P1，而非只检查固定 F 的 76 个选定来源文件。最新原目录实际有 101 项来源、11 条捕获路由、58 个用例定义；与 F 选定范围合并后为 106 项，其中 99 个目录文件未变、两个目录文件改变，另有五个原目录未列入的 F 构建/回归文件。外层 Gamepad 截图委托、公共头、host Portal 边界及排除项仍逐项保留；HwndUtils 与根 CMake 的祖先差异明确记录。映射绑定归档的实现/验证代码及实际行文本，当前只到模块职责层级，不代表完整逐函数迁移或用例通过。58 个用例均保留各自尚未验收的完整范围；旧 U 几何定义不直接继承为 F 预期。

实际格式及只读校验入口为 `cargo xtask reference responsibilities <reference 相对映射文件> [--root <外部目录>]`。它核对完整目录与所选 F 范围的并集、全部用例、F Git blob/来源字节、SHA-256、代码行锚点及已链接的局部证据；不修改记录，不产生捕获或阶段通过结论。八组实际破坏分别覆盖仅保留 76 项、漏用例、将捕获职责改为排除、漏 F 语义差异、错代码行、错来源摘要、虚构 passed 字段和路径逃逸，均被拒绝。此工具检查映射的完整性与来源，不能自动证明人工说明的完整语义。

盘点发现的 Windows manager 查询缺口已补齐：同一私有 manager 新增 `request_uuid` 和 Input=None 下的 `get_features`。前者保持原 MSVC x64 指针流的十六位大写十六进制，包括空 HWND；不验证窗口，也不要求连接。后者因无 mouse/keyboard owner 返回零，不引入输入控制。显式函数体模型从固定 F 提取原 UUID/features 代码，以真实 MSVC 标准库执行，空输入指针分支保持，日志是测试边界；它不构造完整原 manager 或消费真实 C++ 虚表。最终 Debug/Release 各 88 组原版/Rust 查询通过，包含空值、符号位与完整 64 位边界，输出在 manager 作用域后继续保留。两配置原有 Windows/macOS 元数据另共 308 组回归通过；`cargo xtask upstream-metadata win32-identity <外部目录> [--check-only]` 独立复算实际查询记录。当前默认 workspace 120 项通过、6 项忽略，Windows 模型 53 项通过、68 项忽略，严格 Clippy、格式、Windows manager Release 核心构建及 PowerShell 解析通过。初次 Clippy 拒绝测试中不必要的显式 drop，现以词法作用域验证所有者结束；前后实际查询字节相同，失败日志保留。

该早期字段查询批核对 454 份产品源码、两个实际编译读取日志、326 个原生输入和原函数体；新增 Step 的六项 mock 控制拒绝零测试、少案例、缺摘要、非零退出及双侧相同的错误记录。模型、提取代码和完整输出已归档；首个旧缓存仍指向祖先 U 的构建单独保留，最终两配置明确使用 F。该批采用匹配 nightly、meojson、VS/SDK 的离线开发输入，未做屏幕捕获或新的在线依赖验收；其自身不证明真实 DLL 虚接口消费，后续该项进展见本节开头。完整 manager/factory 转发、macOS 原生查询及全平台完整故障矩阵仍待完成；既有历史复测、原版像素失败和缺少正式 Xcode 的结论不变，P1 继续未完成。新 CI 入口已接入 manager 编排；网络 EOF 期间没有取得新托管验证结论。

Desktop Duplication 另新增真实 held-acquisition 拒绝矩阵：夹具先在同一 duplication 上取得并持有实际帧，再沿完整原版/Rust 公开捕获调用真实 `AcquireNextFrame(2000)`，获得系统返回的 `DXGI_ERROR_INVALID_CALL`。捕获失败时没有 CopyResource、Map、Unmap、ReleaseFrame 或转换；原 staging、duplication、尺寸及 warm 状态保持。随后夹具自己的 ReleaseFrame 必须成功，再次诊断释放必须被系统拒绝，最后只调用一次正常捕获并验证已知目标像素。Rust 另断言失败样本没有 native frame、释放或 CPU 复制证据，恢复时保留既有整屏安全复制及转换位置；C++ 沿固定 F 原函数和真实接口转发，原失效映射与 Rust 独立 CPU owner 的区别仍保留。这是私有受控测试前提，不定义公共 SDK 对外部 native frame 共享的支持。

新用例和既有 Map 矩阵在同一新 EXE/DLL 上连续三轮，共十八次命令、720 次公开捕获调用的预期结果全部符合。新矩阵单独包含双方 360 次调用、120 次真实拒绝、120 次夹具成功释放、120 次额外释放拒绝、120 次恢复、240 张持帧及 480 份即时/持帧 BGR；Map 回归另有 564 份 BGR。`cargo xtask upstream-trace desktop-acquire <外部矩阵> [--check-only]` 核对每轮 326 份文件，合并 Map 三轮共核对 1,044 份像素和 2,115 份矩阵文件。十二组破坏覆盖错误拒绝码、提前释放夹具帧、额外释放成功、失败时复制/释放、状态或线程改变、错 staging、缺少 CPU 复制/调用行、改变的合法持帧及失败泄露像素，均被拒绝。六组实际 Step mock 控制另拒绝零测试、不完整/缺少摘要和非零退出。

该批 542 份冻结输入、453 份产品源码、九份 MSBuild 读取日志、878 个原生路径、76 个 F blob 与生成绑定均核对。正式原始/低级鼠标键盘输入为零，系统输入计时和光标不变，最大采样间隔 77.917ms，全部监听及电源请求已注销；没有新的人工正向控制声明。默认 workspace 118 项通过、6 项忽略，显式 Windows 模型 53 项通过、67 项忽略，新原生用例在本批另行实际执行；Clippy、格式及 DesktopDupWindow 核心 Release 构建通过。依赖与匹配 nightly 沿显式离线开发输入复验；在线工具链、托管 CI、自然设备/访问丢失、超时、完整 GPU 计数和 P1 继续待验收，历史旧程序结果不变。

Desktop Duplication 新增原版/Rust 的真实 Map/Unmap/ReleaseFrame 边界矩阵。C++ 独立进程直接编译固定 F 的原 header/cpp，以非拥有表达式视图转发相同 context/duplication 调用；诊断日志只解开视图打印原生地址，不替换 COM 对象、vtable 或源码语句。Rust 观察仅在 `test + prototype-windows` 编译，默认映射仍使用原参数。每生命周期先以正常公开捕获建立 warm 状态，再对一次公开调用施加实际 `UINT_MAX` subresource Map 拒绝、原转换位置的受控异常/错误或无故障控制，随后只取一次恢复帧。断言真实 acquire/copy/map/unmap/release/convert 顺序、相同原生对象和调用线程、2000ms 原等待、失败 Map 不消费输出及不 Unmap、成功 Map 恰好一次 Unmap、转换前已 ReleaseFrame，以及后续恢复和关闭后的持帧。原版失效 mapped Mat 与 Rust 既有整屏 BGRA 安全复制仍是明确不同的所有权路径，不宣称消除了复制或修复原版。

本机连续三轮共九次原生/记录命令全部通过：双方合计 360 次公开捕获调用、240 次直接观察调用、42 次原生 Map 拒绝、36 次受控转换失败、120 次恢复和 282 张持帧。`cargo xtask upstream-trace desktop-map <外部矩阵> [--check-only]` 独立复算每轮 188 份目标 BGR 和 379 份文件；三轮共 564 份像素、1,137 份矩阵文件，保留整屏 BGR Mat 的实际 ROI 行距，不误写为紧密目标行距。十组破坏覆盖 Unmap 计数、释放/转换顺序、错误 subresource/HRESULT、缺少安全复制、失败泄露像素、线程、步长、缺行和合法但改变的持帧图样，均被拒绝。六组实际 Step 控制另拒绝零测试、不完整/缺少摘要和非零子退出；它们是 mock 编排控制，没有再取帧。

该批冻结 539 份输入与 450 份产品源码，实际 MSBuild 九份读取日志、877 个原生路径、76 个 F blob 和生成绑定均核对；初次源码审计遗漏 caret 头中的主输入而停止，修正解析后通过，旧代码/日志保留。首个 C++ 编译的包装值日志转换错误，以及首个 mock 子脚本错误拼接退出命令的失败亦各自保留，未重标为原生运行失败。正式四路输入为零、系统输入计时和光标不变，最大观察间隔 78.7076ms，监听和电源请求已注销；没有声称新的人工正向控制。默认 workspace 共 118 项通过、6 项忽略；显式 Windows manager feature 的模型测试 53 项通过、66 项忽略，另有本批明确执行的新原生矩阵；严格 Clippy、格式及 DesktopDupWindow 核心 Release 构建通过。依赖和匹配 nightly 使用显式离线开发输入，在线最新版本/原生 CI 仍待网络恢复。它不覆盖 warmup 专项、自然 access/device loss、完整 GPU 资源计数、多输出/格式或完整 P1，也不替换历史旧程序结论。

已修正已知标记绘制夹具的一个中间状态：背景填充先排除左上角 16×16 标记，恢复调用方 DC clip 后再绘制新标记。隐藏窗口/私有 DIB 控制编译实际 [renderer](../../tests/upstream-parity/native/print-parity.cpp)，在每次真实 FillRect 后检查全图、clip、bitmap 与资源；旧绘制函数体未改的对照有 12/16 个无标记边界，修正版独立构建及主 CMake 新目标均为 16/16 个有效标记，GDI 均为 2→2、清理错误为零。48 份完整 BGRA 已独立复算；共享 renderer 和 PrintWindow 两种适配编译通过。该控制没有桌面捕获，不能确定历史 WGC 是否采到了该中间状态。实际 Step 的五组控制另验证有效运行、空/不完整摘要拒绝、资源不平衡拒绝与非零退出保留；CI 轻量包已接入控制记录。捕获流程、等待及像素断言未变，新的托管捕获回归仍待验收。

确切 `ffcb3a098286b249deff88ba5c07cc57ceedf52e` 随后通过本机连续三轮五组原版/Rust 回归：Map、Size、查询、真实零客户区的准备恢复及目标销毁，共 45 次程序/数据调用全部通过。独立复算 3,840 份完整 BGR，冻结的 538 份输入、445 份产品源码及 66 份与既有 Rust 程序一致的 Windows 源码均匹配；实际 MSBuild 的 21 份读取日志、1,087 份原生输入、76 个 F blob 和 Size 副本四处表达式替换均核对。正式四路输入为零、系统输入计时和光标不变，最大采样间隔 78.0034ms，监听已确认注销；自动启动没有声称新的人工正向控制。依赖和 Rust 测试 EXE 使用显式归档开发输入，没有新在线 Rust 构建。首个 Git tar 审计漏处理 Windows 换行而停止，失败代码/日志保留；修正后分别保留原始摘要并严格核对仅 CRLF→LF 的 Git 对应关系。旧程序结论、历史失败原因、托管原生验收和完整 P1 不由本批替代。

初版 `bf5303a` 的 [Actions 41247571643](https://github.com/AbyssRei/maafw-screencap/actions/runs/41247571643) Windows 轻量包已复核七组件共 300/306 项通过、15,928 个可用登记路径。完整官方 ZIP/tar 随后核对全部 23 份运行清单、43,158 个登记路径、22 份普通 444 文件 Git 快照、一份不可构建控制、76 个 F blob 和实际 CMake 来源；全部 16,531 个原生路径及原始内外清单与轻量包匹配，补齐其 603 个唯一来源/依赖输入。FramePool 仍为 61/67：四项原 pseudo 颜色断言、原版 Map 标记及随后缺少调用行的 checker 仍失败。Map 只写完两周期六行，第三周期 warm 的 18,432 字节 BGR 恰为三个无标记背景区域；另外五项下载数据检查通过。此原始失败单独保留，不由上述夹具控制重标或倒推原因，整个 Windows job 和 P1 未验收。

确切 `10d73c92fa2953c157373ef252ab28725b14ad5b` 的 [Actions 41240527770](https://github.com/AbyssRei/maafw-screencap/actions/runs/41240527770) PipeWire 164 项检查通过。MemFd 真实流受控 NULL-data 的 40 行记录、240 份原始/持帧像素及 Rust 60 次 map/unmap 已独立复核，既有真实 kernel DMA 回归通过。完整 Linux 工程 ZIP/tar 随后核对全部 19 份清单、79,300 个登记路径、18 份普通 436 文件 Git 快照、一份不可构建控制快照、76 个 F blob 和实际 CMake 来源；全部 40,422 个 PipeWire 原生路径与轻量 native-result 原始字节匹配，补齐其 86 个输入缺口。原版复制/映射次数仍为 unknown，未接受自然 NULL、GPU/Portal、任意故障或完整 P1。

确切 `983da16b7fcc5ba53849c564216b414d8ea38b47` 的 [Actions 41236808686](https://github.com/AbyssRei/maafw-screencap/actions/runs/41236808686) Size 四项及 manager Size 拒绝两项检查通过。独立复核全部 400 份像素及 808 个矩阵文件：Size 共 120 次调用、388 次实际查询、36 次拒绝、40 次恢复和 84 张持帧；manager 共 20 生命周期、40 次 FramePool 拒绝、60 次 GDI 捕获和 20 张持帧。完整 Windows ZIP/tar 核对全部 23 份清单、43,171 个登记路径、22 份普通 430 文件快照、一份不可构建控制快照及 76 个 F blob；FramePool 2,789 和 manager 3,020 个原生路径合并去重为 5,805 个，全部匹配，补齐 174 个输入缺口。该批七组件仍为 302/306 通过，四个原版 pseudo 断言保持失败。manager 证明组合原函数体模型、独立原版 Size 矩阵和真实 Rust native factories，不是完整原生原版 manager 同场执行。

文档工作流的源码/CI 提交与纯 Markdown push 触发范围已用官方任务记录核对：495d884、8f75fef 同时创建文档和原生工作流；只有三个 Markdown 文件变化的 44baa21 只创建成功的 [文档任务 41244200477](https://github.com/AbyssRei/maafw-screencap/actions/runs/41244200477)。所有 PR 和手动原生调度仍保留完整矩阵，未新增 PR/手动原生执行的验收声明。文档检查不构成捕获验收。[独立编排控制](../../tests/prototypes/framepool-independent.ps1) 的三个 mock 子步骤在本机及三种 runner 上验证失败保留、后续执行和参数不变，没有执行捕获；真实新编排效果仍待后续记录。

早期 d8c 的官方 PipeWire release 查询 HTTP 500，以及 627 的原版 zero-client 标记缺失失败均单独保留。后者停止在 45 行之后，未运行后续 Map/Size；不将其当作 Size 结果，也不由后续通过倒推失败原因。完整来源核对不扩展各批运行证明范围，P1 仍未完成。

## 本次连续三轮无输入复测

后续本机桌面测试按用户最新授权直接开始，只在正式期间监听到输入后询问；输入、观察器故障、桌面前提失效、输入计时/光标变化或超过 500ms 的采样缺口仍作废整批。新自动启动批次先核对当前四路 Raw Input/低级 hook 注册和持续采样，记录真实正向控制的实际范围。其所用输入处理实现与此前已通过实际鼠标/键盘控制的版本相同，但不将旧注册的控制冒称为本次新注册的控制；下述旧程序封存结果与要求保持原身份。

ScreenDC 新增独立 `native_pending::matches_first_capture_before_queued_window_adjustment_is_consumed` 用例及 `cargo xtask upstream-trace screen-pending <外部目录> [--check-only]`。它在自有 UI 队列的测试闸门处暂停，要求首次原版/Rust 捕获仍取得旧几何的已知图样，直接释放后才等待真实调整完成，再取最终图并检查持帧、资源与单次移动；捕获/helper 调用顺序不改。门仅干预夹具，失败超时有界，不能据此宣称取消原生调用。预期 40 个原生侧、80 次截图、80 份关闭后持帧及 321 份登记文件；尚未通过原生矩阵。

首个 96×64 夹具的有效三轮均在几何设置处失败，实际客户区宽为 120，尚未截图；原失败单独封存。改为 192×128 后尺寸设置通过，但前台准备三轮失败，早期桥未保留确切状态。补齐原生状态/激活快照后的独立三轮确认：原生返回 5，窗口在其 UI 队列上 active，而真实 foreground 是另一个窗口；各轮按 `missing_environment` 在闸门和截图前停止，未强制改变该前提。新三轮正式输入为零，最大间隔 76.79ms，415 份冻结输入和 407 份源码核对通过，监听已注销；三份未完成矩阵均被只读校核器拒绝。新旧失败均保留，不计入 525 次旧入口调用，也不接受闸门运行、像素或完整 P1。该探针利用 [SetWindowPos 的异步队列语义](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowpos)，实际前台条件防止原前台 helper 向受阻 UI 发出同步请求；其他未完成的窗口竞争路径继续待验收。

ScreenDC 两项测试的夹具清理现已完成本机有效三轮验证：恢复原调用方拥有的基础窗口，保留独立呈现屏障、可见性观测、完整已知像素、GDI 基线及关闭后持帧断言；捕获/helper/等待代码不变，独立 UI owner 夹具继续用于几何和渲染消费者。6 次执行均通过，四路输入为零，最大采样间隔 75.54ms，监听正常注销；匹配固定 F 的原生适配器下，60 轮原/Rust 对照及 60 轮原版呈现控制的 240 份实际 BGR 均独立核对。首次误用缺少 `mfs_screen_window_observation` 的旧适配器，6 次均在取帧前加载失败、没有像素输出；该失败批次已单独封存。后续从已验证的官方完整归档恢复匹配 DLL，先核对 11 个必需导出、相邻原生依赖及登记摘要，再运行同一 Rust 程序和断言。清理只撤回显示器熄屏误判引入的这两处夹具要求，不改变其他历史失败结论或计入下述旧入口清单的 525 次调用，也不完成 P1。

剩余 66 个旧 Rust 原生日志入口已完成有效连续三轮，共 198 次调用：65 个配置三轮全过，1 个 PrintWindow 聚合配置的退出码为 `[101, 0, 0]`，按三轮全过规则保留失败。其第一轮在 target-destruction 场景的 `(12,11)` 像素断言得到黑色而非预期颜色，未单独重试或覆盖；后两轮通过不确定其根因。正式期间四路原始/低级输入事件为零，系统输入计时与光标不变，最大采样间隔 83.43ms，原生监听已确认注销。324 份冻结输入及 3,279 份重建源码哈希一致，逐次所选测试、语义记录与原生证据已独立核对并封存；重建仍使用历史源码/nightly/lockfile与当前 linker，不宣称历史 EXE 逐字节重现。

按固定的 238 份旧 manifest 路径展开清单，82 个确切 Rust 原生入口现均有有效三轮记录，77 个通过、5 个失败。另一个直接原生入口清单共 49 个，其中 9 个按相同绝对 EXE、摘要和递归导入的相邻 DLL 映射至完成批次；其余 40 个随后完成连续三轮、120 次执行，全部通过。该批为 20 个 GDI、12 个 FramePool 上下文和 8 个 Desktop Duplication 入口，使用确切归档 EXE/DLL，不重新构建；202 份冻结输入、2,430 份登记源码及实际像素/布局、持帧与关闭证据均核对。正式四路输入为零，系统输入计时与光标不变，最大采样间隔 80.52ms，监听正常注销，整批独立校验和封存通过。49 个直接入口当前为 45 个通过、4 个失败，待补为零。

上述旧入口与此前各批累计 525 次有效调用；两个固定入口清单均已完成，清单之间及原 36 份失败日志有映射重叠，不能把配置数简单相加为所有曾构建程序。全量执行前的空环境脚本错误发生在零次原生调用时，原失败记录保留；修正仅清理 MFS 环境变量，不改原程序。原始失败继续按实际断言、加载、环境与测试假设分类，当前结果不能倒推历史输入原因。当前结论仍限本机、已枚举来源和实际断言，完整 P1 未验收。

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

`2aca960` 的 [Actions 41199067187](https://github.com/AbyssRei/maafw-screencap/actions/runs/41199067187) Windows 完整归档现已独立核对。初次下载因 unexpected EOF 中止，原部分文件与失败另存；HTTP 206 仅补齐缺少的后缀，拼接后严格匹配官方 ZIP 的完整摘要和字节数。工程 tar 的 23 份清单、39,504 个登记输入/日志路径、22 份普通 402 文件产品快照、一份故意不可编译的 consumer 夹具、固定 F 的 76 个 blob 及七个实际 native CMake 来源根均匹配。七组件 283 项保留 279 项通过和四项原 FramePool pseudo 颜色失败；七个 Custom 与另外 17 个下载数据校核已通过。四张失败原图确实在轻量包中，使用 Windows 长路径读取后为已登记的全黑 96×64 BGR；完整性检查不确定黑图原因，也不接受当前新增实现或完整 P1。

同次 [Actions 41186951837](https://github.com/AbyssRei/maafw-screencap/actions/runs/41186951837) 的 Windows 官方组件与完整工程归档现已独立核对。七组件共 283 项检查，279 项通过；FramePool 四个原版 pseudo 颜色断言保持失败，其余六组件各自全部通过。完整 tar 的 23 份运行清单、39,504 个不同登记输入/日志路径、同一 F 的 76 份原始 Git blob 和七组件实际 CMake 来源均匹配。22 份普通产品快照各含 402 个 Git 文件，文本检出只允许已记录的 CRLF/LF 差异；另一份故意不可编译的配置夹具单独核对两个注入入口和四份 consumer 缓存记录。所有登记文件的原始字节摘要仍严格匹配，缺 SDK 的显式 ON 子运行保持失败，父检查仅接受其预期失败。

下载后七项 Custom 和另 17 项几何、FramePool alpha 诊断、delegate/runtime 及 metadata 数据校验通过。四份失败 FramePool 图像从完整 tar 按登记摘要恢复，均为 96×64、18,432 字节的全黑 BGR；原日志记录返回图像、已知内部颜色不符、关闭与持帧证据成功及 exit 5。alpha 诊断通过不使这四项捕获通过，也不证明输入、保护或其他黑图原因。轻量包缺少的来源由完整归档补齐，先前因轻量包缺图而停止的审阅脚本及日志继续保留。以上是该确切提交的来源与数据审阅，未执行新的本机捕获，不完成 Windows job、其他平台或完整 P1 验收。

PNG 修复随后已在 [Actions 41184885199](https://github.com/AbyssRei/maafw-screencap/actions/runs/41184885199) 的确切 `b9e9f42` Linux PipeWire 组件通过，154 项组件检查全部通过。官方轻量 ZIP 摘要匹配，下载后复核 40,043 个实际登记路径和全部检查日志；原始图像、身份、元数据、实际虚接口、重入与新增编码/缩放的六项数据校验均通过，144 组、288 张实际捕获/持帧及 576 次缓存解码另行复算，真实 PNG 探针记录 libpng 1.6.59/zlib 1.3.2。轻量包包含 487 个未提供的输入或一字节占位，未用它们冒充完整源码/依赖归档；完整来源审计仍待完成，该结果也不覆盖后续新 raw-type 矩阵、其他组件、macOS 或完整 P1。原 41174221177 的 PNG 夹具失败保持失败。

Custom 现另有实际原始像素类型矩阵：显式适配器根据编译头文件查询 13 种深度及 128 种通道，共 1,664 种规范类型，涵盖 16F/16BF、Bool 和扩展整数类型；这些值仅用于私有对照，不是公开 SDK 格式。原 `MaaImageBufferSetRawData` 函数体保持不变，安全验收桥先检查类型、保留深度和可读 span，再在同一原/Rust manager 回调位置调用真实 setter。Windows 归档依赖开发验证的 Debug、Release 各通过 4,992 组、9,984 次真实调用；0/1/255 返回值、输入覆盖后的 clone、false 输出保留、9,984 个 Mat 在 input/context 结束后持有和模块卸载后的独立 CPU 字节读取均核对，两个 profile 的 9,662,208 字节捕获和持帧流逐字节一致。

六类短 span、保留深度、额外类型位及溢出几何输入均在调用原 setter 前被拒绝，原 helper 调用计数和已有输出不变；七组篡改记录被独立 `upstream-trace custom-raw <外部 raw 目录> [--check-only]` 拒绝。实际 OpenCV 类型头随原生批次归档，零测试/缺失完整计数不能通过入口校验；六项既有 Custom native/data 回归、workspace 测试和 Clippy 通过。首个 standalone 命令因父输出目录未创建在实际捕获前失败，记录保持失败，补齐外部目录后使用相同程序执行；新增的 span 拒绝矩阵另有后续独立执行。上述本机结果仍是 Windows 开发对照，不表示所有编码格式、类型标志组合、颜色转换或完整 Custom/P1 已验收，核心截图算法未变。

随后 [Actions 41186951837](https://github.com/AbyssRei/maafw-screencap/actions/runs/41186951837) 的确切 `2730b87` Linux PipeWire 组件通过全部 156 项检查，包含新增 raw 原生与实际记录入口。官方轻量 ZIP 的 SHA-256 与登记摘要一致，下载后独立复核 39,965 个实际登记路径及全部检查日志；原图像、编码/缩放、身份、metadata、实际虚接口、重入与 raw 七项数据校验均通过。Linux 的 1,664 类型、4,992 案例、9,984 次原/Rust helper 调用及 9,984 个保留 owner 均核对；类型目录、逐例记录、六类拒绝记录、作用域/释放计数和两个各 9,662,208 字节的实际捕获/持帧流与 Windows Release 逐字节相同。轻量包有 491 个未提供或占位输入；其后官方完整工程 ZIP/tar 均核对摘要，已逐项验证该组件全部 40,456 个登记输入/日志路径、同一固定 F 的 76 份源码 blob 和全部 402 个产品 Git 文件，另严格核对单独归档的 Cargo.lock。两个实际生成的 Custom helper 与已审阅 Windows 输入仅有 CRLF/LF 差异，源码文本对照只规范换行，各自原始字节摘要仍严格匹配。首轮校验误将额外锁文件计为 Git 文件、第二轮误要求跨平台生成文本原始字节相同，失败日志及原脚本均保留；修正校验后通过，没有更改 native 输出。该完整来源审计只覆盖此组件，其他组件、平台和完整 P1 仍未由它验收。同一 run 的 macOS 两个入口日志明确停在正式 Xcode `27.2 (27B52)` 缺失前提，未新增实际取帧结果；不替用 RC 或覆盖早期失败。

PNG 参照依赖现由 `scripts/prepare-png-codec.ps1` 显式准备：在线批次每次查询正式 libpng/zlib，登记版本、确切 Git 来源、规范源归档与许可，构建静态库并运行六个已知 RGBA 像素的真实编解码探针；Windows 及 Unix 的 imgcodecs 准备使用相同依赖，并检查 OpenCV 确实选择这些库，禁止静默回落到其他 PNG/zlib。libpng 使用官方主页的当前公开版而非没有 Release 条目的 GitHub 镜像；本次为 libpng 1.6.59、zlib 1.3.2。离线批次必须包含登记的 release/page/resolution/source 输入；缓存复用仍进行在线更新检查。

同一 `2730b87` 完整 Linux 工程 tar 另已按全部 19 份运行清单核对 77,689 个不同登记输入/日志路径，18 份产品快照各严格匹配 402 个 Git 文件。独立配置夹具的一份快照按实际控制单独核对：400 个产品文件、两个故意写入 `compile_error!` 的入口和四份 consumer Cargo 缓存记录；原生准备确在捕获 crate 无法编译时独立执行成功。两份显式 ON 且依赖不可发现的子运行保持失败，父检查按预期非零退出验收，未改为捕获通过。首轮整包校验没有区分该受控快照而停止，其原脚本和日志保留；后续仅按已登记夹具和父检查验证这些确切差异，普通 Git 文件及所有原始摘要保持严格比较。Wlr 组件 131 项检查均通过，下载后实际虚接口、构造失败、协议、registry 与失败/断连五项记录校验通过；另复算普通、共享 client、重连、manager、NUL/path-name 和 FD-zero 场景的 280 张原版 BGR、300 张 Rust BGR 和 300 张 raw 图像，841 组原/Rust 路径比较一致，60 张 manager 原 oracle 未被覆盖。Wlr 的实际 CMake 根和同一 F 的 76 份源码 blob 也匹配。以上完整性与数据核对不扩大这些运行的覆盖范围，也不接受未知 live-snapshot close、FD-zero 部分失败、其他平台或完整 P1。

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

启用 `prototype-windows-com` 后，非桌面 COM 探针也使用 `-- --test-threads=1`。其初始化配平检查要求进程中没有另一个测试持有 MTA；独立原生控制确认，另一个 MTA 尚存时，新线程的 `CoGetApartmentType` 会成功报告 implicit MTA。本次未串行的 workspace 调用因此有两项配平前提断言失败，原日志保留；按既有原生编排串行执行后，所选 workspace 的 120 项检查全部通过，另有 22 项显式忽略。该修正是执行方式校正，未修改断言或 COM 实现。

`prototype-windows` 在自有小窗口绘制确定颜色，以 GetDC → compatible bitmap → BitBlt → GetBitmapBits 实际取帧，校验 BGR 字节，并在释放原生资源后验证独立像素及跨线程释放。`prototype-windows-com` 叠加私有 rich 投影与三项测试：平衡 WinRT 初始化计数、宿主 STA 拒绝 MTA 后保持 apartment，以及显式 WARP 设备的 D3D11 → DXGI → WinRT 对象身份往返与 staging 像素读回。后者校验非对称两行 BGRA，按实际 RowPitch 只复制目标像素，释放所有原生对象后跨线程持帧，并核查线程 DPI 上下文。WARP 是明确选择的软件设备，不是硬件失败后的回退。

第四项 COM 测试创建使用上游 `Create` 入口的 FramePool，核查未启动会话时的“原生 HRESULT 成功 + 空帧”与 rich 投影空错误的差异，并验证未启动会话的事件注册、显式注销 HRESULT、Close 和闭包持有状态释放。该用例未启动捕获，没有在途回调，不能作为事件送达或回调排空证据。维护输入还显式选择窄投影需要的工厂接口壳及 delegate 类型，避免生成代码缺少依赖；不修改生成器输出，也不调用 P3 的 CreateFreeThreaded。

另有两个显式桌面用例，分别固定硬件与 WARP 设备（不互相回退），各执行 20 次 FramePool 创建/抓帧/关闭。它们在自有 MTA 线程创建当前线程 DispatcherQueue，沿上游 `Create` 入口捕获自有窗口，验证真实 FrameArrived 的线程、GPU staging 读回与已知内部像素、原生资源释放后的跨线程 CPU 持帧、显式注销和队列关闭完成后的回调状态释放。使用 `windows-future` 只为原生 IAsyncAction 类型，按原生消息循环等待，不引入通用异步运行时。目标尺寸取原生捕获项，避免将窗口创建尺寸直接当成物理像素。

本机连续 MTA 退出/重建原型曾在生成的进程级工厂缓存中复现访问违规；当前改为在所属通道显式取得工厂，以生成的窄 ABI 调用并在 apartment 退出前释放。没有通过永久初始化 MTA 来掩盖计数/寿命问题。失败及修复后的结果分别归档。

同一 feature 还包含显式 Desktop Duplication 原型，已通过本机硬件及原生 Actions 验证。自有窗口从首次呈现起使用本轮已知颜色，在默认硬件设备的实际 adapter 中选择包含目标区域的输出，以旧 `DuplicateOutput` 入口执行 20 次设备/输出/采集生命周期；没有切换到 P3 的 DuplicateOutput1。每次有限等待、按原生 RowPitch 仅复制自有窗口内部像素，在 Unmap 前取得独立 CPU 存储，检查成功 ReleaseFrame 后再次释放得到 INVALID_CALL，并验证第一帧跨后续原生生命周期保留及最终跨线程消费。记录 adapter、目标尺寸、pitch、原生 LastPresentTime 与 AccumulatedFrames；不改变线程 DPI 上下文。初次夹具先呈现另一颜色导致旧帧断言失败，证据保留；修正首次绘制颜色后通过，没有按像素失败重试。该原型明确要求未旋转输出，未覆盖多 adapter/输出切换、ACCESS_LOST、完整超时与格式矩阵，也不是固定上游 DesktopDup 的行为对照。

这些原型不提供产品截图 API，不构成六种 Windows 方法、完整 Desktop Duplication/FramePool 异常/重建/权限矩阵、混合 DPI 或多显示器验收；P3 的 DIB 现代化与全虚拟桌面路径均未提前实施。四项基础 COM 测试不需要桌面交互，默认在选择该 feature 后执行；GDI、两个实际 WGC 和 Desktop Duplication 用例仍须显式启用 ignored 测试。具体本机和 runner 结果分别以外部运行记录为准，缺硬件/桌面或捕获能力不能记为捕获通过。

## 当前 P1 迁移与对照

确切 e9a6da8 的 Linux 完整工程归档现已恢复并独立核对：原 978,649,088 字节下载以 unexpected EOF 终止，保留不变；新目录通过 HTTP 206 补齐后，1,805,569,568 字节官方 ZIP 摘要及全部 CRC 匹配。完整 tar 的十九份清单、78,420 个独立登记输入/日志、十八份正常 409 文件 Git 快照、一份故意不可编译夹具及 76 个 F blob 均通过。Wlr 组件完整 8,955 个路径与既有轻量包清单逐字节对应，补齐了轻量复核缺少的 506 个来源输入；原生通过仍限既有 135 项及其受控运行范围。35 文件、6,523,518,270 字节的归档/审阅封存已只读核对；没有扩展原版立即断开、通用重入/并发或完整 P1 的验收范围。

Rust Wlr 新增关闭寿命安全适配，已通过下述原生编译和受控矩阵。每个 capture 在复制 manager/SHM/output 时保留确切 `Arc<Channel>`；client 仍立即清除 connected 和原顺序全局字段，默认先结束 capture 再关闭 client 的路径仍立即断开。只有活跃旧快照对应的原生连接延后到这些代理/缓冲区结束，避免 foreign Backend 使用失效 display。该额外寿命与原版受控前缀探针中的延后干预一致，不能描述为原版立即关闭行为。重连后的旧 capture 在发请求前按连接身份拒绝，不重新构造、不执行原版旧代理/新 display 的无效组合；关闭状态下仍拒绝且不进入转换。该 identity 只保护私有原生所有权，不提前确定 P3 的 SDK 代次模型。

client 可在 capture 句柄仍存在但未执行操作时串行 close/connect；实际捕获、转换或控制操作期间的关闭/重连重入返回 busy，标量作用域在返回/展开时释放，原生 dispatch 和转换期间不持状态锁。每次真正打开连接新增一个标准 Arc owner 和一份 fallible 临时路径副本，以保留来源路径拼写并避免在 native connect 上持元数据锁；原帧复制/转换/等待时点不变，不宣称消除了工作或接受一般重入/并发。原独占 client 回归仍单独保留，既有来源/FD-zero/registry/manager 受控回归已在新代码的下述组件中重新执行。

新 `early_close::closes_shared_client_after_ending_each_retained_native_snapshot` 入口执行二十轮、四模式，核对 80 个逻辑关闭/重连前缀、60 个延后 native channel、40 个关闭拒绝、20 个旧连接拒绝、120 次真实转换内 busy 控制、60 次捕获、各 60 个 raw/BGR 关闭后 owner、实际 fd/精确 task IDs 和 callback owner 配平。`cargo xtask upstream-trace wlr-retired-channel <Rust目录> <原版前缀目录> [--check-only]` 独立复算 382 份文件，并只读核对原版完整前缀，逐字节比较静态图样；alpha 字节仅保留，不补充原来源未给出的 alpha 保证。

确切 `e9a6da80db53f5641770a3761b5e33cf4132e3cc` 的原生 [Actions 41211922418](https://github.com/AbyssRei/maafw-screencap/actions/runs/41211922418) Wlr 组件全部 135 项检查通过，包括 Rust 实际编译、严格 Clippy、新矩阵和既有回归。官方 ZIP 摘要、8,449 个可用登记路径及独立只读 checker 已核对；80 行 Rust 记录、240 份完整 raw/BGR 即时/持帧、80 份精确 task IDs、原版 202 份前缀文件另经复算。每模式最终 fd 均回到七，callback created/destroyed/dropped 分别配平为 6、6、9、18。首次本机复核使用了旧 xtask 可执行文件，在命令解析前被拒绝；旧工具日志保留，按同一归档锁文件重建当前 checker 后的新复核通过，没有改写原生数据。506 个完整来源输入不在轻量包中；其后完整归档审阅已按本节开头补齐，不扩展运行时证明；该受控适配不接受原版立即断开、一般重入/并发或完整 P1。

后续确切 `627b96d60b5e1d7d5c968d76cdb371d36391a3ef` 的 [Actions 41234816627](https://github.com/AbyssRei/maafw-screencap/actions/runs/41234816627) 两个 Wlr 入口在依赖准备时失败：P0 查询 libdrm 正式标签第一页、P1 成功保存前三页后查询第四页，均连接官方 GitLab 端点超时 15,002ms，curl 退出 28；各自保留标签查询和 preparation 的 failed。两个官方 ZIP 摘要/CRC、59 个可用登记路径及实际日志已核对，未执行原生编译/捕获命令，也没有像素输出。未从部分标签列表选取版本，未用缓存版本替代更新检查，未重跑该失败用例；较早原生通过证据保持各自来源/范围，不把本批准备失败描述为新的捕获验收或改写历史结果。

Wlr 新增显式 `early-close-prefix` 原版寿命边界探针，随 `upstream-context wlr` 编排；实现和 portable 校核已通过，本机不执行 Linux 原生测试，首次原生结果及修正状态见下段。原 WaylandClient/WlrScreencap 函数不改写，链接观察器在真实 disconnect 请求处记录 connected、已清空的全局字段、三个代理快照引用数和原生 fd。二十轮各含无 capture、冷 capture、已分配缓冲 capture、活跃旧快照下重连四种场景，共 80 个关闭前缀；夹具仅对仍有旧代理的 60 次断开延后实际 native disconnect，待旧 capture 析构后再恰好执行一次，无 capture 对照立即转发。40 次关闭状态下的原截图须拒绝并保持调用方输出；20 次重连只在旧快照释放后由新 source 取图，不调用旧代理/新 client display 的组合。正常模式继续立即转发 disconnect。

首次原生 [Actions 41200307038](https://github.com/AbyssRei/maafw-screencap/actions/runs/41200307038) 已编译该探针，133 项 Wlr 检查中 131 项通过。新探针在 cycle 0 的首次重连失败：夹具位于归档目录的 socket 别名长 174 字节，被 libwayland 的 108 字节上限拒绝；记录校核器随后拒绝仅含三行的未完成矩阵。已完成的 control/cold/buffered 前缀分别记录零/一/一份存活快照引用，两个 64×48 原版即时图样正确，但没有完整持帧输出，不接受该批探针。官方 ZIP 摘要和 7,868 个可用登记路径已独立核对，501 个来源输入不在轻量包中；失败及部分记录保留。夹具现将别名放入原 socket 的短运行目录，同一环境下为 90 字节，并按原生 `sockaddr_un.sun_path` 检查含终止符的长度；只有确认自己成功创建的别名才在退出时删除，已有或不可检查的路径保持失败。仅修正夹具，后续确切 451866c 的 Actions 41203380921 已通过全部 133 项 Wlr 检查。官方 ZIP 摘要、8,064 个可用登记路径及只读记录校核均通过；80 行关闭前缀、120 份已知/持帧图像和 80 份最终 task IDs 另经独立复算，501 个完整来源输入不在轻量包中。该成功仍限显式延后断开的受控前缀，不改变首个失败或解除 Rust 的独占 client 限制。

`cargo xtask upstream-trace wlr-early-close-prefix <外部目录> [--check-only]` 独立核对完整 80 行、准确引用/状态/fd、80 份实际 task IDs，以及 60 张即时和 60 张持帧 BGR；拒绝缺行、额外文件、引用/资源不符或错误像素。夹具延后断开是明确的干预，不能用其配平结果证明原版在活跃代理下立即断开的清理有效，也不构成 Rust 早关/重连迁移或 P1 完成。该通过批次的 Rust `Client` 仍限制独占 connect/close；当前分支新增的安全适配见本节开头，须以新原生结果判断，不能继承此批通过。初次校核器编译因记录辅助函数名称不符而失败，日志保留；更正后 68 项 xtask 测试、格式、严格 Clippy 及 PowerShell 语法检查通过。

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

另一个真实 FramePool 控制在 cache clone 的 C++ 边界抛出异常，核对临时结果 owner 释放、原生关闭结果，并在错误返回后观察到 staging 可再次成功 Map；后续精确 Map/Unmap 局部对照见下文。显式返回的清理错误进入固定证据槽；Drop 的失败采用固定原子记录，避免在主错误展开时格式化、分配或因日志 I/O 再次 panic。这些局部控制不替代完整失败矩阵。

原客户区 ROI 为零面积时，OpenCV 路径返回 empty Mat 并以空克隆替换缓存；内部实现单独返回 EmptySample 并清空旧缓存，不能与无帧超时混同。几何单元检查覆盖零面积；真实窗口干预与准备恢复的限定证据见下文，立即恢复及完整相关矩阵仍待验收。初始化的三次 Size 查询保持各自 catch 边界：创建 pool 参数中的失败按原逻辑返回 false，首尾两次异常保留逃出局部 catch 的部分状态；再次 CreateForWindow 前也按 put_abi 释放旧 item。受控 Size 返回/异常的局部对照见下文，真实系统故障、其他初始化/清理错误仍待验收。

2026-11-01 已补真实零客户区控制：窗口 owner 在一次真实 WGC Map 后、GetClientRect 前将客户区宽、高或两者置零，保留非零外窗口。原类返回有值但为空的 Mat；Rust 的私有 EmptySample 清除旧 cache，零分支不复制/转换像素。测试用 Rc 持有窗口直到同步查询干预结束，不改变捕获实现、等待或默认选项。夹具 owner 在零客户区期间停止内部更新 timer，正尺寸恢复后重新启动，关闭前完成 timer/window/class 清理和 join。

立即恢复场景尚未通过：首批原版三轮均失败于恢复颜色，Rust 三轮各完成二十生命周期，既有十模式查询回归全过。随后仅补“断言前保存所有字节”的原版诊断为 `[0,1,1]`，另一个插入一次只读 ContentSize 查询的诊断为 `[1,1,0]`；四路输入均为零、监听注销已核对。灰黑失败图仅有 2×2 或 96×2 灰色区域，而当次 GDI 查询看到已知图样；插桩失败帧仍报告 96×64 ContentSize。它们未证明灰黑原因，不能据此把较早失败归因于旧 ContentSize、用户输入或熄屏。只读插桩的确切单语句变换在原生 build 目录生成，原 F 源码不改；它的时序观察不代替未插桩结果。

另一个明确准备画面的恢复对照已完成有效连续三轮：恢复后先全幅重绘，并等待两次真实 owner 呈现，再只调用一次原版/Rust 捕获。共 360 次调用、120 次真实零客户区、120 次准备后的恢复及 240 张关闭后持帧全部通过；720 份 BGR 中 480 份非空，1,464 个零客户区文件及另 1,452 个查询回归文件经独立复核，十三组破坏控制被拒绝。506 个冻结输入、418 份产品源码和 76 个 F blob 匹配；正式输入为零，最大间隔 81.79ms，监听正常注销，无新增物理正向控制声明。首份审阅报告误把另一诊断批次的 70.27ms 写进汇总字段，初始报告保留，新目录完整复算已改正为本批 81.79ms。随后按 [KillTimer 的已排队消息规则](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-killtimer)补上停止状态的迟到 WM_TIMER 拒绝，最终源码再次连续三轮通过同一十八次程序调用和完整像素/记录核验，输入为零、最大间隔 77.38ms；两个准备批次分别保留，不修改任何立即恢复失败。

`MFS_FRAMEPOOL_EMPTY_ROOT` 选择外部矩阵，`MFS_FRAMEPOOL_EMPTY_RESTORE_BARRIER=1` 显式启用准备控制；原程序 `--empty-client` 保留立即恢复，`--empty-client-prepared` 启用准备控制。`cargo xtask upstream-trace framepool-empty <外部矩阵> [--check-only]` 要求完整几何、空输出/持帧、cache/转换/owner、两次呈现和 488 个文件；正式编排使用准备控制。它不接受立即恢复竞态、零尺寸 GraphicsCaptureItem、完整 GPU 资源、一般窗口竞争或 P1；限定原生 CI 结果见下文。默认核心没有新增行为。

确切 `0c3ae155a7245cb1a692f329d5ea2ad9ba6bcb03` 的 [Actions 41229336665](https://github.com/AbyssRei/maafw-screencap/actions/runs/41229336665) FramePool 组件为 55/59 项通过：原类/Rust 零客户区准备恢复及 records 均通过，既有查询和目标销毁也通过，四个原 pseudo 已知颜色断言仍失败。官方 95,751,571 字节 ZIP 摘要/CRC、1,240 个可用登记路径、488 个零客户区文件、484 个查询文件和 163 个目标销毁文件独立核对；零客户区和查询共 480 份完整 BGR 另经全幅图样复算。该次零矩阵为 120 次调用、40 次真实零客户区、40 次准备恢复及 80 张持帧，两次呈现屏障均有记录；它不接受立即恢复或新 manager 分类修复。轻量包缺 86 个来源/依赖输入，下述完整归档已补齐。首份审阅汇总误用临时布尔变量覆盖四个失败名称，初始报告原样保留，新目录复算已恢复实际列表，原生数据与状态不改；四张失败 pseudo 图仍为全黑。该组件结果不接受整个 Windows job 或完整 P1。

随后同一确切 head 的完整 Windows 官方工程 ZIP（1,937,494,929 字节）摘要和所有 ZIP CRC 均通过；独立读取其中工程 tar，核对全部 23 份清单、41,347 个不重复登记输入/日志、22 份正常 418 文件 Git 快照、一份明确的不可构建控制夹具、76 个固定 F blob 及实际 native CMake 来源根。Windows 文本检出只在 Git blob 对比时归一化 CRLF/LF，所有原始文件 SHA-256 保持严格。FramePool 的完整 `native-result` 与轻量包逐字节相同，1,326 个原生登记路径均已验证，补齐此前缺失的全部 86 个来源/依赖输入。七个 P1 组件合计 292/296 项通过，四个原 pseudo 失败及配置控制的预期失败原样保留；这是该批来源完整性证明，不扩大立即恢复、新 manager 分类修复、其他未验证数据或完整 P1 的运行时范围。

Rust 端另有真实目标销毁控制：先取帧并核对独立夹具内部像素，再在 UI owner 上销毁目标并完成 join；下一次捕获在窗口有效性检查立即失败，不继续轮询、转换或替换旧缓存。显式 session/pool 关闭通过，关闭后 CPU 图像仍有效。此项保留上游失败时的部分状态；后续重复原源码对照见下一段，其他原生故障全过程仍待完成。

2026-11-01 新增独立 `mfs_framepool_target_loss_reference`，直接编译固定 F 的原类；Rust 对照入口为 `target_loss::matches_original_repeated_target_loss_without_replaying_cached_pixels`，`MFS_FRAMEPOOL_TARGET_LOSS_ROOT` 指定外部 source/rust 矩阵。两侧各二十个 MTA 生命周期：一次真实取帧后在窗口 owner 上销毁并完成 join，同一失效 HWND 连续调用三次，必须拒绝返回缓存图像；原类实际析构，Rust 还检查失败时 pool/session/item 和 cache 不变、poll/复制/转换为零、显式关闭及最终 CPU/completion owner。所有首次图像保持至 native 捕获和调用方初始化结束后，再逐字节保存持帧。`cargo xtask upstream-trace framepool-target-loss <外部矩阵> [--check-only]` 核对 163 份实际文件、已知内部图样及 source/Rust/持帧的完整字节一致性；耗时只作观察，不作为延迟保证。正式 FramePool 编排显式构建原程序，分别运行 source、单项 Rust 和 records，最终轻量包收集该目录。

这项矩阵在归档开发依赖下已完成有效连续三轮，共九次程序调用、120 次初始捕获、360 次销毁后拒绝和 120 张关闭后持帧。独立审阅核对 240 份完整 BGR、489 个登记 native 文件、500 个冻结输入、412 份产品源文件和 76 个固定 F blob；原始/低级鼠标键盘事件为零，系统输入时间/光标不变，最大采样间隔 78.70ms，监听已确认注销。完整不变副本复算通过，九组缺失持帧、边缘字节不符、黑图、布局、回放、owner 计数、调用线程替换、额外文件及拒绝次数破坏均被拒绝。没有声称新注册的四路监听获得了新的物理正向控制，也不改写旧程序结论。新原生 CI 尚待运行；原 C++ 私有字段未被插桩，完整 GPU 资源计数、其他宿主/故障/窗口身份重用和完整 P1 均未验收，捕获实现与等待语义未改。

2026-11-01 另补映射后的查询/裁剪对照。`mfs_framepool_query_reference` 直接包含固定 F 的原类，仅在测试翻译单元拦截四个 Win32 查询；Rust 的同类拦截仅在显式原生测试配置下编译。十种模式分别为普通查询、GetClientRect 失败、ClientToScreen 失败、DWM 失败后 GetWindowRect 回退、回退也失败、客户区宽/高超过纹理、客户原点右/下越界及负向原点钳制。失败通过向真实 OS 查询提交无效不透明句柄产生；矩形变化是明确的测试干预。两侧各二十生命周期，每次执行 warm/probe/recovery 三次实际 WGC 捕获，不重试截图。成功图像必须全幅符合已知图样和实际更新标记，失败不得交付像素；持帧在 native/窗口/调用方初始化结束后再次保存。Rust 还验证真实 Map、失败时旧 cache 不变且没有复制/转换、恢复后的重新映射/转换、native 槽位及最终 CPU/completion owner。

入口 `queries::matches_original_mapped_query_failures_fallback_and_roi_clamps` 使用 `MFS_FRAMEPOOL_QUERY_ROOT`；`cargo xtask upstream-trace framepool-queries <外部矩阵> [--check-only]` 独立验证双方查询顺序、真实错误、恢复、全部布局/像素与 484 个登记文件。归档依赖开发构建的连续三轮全部通过：360 次映射捕获调用，其中 36 次按预期拒绝；120 次 recovery、324 张持帧、720 份 BGR 文件中 648 份非空，另 72 份是失败分支的空输出。四路正式输入为零，系统输入时间和光标不变，最大间隔 78.00ms，监听正常注销；504 个冻结输入、416 份产品源码、76 个 F blob、1,048 个原生编译/链接读取记录和四份生成绑定均已核对。不变副本通过，十三组缺失调用/错误、查询顺序、恢复、cache/转换、owner、像素、布局和文件/报告破坏被拒绝。

首次监听在注册前遇到随附 PowerShell 7.6.5 的 WinForms 编译引用问题，没有执行截图；后续使用系统 Windows PowerShell 5.1 运行原有完整 C# 监听代码，先通过独立编译再注册。失败准备及两次原程序头文件/名称解析编译失败均单独保留，没有把它们计为捕获执行。此批按用户授权自动开始，没有声称新注册获得了新的物理正向控制。Release 核心构建、格式、Clippy、串行 workspace 和 PowerShell 解析通过；后续查询/目标销毁的原生 CI 范围见下一段，完整依赖来源、真实外窗口竞争、其他原生故障/完整 GPU 资源计数仍待完成，不接受完整 P1。

确切 `b200f563c8319cc0f322af43b6f04dc696833357` 的 [Actions 41224789341](https://github.com/AbyssRei/maafw-screencap/actions/runs/41224789341) FramePool 组件为 52/56 项通过：原源码/Rust 查询与目标销毁及各自 records 均通过，四个既有原 pseudo 颜色断言仍失败。官方 94,002,736 字节 ZIP 的完整摘要/CRC、748 个可用登记路径、484 个查询文件和 163 个目标销毁文件经下载后复核；240 份查询 BGR 另经独立全幅图样计算。四张失败 pseudo 原图均为 96×64 全黑 BGR，不据此推断原因。轻量包未含 86 个登记输入，完整来源/依赖审阅待补；该 CI 不替代本机无输入三轮结果，也不接受整个 Windows job 或 P1。

2026-11-01 另新增真实 Map/Unmap 边界对照。独立 `mfs_framepool_map_reference` 与既有查询程序共同直接包含同一固定 F 原类；仅前者使用词法表达式视图转发原 `ID3D11DeviceContext` 和 cache 字段，不替换 COM 对象或 vtable。成功路径保持真实 CopyResource、Map、转换、cache clone/赋值及 Unmap 顺序。故障一向真实 Map 提交 `UINT_MAX` 子资源，记录原生 `E_INVALIDARG`，不读取失败的映射输出，也不调用 Unmap；故障二在真实成功 Map 后让 C++ 转换抛出受控异常、Rust 转换器返回错误，核对恰好一次同线程 Unmap、旧 cache 地址/字节不变及下一次单独捕获恢复。两侧均保持返回图像与 cache 独立，持有 CPU 图像至 native/窗口/调用方初始化结束后再次保存；Rust 另记录必要 BGRA 安全复制、最终 CPU/completion owner 和关闭错误。测试钩子仅在原生测试配置下启用，正式调用仍使用子资源零，等待/默认值不改。

入口 `map_boundary::preserves_warm_cache_and_unmaps_exactly_once_across_native_map_and_conversion_failures` 使用 `MFS_FRAMEPOOL_MAP_ROOT`；`cargo xtask upstream-trace framepool-map <外部矩阵> [--check-only]` 核对双方调用顺序、HRESULT、原生身份、cache、布局、完整像素、持帧和 724 个文件，正式编排及 CI 收集均已接入。归档开发依赖下连续三轮全部通过，连同独立十模式查询回归共十八次程序调用。Map 矩阵共 360 次调用、42 次预期原生 Map 拒绝、36 次受控转换失败、120 次 recovery 和 282 张持帧；1,080 份 BGR 中 924 份非空，2,172 个 Map 文件、另 1,452 个查询回归文件、512 个冻结输入、423 份产品源码、76 个固定 F blob、1,059 个实际 MSBuild 读取记录和四份生成绑定独立核对。输入为零、最大采样间隔 79.59ms，监听正常注销；不变副本通过，十七组调用/清理/错误/cache/owner/像素/文件/报告破坏被拒绝。没有声明本次注册的新物理正向控制；构建审阅最初误查独立 `.cpp` 文件名，在写归档前被拒绝，修正为实际共享翻译单元后通过，原生结果不变。串行 workspace 122 项通过、24 项忽略，严格 Clippy、格式与 PowerShell 解析通过。新矩阵原生 CI、设备丢失、分配失败、Rust panic、完整 GPU 资源计数和完整 P1 仍待验收。

确切 `c407c0f58cf98d8ba8efb48962a609820f95ac4d` 的 [Actions 41233271907](https://github.com/AbyssRei/maafw-screencap/actions/runs/41233271907) 现已通过 Map 原生构建、原/Rust 执行和记录校验四项。官方 Windows 轻量 ZIP 摘要、1,969 个可用登记路径、724 份 Map 文件及全部 360 份像素经下载后独立复核；120 次调用包含 14 次真实 Map 拒绝、12 次受控转换失败、40 次 recovery 和 94 张持帧，cache、Unmap 和关闭后 CPU 内容一致。FramePool 组件共 63 项、59 项通过；四个原版 pseudo 颜色断言仍失败，四张原图仍全黑，不推定原因。轻量包缺 87 个完整来源/依赖输入，完整源码核对仍待补；本记录不接受后续 Size/manager 修复、整个 Windows job 或 P1，也不替代本机输入排除三轮。

随后下载并核对同一 c407c0f 的完整官方工程 ZIP/tar：全部 23 份清单、42,192 个唯一登记路径、22 份普通的 423 文件 Git 快照、一份明确不可构建控制快照、76 个固定 F blob 和各实际 CMake 来源根均匹配。全部 2,056 个 FramePool 原生输入/日志与轻量包的原始 native-result 对上，补齐上述 87 个缺失输入。七个 P1 组件合计 300 项、296 项通过，四个原版 pseudo 断言保持失败；配置负向子步骤保留失败及其预期失败父校验。源码完整性已核对，不据此接受其他故障范围、更新 head、整个 Windows 或完整 P1；早期轻量缺口评估保留原状态。

确切 `627b96d60b5e1d7d5c968d76cdb371d36391a3ef` 的 [Actions 41234816627](https://github.com/AbyssRei/maafw-screencap/actions/runs/41234816627) Windows FramePool 组件在更早的原版 zero-client 颜色断言停止：34 项中 32 项通过，两项失败，Map/Size 及后续矩阵尚未运行。官方轻量 ZIP、484 个可用登记路径及部分数据另经核对；已有 45 行，接下来的 `15-warm` 保存了 18,432 字节完整三色背景却没有两种标记，缺少对应行及关闭后持帧。严格 checker 拒绝这份未完成矩阵，不推断时序/几何/捕获原因。编排现对五组原/Rust 捕获及其记录校验分别保留失败、继续其他独立进程/夹具，编译与来源检查仍立即失败；整批仍不通过。实际函数的三个 mock 子步骤在本机及三平台 runner 均通过失败保留/继续执行/参数不变控制，未执行捕获；新原生编排效果仍待独立记录。

2026-11-01 新增十模式 Size 边界对照，入口为 `size_boundary::preserves_original_size_query_catches_partial_states_and_recovery`，使用 `MFS_FRAMEPOOL_SIZE_ROOT`。独立原程序 `mfs_framepool_size_reference --size-boundary <source>` 从固定 F 原 `.cpp` 生成只替换四处 Size 表达式的副本；每次先查询真实 `GraphicsCaptureItem`，再按测试模式返回受控 `E_ABORT`、零宽度或一次尺寸差异。独立翻译单元在依赖已包含后仅将原头的 `private` 访问改为 `public`，直接观察原类字段，字段和其他函数体不改。记录中的 `native_hresult` 为投影异常码，零表示投影调用成功；不声称读取了成功调用的底层 HRESULT。它验证冷初始化首/第二/末查询及正常检查的异常边界，另覆盖触发重建后的三处异常、首次零宽度和第二次零宽度参数的真实 pool 创建拒绝。尤其第二次 Size 异常在冷启动清空 item，在重建路径保留 item；末查询异常保留已启动 session/pool，下一次捕获经原尺寸检查重建。全部失败保持旧 cache 地址/字节，并以一次单独调用恢复，没有截图重试。Rust 使用同一私有实现，测试钩子在真实查询后返回受控结果；正式构建继续直接查询原 item。

`cargo xtask upstream-trace framepool-size <外部矩阵> [--check-only]` 核对双方异常/空返回区别、原生槽位、last size、全部 Size 原始成功结果与受控结果、cache、像素及 726 个文件。归档开发依赖下连续三轮及独立十模式查询回归的十八次程序调用全部通过：Size 矩阵共 360 次捕获调用、108 次预期拒绝、120 次 recovery、252 张关闭后持帧和 1,164 次真实 Size 查询；1,080 份 BGR 中 864 份非空，2,178 个 Size 文件、另 1,452 个查询回归文件、517 个冻结输入、428 份产品源码、76 个 F blob、1,060 个实际 MSBuild 读取记录和四份生成绑定独立核对。原始/低级鼠标键盘事件为零，系统输入时间与光标不变，最大间隔 162.61ms，监听已注销；没有新物理正向控制声明。不变副本通过，十九组错误边界、部分状态、原生/受控结果、线程、复制、owner、像素和文件/报告破坏被拒绝；其中十八组先删除保存的 report，确认由实际语义检查拒绝。串行 workspace 123 项通过、25 项忽略，Clippy、格式通过。最初 Rust 诊断误要求 Image 实现 Debug 的编译失败，以及构建审阅输出目录未创建的失败分别保留，均未执行额外截图或改变原生结果。

正式编排已接入 Size 入口，并新增生成源码的四表达式校验、实际副本归档和轻量包收集；记录归档代码在上述三轮后加入，已对实际编译的生成源码独立执行并核对哈希，完整原生编排/CI 仍待验收。此矩阵仅验证受控返回如何经过真实原生对象和原有控制流，不表示观察到系统 Size API 自发失败、真实零尺寸捕获 item 或外部窗口尺寸竞争；完整 GPU 资源计数、设备/分配故障、其余原生矩阵和 P1 仍未通过，核心默认值和等待行为未改。

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

2026-11-01 在 Size 边界与 manager 的职责核对中发现实际迁移偏差：原类首次 item Size 非正时 `init` 返回 false、`screencap` 返回无图，manager 仍保留其他成功候选；Rust 原先复用 `InvalidLayout`，被 native manager 归为 Safety，从而在已有 GDI 成功图像时提前中止选择。修复将这个原生拒绝拆为私有 `InvalidItemSize { width, height }` 并归入 NoImage；像素跨度、布局及溢出的安全错误分类保持原样，没有增加新的选法或 fallback 策略。

独立回归 `size_tests::continues_to_gdi_after_framepool_rejects_nonpositive_initial_item_size` 用真实 Rust native factories 构造 GDI 与 FramePool/pseudo 候选，每次 FramePool 先执行真实 item Size 查询，再按显式测试控制返回零宽度。原 manager 的固定函数体模型通过后台/时钟/input=None 边界提供无图仍选 GDI 的预期；另一个完整原类/Rust Size 矩阵提供原生无图及清理证据，二者组合验证，不冒称完整原生原版 manager 同场执行。修复前的三轮均在首周期记录“原模型已选择 GDI、Rust warmup=Safety、没有胜出者”，退出码 `[101,101,101]`；原类/Rust Size 及 records 每轮仍通过。正式输入为零、最大间隔 78.35ms，监听正常注销，失败保持原始状态。

修复后新程序的连续三轮全部通过，共十五次程序调用；原 C++ EXE、适配 DLL 与依赖 DLL 摘要同修复前一致。六十个 manager 生命周期中，120 次真实 FramePool 候选拒绝均为 NoImage，两次 GDI 预热/测速后仍能交付第三张图，共 180 次 GDI 捕获、60 张关闭后持帧和 120 份完整 BGR；246 个 manager 文件经独立复核，每个已记录 GDI 前后计数均相等。同期原类/Rust Size 矩阵再次通过 360 次调用、108 次预期拒绝、120 次恢复、252 张持帧、1,164 次真实查询及 1,080 份 BGR。518 个冻结输入、430 份产品源码、76 个 F blob、1,061 个实际 MSBuild 读取记录、七份生成绑定，以及八段原 manager 函数体均核对。正式输入为零、最大间隔 80.06ms，监听正常注销，无新物理正向控制声明。十六组 manager 破坏与十九组 Size 破坏被拒绝，两份不变副本通过；破坏语义记录前移除保存 report，另单独检查错误 report。

新入口使用 `MFS_MANAGER_SIZE_ROOT`，校核命令为 `cargo xtask upstream-trace manager-size <外部目录> [--check-only]`；`upstream-parity manager` 已分别编排单项 native test、records、失败时登记与 CI 收集。Windows manager 能力的串行 workspace 138 项通过、73 项忽略，Release 核心构建、Clippy、格式和 PowerShell 解析通过。完整新原生 CI、系统真实零尺寸 item、其他原生候选故障/清理、全部混合方法与 framework/全平台验收仍待完成，旧失败和完整 P1 状态不变。

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

早期据此修正测试的 proc 退出时点前提，已在当时受控原生批次通过：关闭前记录活跃 MaaScreencap 的 TID/starttime；原生 stop/join、对象销毁及严格 fd 检查之后，仅允许同身份任务已经消失或带 PF_EXITING 且无地址空间/RSS 时进入最长 100ms 的内核任务移除观察。原 baseline TID 必须全部保留，最终集合须完全恢复；未知任务、TID 替换、仍有用户态地址空间、fd 不符或逾期均失败。C++/Rust 均实际观察到约 1.1–1.3ms 的内核任务移除。测试没有等待仍活跃的 callback 完成；旧批次的即时数量断言不改判，捕获实现、等待和销毁顺序保持原样。

私有 Rust driver 的 supplied-fd 建连切片已通过受控成功路径验证：与 daemon 方式共用原顺序的 loop/context/core/listener/stream 建立，只在非空 core 后将原 fd 数值置为 -1，close 不恢复它，后续 init 沿用原源码的 daemon 选择。该入口是显式 unsafe 的 U 适配，要求调用方保证独占 socket、移交前解除原 owning handle、完成原生清理后才处理失败残留；没有推导空 core 的一般所有权，也没有添加泛用 fd 清理或重试。独立进程用例通过 20 次私有 socket、60 张 RGB、归还/持帧及已消费 fd 在原生关闭后为 EBADF；它验证 fd 传输成功路径，重新初始化、实际 Portal 授权、通用失败/重试和安全宿主接口仍未完成。

同一最新受控 RGB 批次中，完整原 C++、Rust daemon、Rust supplied-fd、Rust OpenCV 各通过 20 生命周期、60 张关闭后持帧，原图独立校验共 240 张；路由 80 次创建、绑定、销毁配平且自有进程正常退出。真实七格式/28 个有效 POD 和转换字节、私有 ABI/异常测试继续通过。源码与 Rust 分别采集生产端的不同序号，按各自序号核对已知图样，未声称同一时刻采样或源码延迟对照。owned mapping、DMA-BUF、完整流格式/故障/缓存矩阵、mobility、Linux manager 与其他平台完整迁移仍未完成；这些局部通过不放行整个 P1。

无 data 指针的 MemFd 映射已迁移，原 helper/手动描述符路径已有原生 Actions 对照，真实流的受控 NULL-data 对照见下文；自然产生 NULL 指针的来源仍未覆盖。私有 rustix owner 保持原 `PROT_READ`、`MAP_PRIVATE`、零映射偏移及 maxsize/缺省 chunk.size 长度选择，只借用 native fd；目标行复制后先 unmap，再通知成功和归还 buffer，失败/展开同样释放映射。新增 fstat 验证实际文件覆盖目标行可读跨度；这项安全检查增加一次系统调用，不将其称为性能优化。非零 owned mapoffset 仍显式拒绝并留待完整来源/偏移审阅，不提前改为 P3 偏移处理。已有 SHM 指针仍由 native lease 保活；DMA-BUF 同步和完整清理失败矩阵仍未验收。

`upstream-context pipewire` 新增普通真实流的七格式入口（RGB、RGBA、RGBx、BGR、BGRA、BGRx、YUY2）；初版及修正版受控原生结果见下文，完整来源与 P1 验收仍单独核对。它沿用现有自有 daemon/精确路由和生产器，显式格式模式提供两种交替 raw 图样及十二字节行填充，不改变默认 P0 和原 RGB 序号模式。原完整 C++ 与 Rust 每格式各要求二十生命周期、六十张即时/关闭后持帧、两种实际图样及 native fd/线程退出；Rust 另检查真实协商后的 raw 格式/紧致行、独立 CPU owner、复制/归还配平、原转换分支和 BGR 同一存储复用。保留 raw 的测试观察会阻止 cache 存储复用，不作为默认分配量或性能对照。

`cargo xtask upstream-trace pipewire-format-stream <外部目录> [--check-only]` 独立复算原始图样、行填充和 RGB/BGR/四通道转换；YUY2 使用与当前 OpenCV 一致的 BT.601 定点舍入/饱和公式。它要求十四份完整调用表、七份原版关闭后持有表、二十八份原表/OpenCV oracle 和 2,520 份即时/持帧字节，不能只比较共用转换器。既有 CLI 的 2,534 计数另含十四份 BGR oracle，见[本轮计数澄清](#本轮原生与-ci-复核)。Windows/OpenCV 生成的合成记录通过本机 checker，十二组缺失/重复/像素/填充/持帧/报告破坏被拒绝；这只验证夹具和记录逻辑，不是 Linux 编译、真实流或 P1 验收。Portal/GPU、动态重协商及完整故障矩阵仍未覆盖。

源码复核发现上述入口初版误要求原版 BGR 的早期输出始终不变：F 的 `latest_frame_.create` 加 `memcpy` 会复用同尺寸 Mat，BGR 返回的浅头共享该缓存，后续捕获会改写早期像素；其他六格式经转换得到独立 BGR。校核现分别要求原版 BGR 的三个同生命周期头地址相同、关闭后均匹配该周期最后一次输出，并保留即时与关闭后图样；Rust 则验证既有 CPU 所有权适配在仍有持帧时分配新存储，原始与 BGR 持帧不变。这是已有安全适配的显式差异，未修改产品或原版算法。实际新 C++ 验收头在 Windows/OpenCV 的自有 helper 控制中观察到 40 对缓存别名、20 次早期输出被覆写；SPA 枚举在该控制中为符号替身，没有执行 Linux 流。原生 Linux 结果及初版批次各保留自己的实际状态。

初版确切 `bf5303a125747f2926a9f94150b644820eb1f29e` 的 [Actions 41247571643](https://github.com/AbyssRei/maafw-screencap/actions/runs/41247571643) PipeWire 组件实际为 192/194 项通过：七种 Rust 流均通过 420 次捕获及原始/BGR 持帧，六种原版转换格式通过 360 次捕获。原版 BGR 完成第零周期三次捕获后，旧持帧不变断言失败；其即时/持帧文件确认第二次输出由 phase 0 被改写为最后一次的 phase 1，随后初版 checker 严格拒绝缺少调用行。官方 ZIP、43,006 个可用登记路径、2,418 份捕获/持帧字节和二十八份 Linux oracle 均独立核对；oracle 与已独立复算的 Windows/OpenCV 夹具一致，含 YUY2。轻量包当时缺少的 87 项来源输入随后由完整归档补齐，初版两个失败不改写为通过；修正版范围见下文，不据此接受整个 job 或 P1。

初版同一 run 的完整官方 Linux ZIP/tar 随后核对全部十九份清单、82,123 个登记路径、十八份普通 444 文件 Git 快照、一份不可构建控制、76 个 F blob 和实际 CMake 来源。全部 43,093 个 PipeWire native 路径及原始内外清单与轻量包匹配，补齐 87 项输入；原版 BGR 持帧断言和不完整 checker 仍失败，来源核对不将其改为通过。

修正版确切 `206755edd8efa4bfaa023d2b957c917bbfc08e69` 的 [Actions 41248794571](https://github.com/AbyssRei/maafw-screencap/actions/runs/41248794571) PipeWire 组件通过全部 195 项检查；比初版多一项缓存未命中的 `source-drm-clone`，没有删除原检查。七格式原版/Rust 各完成 140 生命周期、420 次捕获；独立下载复核 43,088 个可用登记路径、2,520 份实际即时/持帧字节、28 份 oracle 和 21 份表。既有 CLI 的 2,534 计数还含十四份 BGR oracle，原报告和日志不改。原版 BGR 实际记录四十对缓存别名及二十次早期持帧被覆盖；Rust 既有所有权适配保留不变 raw/BGR。轻量包当时缺少的 87 项完整来源输入已由下述完整归档补齐。同一 run 的另一个 PipeWire P0 命令在显式配置阶段失败、尚未运行其取帧；macOS 实际日志仍为正式 Xcode 27.2/27B52 缺失。组件通过不代替这些独立失败或整个 Linux job/P1 验收。

修正版同一 run 的完整官方 Linux ZIP/tar 已核对八份实际运行清单、63,289 个登记路径、八份普通 444 文件 Git 快照、76 个 F blob 和实际 CMake 来源。全部 43,175 个 PipeWire native 路径及原始内外清单与轻量包逐字节匹配，补齐全部 87 项来源输入。另两份 Rust 准备批次的最新版本检查分别因查询 `target-lexicon`、`pipewire-sys` 的 crates.io 请求发生 curl 28／10 秒连接超时而失败；P0 PipeWire、该批 Wlr 与最后配置检查继续拒绝未通过准备的输入，未执行对应捕获或配置控制场景。缺少的配置控制不记为通过，两个超时失败保留；独立准备且通过的七格式 P1 组件也不接受整个 Linux job 或 P1。

确切 `8f75fef1cad09d386a39f9debbf3f59e497d51d4` 的 [Actions 41243712698](https://github.com/AbyssRei/maafw-screencap/actions/runs/41243712698) FramePool 组件为 61/67 项通过。原四项 pseudo 颜色失败保留，Rust Size 另在第 16 周期恢复时记录 marker `[31,83,167]`，尚未保存恢复图和对应完整行，随后严格 checker 拒绝缺失 Size 调用；不能据 marker 推断完整图像或根因。Size 失败后八项独立 factory/调用环境/生命周期/参考捕获检查仍执行并通过，失败状态和整批失败均保留。下载后五项其他 FramePool 数据校验通过，Size 仍严格失败；2,540 个轻量登记路径核对，缺少的 88 项输入随后由完整归档全部补齐。

同一批官方 Windows ZIP/tar 核对 16 份实际运行清单、33,084 个唯一登记路径、十五份普通 439 文件 Git 快照、一份不可构建控制、76 个 F blob 和实际 CMake 来源；FramePool 全部 2,628 个 native 路径及原始清单与轻量包逐字节匹配。SDK 下载页此前出现 curl 28 超时，GDI/ScreenDC/DesktopDup 的该批捕获没有获得验收；Custom、manager 和便携 PipeWire 控制各保留自身通过范围。macOS 实际失败日志仍为正式 Xcode 27.2/27B52 缺失。这些来源和失败续行证据不接受完整 Windows job 或 P1，也不改变旧程序的有效连续复测结论。

`upstream-context pipewire` 另接入真实 MemFd 流的 NULL-data 受控入口，原生 Linux 结果见下文。专用生产器为每个缓冲创建独立、禁止伸缩的 4096 字节 MemFd，以保持零 mapoffset；普通 SHM 生产器共用 fd 的偏移不能冒充此映射前提。PipeWire 会为 MemFd 补回 MAPPABLE，因此采用只加载到两个独立消费进程的 ELF 测试介入：真实 dequeue 后暂时隐藏 `spa_data.data`，每个归还/异常清理边界先恢复原指针；保留真实 fd、chunk、布局、生产器、路由、MAP_BUFFERS 和完整原 C++。原版与 Rust 各要求 20 生命周期/60 次已知像素取帧、关闭后全部持帧、原生 fd/线程检查及 dequeue/restore 配平；Rust 另要求每次真实回调独立 map/unmap，且 DMA 计数为零。原版没有额外插桩复制/映射次数，记录为 unknown。`cargo xtask upstream-trace pipewire-memfd-stream <外部目录> [--check-only]` 校核 40 行记录、240 份原始/持帧图像及摘要，缺失、未恢复、残留 lease、错误计数或像素不符均失败。它不证明自然产生 NULL 的流、DMA/GPU、Portal、完整失败矩阵或 P1；既有 DMA 实际流通过记录保持独立。

确切 `10d73c92fa2953c157373ef252ab28725b14ad5b` 的 [Actions 41240527770](https://github.com/AbyssRei/maafw-screencap/actions/runs/41240527770) PipeWire 组件现通过全部 164 项。官方轻量 ZIP 与 40,336 个可用登记路径独立核对；40 行介入记录、240 份 RGB/BGR/关闭后持帧字节、每侧 60 次 dequeue/restore 及 Rust 的 60 次 map/unmap 均通过只读复核，DMA 计数为零。自有 MemFd 生产器的 320 次分配/移除、120 次写入/归还配平，后续原 kernel DMA 流回归也通过。轻量包仍缺 86 个完整输入，完整来源审阅待补；原版映射次数仍为 unknown，不接受自然 NULL、GPU/Portal、任意失败或完整 P1。首个 d8c 的独立批次在官方 latest release 查询收到 HTTP 500、curl 22 后停止，原生编译和捕获均为零；原失败保留，不能用来判断其 MemFd 夹具运行结果。

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

私有 Wlr 现已分离共享 client 的 registry/连接状态与每个 capture 的回调、缓冲区和结果；capture 复制原版 manager/SHM/output 快照并保活同一个标准 Arc client。这一已验收旧切片的可变 close/reconnect 只在旧 capture/proxy owner 全部结束、取得唯一 client 访问时执行；当前新增的保留连接适配另见本节开头，其新原生结果待确认。新增两个独立进程测试，各 20 轮，分别检查调用方及第一个 capture 释放后继续取帧、最终 client 释放，以及同路径不重新绑定、显式关闭后重开、别名路径变化、缺失路径失败和后续显式连接；要求独立 raw/BGR 对照原图、分离的 native buffer、回调 owner 配平、关闭后持帧和精确 FD/task 基线，最新 93 项原生组件全部通过，新增 60 组共享及 20 组重连 raw/BGR/source 图和 40 行资源记录均已独立复核。此前成功 FD-zero 路径的 91 项组件及独立图像复核已有通过证据；构造失败/部分状态、早关、格式与完整后端仍未验收。

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

显式 client 关闭的另一项限制仍保留：固定原 WlrScreencap 持有的 manager/shm/output 共享快照不会随 WaylandClient::close 的全局字段 reset 自动失效，而该函数随后断开 display。[Wayland 客户端 API](https://wayland.pages.freedesktop.org/wayland.freedesktop.org/docs/html/apb.html)要求先销毁代理和事件队列；[wayland-backend 的 foreign display 前提](https://docs.rs/wayland-backend/latest/wayland_backend/sys/client/struct.Backend.html#method.from_foreign_display)也要求 display 覆盖所有 Backend 克隆的寿命。这里是源码与接口前提审阅所得的风险，未据此声称复现崩溃；后续原版关闭前缀已在明确延后断开的干预下观测，当前 Rust 安全适配保留连接到对应代理结束；该新实现尚待原生编译/执行，不能直接把构造失败或受控原版矩阵当作 Rust 早关验收。

首个原生 Linux 配置批次在 meojson 准备阶段失败：PowerShell 函数的 Inputs 参数遮蔽调用方同名记录列表，使 Save 对字符串调用 Add；尚未编译或执行配置 oracle/捕获。该记录保留，参数现改为 InputDirectory，并以与原生调用方相同的 inputs 列表作用域复验；原生修复结果仍待新的 Actions，不将先前 Windows 的不同列表名称包装器当作该修复的原生验证。

后续原生批次在更早的 Rust 测试目标编译处失败：JSON 创建夹具把节点 ID 放进 format 后失去原 Config 字段提供的 u32 推断，导致 E0284；该批次的 P0/P1 consumer 步骤均失败，尚未执行配置/捕获。现已显式声明 u32，修复后原生结果见下文；主机 Clippy 不编译该 Linux-only 测试体。

原 C++ 调用器另新增 `--task-retirement-read-fixture <外部目录>`，仍与取帧实现分离。普通 proc task stat 读取保持严格，并保存 errno、bad/fail/eof 和实际字节数；只有原生 close/stop/join 与所有者退出之后，才允许对已知原先活跃 MaaScreencap 的精确 ESRCH/bad/fail/非 EOF/零字节失败，再做一次新的 stat open 与任务目录 stat，二者均为 ENOENT 才证明已消失。初版成功读出的任务还要求相同 id/name/starttime、PF_EXITING、vsize/RSS 为零；未知、替换、活跃、部分数据或其他错误保持失败。没有增加取帧重试或改变既有 100ms 内核退出界限。独立原生控制通过真实命名线程的 held-stat 描述符产生 ESRCH，并要求已 join、文件/目录缺失；真实活线程路径与实际耗尽当前子进程 fd 后的 EMFILE 必须拒绝，随后关闭临时 fd、恢复原 soft/hard limit，并精确核对基线 task IDs/fd。另有 24 个明确标注的合成 metadata 反向控制，不代替真实原生退出证据。实现已加入显式 PipeWire 源码入口，并已通过下文的 94 项原生组件；普通 strict read-exit 原控制及所有旧失败记录继续保留，核心捕获/等待/像素与生命周期代码未改写。 此初版的内存字段附加条件已按[本轮内核退出复核](#本轮原生与-ci-复核)修订，新判据的原生执行仍待验证。

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

macOS 本机默认准备查询 Apple 发布 RSS，匹配正式版版本与实际 `xcodebuild -version` 构建号。Actions 显式设置 `MFS_TOOLCHAIN_POLICY=preinstalled`，保留 runner 可用的已安装选择；最新正式版/build 不匹配或发布查询失败只进入版本记录，继续实际 SDK、编译、链接和取帧检查。预装 RC 保留其真实 build 身份。选择仍仅通过进程环境 DEVELOPER_DIR/SDKROOT 完成，不修改系统 xcode-select。历史正式 Xcode 27.1（27A9275）与 SDK 27.0 的二十次 P0 捕获/所有权验证保持原范围；旧严格版本门禁的缺环境结果也不被改写。

同一 CI 策略用于 VS2026/Windows SDK、具备所需生成器的 CMake、Linux LLVM/linker/libclang 和可用 nightly。入口 [Rust 工具链准备](../../scripts/prepare-rust-toolchain.ps1)在 nightly 已可用时复用它，补齐所需组件；本机默认仍更新 nightly。项目和原生库依赖、任务自行获取的工具继续解析并验证对应输入。配置缺失、真实编译/捕获失败、零测试和回调/资源错误保留各自失败，不因版本策略变化跳过。缓存和离线重放继续使用实际版本及摘要，预装工具的原生离线重放需提供匹配的系统工具身份。完整政策见[计划 §5.6](../plans/maafw-screencap-extraction-plan.md#56-依赖选型与最新版本策略)。

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

文档校验使用 `python -B scripts/check-documentation.py`，内部失败控制使用 `--self-test`。它检查全部 Git 跟踪 Markdown 的仓库内链接、锚点、空白及 Conventional Commit subject，不更新 Rust/原生工具链或执行捕获。新增 `Documentation and commit checks` 工作流在分支 push、PR 和手动调度运行；纯 Markdown 分支 push 不再触发原生工作流，源码、脚本和 CI 配置变更仍触发三平台，所有 PR 及原生手动调度保留完整矩阵。源码/CI 提交的文档工作流已在托管 runner 通过四项内部控制、链接及消息检查；纯 Markdown 推送的触发范围尚待验证。文档绿色状态不构成任何原生或阶段验收。两种检查在 CI 中都以 `MFS_COMMIT_MESSAGE_REF` 指定实际贡献者提交，限制为 HEAD 或完整小写 SHA；PR 原生构建仍使用正常 merge checkout，不把 GitHub 合成合并 subject 当作贡献者消息。

网络不可用时，可显式选择已登记、具备所需依赖归档的原生运行记录，使用其依赖快照验证当前源码：

```text
cargo xtask native --desktop --offline-inputs runs/<prepared-run-id>/manifest.json
```

此模式恢复归档源码和 OpenCV 下载输入，以空 Cargo home、归档 vendor 和匹配的已安装 nightly 执行 Rust 离线验证，不查询在线最新版，也不自动补下载缺失依赖。系统 SDK/VS 仍需按记录准备。当前源码在执行前另存快照，原生构建及 Rust 测试从该副本执行；复验当前构建适配与重放原批次源码是不同用途，后者还须恢复该批次源码。下次在线构建仍重新检查更新。依赖归档不完整、哈希变化或本机工具链不匹配时明确失败，不计为通过。

实际记录格式由 `xtask/src/records.rs`、`reference.rs`、`batch.rs` 的版本化 serde 结构及校验逻辑维护，未知字段会失败。`manifest.json` 仅索引 baselines/patches/runs；已完成来源记录与运行记录不覆盖，运行中新失败保留在新目录。准备中断的目录保留以供检查，未索引目录不是有效基线。Cargo.lock 只约束同次验证与离线重建，不作为后续在线构建的版本约束。

Linux PipeWire 的 P0 原型日志在该步骤完成后即上传为 p0-linux-pipewire-diagnostics；P1 组件日志/图像在其源码步骤完成后即上传为 p1-linux-pipewire-diagnostics，不等待后续 Wlr 或完整工程归档。上传保留 failed/missing_environment，不改变门禁结果。

Actions 通过 `scripts/archive-evidence.ps1` 将 reference 打包为 `p0-evidence.tar`：显式枚举隐藏条目，用 PowerShell 的 .NET TarWriter 写入文件、空目录、链接及权限，再解包并执行 `reference verify` 后上传。该脚本拒绝覆盖输出，也拒绝将输出放进源 reference。下载 artifact 后先解包 tar；不要直接把展开的 reference 文件列表当成可保留上述元数据的归档。校验工具对 Git 子进程启用长路径支持，不修改宿主 Git 全局配置。实际失败状态保留在原运行记录，证据包校验通过不把失败的捕获或构建改记为通过。

Windows CI 另上传 `p1-windows-component-diagnostics`，包含当次 P1 component 的清单、原生结果、嵌套日志及参考程序原始 BGR/几何记录；捕获失败后也执行上传。它用于快速读取具体断言，不包含完整来源、依赖和归档元数据，不能代替上述完整证据包，也不改变失败状态。准备阶段未产生对应文件时明确报告无匹配文件。

ScreenDC 步骤结束后另立即上传 `p1-windows-screen-dc-diagnostics`，包括该组件的清单、原生结果、日志和全部参考程序 artifact，不等待后续 DesktopDup、Custom、manager 或 FramePool。异步调整首帧的 `screen-native-pending` 部分记录、布局与持帧文件同时加入最终 Windows 轻量包；缺少文件和未完成矩阵保持实际状态，上传本身不构成捕获验收。

确切 bd79217 的 [Actions 41207497250](https://github.com/AbyssRei/maafw-screencap/actions/runs/41207497250) ScreenDC 组件为 30/32 项通过：首帧原生测试完成 40 组、80 次捕获和 80 张关闭后持帧，随后 records 步骤及外层 preparation 失败。独立 checker 误将 `ClientToScreen` 的屏幕原点纳入不变字段，同时又要求窗口真实移动；现保留客户区本地矩形、样式和配置不变，明确核对屏幕原点按相同 delta 平移，并用回归测试拒绝原点、尺寸、样式、消息与后续状态变化。71 项 xtask 测试、严格 Clippy 和格式检查通过，不据此接受原生矩阵。官方 ZIP 摘要、427 个可用登记路径、160 份完整首帧/最终/持帧 BGR 及既有 152 次几何捕获、380 份快照已独立核对；旧轻量包未包含 pending TSV/布局，不具备完整 321 文件复算条件。还发现这两个原生 artifact 目录没有进入 `native-result` 输入登记，现由 finally 在成功或失败时均枚举登记，后续确切批次验证见下段。旧 CI 失败、缺失记录和本机前台环境失败保持原状态，该 CI 也没有本机输入观察器，不替代旧程序的有效连续复测结论。

随后 a3355f2 的 [Actions 41214537955](https://github.com/AbyssRei/maafw-screencap/actions/runs/41214537955) 即时诊断包包含全部首帧记录：404 行、321 个文件和 160 份完整 BGR。原生 test 通过，旧 checker 仍失败；独立复制并核对每份输入后，修正 checker 全部复算通过。四十组窗口均在取帧期间保持 queue gate 不变，直接释放后各收到一次真实调整，客户区屏幕原点实际向上移 48px，局部客户区/样式不变，最终几何不再变化；每组 GDI 前后均为十一。第一次下载在零字节处 EOF，失败单独保留，新目录中的官方 ZIP 完整摘要已核对。

修正后的确切 `12216e8009e8d4dd8b6368cd05262c30e8bb8056` [Actions 41215370477](https://github.com/AbyssRei/maafw-screencap/actions/runs/41215370477) ScreenDC 组件再通过全部 32 项原生检查。官方 ZIP 摘要、867 个可用登记路径、完整首帧像素/几何和独立 checker 均核对通过；finally 已登记全部 322 份 pending 文件及 616 份 geometry 文件。453 个完整来源输入不在该轻量包中，完整来源审计仍待补。该受控原生结果不接受任意外部窗口竞争或 DesktopDupWindow，也不将缺前台的本机三轮改为通过；同一 run 的 PrintWindow 步骤另有失败，不能把 ScreenDC 组件通过当作整个 Windows job 或 P1 通过。

`batch --prepare-only` 只更新 nightly、解析/核验最新 Cargo 依赖并归档锁文件、元数据和 vendor；不编译捕获 crate，不运行测试或离线消费，不能作为构建通过记录。

后续本机批次发现 WGC/WARP 原型的间歇性访问冲突，系统事件将故障地址归到已卸载的 `GraphicsCapture.dll`；该批次已失败，尚未进入 GDI 对照。调试器或独立压力诊断未复现不撤销原失败，具体根因和修复仍待确认。[WGC 维护者的相关调查](https://github.com/robmikh/Win32CaptureSample/issues/99)记录了内部异步工作与模块卸载的竞态，但目前不能据此断定本机根因。显式离线 GDI 对照只证明其指定输入和 GDI 组件，不替代该失败批次或 WGC 退出验收。

`configure` 为现有 P0 原型族解析 AUTO/ON/OFF，并输出外部 `build.conf` 路径与所需 Cargo features。族名为 windows、pipewire、wlr、screen-capture-kit，不代表最终 SDK 的方法/API。默认先执行上述在线准备；`--offline-inputs <Rust batch manifest>` 明确复用已归档解析，核对安装工具链及消费用 Cargo.lock，下一次在线准备仍重新检查更新。原生快照构建可用 `--consumer-lock <路径>` 指定其实际锁文件。请求、环境/原生依赖证据、源码与配置按新批次归档；ON 不满足时返回失败并保留记录，不发布可用配置。OFF 及其他平台不执行原生探测。

配置中 Windows 检查 VS2026 MSVC x64 与 SDK 库；PipeWire 检查 pkg-config 的 PipeWire/SPA、原型 v1_2_0 所需版本、clang 与 libclang 文件；Wlr 检查 system libwayland-client；macOS 检查原生 SDK/framework stub。非标准 libclang 安装通过 LIBCLANG_PATH/LLVM_CONFIG_PATH 指明。探测不代替生成器实际加载、编译/链接、运行时能力、权限和真实捕获。现有原生原型入口在依赖准备后传入配置，`build.rs` 根据 TARGET/CARGO_CFG_TARGET_OS 复验本机目标、准确 feature 集及选中依赖的相同探测证据；SDK、版本或路径漂移时要求重新准备。旧格式仅保留归档解释，新原生构建使用含证据的新配置；只向 OUT_DIR 写归一化配置/声明，声明配置、环境和发现文件的增量失效输入，不在 Cargo 启动后改变后端选择。

共用 `configure-prototype.ps1` 现在先保存子进程退出码并打印已捕获的标准输出，再检查失败和返回路径；原先失败时会在打印前抛出异常，丢失该输出。Windows 上的 PowerShell 隔离控制确认非零子进程仍以相同异常和退出码 17 失败，四族八次原/新成功调用的参数与配置路径也一致。该控制使用配置替身，没有执行原生配置或取帧；它不把 Actions 41248794571 的另一次 PipeWire P0 显式配置失败改为通过，实际子配置失败原因继续按归档记录审阅。

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
