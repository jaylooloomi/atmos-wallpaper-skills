# Runtime migration

This branch does **not** remove the product's Node.js runtime yet. Stage 1 establishes
the dependency baseline and extracts the HTTP adapter. Stage 2 migrates the static
CLI catalogs and global provider/focus policy entry points to native Rust. Do not
ship a payload without Node while the remaining services still depend on it.

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

## Capability inventory and parity requirements

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

## Stage 2: native policy entry points

`--agent-help`, `--agent-schema`, `--host-schema`, `--appearance-help`,
`--providers-command get|update`, and `--focus-command get|update` now execute
without Node or a payload. The tray's normal/focus/showcase actions also write
native focus policy; next-preset still uses Node. Static catalogs are generated
from the existing contract modules at build time. Installer builds reject stale
catalogs before running the unchanged Chromium packaging gate.

The native policy store uses the existing owner `.lock`, not an unrelated named
mutex. Provider settings, previous settings and request receipt names match the
legacy JS service. The focus file, session reset, global scope, request fingerprints
and receipts also remain compatible. Atomic writes use unique exclusive temporary
files, file sync and Windows replace/write-through; path components reject reparse
points. Dead-owner recovery marks the exact opened stale lock for deletion rather
than unlinking a potentially replaced pathname.

The policy fingerprint implementation is intentionally limited to validated ASCII
keys/IDs/enums, fixed city names and normalized safe integers. It must not be used
for arbitrary desired-state JSON: JS floating-point formatting, UTF-16 key ordering
and lone-surrogate strings need a separate compatibility implementation. CLI policy
commands do not initialize, migrate or rewrite appearance state as a side effect;
the existing appearance service remains responsible until that domain is ported.

Differential checks invoke the source candidate with no adjacent payload, empty
PATH and poisoned Node environment variables. They compare JS/native reads, mutual
receipt replay, numeric spellings, CAS/conflicts, pending recovery, live-owner and
dead-owner locks, junction denial, concurrent writers, host-session freshness/reset
and unchanged appearance/backups. Synthetic host-status records support focus-policy
tests only; no renderer or OS visibility acknowledgement is fabricated.

## Stage 3: native appearance Agent and watcher

`--get-state`, `--validate-json`, `--apply-json`, `--get-result`, and `--watch-json`
now use Rust, including current and legacy desired validation, byte-preserved
migration backups, preview/apply fingerprints, revision watermarks, retained
history/receipts, configured asset-library resolution and independent target
seeding/ownership. Mixed JS/native writer tests establish interoperability while
the HTTP backend continues to use the original JS store implementation.

The JSON AST preserves UTF-16 code units, including lone surrogates. Canonical
keys sort as JS keys do; numbers use the pinned [ryu-js implementation](https://github.com/boa-dev/ryu-js)
of ECMAScript number formatting. Tests compare 4,096 deterministic floating-point
values and Unicode boundaries against the original `canonical()` function. A
further 335 source-derived fixtures cover all four desired formats and their
normalization. The existing [time crate](https://docs.rs/time/0.3.55/time/struct.OffsetDateTime.html)
parses the RFC3339 timestamps produced by the application. Native JSON parsing
uses a depth bound of 256; supported desired/receipt envelopes are much shallower.

The watcher reads one exact external file, checks opened-file identity/size/write
timestamps before/after reading and against the current path, settles unchanged
bytes, and submits those same bytes. It keeps the initial target mapping, retries
busy requests at most three times with backoff, deduplicates errors/results and
supports finite sessions and console cancellation. Its former Node subprocess
and parent/child gate are removed: the watcher runs in the CLI process itself.

Validation at this checkpoint: 63 Windows native tests pass; 20 native/legacy
differential tests cover policy, Agent, watcher, asset-library and concurrency
contracts. Candidate CLI checks use no bundled payload, empty PATH and invalid
Node environment settings. The full frontend gate remains the stage 1 result;
frontend bytes and its verification limits are unchanged. The backend HTTP service,
preset/schedule automation, next-preset tray action and library jobs still require
Node and remain the next migration domains. No Node-free installer is claimed.
