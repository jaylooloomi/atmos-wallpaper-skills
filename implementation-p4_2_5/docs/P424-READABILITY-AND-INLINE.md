# P4.2.4 — readability, actual inline order and spacing

## User evidence and scope
Personally inspected180945 and181114: the selected preview-size success sentence was readable only under selection. The latest instruction removes that exact success feedback, not resolution controls or validation errors. The renderer no longer emits it; invalid-size feedback retains role=status and explicit foreground/background per skin. Four fixed color pairs are unit-checked against4.5 contrast; actual rendered themes/locales require independent QA.

Personally inspected181031: Todo expanded header was followed by Timer/System/Media headers before Todo properties. This was the former all-headers-first grid design, not a true inline accordion. That design is superseded. Each card is now a real flex column in unchanged DOM order: header then its own properties, then the next card. Keyboard Tab order follows the same sequence, with one active card only.

The active property region has a minimum280px readable height and its own overflow. Other headers remain visible and reachable; the outer panel does not scroll. If the target preview is short, measured minimum panel content height expands only the non-canvas explanation area, keeping panel/explanation bottoms aligned. The canvas is never stretched. This intentionally trades extra non-image vertical space for usable properties instead of accepting a74px viewport merely because Tab technically worked.

Latest181334 overrides the previous footer instruction: original buttons are moved to one toolbar in order Position, Restore, Hide all, Arrange. No duplicate IDs or empty footer remain. Original listeners/draft semantics are retained.

Personally inspected181424: preview→note blank≈14px while note→Scene≈25px. Found an empty legacy `.below-preview` wrapper still contributing margin-top10px after its real children were moved in earlier versions. Remove only when empty. Both new gaps share a15px CSS token. Scene/Prompt row stretch and main bottom alignment remain.

No state schema change (format4), no new feature scope, no OS access. Frozen P4.2.3 source remains intact. Pure tests and real visual/keyboard checks are reported separately.

Real mobile QA initially exposed a zero-height inspector: the new desktop flex:1 1 0 rule overrode the old mobile natural-height rule, so later Scene/Prompt content overlapped headers/properties. This was treated as blocking, not accepted because keyboard scrolling existed. The <=1000px parent chain now explicitly uses flex:none and height:auto; property-only scrolling and minimum280px are retained. Desktop geometry rules are unchanged. Retest evidence records the correction.
