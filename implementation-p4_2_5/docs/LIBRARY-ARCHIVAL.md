# Reference-aware library archival

The installed manager provides **Audit unused assets**, then **Archive unused assets**. These actions run in an isolated Node worker and use the same owner-store transaction lock as imports, target stores and library relocation. Nothing runs automatically.

The audit hashes the active library and retained JSON references under the owner and every `targets/display-*` profile, including offline/deselected targets. Renderer observations (including stale reports), current desired state, retained history, previous live state, presets, seed intents, previews, requests/receipts, exports, backups, commit intents and recovery JSON all protect referenced SHA256 assets. Invalid JSON, uncertain live/history integrity, binary corrupt-state recovery records or links/junctions block the audit. Conservative receipts may retain assets that are no longer visible; this is intentional.

The archive command must carry the exact audit hash and current library revision. Changes to the inventory or retained reference records invalidate the audit. Verified orphan PNG/JSON pairs are copied under the owner's `library-archive/<request-hash>` and hashed again before removing their active-library entries. There is no recursive deletion, permanent deletion or claim of disk savings. Archives remain local and are retained after relocation/uninstall along with the store.

A durable pending intent is saved before removals. Reads fail with `LIBRARY_RECOVERY_REQUIRED` during an interrupted archive; the next library status/move/archive operation takes the owner lock, verifies retained copies and restores the complete original pairs before continuing. Post-commit response loss is recovered as committed, rather than falsely rolled back. Do not edit internal journals or manually remove archive directories while Atmos runs.

CLI examples (local Windows app):

```
atmos-wallpaper.exe --library-command audit
atmos-wallpaper.exe --library-command archive
```

`archive` reads JSON stdin:

```json
{"schemaVersion":1,"requestId":"library.archive.001","expectedRevision":1,"auditHash":"<auditHash from audit>"}
```

Query `--library-command result` with `{"schemaVersion":1,"requestId":"library.archive.001"}` after an unknown outcome. Reuse a request ID only with identical inputs. The API is manager/same-origin/session restricted and accepts no filesystem paths for either action.

Automated coverage: current/history/backup/receipt/export/preview/offline-target reference protection, stale audit, invalid references, junction refusal, mid-pair failure restoration, actual abrupt worker death, post-commit response loss, replay, worker service and HTTP authorization. Human UX validation is deliberately not part of this round.
