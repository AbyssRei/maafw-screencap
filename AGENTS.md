# Repository Guidelines

## Actual State and Reading Order

P0 preparation has passed the plan section 16 gate; P1 is in progress, starting with window GDI and an explicit partial parity entry. The default binary still prints Hello World; a private capture library now begins the window GDI migration. An independent `xtask` prepares/verifies external fixed-source records, archives validation batches and checks project conventions. Its capture catalog validates external source identities, exact line evidence and route/target/case references while retaining pending reviews and unexecuted cases; catalog validation is not capture or phase acceptance. Registry license inventories retain target-graph declarations and exact vendor documents; source/native/system/distribution review remains separate. P0 configuration preparation resolves prototype AUTO/ON/OFF independently of capture compilation, and build.rs verifies the selected native target/features/dependencies. Explicit configuration checks exercise missing dependencies and an unbuildable capture consumer; native results remain platform-specific. P0 tests cover fallible allocation, controlled-window GDI, COM accounting, WARP D3D/DXGI/WinRT interop/readback, FramePool empty results and controlled-window capture/events on an owned MTA/dispatcher, plus private C++/OpenCV callback/FFI boundaries. Hardware and WARP WGC cases are explicit, each with 20 lifecycle cycles; results remain platform-specific. A controlled-window Desktop Duplication prototype passed twenty default-hardware device/output/acquisition lifecycles locally and on native Actions, with checked target-only CPU copies before Unmap, explicit ReleaseFrame evidence and retained pixels; both results remain specific to the tested native environments. Its multi-output/access-loss/rotation and the full FramePool failure/resize/permission matrices remain unvalidated. `xtask native` prepares latest native dependencies and builds an independent unpatched GDI reference; explicit offline dependency snapshots are supported. Linux PipeWire native validation covers an owned C producer with padded RGB rows, twenty stream cycles of 32 frames each, native buffer returns/reuse, callback ownership, independent CPU pixels and graceful producer/daemon shutdown. The earlier SPA fixture teardown failure is retained externally. Portal desktop capture and DMA-BUF remain unvalidated. A real headless wlroots/Pixman compositor and system-libwayland Wlr prototype passed twenty native Actions capture/SHM/fd lifecycles, foreign event-queue isolation, proxy data release and graceful compositor shutdown. Full compositor/format/failure and original-reference parity gates remain outstanding. The macOS arm64 objc2/ScreenCaptureKit harness passed twenty controlled-window captures, checked BGRA copy/lock/unlock, callback/input-owner release and one abandoned in-flight request on native Actions; AppKit remains on the process main thread. Full display/format/failure and original-reference parity gates remain outstanding. A native Actions matrix is defined. The private GDI implementation uses the original GetBitmapBits path and calls a caller-supplied conversion boundary before native cleanup; it defines no public SDK. A private PrintWindow migration now shares DC/bitmap cleanup, with an independent feature and explicit original-reference parity entry; temporary bitmap deselection satisfies GetDIBits preconditions, and popup/native-failure coverage remains partial. Other migrated backends, complete manager chains, bindings and examples are not implemented. `xtask upstream-parity gdi` explicitly builds the original C++ reference/conversion adapter and runs partial GDI parity; native failure/cleanup and iconic scenarios are included, while actual nondefault display depth and complete backend/phase coverage remain pending. Fault fixtures separately report and reclaim residual native resources; that recovery is not a product cleanup success. A later local WGC/WARP prototype batch failed with an access violation attributed to unloaded GraphicsCapture.dll; its root cause remains under investigation, and non-reproduction in diagnostic runs does not revoke the failure. P0 acceptance covers preparation and minimal native feasibility; complete U parity belongs to P1. A prototype or scaffold build is not validated product capture support.

The private PseudoMinimize helper and PrintWindow wrapper preserve synchronous ensure, original 100ms polling, inactive versus stop behavior and join-before-restoration. Twenty-two exact original-body scripts use explicit fake scheduling; real worker join order and desktop lifecycles are separate evidence. PrintWindow fixtures now have dedicated UI owners/message loops and UI-thread statistics/close commands; creation/activation and callback shutdown remain on those owners, followed by a completed join before module/context release. Same-thread layered first-frame failures are retained externally; independent pumped-owner controls and strict layered/nonlayered stop/inactive plus target-loss lifecycles now pass. Foreground recovery remains a distinct native case: missing foreground control is recorded as missing_environment and never a pass. The earlier independent popup black-bitmap failure is not explained by subsequent non-reproduction. Full helper/wrapper and P1 acceptance remain incomplete.

Read these authorities before changing the corresponding area:

