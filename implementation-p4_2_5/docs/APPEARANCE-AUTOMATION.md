# Appearance presets, automatic changes and presentation modes

Implemented surfaces: manager controls, native tray shortcuts, strict local Agent CLI and bounded same-origin HTTP APIs. The installed application bundles the required runtime; users do not need to install Node separately. These features extend the existing core and do not launch third-party wallpaper software.

## Named appearance presets

A preset saves the scene/custom-image ID, weather effect, clock/calendar appearance and positions, card appearance/positions and desktop placement zone. It deliberately excludes todo content, calendar data/subscription credentials, timer progress, city/provider state, language, preview dimensions and host/device policies. Applying a preset merges only the saved appearance into the current desired state. Imported assets remain referenced by ID; a missing/offline custom image fails the application instead of substituting a different image.

The manager can save the currently committed appearance, apply a saved entry or remove its catalog entry. It requires existing page drafts to be applied/cancelled first. Deleting a catalog entry does not delete source images. A maximum of 40 presets is retained per appearance scope; name text is bounded and displayed with textContent. The catalog has a separate revision. Save/delete do not increment the desired revision; apply uses the existing durable desired commit. Request IDs and CAS prevent silent replacement; an unknown HTTP result retains the original payload/ID for retry.

Native commands:

```
atmos-wallpaper.exe --appearance-help
atmos-wallpaper.exe --preset-command get
atmos-wallpaper.exe --preset-command save
atmos-wallpaper.exe --preset-command apply
atmos-wallpaper.exe --preset-command delete
```

Mutations read JSON on stdin (16 KiB maximum). Each also supports `--store ABSOLUTE_LOCAL_DIRECTORY`; independent display mode additionally requires `--target display-<exact 64 hex ID>`. Shared mode uses the shared catalog. Read the current scope and catalog before creating an intent.

Save example:

```json
{"schemaVersion":1,"action":"save","requestId":"preset.work.001","expectedRevision":4,"expectedCatalogRevision":0,"name":"Work"}
```

Apply/delete replace `name` with the exact listed `presetId` and set `action` accordingly. Retry identical request IDs with identical payloads only. Reconcile `REVISION_CONFLICT`, `PRESET_CATALOG_CONFLICT` and `REQUEST_CONFLICT`; never silently rewrite a revision. A saved or committed result is not a renderer/OS visibility acknowledgement.

## Playlists and fixed local times

`--schedule-command get|update` uses a separate schedule revision and the same scope selection rules as presets. Updates read the following complete policy on stdin:

```json
{
  "schemaVersion": 1,
  "requestId": "schedule.work.001",
  "expectedRevision": 0,
  "policy": {
    "enabled": true,
    "mode": "interval",
    "intervalMinutes": 30,
    "presetIds": ["<exact listed preset ID>"],
    "entries": [],
    "manualGraceMinutes": 30
  }
}
```

`interval` uses the selected preset order and wall-clock intervals; slots are aligned to the clock, not to the time the settings panel was opened. `times` uses an empty `presetIds` and entries such as `{ "time": "08:00", "presetId": "..." }`. Times are Windows system-local, and the previous day's last slot remains active before the first slot of the next day. Duplicate times are rejected. No network time service, shell commands or Windows Task Scheduler jobs are created.

The host checks eligible scopes periodically. The default policy is disabled. Screensaver rendering does not start wallpaper scheduling. Each event has a deterministic desired-store receipt, so duplicate timer notifications and restart recovery cannot repeatedly apply the same event. Manual desired changes trigger a grace period before a new event can apply; an already-applied event never reapplies merely to undo a manual change. A manager's unsubmitted draft remains protected by the existing incoming-state conflict workflow.

A missing/deleted preset, unavailable custom image, busy transaction lock or corrupt policy cannot produce a successful commit. The status records errors, and a future tick can recover when the dependency returns. This is not an always-running Windows service: no wallpaper changes are scheduled after Atmos exits.

## Focus and showcase

Focus retains the clock/timer while hiding the calendar and other information cards. Showcase hides the clock, calendar and cards while retaining the background/effects. Normal restores the underlying presentation. This changes visibility only and never deletes/rewrites personal desired data.

The manager and native tray control the same owner-scoped runtime policy across wallpaper views. `--focus-command get|update` operates only when a fresh, living native host session can be identified. Update JSON is:

```json
{"schemaVersion":1,"requestId":"focus.001","expectedRevision":0,"mode":"focus"}
```

Modes are `normal`, `focus`, `showcase`. The policy is session-bound and resets to normal after a host restart. It is a presentation feature, not a data-access boundary: hidden data remains in the local store. Screensaver presentation has a separate runtime file/session and defaults to showcase; it cannot clear wallpaper privacy state.

The tray's next-preset shortcut changes the shared preset in shared layouts. In independent mode it requires the actual primary monitor to be selected and uses that exact scope. It never substitutes another display when the primary is not selected. Native quick actions execute only a fixed bundled helper with known store/scope/session arguments; no web-supplied shell is evaluated.

## HTTP and Agent access

`GET/POST /api/presets`, `GET/POST /api/schedule` and `GET/POST /api/host/focus` use the existing loopback session/role/Origin checks. Preset and schedule endpoints are manager-only. Renderers may read focus state, but cannot write it. HTTP accepts no arbitrary file paths, programs or shell expressions. Local agents should use the native CLI without obtaining browser session tokens. `--agent-help` links the new command families and `--appearance-help` supplies payloads/rules.

## Rendering and layout

The prompt and optional diagnostics now follow the left preview's natural content height with a 12px prompt gap. They are no longer after the full two-column row, so a taller right panel cannot push them down. The preview's target aspect ratio and the right panel's top alignment remain unchanged.

The storm renderer replaces the frozen baseline's fixed-period global flash at build time. Distant cloud glow, seeded jagged branches and uneven return strokes use a common wall-clock event model; no private/frozen source is edited. Glow and line opacity are bounded, all weather remains behind widgets, and the lightning layer suppresses drawing for reduced motion, paused/hidden documents and nonstorm weather. This visual event is separate from native synthesized audio and does not claim physically synchronized thunder.

## Automatic validation

- `node --test tests/presets.test.mjs tests/schedule.test.mjs tests/focus.test.mjs tests/appearance-cli.test.mjs tests/lightning.test.mjs`
- `node scripts/audit-presets.mjs` runs an isolated synthetic Chromium CLI fixture: real manager save/apply flow, private task preservation, interval/fixed-time controls, focus/showcase visibility, prompt placement and actual canvas pixels/reduced-motion suppression.
- `node scripts/audit-layout.mjs --quick --native` checks 390px and 1280px synthetic views for layout/overflow regressions.

These checks do not replace human assessment of lightning aesthetics, desktop visibility, lock/unlock, hardware plug/unplug or power consumption. No user browser/desktop UI automation is used.
