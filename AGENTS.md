# Repository Guidelines

## Current State & Reading Order

The repository currently contains a Rust 2024 Hello World binary in `src/main.rs`, nightly configuration, and design documents. The capture library, backends, `xtask`, bindings, tests, examples, and CI remain planned. Do not describe planned paths or confirmed decisions as implemented or validated support.

Read `docs/README.md`, `docs/plans/maafw-screencap-extraction-plan.md`, and `docs/decisions/interactive-technology-selection.md`. Detailed architecture and execution rules live in `docs/design/`; proposed Frame, lifecycle, and C ABI contracts live in `docs/contracts/`. Current commands and planned build/test responsibilities are distinguished in `docs/guides/development.md`.

## Phase & Implementation Boundaries

Follow P0 preparation → P1 complete unpatched upstream migration U → P2 fixed PR #1587 patch P → P3 SDK/correctness → P4/P5 strategies/bindings/assets → P6 measured optimization. All platform backends and screenshot helper/manager paths must pass P1 before P2 starts; final SDK types, ABI signatures, state encoding, strict geometry, shared leases, and extended scheduling are designed after P2 acceptance.

Preserve upstream defaults, selection, pixel conversion timing, waiting/replay, native lifetimes, window adjustments, observation, and shutdown in U/P. U/P parity combines Rust capture with explicitly built C++/OpenCV adapters; keep conversions at their original point when timing or ownership requires it. E evolves the same implementation with independently justified behavior changes. Keep reconstructable U/P snapshots externally, rather than maintaining duplicate legacy/product backends or U/P/E semantic features.

## Architecture & Project Structure

Use one capture crate plus an independent `xtask`. Gate modules, runtime dependencies, native discovery, generation, and linking by target and selected backend. Runtime dependencies belong in target tables; `build.rs` reads `TARGET`/`CARGO_CFG_TARGET_OS`, because build-script cfg describes the host.

Keep Frame/target/error models independent of session orchestration and C ABI. Session owns mutable lifecycle, operation, cache, and publication state; policy returns decisions, and backends return samples/native evidence without controlling Session or cache. Keep Windows coordination, native declarations, and resource wrappers in platform modules; use `src/c_api/` for outward C exports. Maintain generator inputs in `build-support/` when needed, with generated native/Rust declarations in `OUT_DIR` and handwritten safety wrappers separate.

Native buffer leases stay backend-private. Within their valid lifetime and required execution channel, validate layout, isolate the target, and copy or acquire independent CPU storage. The internal BackendSample owns that storage; final publication transfers or immutably shares it without a default second full-frame copy. Frame storage and quota accounting outlive Session/Runtime handles. Views retain the actual Frame owner; writable outputs use isolated copies.

Record each backend's create/access/destroy thread, callback work, waits, unregister completion evidence, and late-result owner. Serial Session use does not establish thread mobility; prove Send/Sync independently. Standard synchronization and required native loops come first; the base library requires no general async runtime.

## Build & Dependency Workflow

Run commands from the repository root. Update nightly before overriding `CARGO_HOME`; if already overridden, use `rustup update nightly --no-self-update`. `rust-toolchain.toml` selects nightly, rustfmt, and Clippy.

```powershell
$env:CARGO_HOME = "$PWD/../maafw-screencap-deps/cargo-home"
$env:CARGO_TARGET_DIR = "$PWD/../maafw-screencap-deps/target"
```

- `cargo build --release`: compile the current scaffold.
- `cargo run`: run Hello World until the scaffold is replaced.
- `cargo test --workspace`: run the built-in tests; currently zero.
- `cargo fmt --all -- --check`: check Rust formatting.
- `cargo clippy --workspace --all-targets -- -D warnings`: reject warnings.

Future xtask preparation must run without directly or indirectly depending on the capture crate. It resolves latest dependencies and backend AUTO/ON/OFF before invoking Cargo, producing explicit per-build configuration. `build.rs` verifies selected configuration, generates/builds only necessary native pieces in `OUT_DIR`, and declares input/environment invalidation; it does not download OpenCV or generate external acceptance reports. Direct Cargo consumers select features and documented system dependencies without requiring reference data or xtask.

Features add compiled capabilities; runtime options select methods/policies. Do not model AUTO/ON/OFF or mutually exclusive U/P/E behavior as Cargo features. Keep OpenCV outside the core crate dependency tree; optional Rust image adapters belong in binding-side packages or consumer source. Default product builds and assets exclude parity adapters and reference inputs.