ScreenDC now has a private independent feature and explicit `xtask upstream-parity screen-dc` component entry, with unpatched default ForegroundUtils/HwndUtils helpers. Compiled original-header observation covers native call/sleep/geometry semantics; scripted capture/resource faults remain separate from native desktop acceptance. Local presented-window parity passed twenty paired captures with strict known pixels, real GDI counts and retained CPU images. Fixture-only controls locate cold window initialization outside capture accounting; source-only before/after presentation controls reproduce stale visible content and establish fixture readiness without changing capture or retrying failed pixels. Both earlier failures remain archived. Occlusion, real native fault/cleanup, move/resize, compatible-mode scale, complete method/helper/manager and P1 acceptance remain incomplete.

| Authority | Responsibility |
| --- | --- |
| [Documentation index](docs/README.md) | Actual documents, their status, and maintenance rules |
| [Extraction plan](docs/plans/maafw-screencap-extraction-plan.md) | Scope, phase order, acceptance gates, dependency and release policies |
| [Technology decisions](docs/decisions/interactive-technology-selection.md) | R1–R8 rationale and links to subsequent numbered ADRs |
| [Architecture](docs/design/architecture.md) and [execution model](docs/design/execution-model.md) | State ownership, platform execution channels, callbacks, window coordination |
| [Frame](docs/contracts/frame.md), [lifecycle](docs/contracts/lifecycle.md), and [C ABI](docs/contracts/c-abi.md) | Proposed caller guarantees; concrete SDK contracts are finalized in P3 |
| [Development guide](docs/guides/development.md) | Commands available now and build/test responsibilities to implement later |

The later formal ScreenDC component run failed two black-image assertions after GDI regression: both source/Rust were black, including source-only after the presentation barrier. All raw images/logs remain archived; earlier isolated passes do not supersede this failure. ScreenDC fixtures now reuse the existing dedicated rendering/UI owner, without compiling the PrintWindow backend in ScreenDC-only native targets. Validate that changed fixture premise separately and retain strict pixels; do not assume DwmFlush alone establishes readiness.

The pumped-owner follow-up still failed both black-image assertions despite observed WM_PAINT and valid owner identities; do not claim a fixture fix. Read-only idle/power/foreground evidence suggests a powered-off display, pending user confirmation, and does not retroactively reclassify assertion failures. Independent callback-module lifetime and scripted helpers pass. The PrintWindow-only filtered regression also retained cold-process resource and original-popup black-image failures; its Print-enabled source branch is unchanged by the fixture-sharing guards. No component/phase acceptance or successful PrintWindow regression is asserted.

Keep each detailed rule in its authority; this file summarizes constraints and links to them. User authorization and preferences from the current task still apply; do not invent another approval requirement for work already authorized.

## Phase Order and Scope

Follow **P0 preparation → P1 complete upstream migration U → P2 fixed PR #1587 patch P → P3 SDK/correctness and planned native migrations → P4/P5 strategies, bindings and assets → P6 measured optimization**. All platform backends and direct screenshot helper/manager paths must pass P1 before P2 starts. Do not start final SDK implementation or additional behavior changes before P2 acceptance.

U/P must preserve upstream defaults, method selection, pixel conversion timing, waiting/replay, native lifetimes, window adjustments, observation, and shutdown. Their parity target is Rust capture plus explicitly built C++/OpenCV acceptance adapters; conversions stay at the original point when timing or ownership requires it. E evolves the same implementation through separately justified changes. Store reconstructable U/P snapshots externally; do not maintain duplicate legacy/product backends or U/P/E semantic features.

P3 includes two planned additions, neither implemented or validated yet:

- [ADR 0001](docs/decisions/0001-gdi-virtual-desktop-capture.md): independent Windows GDI virtual-desktop capture, with an explicit target distinct from window GDI and ScreenDC. Do not infer desktop scope from a null HWND. Details: [GDI virtual desktop](docs/design/gdi-virtual-desktop.md).
- [ADR 0002](docs/decisions/0002-native-interface-modernization.md): DIB, DXGI, FramePool and PipeWire migration, plus ExtImage output capture. Follow the [migration checklist](docs/design/native-interface-modernization.md); distinguish deprecated uses from supported older APIs. Verify runtime, format, source, thread and cleanup requirements. Retained compatibility paths require documented conditions and validation, not silent fallback.

Finalize public types, ABI values, state encoding, strict geometry, shared leases and extended scheduling in their assigned phases. Input control, ADB/Android, PlayCover, OCR/recognition, task pipelines, recording/replay and Qt application logic are outside this capture SDK. Host authorization, source selection and capture recovery remain distinct responsibilities.

## Architecture and Platform Boundaries

