# Independent P4.1 legacy compatibility review

2026-11-01. Scope: removed tornado effect and fixed configuration migration. Reviewer wrote `tests/legacy-effect.test.mjs`; did not alter application source or frozen P1/P4. Fixtures are synthetic local temporary stores generated via frozen P1 API. No browser, server or Windows execution.

## Final verification

`node --test tests/legacy-effect.test.mjs`: **10/10 passed**.

1. New schema/preview reject tornado explicitly with EFFECT_REMOVED, live unchanged.
2. Legacy live creates a normal new revision, clears tornado, fixes playback flags, preserves intensity and unrelated theme, exact original live bytes archived.
3. Reopening is idempotent; persistent notice remains.
4. Invalid original live hash cannot be blessed by migration.
5. Rollback authentic old history uses clear fallback without overwriting original historical tornado state; same request replay does not add revision.
6. Restore authentic backup creates preview only; original backup bytes and caller object remain unchanged; apply uses clear.
7. Tampered backup hash rejected without changing live.
8. Tampered historical hash rejected before rollback fallback.
9. Corrupt migrated live can be explicitly recovered by a newly constructed core; original historical revision remains authentic tornado, target becomes clear and fixed flags.
10. Parseable legacy live with bad hash is not auto-migrated; init reports error, but explicit recovery remains reachable and uses the authenticated backup.

## Defects found and closed

- Recovery initially normalized the backup *before* using it as the commit base, relabelling historical revision 1 from tornado to clear. Added an exact historical assertion; observed 8 pass / 1 fail. Implementer now retains original backup as commit base and normalizes only the new desired target. Final pass confirmed.
- Constructor migration initially prevented explicit recovery of a parseable legacy live with invalid hash. It now skips unsafe automatic migration, lets normal init/status reject invalid live, and permits explicit recovery. Test 10 confirms.
- Migration notice originally used completed-apply language during restore preview. Implementer changed it to loaded/candidate wording, avoiding a false saved-state claim.

## Boundaries

These ten tests are not Windows host/browser QA and not a new run of all historical tests. No OS wallpaper installation or actual host visibility is established. Existing main-core fault/atomic tests remain separate; this suite specifically covers compatibility integrity and recovery entry points.
ff412ad356a2ed5ab8450089d6d962fc6bd8db0d817fb690176babdf32770950  wallpaper-product/implementation-p4_1/src/portable-core.mjs
c6dba1c940df2a389faa87a2edbe39705f275c8bfe277ea617292575dde6c160  wallpaper-product/implementation-p4_1/src/portable-store.mjs
56ecdbe1abf020550c53785d33076f4201163d5ad8e303db3a2e913b8a3423c8  wallpaper-product/implementation-p4_1/src/legacy-config.mjs
66749923ebfa250f6a1a0b20e9e153ae924143f73b04c5054c469f0f53fb1318  wallpaper-product/implementation-p4_1/src/state.mjs
