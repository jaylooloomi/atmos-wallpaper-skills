# P4.2 independent desired-contract review

2026-11-01. User-confirmed implementation resumed before this test run. Reviewer only added `tests/p42-contract.test.mjs` and this document. Main source, frozen versions, browser, server and Windows were not modified or launched.

## Result

`node --test tests/p42-contract.test.mjs`: **15/15 passed** on the local POSIX environment. No test assumptions were relaxed after execution. No new confirmed contract defect found in the reviewed version dispatch, migration, backup, restore, rollback or recovery paths.

## Coverage

- Outer store/backup schemaVersion remains 1; desired explicitly uses formatVersion 2. Missing, old explicit, future, null and string format values reject in the new validator.
- Defaults: locale zh-TW; typography follows theme with null size/color; calendar horizontal anchors 87 and 67.
- All five locales and six font choices validate; invalid enums and unknown fields reject.
- All eight clock/calendar desktop/mobile position coordinates accept 0 through 100, reject nonfinite, coerced and outside values. Renderer clamp remains separate browser scope.
- Visual text sizes accept null or 12–160; card text sizes null or 12–48. Text colors accept null or six-digit hexadecimal only, not CSS expressions, URLs or short hex.
- Explicit typography overrides survive theme-only commits. Resetting font/size/color to theme/null/null preserves theme and unrelated component overrides.
- Real synthetic P1 legacy store migration creates one new revision, keeps exact original live bytes in backup and authentic original desired/hash in history, preserves existing intensity and coordinates, and is idempotent on reopening.
- Invalid legacy hash cannot be normalized into an authenticated live state.
- New format2 data survives actual preview/apply, backup/restore, rollback and recovery. Restore leaves live unchanged until apply and does not mutate caller backup.
- Tampered backup and unsupported future format reject.

## Source review

`validateSupportedDesired` sends formatVersion 2 to the new validator, absent format to the frozen P1 validator, and rejects any other explicit format. Legacy original hashes are verified before normalization. Recovery retains the original validated backup as commit base and normalizes only the new target desired, preserving historical data. This avoids treating new-format data as legacy or rewriting old history.

## Boundaries

Fixtures are synthetic local temporary stores, not user data. These 15 cases are not browser/Windows/OS-host verification and are not a rerun of all historical suites. Font rendering, calendar anchoring and viewport clamp require official UI QA. System-monitor observations are intentionally outside desired and outside this review; no fixture result is presented as real monitoring.

## Reviewed source SHA-256
ee5df400a1efe9a32538110f1cec14d6cb5f6bfb2d78e13625835c8c83fa6bd4  wallpaper-product/implementation-p4_2/src/state.mjs
d56ce5d1ec33b53663c5a00e86626e7cfb0109ed4c0ab6b5fb7d280a6e751e43  wallpaper-product/implementation-p4_2/src/legacy-config.mjs
244bcdef66f96ab3214b82adb5ba2c89b503a9df2db13a9c4b6d24f092c21b1e  wallpaper-product/implementation-p4_2/src/portable-core.mjs
a8789d2feceff8a69fdef9e57f40a6eea0ba375af894b8e7a9a915bc3d2471d6  wallpaper-product/implementation-p4_2/src/portable-store.mjs
