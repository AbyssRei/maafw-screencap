# maafw-screencap

计划使用 Rust 实现的独立桌面截图库，目标平台为 Windows、Linux 和 macOS，对外动态链接边界采用 C ABI。

P0 准备阶段已按计划第 16 节验收，P1 实施中：已有独立 xtask、外部来源/补丁校验、分配与 Windows 原生取帧原型，Linux PipeWire/Wlr 和 macOS ScreenCaptureKit 的受控取帧已通过原生 Actions；Desktop Duplication 的 20 次硬件采集/释放原型已通过本机及原生 Actions 验证。默认 binary 仍为 Rust 2024 Hello World，内部 library 已开始窗口 GDI 与 PrintWindow 迁移，尚无完整上游基线、公共 SDK、语言绑定或发布资产；P0 的最小原生可行性验收不代表完整上游行为对照或产品捕获支持。实施顺序为完整迁移固定上游 U → 应用固定 PR #1587 得到 P → 在验收基线上建设 SDK 与扩展 E。

设计与实施从[文档导航](docs/README.md)开始；范围和阶段门禁见[实施计划](docs/plans/maafw-screencap-extraction-plan.md)，当前命令和后续构建要求见[开发与验证指南](docs/guides/development.md)。贡献与协作要求见 [AGENTS.md](AGENTS.md)。

默认工具链为最新 nightly，所需实验分配 API 保持内部封装。最终基础捕获核心及绑定不强制依赖 OpenCV；迁移对照采用显式启用的 C++ + OpenCV 验收适配。选型确认不表示平台能力已经验收。

项目许可为 LGPL-3.0-or-later，见 [LICENSE](LICENSE)；已迁移来源声明见 [THIRD_PARTY_NOTICES](THIRD_PARTY_NOTICES)；完整分发材料按实施计划在实际交付阶段补齐。
