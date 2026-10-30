# 原生捕获接口现代化设计

日期：2026-10-27。状态：已纳入产品实施计划，尚未实现或验收。依据 [ADR 0002](../decisions/0002-native-interface-modernization.md)，仅在 P2 验收后实施；公共 SDK 类型、能力值和 ABI 在 P3 定案。

## 迁移清单

下表定义产品 E 的原生迁移目标。GetBitmapBits 和数字节点目标选择有明确旧兼容/弃用说明；其他行分别属于有效接口升级、对象职责整理或新增协议接入，不统称为已弃用。版本号更大不是完成标准。

| 路径 | 固定上游入口或现状 | 产品迁移目标与边界 |
| --- | --- | --- |
| GDI / ScreenDC 像素读取 | CreateCompatibleBitmap + GetBitmapBits | 改为显式 32 位 top-down CreateDIBSection，完成 GDI 同步后取得独立 CPU 存储，移除 GetBitmapBits；保持窗口 DC 与屏幕 DC 来源区别 |
| PrintWindow 像素读取 | CreateCompatibleBitmap + GetDIBits | 改为显式 DIB 接收 PrintWindow 输出，核验写入完成、格式和持帧寿命；GetDIBits 仍有效，不能将其误列为旧 16 位兼容接口；保留客户区、弹窗合成和窗口包装职责 |
| DesktopDup 工厂与设备 | CreateDXGIFactory / IDXGIFactory / EnumAdapters | 转向 CreateDXGIFactory1 或 CreateDXGIFactory2，并使用 IDXGIFactory1::EnumAdapters1 等实际需要的接口；明确功能所需版本，不仅为使用编号更高的函数增加依赖。按目标输出匹配 adapter，并从正确 adapter 创建 device，联合核验工厂来源和资源失效，不以性能偏好枚举替代目标匹配 |
| DesktopDup 输出复制 | IDXGIOutput1::DuplicateOutput | 转向 IDXGIOutput5::DuplicateOutput1，显式提供支持格式列表；先兑现已验证的 8 位输出，其他格式分别验收。格式变化不得绕过 staging 描述复验或静默改变公开格式；DesktopDupWindow 复用相同底层路径 |
| FramePool 创建 | Direct3D11CaptureFramePool::Create | 改用 CreateFreeThreaded，去除 frame-pool 对 DispatcherQueue 的依赖；WinRT 资源仍由约定 MTA 通道拥有。若注册 FrameArrived，其内部工作线程回调只执行允许的通知/状态工作，验证保活、撤销与排空 |
| FramePool 设备初始化 | D3D11CreateDeviceAndSwapChain，保存未见捕获用途的 swap chain | 核验用途后改为 D3D11CreateDevice，只保留所需 device/context 及互操作对象，移除无用 swap chain 和目标窗口关联；不为截图换建另一种新 swap chain，若确有用途须记录具体证据及独立处理 |
| PipeWire 目标选择 | pw_stream_connect 的 target_id 直接传 node_id | 改为 PW_ID_ANY + PW_KEY_TARGET_OBJECT，优先使用已解析目标的 object.serial，节点名称仅在身份无歧义时采用；新 provider 描述保留来源证明范围。旧 node ID 输入如需兼容，在同一已授权 remote 中即时解析并验证 serial，不把可复用 ID 当持久身份，不连任意节点或 daemon 替代目标 |
| PipeWire 等待 | pw_thread_loop_timed_wait 的整数秒相对等待 | 产品等待改用 pw_thread_loop_get_time / pw_thread_loop_timed_wait_full，按原生时钟构造绝对期限，并从同一 Operation 剩余预算推导；保留锁/谓词纪律，覆盖伪唤醒、错误及取消。原函数仍有效，迁移动机是预算精度和语义 |
| Wayland 输出捕获 | zwlr_screencopy_manager_v1 / Wlr | 接入 ext-image-copy-capture-v1 及其输出 image capture source 协议，作为明确的 ExtImage 方法；首期只承诺通过验收的输出来源，不自动加入窗口捕获。按服务端广播、客户端支持和所需能力绑定；保留 Wlr 作为已迁移的独立兼容方法，不冒充 ExtImage 成功，不在固定方法中隐式切换 |
| Linux 上游兼容导出 | MaaKWinControlUnit / MaaWlRootsControlUnit 转接入口 | 新产品只提供自身 Rust/C ABI/C++/Python 契约，不再导出旧 Maa 接口或引入其输入控制；保留 F 对截图构造、授权输入接管和退出职责的覆盖 |

独立 GDI 全虚拟桌面方式从首次产品实现即采用 [ADR 0001 的 DIB 路径](gdi-virtual-desktop.md)，不先新增一条 GetBitmapBits 实现再迁移。DIB、布局初始化和独立 Frame 保证继续以[像素与内存契约](../contracts/frame.md)为准。
新建或复用的 DIB 在每次取图前初始化所有可能未被本次原生写入覆盖的字节；PrintWindow 等无法证明完整覆盖时清零整个画布，不能把未初始化内存或上次残留当作本次像素。CPU 初始化/读取与 GDI 写入按同步契约排序，覆盖部分绘制及复用后失败的夹具；清零本身不证明取得了目标内容，仍须验证原生结果及受控图样。

