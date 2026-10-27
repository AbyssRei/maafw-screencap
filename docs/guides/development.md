# 开发、构建与验证

日期：2026-10-27。状态：P0 部分实现。下节列出已可运行的工具与原型；后续构建责任、完整对照、SDK 和桌面验收要求仍随 P0–P5 实施，不能据此宣称完整阶段通过。

## 当前仓库与本机命令

产品入口仍是 Rust 2024 Hello World binary。独立 workspace xtask 与 P0 测试已实现，尚无迁移后端、产品捕获库或 binding。`.github/workflows/p0.yml` 定义三个原生 runner 的工具验证和现有原型；Linux/macOS 原生捕获尚未实现，不能将矩阵通过解释为 P0 通过。命令从仓库根执行，本机只验证宿主 OS，不以 WSL 代替 Linux 验收。

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

默认 workspace 测试包含 4 项分配原型与 6 项工具测试。Windows GDI 桌面原型仅在 `prototype-windows` 下编译，默认忽略，须显式串行执行：

```powershell
cargo test --release --test p0-interop --features prototype-windows -- --include-ignored --test-threads=1
cargo clippy --workspace --all-targets --features prototype-windows -- -D warnings
```

该原型在自有小窗口绘制确定颜色，以 GetDC → compatible bitmap → BitBlt → GetBitmapBits 实际取帧，校验 BGR 字节，并在释放原生资源后验证独立像素及跨线程释放。它不提供产品截图 API，不构成六种 Windows 方法、COM/WinRT、回调退出、混合 DPI 或多显示器验收；P3 的 DIB 现代化与全虚拟桌面路径均未提前实施。

## 已实现的 P0 工具

```powershell
cargo xtask reference prepare
cargo xtask reference verify
cargo xtask check
cargo xtask batch --desktop  # Windows 显式桌面批次；其他平台暂不加此参数
```

- `reference prepare` 获取固定上游与固定 PR head，核验两个提交及唯一变更文件，外置保存完整未打补丁源码归档、相关文件发现清单、许可声明记录、累计补丁和 head 文件。它只做来源准备，不应用补丁，不构建参考程序，也不生成已验收 U/P。可用 `--root <目录>` 或 `MFS_REFERENCE_ROOT` 指定独立参考根。
- `reference verify` 校验布局版本、未知字段、必需身份、父 U、Git 对象/文件内容、完整归档与记录 SHA-256、来源清单覆盖和相对路径越界，包含 symlink/junction 的实际路径检查。完整职责审阅、方法/目标矩阵和依赖许可审计仍待完成。
- `check` 校验 HEAD 的 Conventional Commit 标题及 workspace 版本/许可元数据；提交前可用 `--message-file <文件>` 校验拟用消息。它不代替发布阶段的 API 兼容性与版本协商验收。
- `batch` 先更新 nightly，再更新本次 Cargo 解析并查询每个 registry 包的最新正式版（包括范围外主版本），归档锁文件、元数据、vendor 源码、实际源码快照和逐项日志，再执行 release build、workspace tests、fmt、Clippy 和项目约定检查。Windows 的 `--desktop` 追加显式原型及其 Clippy。失败单列且不自动重试；所有批次均明确记录 P0 仍缺失的门禁。

`xtask` 不依赖捕获 package，默认 Cargo 构建只构建产品入口。普通构建不读取 reference，不链接 OpenCV。当前 `build.rs` 仅在目标 Windows 且显式选择原型 feature 时读取维护的过滤输入，向 OUT_DIR 生成私有 Win32 声明；其他平台声明/生成与 AUTO/ON/OFF 尚未实现。

实际记录格式由 `xtask/src/records.rs`、`reference.rs`、`batch.rs` 的版本化 serde 结构及校验逻辑维护，未知字段会失败。`manifest.json` 仅索引 baselines/patches/runs；已完成来源记录与运行记录不覆盖，运行中新失败保留在新目录。准备中断的目录保留以供检查，未索引目录不是有效基线。Cargo.lock 只约束同次验证与离线重建，不作为后续在线构建的版本约束。

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
