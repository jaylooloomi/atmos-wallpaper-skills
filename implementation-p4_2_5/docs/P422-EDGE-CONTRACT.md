# P4.2.2 component edge alignment

Reference personally inspected: image(20261101-171957).png, differently sized Timer / Now Playing / Calendar. This change adds TOP-to-TOP and RIGHT-to-RIGHT to existing LEFT-to-LEFT; no new bottom or cross-edge modes.

Pure API: `snapComponentEdges(moving, targets, options)` in src/edge-snap.mjs. Rectangles are final screen CSS pixels after target-canvas scale; thresholds do not multiply by devicePixelRatio. Only explicit visible, positive finite target rectangles qualify; runtime additionally excludes off-screen, display:none, visibility:hidden and the moving element.

Each axis chooses at most one candidate. X chooses nearest left/left or right/right; Y chooses nearest top/top. Equal-distance X ties prefer left then stable target ID, then coordinate; independent input order. Two axes may target different cards. Snap distance <=6 CSS px; guide-only distance <=10 CSS px. Existing canvas-center snapping is suppressed only on an axis with active component snap. Boundary clamp remains final authority.

Guides `#componentEdgeGuide-x/y` expose data-edge and data-target for read-only QA. X draws vertical line, Y horizontal. Pointerup/cancel/blur clears both. Drag candidates always derive from pointerdown position + total pointer displacement, never previous snapped output, avoiding sticky small-step movement. Keyboard remains free positioning rather than new edge snapping. Draft/apply/CAS/state format are unchanged.

Legacy `snapLeftEdge` is retained and its ten tests remain; new runtime uses snapComponentEdges. Eleven new pure checks include different widths, independent axes, deterministic competition, thresholds/release, hidden/self/malformed targets and scaled floating-point coordinates (1e-10 tolerance for arithmetic, not relaxed browser geometry).

This is cloud-only; no Windows/Lively, hardware DPI or OS display observation is claimed. Browser mouse/contain/persistence QA is separate from pure tests.