## 能力、来源与执行约束

每项新入口分别声明最低 SDK、运行时/扩展接口及构建能力，执行必要的 QueryInterface/运行时探测，不以新版绑定中存在声明推断宿主可用。CreateDXGIFactory1/2 的最终选择按实际需要确定，新增显示格式、窗口目标和 HDR 仍需独立能力及验收，不在本次迁移中默认开放。

PipeWire 的 serial 必须来自同一授权 remote 和目标解析证据；授权输入消费及重建沿用[来源契约](../contracts/lifecycle.md)。新接口不会消除节点消失、权限撤销或 socket 消费责任，旧来源失效后不得使用复用的 ID 选中另一来源。

ExtImage 需要新的协议输入、代理/会话/frame 生命周期和事件状态机；不能只替换 Wlr 的函数名。处理缓冲约束批次、格式重协商、transform、damage、presentation_time、stopped/failed 和取消后晚到事件。无更新可能等待内容变化，必须受产品预算约束；继续使用既定 Wayland 执行通道及队列隔离。协议时序以本次解析 XML 为准，不宣称全部 Wayland compositor 均支持。

Wlr 与 ExtImage 的方法能力分别查询；Auto 只能在同一已验证输出、授权、画面和副作用约束内选择。服务端只支持旧协议时可显式选 Wlr；固定 ExtImage 缺能力时报告不支持。新增协议按 target、选定后端及系统依赖门控生成/链接，不能迫使只选 PipeWire 或 Wlr 的消费者准备 ExtImage 输入。

FramePool 新工厂不赋予原生对象任意线程可用性。迁移遵循[执行模型](execution-model.md)，事件唤醒、共享 producer 或调度优化仍单独遵守 P6 门禁。标准线程和同步先完成正确性，不引入通用 async runtime 作为迁移前提。

## 实施顺序与兼容边界

P0/P1 记录旧入口及实际调用职责，P1 按 ADR 0003 同步修正 fork 的窗口修复；P2 只冻结来源并汇总验收。P3 沿同一实现逐项迁移 DIB、DXGI、FramePool、PipeWire 并新增 ExtImage；各项可独立对照，不混入源码格式整理或未经解释的像素/等待变化。P4 验证新鲜度、预算、恢复和选法，P5 验证声明、符号、打包及真实消费。

明确弃用的内部用法在对应产品路径放行前移除；仍有效的接口按清单迁移。新接口缺失时报告缺少的能力；若支持范围确需兼容原生调用，实施前记录环境、选择条件、语义与独立验收，不因“兼容性更好”自动保留隐藏回退。F 快照外置，不用语义 feature 或复制 legacy/product 实现保留旧行为。

以下不因本次清单而强制替换：GetDC、BitBlt、PrintWindow、仍有用途的 GetDIBits、D3D11 和系统 Wayland/PipeWire 基础 API；macOS 的 SCShareableContent、SCScreenshotManager sample-buffer 及 CoreVideo 路径。macOS 26 的 captureScreenshot 等新增能力按实际需求另行决定，不能仅为接口更新抬高最低 OS。

## 完成证据

每项保存旧/新入口、适用方法、来源/格式/线程差异、最低能力及兼容选择，验收要求以[计划第 14.2 节](../plans/maafw-screencap-extraction-plan.md#142-必需夹具)为准。真实取帧、资源关闭、功能缺失和平台未验证分别报告；生成绑定或编译通过不等于迁移完成。对照来源和实际报告按计划第 6.2 节外置。

## 原生依据

- [GetBitmapBits 的旧兼容说明](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-getbitmapbits)、[CreateDIBSection 与同步](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/nf-wingdi-createdibsection)
- [CreateDXGIFactory](https://learn.microsoft.com/en-us/windows/win32/api/dxgi/nf-dxgi-createdxgifactory)、[DuplicateOutput1](https://learn.microsoft.com/en-us/windows/win32/api/dxgi1_5/nf-dxgi1_5-idxgioutput5-duplicateoutput1)
- [CreateFreeThreaded](https://learn.microsoft.com/en-us/uwp/api/windows.graphics.capture.direct3d11captureframepool.createfreethreaded)、[D3D11 设备与 swap chain](https://learn.microsoft.com/en-us/windows/win32/api/d3d11/nf-d3d11-d3d11createdeviceandswapchain)
- [PipeWire 目标选择](https://docs.pipewire.org/group__pw__stream.html)、[Thread Loop 等待](https://docs.pipewire.org/group__pw__thread__loop.html)
- [ExtImage 协议源](https://gitlab.freedesktop.org/wayland/wayland-protocols/-/blob/main/staging/ext-image-copy-capture/ext-image-copy-capture-v1.xml)
