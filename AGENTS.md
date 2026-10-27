# Repository Guidelines

## Actual State and Reading Order

P0 is in progress. The product entry point is still a Rust 2024 Hello World binary in `src/main.rs`. An independent `xtask` prepares/verifies external fixed-source records, archives validation batches and checks project conventions. P0 tests cover fallible allocation, controlled-window GDI, COM accounting, WARP D3D/DXGI/WinRT interop/readback, FramePool empty results and controlled-window capture/events on an owned MTA/dispatcher, plus private C++/OpenCV callback/FFI boundaries. Hardware and WARP WGC cases are explicit, each with 20 lifecycle cycles; results remain platform-specific. Desktop Duplication and the full FramePool failure/resize/permission matrix remain unvalidated. `xtask native` prepares latest native dependencies and builds an independent unpatched GDI reference; explicit offline dependency snapshots are supported. Linux PipeWire native validation covers an owned C producer with padded RGB rows, twenty stream cycles of 32 frames each, native buffer returns/reuse, callback ownership, independent CPU pixels and graceful producer/daemon shutdown. The earlier SPA fixture teardown failure is retained externally. Portal desktop capture, DMA-BUF and Wlr remain unvalidated. The macOS arm64 objc2/ScreenCaptureKit harness passed twenty controlled-window captures, checked BGRA copy/lock/unlock, callback/input-owner release and one abandoned in-flight request on native Actions; AppKit remains on the process main thread. Full display/format/failure and original-reference parity gates remain outstanding. A native Actions matrix is defined. The capture library, migrated backends, bindings and examples are not implemented, and P0 has not passed its full gate. A prototype or scaffold build is not validated product capture support.

Read these authorities before changing the corresponding area:

| Authority | Responsibility |
| --- | --- |
| [Documentation index](docs/README.md) | Actual documents, their status, and maintenance rules |
| [Extraction plan](docs/plans/maafw-screencap-extraction-plan.md) | Scope, phase order, acceptance gates, dependency and release policies |
| [Technology decisions](docs/decisions/interactive-technology-selection.md) | R1–R8 rationale and links to subsequent numbered ADRs |
| [Architecture](docs/design/architecture.md) and [execution model](docs/design/execution-model.md) | State ownership, platform execution channels, callbacks, window coordination |
| [Frame](docs/contracts/frame.md), [lifecycle](docs/contracts/lifecycle.md), and [C ABI](docs/contracts/c-abi.md) | Proposed caller guarantees; concrete SDK contracts are finalized in P3 |
| [Development guide](docs/guides/development.md) | Commands available now and build/test responsibilities to implement later |

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
