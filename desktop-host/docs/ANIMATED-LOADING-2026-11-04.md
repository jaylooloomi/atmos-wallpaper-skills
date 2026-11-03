# Animated loading — 2026-11-04

Version: Windows 0.3.9.

## Design and behavior

Original self-contained SVG/CSS composition: layered mountain silhouettes, drifting aurora, stars, glass Atmos emblem, mint orbit strokes and gold satellite, plus an indeterminate light track. No React, external fonts, network assets or new dependency is required before the application loads.

The bridge reports real connection, scene and layout stages. There is no fake percentage, minimum wait or artificial renderer acknowledgement. Existing stylesheet/font/image readiness and geometry gates remain in place; the loading overlay is removed before the renderer reports completion. Polling cannot overwrite a failure with a loading message. A successful resource recovery can resume loading. Failure pauses decorative animation, displays a readable error and retains reload. Reduced-motion preference disables animation. Short and narrow viewports retain the message and retry without horizontal overflow.

Inspiration, not imported source code:

- [Codrops Page Loading Effects](https://tympanus.net/codrops/2014/04/23/page-loading-effects/): SVG atmosphere and content reveal.
- [SVG Spinners](https://github.com/shubhamjain/svg-spinners): lightweight orbit rhythm.
- [MDN prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/At-rules/@media/prefers-reduced-motion): motion preference handling.

## Verification

- Product verification: 228 Node tests pass, 2 skip; core: 41 pass, 2 skip. Draft/path reviews and CLI-help checks pass. Loader unit suite: 10 pass, included in product count.
- Isolated Chromium CLI loading fixtures: desktop 1280×800, phone 390×844, compact 320×160, reduced motion, and error all pass. Error content remains visible even when failure happens during entrance animation. Motion checks seek actual CSS keyframes; they are not wall-clock FPS or GPU measurements.
- Real generated-entry boot fixtures: slow load, two reloads, service failure and stylesheet failure pass. See `implementation-p4_2_5/evidence/animated-loading-boot/report.json`.
- Design preview and fixture report: `implementation-p4_2_5/evidence/loading-animation/`. The GIF samples the design animation; it does not simulate actual progress. `preview.html` runs the continuous animation locally.
- Windows release build includes embedded runtimes; installer is local only, unsigned, 292411350 bytes, SHA-256 `ada635632b1ce6d7a38a9f8490a1801ff641e9b60d1378c9b2b0d3a82a6bb61d`.
- Installed upgrade results are recorded in `desktop-host/evidence/animated-loading-upgrade.json`: payload and embedded-runtime integrity, unchanged saved settings and startup preference, native WorkerW attachment, genuine wallpaper and manager acknowledgements, installed entry/styles matching the built source.

No Computer Use or user browser/desktop UI automation was used. No new human visual confirmation, reboot, sleep, GPU, power or long-duration measurement is claimed. Rust application behavior is unchanged; this release updates web assets and version metadata.
