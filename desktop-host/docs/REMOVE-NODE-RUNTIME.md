# Runtime migration: stage 1

This branch does **not** remove the product's Node.js runtime yet. It establishes
the current dependency baseline and extracts the existing HTTP adapter without
changing the bridge or frontend. Do not ship a payload without Node at this stage.

Base: `f388bcdf0ac3833a2c1c926f1895c3ebbf01620b`. The new branch is
`refactor/remove-node-runtime`; `main` is unchanged. No `.agents/skills` directory
exists in this checkout; the root `AGENTS.md` applies.

## Measured boundary

Run `node desktop-host/scripts/inventory-node-runtime.mjs` from the repository.
The source-only result is checked in as `desktop-host/evidence/node-runtime-baseline.json`.
It finds 45 modules / 250,075 source bytes, 11 Node builtins, three direct external
packages (`ical.js`, `luxon`, `linkedom`), and six native Node launch sites.
Worker URL edges are included. Computed imports, package internals and generated
browser bundles are excluded. API literals include prefixes and retired API
guards; their presence does not authorize reactivating a retired capability.
These numbers are not installed size, startup time or process-memory measurements.
Those require a built candidate and an isolated synthetic run before release.

Native launch sites cover the long-running local HTTP service, Agent/appearance
CLI, gated JSON watcher, tray quick actions, library CLI and bundled-runtime
self-check. Node subprocesses/workers also implement library jobs and calendar
parsing. Deleting `node.exe` alone disables these capabilities.

## Implemented

`http-transport.mjs` now owns the HTTP/Fetch conversion previously embedded in
`windows-runtime.mjs`. Production still binds to `127.0.0.1` and uses the existing
bridge. This small boundary can be exercised independently during a native port.
The same body ceilings, raw path forwarding, request timeouts, headers, status,
binary responses and error envelope are retained.

The new transport fixtures exercise a real loopback socket. A real bridge fixture
also checks denied sessions/origins, wallpaper write denial, successful manager
writes, durable request replay and stale revision rejection, with no renderer ACKs.
Mock bridge fixtures check wire conversion separately from business semantics.

## Remaining migration work

| Capability | Existing implementation | Required equivalence before switching |
| --- | --- | --- |
| HTTP, static assets and manager/wallpaper pages | `windows-runtime.mjs`, `router.mjs`, `host-router.mjs` | Preserve paths, security headers, nonce/CSP injection, Host/Origin/fetch metadata checks, distinct role tokens, body bounds and raw-path validation. Keep frontend unchanged. |
| Settings, preview/apply/rollback and backups | `portable-core.mjs`, `portable-store.mjs`, `implementation-p1/src/store.mjs` | Match JS canonical JSON/hash bytes and schema migrations; preserve CAS, locks, request fingerprints/receipts, crash recovery and old backups. No parallel independent writers. |
| Agent CLI and JSON watcher | `agent-cli.mjs`, `agent-watch.mjs`, `agent.rs` | Preserve CLI JSON/JSONL schemas, exit codes, input path exclusion, exact snapshot bytes, UTF-8 checks, settling, bounded busy retries, conflict handling and parent-death lifecycle. |
| Multimonitor scope and renderer observations | `scope-registry.mjs`, `host-router.mjs`, native `multi.rs` | Keep shared/independent store ownership, inactive target rejection and projection/session-bound ACK checks. Renderer ACK, native attachment and actual visibility remain distinct claims. |
| Presets, schedules, focus and tray | `presets-core.mjs`, `schedule-core.mjs`, `focus-core.mjs`, `quick-actions.mjs`, `app.rs` | Preserve revisions, durable replay, schedule selection/ticks, target selection and session-bound focus. |
| RSS, weather, system data and calendar | `daily-feeds.mjs`, `providers-service.mjs`, `calendar-*.mjs` | Retain timeout/size bounds, SSRF/DNS/private-address/redirect protections, opt-in behavior, timezone/recurrence parsing, worker resource limits and existing calendar APIs/data. |
| Library, assets, exports and archive | `library-*.mjs`, `export-renderer.mjs`, `library.rs` | Keep content-addressed assets, link/path protections, library move/archive intents, receipts, protected references, cancellation/error handling and recoverable backups. |
| Native controls | Rust display/audio/layout/performance/startup modules and their JS services | Preserve native validation, CAS/receipts, telemetry freshness and request/response contracts when eliminating JS wrappers. |
| Payload and installation | `prepare-payload.mjs`, `installed.rs`, `options.rs`, `runtime.rs`, runtime lock/notices | Remove Node downloads/binary/npm production modules and all launch paths only after equivalent services pass. Update integrity verification and runtime compatibility checks together. |

Keep embedded Python/Pillow for image normalization: its exact normalized PNG
bytes determine asset IDs and backup verification. No codec replacement has been
proven equivalent. Build-time Node/npm for assets, tests and installer orchestration
can remain; they are separate from an installed product dependency.

## Release gate

Run the unchanged full `implementation-p4_2_5` `npm run verify`, including the
injected-overlap negative control and all required Chromium matrix cases. A missing
measurement or browser failure remains a failure. Run Windows Rust tests and
native isolated CLI/host/upgrade tests against the actual candidate. Compare old
fixtures, backups, request receipts, asset hashes, schedule/data results and role
denials across implementations. Measure startup/process memory/package size and
confirm no Node process is launched by any product entry point. Build installers
only with `desktop-host/scripts/build-installer.mjs` after these checks pass.

No installer was built, installed, deployed, merged or pushed by stage 1. Product
runtime replacement, compatibility validation and installed footprint measurement
remain unfinished; this branch is preparatory work, not release-ready Node removal.

Stage 1 verification: the unchanged full frontend gate passed, including all 12
viewports and its negative control. The five added loopback transport tests passed.
Windows Rust tests passed 52/52 with desktop metadata access. The unit-test build
used `TAURI_CONFIG={"bundle":{"resources":[]}}` because this source checkout has no
generated installer payload; this does not establish packaging readiness. Earlier
failed attempts and unexecuted checks are recorded, without raw logs or personal
data, in `desktop-host/evidence/node-runtime-stage1-validation.json`.
