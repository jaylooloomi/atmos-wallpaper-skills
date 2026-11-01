# P4.2.3 — actual bottom edge and toolbar refinement

The user's screenshot image(20261101-174240).png showed the right panel about9px below the explanation card (including shadow about11px). Independent frozen static P4.2.2 reproduction at a true1672px iframe measured explanation bottom1282.15625 and side bottom1292.15625: +10px. Prior1180px zero-error QA did not invalidate this larger-viewport defect.

Old code used the entire left section height and assumed matching tops. New code measures the requested `.preview-explanation` bottom minus the actual `.side` top, accounts for content-box additions if present, keeps fractional CSS pixels, and sets only the panel height. It observes the left column, target card and toolbar, plus resize and completed font loads. It does not change target resolution, contain aspect ratio or stored positions. Under1000px, single-column natural height remains.

Updated user request: resolution select / width / height / Apply stay on one horizontal row. The single existing language control is in a separate panel to the right, outside the resolution panel. At insufficient width the toolbar alone scrolls horizontally; the page and preview must not overflow. Controls retain labels and keyboard access.

Accordion focus now highlights the full header row, including title/chevron/independent switch. Focus-within background/inset border and keyboard focus-visible outline replace the old isolated title-button focus block. The switch still toggles visibility only and does not open properties. No blanket removal of keyboard indication.

Five additional pure tests cover the reproduced geometry, different tops, border/content-box behavior and fractions. True browser verification across1876/1672/1180/390 and the static adapter remains separate. No Windows access or OS wallpaper changes in this phase.

Additional user screenshot image(20261101-175221).png explicitly targeted the bottom Scene and external-AI Prompt cards: shared top but17px bottom difference. Their existing two-column grid now stretches both items to the row height; on the single-column mobile layout each card naturally follows its own row. This is separate from the main control-panel/explanation-card alignment.
