# 文档导航与维护约定

日期：2026-10-27。P0 实施中：独立 xtask、外部来源校验、分配原型、显式 Windows GDI、COM apartment、D3D/DXGI/WinRT 读回及受控窗口 FramePool 捕获/事件原型已实现，Actions 原生矩阵已定义；Linux PipeWire 初版取帧通过、原生 SPA 夹具退出失败；已改用自有 C 源夹具，完整原生验证待完成；结果按平台记录，完整 P0 门禁待完成。产品入口仍为 Hello World，迁移后端、捕获库及绑定尚未实现。实际命令及原型边界见[开发指南](guides/development.md)。

## 阅读顺序与权威来源

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