- Use one capture crate plus an independent `xtask`. Gate modules, runtime dependencies, native discovery, generation and linking by target and selected backend. Runtime dependencies belong in Cargo target tables; `build.rs` uses `TARGET`/`CARGO_CFG_TARGET_OS`, because its own cfg describes the host.
- Keep Frame/target/error models independent of orchestration and C ABI. Session owns mutable lifecycle, operation, cache and publication state; policy returns decisions; backends return samples and native evidence without controlling Session or cache.
- Keep Windows coordination, native declarations and resource wrappers in platform modules. Use `src/c_api/` only for outward exports. Maintain generator inputs in `build-support/`, generate into `OUT_DIR`, and keep handwritten safety wrappers separate.
- Native leases remain backend-private. In their valid lifetime and required execution channel, validate layout, isolate the target and obtain independent CPU storage. BackendSample owns it; publication transfers or immutably shares it without a default second full-frame copy.
- Record every backend's create/access/destroy thread, callback work, waits, unregister completion evidence and late-result owner. Serial Session use does not prove mobility; establish Send/Sync independently. Standard synchronization and required native loops come first; the base library requires no general async runtime.
- Preserve host DPI/COM context according to the execution contract. Stable callback addresses do not replace unregistering and draining callbacks. Source, format, device/session and window-lease generations are distinct; late results cannot restore invalidated state.

## Pixels, Ownership and ABI Safety

Use checked size, stride, offset and narrowing arithmetic in every profile. Describe exact format, row direction, alpha, color evidence and readable span; do not assume stride equals width times channel count or that every backend supports every format. Expose only target pixels and defined initialized padding.

Frame storage and quota accounting outlive Session/Runtime handles. Views retain the real Frame owner; writable outputs use isolated copies. Do not reuse storage while a caller still owns it, prolong an unbounded native lease, or describe a moved conversion as eliminated work. Keep sample identity and capture-time evidence through reuse and conversion.

Keep unsafe local and document pointer validity, ownership transfer, aliasing, thread affinity and release invariants. Use nightly standard-library fallible allocation APIs behind internal owner helpers, retaining standard Arc/Weak; no stable fallback or custom general reference counting. Preserve primary and cleanup errors, panic/exception boundaries, and an allocation-independent minimal error/wakeup path. Do not claim recovery from process-aborting OOM or cancellation of non-interruptible native calls.

Public C headers are the ABI authority. Generate or automatically verify Rust/ctypes declarations, constants, capabilities and export lists from that contract. Safe Rust APIs use the shared implementation directly. Verify structure prefixes, failure outputs, integer widths, real native consumption, callback ownership and shutdown; generated declarations alone do not prove safety.

Use rustfmt's four spaces, snake_case functions/modules, UpperCamelCase types and SCREAMING_SNAKE_CASE constants.

## Dependencies and Build Workflow

Run commands from the repository root. `rust-toolchain.toml` selects nightly, rustfmt and Clippy. Update nightly before overriding `CARGO_HOME`; if already overridden, use `rustup update nightly --no-self-update`. For a build/validation batch:

