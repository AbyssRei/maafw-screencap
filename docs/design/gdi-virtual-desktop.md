# GDI 全虚拟桌面捕获设计

日期：2026-10-27。状态：产品扩展规划，尚未实现或验收。范围与理由见 [ADR 0001](../decisions/0001-gdi-virtual-desktop-capture.md)；P2 验收后在 P3 实现，具体 SDK 类型、方法名与 ABI 值届时定案。

## 方法边界

工作名 `GdiVirtualDesktop` 表示独立的运行时截图方式，只匹配[全虚拟桌面目标](../contracts/lifecycle.md#全虚拟桌面目标)。窗口 GDI、ScreenDC 及其 U/P 默认选型保持原行为。后端编译能力仍按 Windows/GDI 原生依赖门控；单独的运行时方法不要求额外 crate 或一组 U/P/E feature。

捕获从当前可访问桌面的屏幕 DC 读取可见画面，不调用窗口激活、置顶、伪最小化、移动、缩放或样式 helper。固定方法不隐式降级到主屏、窗口或其他截图方式；来源或画面条件不满足时返回明确失败。

首期不增加手动光标绘制、HDR/tone mapping、任意用户 ROI 或逐显示器合成。光标、layered 窗口及内容排除的实际捕获行为按原生证据和验收声明；必需画面选项不能静默忽略，黑色画面不单独作为失败或受保护内容的判据。

## 原生流程

1. Session 校验目标和方法范围，建立本次 Operation、绝对期限与取消代次；按布局依赖规则形成来源快照。
2. 在已验证的线程 DPI 上下文中查询活动逻辑显示区域及其桌面矩形，确认与目标 DIB 像素的一比一映射，计算包围框 `(left, top, width, height)`。排除不属于实际桌面来源的伪监视器；镜像、重叠及坐标歧义必须在快照中得到解释，否则拒绝。
3. 获取屏幕 DC（例如 `GetDC(nullptr)`），创建兼容 memory DC 和显式 32 位 BI_RGB top-down DIB，保存被替换的原位图。所有尺寸、stride、总字节和 Win32 整数窄化先校验容量与溢出；不以 dpi 百分比缩放截图来掩盖坐标不一致。
4. 从屏幕 DC 的 `(left, top)`，向目标 DIB 的 `(0, 0)` 执行一次覆盖包围框的 BitBlt。负桌面坐标合法；layered 窗口的 raster-operation 选择单独声明并验证，不沿用窗口方法的 flags 作为默认依据。
5. 检查 BitBlt 与 GdiFlush 等同步结果，在原生有效期内取得独立 CPU 存储，按 [Frame 契约](../contracts/frame.md#全虚拟桌面帧)处理空隙、填充和未使用字节。DIB 复用或销毁不能改变已交付像素；发布 BackendSample 后不再为包装 Frame 默认完整复制一次。
6. Session 复验来源、布局、期限和取消状态，成功后发布同次像素与元数据。原生调用失败、布局变化或操作已终结时拒绝结果，并保留主要及清理错误。

步骤中的布局、资源、样本和最终发布状态分别遵守[架构职责](architecture.md)，后端不反向控制 Session 或缓存。可共享底层资源封装，但不维护两份窗口 GDI 或改写冻结的 U/P 快照。

`SM_*VIRTUALSCREEN` 描述虚拟桌面范围，但不能将系统指标、显示器枚举与不同 DPI 上下文的数值混用。微软说明 GetSystemMetrics 不适用于 per-monitor DPI aware 调用线程；具体布局查询组合和物理像素证据须在 P3 原型中验证，不将四个系统指标直接当作混合 DPI 支持已成立的证明。

## 资源与执行责任

屏幕 DC 的获取与释放在同一线程配平；同一 DC 不并发使用。memory DC、所选位图、DIB 指针、同步及恢复/删除顺序由 Windows 资源封装管理，覆盖部分初始化失败。临时线程 DPI 上下文在成功、失败和重入路径恢复，不改变宿主进程 DPI 设置。

首期不引入通用异步运行时或常驻逐屏 producer。Session 是否可在线程间迁移仍按[执行模型](execution-model.md)验证；需要复用的原生资源及其失效依据明确归属。取消和超时不承诺打断 BitBlt，返回后拒收晚到结果，退出清理耗时另计。

## 原始输出与时间证据

首期以已验证的 BGRX8 独立 CPU Frame 为输出目标，不强制 BGR 转换；GDI 可能在写入 DIB 时完成设备格式转换，因此“原始”表示未编码的已声明捕获格式，不表示显示设备或 HDR 源的逐位原样数据。颜色信息未知时如实标记，不由 BI_RGB 推断 sRGB。

记录 API 取图起止与交付时间，区别于显示器呈现时间。一次 BitBlt 不证明全桌面同一时刻取样，不产生逐屏原生时间戳。新鲜度保证按实际时间证据协商，不能把复制完成时间填成新的原生呈现时间。

本方式不依赖逐屏 Frame 合成；对布局空隙进行初始化不意味着逐屏独立捕获。跨 GPU、远程会话、旋转和混合 DPI 分别记录真实验收条件，不因 API 成功或单屏通过推断全部组合可用。

## 验证入口与依据

确定性几何/布局/容量及失败路径由 owning module 单元测试验证；公开 Frame/目标与语言消费走契约测试，真实显示场景走显式桌面任务。必需断言以[计划第 14.2 节](../plans/maafw-screencap-extraction-plan.md#142-必需夹具)为准；本文件不建立尚无实现的测试入口。

- [GetDC](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getdc)、[CreateDIBSection 与 GDI 同步](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createdibsection)
- [BitBlt](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-bitblt)、[GetSystemMetrics 的 DPI 与监视器限制](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getsystemmetrics)
