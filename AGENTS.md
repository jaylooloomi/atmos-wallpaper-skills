# Validation rules

- For manager UI, CSS, localization, widget controls, layout or renderer preparation changes, run `npm run verify` in `implementation-p4_2_5`. It includes the required real Chromium layout gate. `verify:unit` is a diagnostic shortcut and does not establish layout/release readiness.
- Layout coverage requires all 12 viewport scenarios and all 28 distinct component/theme states per scenario, visiting desktop, screensaver and appearance tabs for every state. Also cover all three information-card templates (84 template visits per viewport), with GitHub, Yahoo and custom RSS fixture cards visible, and information-saving mode in all 28 states per viewport. Missing browser, timeout, incomplete measurements or geometry failures must fail; do not turn them into skipped/passing results or weaken limits to hide a regression.
- Preserve the injected-overlap negative control: the checker must reject deliberately broken fixture controls before accepting the positive matrix.
- Build Windows installers through `desktop-host/scripts/build-installer.mjs`, which runs the same gate before packaging. Reused frontend payloads must match the tested files/bytes.
- Use isolated synthetic Chromium CLI fixtures for automated UI checks. Do not operate the user's browser or desktop with UI automation. Do not fabricate live renderer acknowledgements.
- Keep installers, executables, private settings, backup contents, browser profiles and raw logs out of Git. Public evidence must be aggregate-only.
- Preserve `prototype-v8.1-frozen` and the original source-release provenance. Refresh `SOURCE-MANIFEST.json` from staged Git blobs when committing source changes.
