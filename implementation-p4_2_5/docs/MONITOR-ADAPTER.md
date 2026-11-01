# P4.2 read-only monitor metadata

## Contract / integration
`readMonitorMetadata({enabled:false,targetId:null,fixture,runner,now})` is synchronous and **does not run on import**. Actual Windows probing requires `enabled:true` and actual `process.platform==='win32'`; never invoke a real Windows process on cloud. Non-Windows returns `WINDOWS_ADAPTER_UNAVAILABLE`. No store changes, monitor settings changes, installs, persistent credentials, network, or browser APIs.

Runtime integration should call once at authorized local startup (or an explicit bounded refresh), cache metadata, and return it through the existing read-only snapshot. Do not run the synchronous process every polling request. A startup measurement can become stale after resolution/topology changes: `capturedAt` is the observation time, not a guarantee of live topology. Metadata is separate from authoritative desired settings and must not create a desired revision.

Output schema 1:
- `source`: `windows-native-readonly`, `fixture`, or `unavailable`. Injected fixture/runner **always** says fixture; never treat fixture as actual host detection.
- `status`: detected / partial / unavailable; `reason` is a stable code.
- `target`: selected `{id,idKind,primary,width,height,scalePercent,resolutionUnit,scaleSource}` or null. Resolution means physical pixels; missing API fields stay null, never substituted with viewport or guessed 100% scaling.
- `selection`: explicit / primary / unresolved. Without targetId, exactly one true primary is selected; absent/ambiguous primary does not select the first monitor.
- `monitors`: validated monitor list; malformed records fail the snapshot rather than pretending partial success.
- `previewFallback`: only when resolution is unavailable, explicitly `{width:1920,height:1080,source:'default-aspect',detected:false}`. This is a presentation ratio, **not detected hardware**.

ID is the Windows GDI device name, e.g. `\\.\DISPLAY2`. It is not an EDID serial, stable hardware identity, Windows Settings Identify number, or Lively monitor index; never automatically map it to a wallpaper-host monitor command. If the actual host supplies a selected target, use its validated GDI device mapping, not the manager window's location or viewport.

UI should display the source/status, dimensions and scale independently, show unknown for unavailable fields, and keep preview `aspect-ratio: width / height` plus contain behavior. Panel expansion must not stretch the preview. If manual fallback is offered, label it as manual; do not overwrite the hardware metadata. Scale is presentation metadata, not a multiplier to physical width/height.

## Native read-only implementation
The constant, encoded PowerShell command uses existing Windows PowerShell/.NET `Add-Type` and Win32 P/Invoke; no input values are interpolated into script and `shell:false`. Process limit: 8 seconds, 128 KiB response. No ExecutionPolicy bypass, privilege request or installation. Constrained-language/code-compilation restrictions return unavailable rather than trying another execution route.

- `EnumDisplayMonitors` and `GetMonitorInfoW` give monitor handles, GDI device name and primary flag. [Microsoft GetMonitorInfoW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getmonitorinfow)
- `EnumDisplaySettingsW(..., ENUM_CURRENT_SETTINGS, ...)` gives resolution in physical pixels independent of DPI virtualization. We do not use monitor desktop coordinate bounds to infer physical resolution. [Microsoft EnumDisplaySettingsW](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-enumdisplaysettingsw)
- `GetScaleFactorForMonitor` supplies scale only on S_OK; failure's suggested fallback is deliberately discarded as unknown. [Microsoft GetScaleFactorForMonitor](https://learn.microsoft.com/en-us/windows/win32/api/shellscalingapi/nf-shellscalingapi-getscalefactorformonitor)

## Verification / gaps
`node --test tests/monitor-adapter.test.mjs`: 9 pure cases pass on cloud POSIX. They verify primary/explicit selection, unknown fields/fallback, invalid metadata, denied launch on non-Windows, constant shell-free command and failed/oversize responses. No Windows execution, native struct/layout execution, real monitor/DPI observation, Lively mapping or host integration is claimed. Runtime/renderer integration is owned by the main implementation worker and not covered by these nine tests.