Use latest dependency releases and check new major versions; cache reuse never skips update checks. Archive each build's resolutions, Cargo.lock, toolchain identity, and required offline inputs externally; do not commit a long-term lockfile or silently fall back to an older dependency. Offline replay uses the archived batch, while the next online batch checks updates anew.

Initial Windows native validation uses `x86_64-pc-windows-msvc`, CMake, VS2026, and MSBuild. VS2022/Ninja are deferred and need separate implementation/validation.

## Coding, ABI & Safety

Use rustfmt's four spaces, snake_case functions/modules, UpperCamelCase types, and SCREAMING_SNAKE_CASE constants. Keep unsafe local and document pointer validity, ownership transfer, aliasing, thread affinity, and release invariants. Use checked size/stride/offset arithmetic in every profile.

Keep nightly standard-library fallible allocation APIs behind internal owner helpers, retaining standard Arc/Weak; do not add a stable fallback or reimplement general reference counting. Stable addresses do not replace unregistering and draining callbacks. Preserve primary and cleanup errors, panic/exception boundaries, and an allocation-independent minimal error/wakeup path.

Public C headers are the ABI authority. Generate or automatically verify Rust/ctypes declarations, constants, capability values, and export lists from the same contract; maintain inputs, not divergent handwritten raw definitions. Safe Rust APIs use the shared implementation directly. Verify versioned structures, failure outputs, integer narrowing, and real native consumption; generation alone does not prove safety.

## Testing & Acceptance

Put private unit tests in their owning modules under `#[cfg(test)]`. Integration tests use public APIs and `tests/<name>/main.rs` or explicit `[[test]]` paths; `tests/contracts/main.rs` corresponds to `--test contracts`. Put shared helpers in `tests/common/mod.rs`, self-owned inputs in `tests/fixtures/`, and cross-language/parity/desktop/consumer entry points under their named test areas with explicit xtask orchestration. Do not expose internals just for integration tests.

Test failures, frame lifetimes, resource counts, and native boundaries with meaningful observable assertions; names such as `rejects_invalid_stride` state behavior. Add examples only when their API exists, and compile/test them through the relevant consumer path. No coverage threshold exists.

Validate locally on the host OS and use native Actions runners for other platforms, never WSL for Linux acceptance. Desktop tests are explicitly enabled and share desktop/window state serially. Report build, load, actual capture, missing environment, skipped cases, and assertion failures separately; zero tests or absent desktop scenarios are not acceptance. Do not retry away failures.

## Documentation & External Records

Use `docs/plans/` for scope/order/gates, `docs/decisions/` for rationale and ADR history, `docs/design/` for internals, `docs/contracts/` for caller guarantees, and `docs/guides/` for operational instructions. `docs/README.md` indexes actual files and their status. Each detailed rule has one authority; the plan and this guide link or summarize it. Distinguish proposed, confirmed, implemented, verified, and superseded states.

Use relative repository links; when moving files, update navigation, AGENTS.md, plan/decision references, and anchors together. Create deeper backend or language-guide directories only with actual content. Preserve R1–R8 identifiers; future independent ADRs use four-digit numbers and explicit supersession links.

Keep caches in sibling `maafw-screencap-deps/` and baselines, patches, mappings, source/license audits, reports, per-build inputs, and local consumer/environment paths in sibling `maafw-screencap-reference/`. Do not copy reference material into the product or add ignore rules for it. Follow plan §6.2 for baselines/runs/local separation; schemas and validation code enter xtask with the actual format, while generated records stay external. Product builds/consumption must work with reference moved away.

## Commits, History & Release

Review this AGENTS.md before each commit and update it when conventions change. Follow Conventional Commits, e.g. `feat(windows): add GDI capture`; declare breaking changes with `!` or `BREAKING CHANGE:`. Keep formatting, source migration, behavior changes, and performance changes reviewable. PRs describe phase, behavior, linked issues, validation, and platform limitations.

Rewrite history only when explicitly requested; preserve a recoverable ref or external bundle and inspect remote/tag relationships first. Rewriting local commits does not itself authorize publishing a forced update. Never overwrite unrelated user changes.

Preserve LGPL-3.0-or-later, upstream statements, and required third-party notices/source materials. Python starts with ctypes/C ABI and later migrates to PyO3/maturin. First releases use GitHub Release only. Before first releases/bindings, prereleases, or new channels, remind the user to configure actual repository secrets/release settings per plan §15.6.

Push branches and release tags separately with `--no-follow-tags`, following release authorization and required CI. Do not combine branch/tag pushes or use bulk tag/ref publishing.
