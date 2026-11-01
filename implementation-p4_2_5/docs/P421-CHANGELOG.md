# P4.2.1 — approved-reference visual correction

Source forked from the frozen P4.2 release. This is a visual correction, not a claim that P4.2's functional QA proved screenshot fidelity.

## Latest reference contract
- Natural manager chrome: light sage, off-white cards, olive borders/switches. Other three skins retained.
- Atmos title; top four-style banner; target-monitor source and proportional full-canvas preview retained.
- Right panel is now 510 CSS px (the user's later 1.5× request supersedes the reference's original 340 px). Smaller screens stack without stretching the canvas.
- Seven component rows, actual SVG chevrons, single inline property expansion, independent visibility switch.
- Calendar provider/demo controls moved inside Calendar properties; no provider integration is claimed.
- Removed separate AI connection button/helper. Prompt generation remains below.
- Six actual local scene thumbnails, selected border, labels; no mockup image used as the UI.
- Manager diagnostic JSON is available in native disclosure, not the main visual composition.

## Clock and alignment
- Eight fixed time slots; mono uses eight equal slots because a monospaced colon has the same advance as a digit. The selected font is never silently changed. The glyph multiplier is rendering fit within the fixed time frame, not a stored font-size rewrite.
- Default clock scales to target logical width (0.095, capped at 180 px at size 100); frame width is 3.5× the computed clock base font size. Explicit fontSize and position values remain unchanged. Renderer bounds may clamp the displayed frame to the target canvas without writing state.
- Left-edge component snapping is separate from canvas-center snapping. Visible on-screen positive-area targets only, excluding the dragged component. Screen-distance threshold: snap at ≤6 px, guide at ≤10 px. Recomputed each move; no sticky lock. Existing bounds still apply.
- Dragging, keyboard changes, Apply/Cancel/CAS and persistence use existing shared state paths.

## Verification status
Pure tests are not screenshot comparison. Independent browser visual acceptance is tracked in qa-p4_2_1. This phase has no Windows-host execution, OS wallpaper change, new service credentials, or real calendar/weather/media API integration.
