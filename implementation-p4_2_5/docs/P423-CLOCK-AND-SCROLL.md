# P4.2.3 added clock format and component-only scrolling

## Clock contract
Current inner desired format4 adds `visual.hourCycle` (number12 or24) and `visual.showSeconds` (boolean). Defaults24 andtrue. Unsupported types/ranges reject. The outer transaction schema remains1. Frozen format2 andformat3 validators authenticate older live/backup/history values before adding defaults; original bytes/history remain recoverable and all existing settings are preserved. Current-format requests cannot omit the new fields. No OS/timezone configuration changes.

`clockPresentation` uses local Date hours/minutes/seconds, with12-hour midnight/noon displayed12 and24-hour midnight00. Locale-specific dayPeriod comes from Intl.formatToParts with no timeZone override. Four combinations displayHH:mm:ss orHH:mm and optional localized AM/PM. The clock outer frame/anchor is independent of this choice. Five-digit text is centered inside the same frame; eight-digit text keeps fixed slots; the selected mono font is retained. A small absolute dayPeriod label occupies reserved time-frame space without moving the date. Every tick uses current Date; it never writes desired/revision. Visibility/pageshow recalculate normally.

## Latest scrolling instruction supersedes older panel-scroll QA
The entire control panel must not own a scrollbar. Original Hide all / Arrange buttons are moved, not recreated, into the panel footer with an upper divider. They retain their event handlers and draft/save semantics.

All seven component headers stay available above the active property region. A constrained panel uses two columns of compact headers; a sufficiently tall panel uses one column. This is a necessary layout accommodation to retain all headers, footer and the target-bottom alignment without hiding off-screen rows. It is not a new tab mode or a second management UI. The same single activeId and independent switch actions remain.

Only the active `.inspect-props` owns overflow:auto and fills the remaining grid region above the footer. Calendar connections and each component's operations stay within this region. On mobile, the panel uses natural page flow, while long active properties are bounded to55vh and scroll internally. True QA must verify all header/footer bboxes and keyboard reachability, not merely inspect CSS overflow values.

Separate evidence records previous layout/focus batch and this later time/scroll batch. No Windows or actual host lifecycle test is implied.

First real scroll QA exposed a historical `.component-section{flex:none!important}` rule overriding the new flex growth: component section11px, manager/inspector0px, active props26px with415px unused before the footer. The final override explicitly replaces that legacy rule on desktop while retaining natural mobile flow. This regression and retest are recorded, not hidden by CSS-only claims.
