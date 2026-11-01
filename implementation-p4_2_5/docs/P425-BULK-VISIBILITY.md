# P4.2.5 — bulk visibility and duplicate separators

- Toolbar order: Position, Restore, Show all, Hide all, Arrange. Show all opens clock/calendar plus five optional cards; Hide all symmetrically hides all seven. It creates a normal draft; formal persistence requires Apply. Static preview remains page-local only.
- Show all changes visibility only. Renderer layout/clamp is a projection, not a write-back of saved position/zone/custom/font/data. Non-geometric renders (including demo data edits), bulk visibility, snapshot application, resize and geometry refresh preserve the original widget data after projection. Arrange remains the explicit position-changing operation.
- Removed nested `inspect-operations` and ID-specific legacy `widget-config` separators; outer inspector card and header/body boundary remain. No demo inputs or data functions removed.
- Five-language label and native keyboard button. Mobile toolbar may locally scroll; entire page must not overflow.
- Schema remains format4. No new command/API privileges, no Windows operation.
- Pure tests include the generated bulk operation in isolation and generated projection restoration wiring. Browser QA independently checks real draft, persistence and other-field equality; unit tests are not browser proof.