```powershell
rustup update nightly --no-self-update
$env:CARGO_HOME = "$PWD/../maafw-screencap-deps/cargo-home"
$env:CARGO_TARGET_DIR = "$PWD/../maafw-screencap-deps/target"
cargo build --release
cargo test --workspace
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

`cargo run` currently prints Hello World. `cargo test --workspace` runs allocation prototypes and xtask tests; it does not run desktop capture. The explicit Windows prototype command and external batch records are documented in the development guide. For documentation-only work, validate links, anchors, status and diffs; do not describe tool/prototype tests as full desktop acceptance or update the toolchain solely to check Markdown.

Future `xtask` preparation must run without directly or indirectly depending on the capture crate. Resolve latest dependencies and backend AUTO/ON/OFF before invoking Cargo, producing explicit per-build configuration. `build.rs` verifies it, builds/generates necessary native pieces in `OUT_DIR`, and declares input/environment invalidation; it does not download OpenCV or write external acceptance reports. Direct Cargo consumers use explicit features and documented system dependencies without reference data or `xtask`.

Features add compiled capabilities; runtime options select methods and policies. Do not encode AUTO/ON/OFF or mutually exclusive U/P/E behavior as features. Keep OpenCV outside the core dependency tree; optional Rust image adapters belong in binding-side packages or consumer source. Default product builds and assets exclude parity adapters and reference inputs.

Use latest direct dependency releases and check new major versions; cache hits never skip update checks or silently select an older dependency. For selected platform bindings, the user has authorized their exclusive transitive dependencies to use the newest formal release satisfying verified upstream constraints. Record global latest, required compatible latest, parent constraints and upgrade gaps each batch. Direct and shared nonbinding dependencies retain the global-latest requirement; do not waive a lagging compatible release. Archive resolutions, Cargo.lock, toolchain identity and required offline inputs externally. Do not commit a long-term lockfile. Offline replay uses the archived batch; the next online batch checks updates anew.

Initial Windows native validation uses `x86_64-pc-windows-msvc`, CMake, VS2026 and MSBuild. VS2022/Ninja require separate implementation and validation; they are not already supported combinations.

The Windows environment preparation script queries current official VS2026/SDK releases and checks signed installer identities. Its default is read-only; explicit `-Install` requires an elevated process and uses a dedicated deps Build Tools instance plus a machine-wide SDK. Preparation, compilation and minimal native/reference capture passed on Actions with current VS2026/SDK; the matching local environment, native ownership/FFI/reference capture and configuration checks have also passed. The external P0 exit assessment combines these per-platform results. Installer/registry success alone is not compiler or capture acceptance.

Linux prototype preparation resolves the latest formal LLVM release into an isolated, integrity-checked deps installation; LLVM 23.1.4 passed native PipeWire/Wlr prototype checks, with libclang active only for the PipeWire generator. macOS preparation matches Apple's latest formal Xcode version/build to an installed bundle using process-local developer/SDK selection; Xcode 27.1/SDK 27.0 has passed the native P0 capture/ownership prototype. Local Windows and hosted results remain distinct evidence; the P0 exit assessment now includes both.

## Testing and Acceptance

- Put private unit tests in owning modules under `#[cfg(test)]`. Integration tests use public APIs and `tests/<name>/main.rs` or explicit `[[test]]` paths; `tests/contracts/main.rs` corresponds to `--test contracts`. Shared helpers go in `tests/common/mod.rs`, self-owned inputs in `tests/fixtures/`; do not expose internals for tests.
- Cross-language, parity, desktop and consumer entry points use their named test areas with explicit `xtask` orchestration. Add examples only when their APIs exist, and compile/test them through the relevant consumer path.
- Assert observable failures, pixels, lifetimes, resource counts and native boundaries. Use behavioral test names such as `rejects_invalid_stride`; no coverage threshold exists.
- Validate locally on the host OS and on native Actions runners for other platforms, never WSL for Linux acceptance. Desktop tests are explicitly enabled and share desktop/window state serially.
- Report build, load, actual capture, missing environment, skipped cases and assertion failures separately. Zero tests, missing desktop scenarios, generated bindings and API success alone are not acceptance. Do not retry away failures.

## Documentation and External Records

Use `docs/plans/` for scope/order/gates, `docs/decisions/` for rationale/history, `docs/design/` for internals, `docs/contracts/` for caller guarantees, and `docs/guides/` for operations. Distinguish proposed, confirmed, implemented, verified and superseded states. Use relative repository links; update navigation, this file, plan/decision references and anchors together when moving documents.

Preserve R1–R8 identifiers. New independent ADRs use four-digit numbers and explicit supersession links. Create deeper backend or language-guide directories only with actual content; do not create empty SDK examples, test targets or build scaffolds merely because they appear in a plan.

Keep caches in sibling `maafw-screencap-deps/`. Keep baselines, patches, mappings, source/license audits, reports, per-build inputs, history backups and local consumer/environment paths in sibling `maafw-screencap-reference/`, using plan §6.2's baselines/runs/local separation. Do not copy reference material into the product or add ignore rules for it. Schemas and validation code enter `xtask` with the actual format; generated records remain external. Product builds and consumption must work with reference moved away.

## Commits, History and Remote Operations

Review this file before every commit and update it when conventions change. Follow Conventional Commits; use `!` or `BREAKING CHANGE:` for breaking changes. Keep formatting, source migration, behavior changes and performance changes reviewable. PRs state phase, behavior, linked issues, validation and platform limits.

Rewrite history only when explicitly requested. Inspect local/remote branches, tags and worktrees first; preserve a verified bundle or recoverable ref plus uncommitted/untracked work outside the product before changing refs. Verify the rewritten tip's full tree against the intended files. Never overwrite unrelated changes or publish backup refs merely to preserve them.

A local history rewrite does not authorize a forced push, remote deletion or repository recreation. When those operations are explicitly authorized, first archive Git refs and relevant repository metadata, inspect issues/PRs/releases/settings and unrecoverable resources, then act on the exact owner/repository. For recreation, preserve agreed visibility and applicable settings, verify the repository identity changed, and publish only intended branches/tags. Credentials and secret values are not recoverable from metadata exports; report any required reconfiguration accurately.

Preserve LGPL-3.0-or-later, upstream statements and required third-party notices/source materials. Python starts with ctypes/C ABI and later migrates to PyO3/maturin. First releases use GitHub Release only. Before first releases/bindings, prereleases or new channels, remind the user to configure actual secrets/release settings per plan §15.6.

Push branches and release tags separately with `--no-follow-tags`, following release authorization and required CI. Do not combine branch/tag pushes or use bulk tag/ref publishing.
