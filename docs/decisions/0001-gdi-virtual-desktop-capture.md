# ADR 0001：独立的 GDI 全虚拟桌面截图方式

日期：2026-10-27。状态：纳入产品扩展规划；尚未实现或验收。影响阶段：P2 验收后的 P3，策略与绑定分别随 P4/P5 验收。

2026-10-30 来源与阶段引用按 [ADR 0003](0003-corrected-fork-extraction-baseline.md)调整为 F：修复随 P1 提取，P2 冻结并验收，不重复应用旧 PR。本 ADR 的独立 GDI 产品扩展范围及 P3 实施时机不变；历史 U/P 证据不改写。

本 ADR 补充[既有技术选型 R1–R8](interactive-technology-selection.md)，不替代其中任何决定，也不改变 F 的迁移范围、输出或选型语义。具体方法名、目标类型、能力值和 ABI 签名在 P3 定案。

## 背景与选择

用户提出将 GDI 全虚拟桌面捕获作为新项目的一种独立截图方式。固定上游的 GDI 从窗口客户区 DC 取图；ScreenDC 从屏幕 DC 复制指定窗口客户区，并有前台及窗口位置调整。二者均不等于直接输出全虚拟桌面。

为覆盖一次获取当前可见桌面的用途，新增 Windows 专用方式，工作名为 `GdiVirtualDesktop`，仅匹配明确的全虚拟桌面目标。它通过屏幕 DC 和一次矩形 BitBlt 取得整桌面画面，不依赖逐显示器取样合成、OpenCV 或目标窗口句柄。

与既有 GDI 共享经验证的 DC、位图及复制封装属于实现复用，不合并两种方法的目标语义，不用空 HWND 隐式扩大窗口目标的捕获范围。Auto 仅在目标及方法范围明确允许时考虑该方式；当前窗口方法的默认值不因新增方式而改变。

## 理由与替代方案

- 独立方式能分别描述全桌面来源、布局依赖、输出原点和能力，避免把窗口客户区与可见桌面混为一类。
- 屏幕 DC 原生覆盖整桌面的路径无需先取得每个显示器的独立 Frame；逐屏捕获后合成仍可作为其他后端的未来扩展，不能由本决定推导为已纳入交付。
- 显式 32 位 top-down DIB 便于定义原始布局，替代读取设备相关位图的方案在 P3 单独验证；不回改 F 的 CreateCompatibleBitmap/GetBitmapBits 和 BGR 转换行为。
- 该方式捕获当前桌面可见内容，不提供后台窗口、最小化窗口内容或跨显示器同一呈现时刻的保证，也不预设比现有方法更快。

## 落实与验收

[GDI 全虚拟桌面设计](../design/gdi-virtual-desktop.md)维护原生流程与资源责任；[来源契约](../contracts/lifecycle.md#全虚拟桌面目标)和[Frame 契约](../contracts/frame.md#全虚拟桌面帧)维护调用方保证。
[计划第 14.2 节](../plans/maafw-screencap-extraction-plan.md#142-必需夹具)维护验收要求。当前仅更新规划，不创建后端、示例、能力常量或验收通过记录。

## 依据

- [基线窗口 GDI](https://github.com/MaaXYZ/MaaFramework/blob/0fc6025c2fc224a171b57d89505e8d47acd2a8b1/source/MaaWin32ControlUnit/Screencap/GdiScreencap.cpp)
- [基线 ScreenDC](https://github.com/MaaXYZ/MaaFramework/blob/0fc6025c2fc224a171b57d89505e8d47acd2a8b1/source/MaaWin32ControlUnit/Screencap/ScreenDCScreencap.cpp)
- [GetDC](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdc)、[虚拟屏幕](https://learn.microsoft.com/en-us/windows/win32/gdi/the-virtual-screen)
- [CreateDIBSection](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createdibsection)、[BitBlt](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt)
