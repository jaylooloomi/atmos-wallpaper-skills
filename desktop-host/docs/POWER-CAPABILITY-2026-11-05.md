# Windows power/energy capability — 2026-11-05 (Taipei)

This is a read-only capability investigation on the current Windows machine. It does **not** measure Atmos watts, joules, battery life or per-process energy. `power.available` remains false, with watts/joules null; CPU and GPU resource measurements must keep their own units.

## Actual observations

- Native `GetSystemPowerStatus` succeeded: AC line status 1, battery flag 128 (no system battery). Battery percentage/lifetime are unknown, not zero. `Win32_Battery` returned zero instances; the WMI `BatteryStatus` class was unavailable. No battery discharge/charge rate was obtained.
- SetupAPI enumeration of present Energy Meter Interface devices (`45BD8344-7ED6-49cf-A440-C276C933B053`) succeeded and found **0** interfaces.
- The `Energy Meter` and `Power Meter` performance counter sets are registered, but querying their actual counters failed. Registered counter names alone are not a working meter or measured power reading. Two commonly named EnergyMeter WMI classes were unavailable; this does not prove that all vendor-specific telemetry interfaces are absent.
- `powercfg /a` reports hibernation and fast startup available, while S0 low-power idle and S1/S2/S3 standby are unsupported by firmware. Hybrid sleep is unavailable. The availability check did not enter any power state or change settings.
- Existing vendor telemetry was also checked: the prior read-only NVIDIA query identified GeForce GT 710 / driver 475.14 but returned `[N/A]` for power draw and power limit. An additional one-off query of **power.draw only** also returned unsupported. Existing NVML availability does not imply this GPU exposes board watts. No limits, clocks or driver settings were changed.
- No elevation request, settings change, service/driver install, raw MSR access, SRUM export, battery report or `powercfg /energy` diagnostic was performed. A power/energy diagnostic was deliberately not run alongside the resource collector.

## Reproduce the bounded native capability check

```powershell
. .\desktop-host\scripts\probe-power.ps1
$powerCapability = Invoke-AtmosPowerProbe
$powerCapability | ConvertTo-Json -Depth 5
```

Dot-sourcing defines one function and performs no probe. Invoke once before a resource collection window; do not poll this capability discovery every sample. It uses native power-status and SetupAPI enumeration only, returns no device paths, and caps enumeration at 64 interfaces. It makes no driver IOCTL measurement or WMI/counter poll. Even on a future machine with an EMI interface, the helper still reports `available=false` until a separate measurement implementation validates metadata, units, scope and interval readings.

## What may and may not be concluded

| Signal | Meaning | Not evidence of |
| --- | --- | --- |
| Process CPU time / GPU engine busy | Work/activity during the sampled interval | Physical watts or energy consumed |
| Task Manager Power Usage / trend category | Windows activity-derived impact indication | A calibrated watt/joule measurement |
| E3 app attribution | Software energy model, potentially informed by qualifying EMI rails | Automatically a measured value or whole-PC draw |
| Battery discharge rate, if available | Battery/system-side rate under the reported conditions | Atmos-only power or mains AC draw |
| Validated EMI energy delta | Energy for the hardware rail(s) identified by meter metadata | Automatically whole-PC or per-process energy |

Microsoft describes Task Manager's Power Usage calculation as considering CPU, GPU and disk. E3 preferentially consumes qualifying EMI rail data when present and otherwise uses a software estimated model. EMI discovery is only the first step: its official protocol requires version/metadata/unit verification and interval measurements before deriving energy or average power. These are distinct from process utilization.

On this machine, no validated watts/joules source was obtained from the probed built-in interfaces. A future controlled energy test needs a validated hardware meter/appropriate telemetry source, known measurement scope, and comparable Atmos-on/off windows with other workloads controlled. Until then, report resource usage and **power not measured** separately; do not claim energy savings or zero power consumption.

## Official references

- [SYSTEM_POWER_STATUS flags and unknown sentinels](https://learn.microsoft.com/en-us/windows/win32/api/winbase/ns-winbase-system_power_status).
- [EMI discovery, metadata and interval measurement protocol](https://learn.microsoft.com/en-us/windows-hardware/drivers/powermeter/energy-meter-interface).
- [E3 measurement rails versus software estimated model](https://learn.microsoft.com/en-us/windows-hardware/drivers/powermeter/rail_naming).
- [Microsoft's description of Task Manager Power Usage / trend](https://blogs.windows.com/windows-insider/2018/06/27/announcing-windows-10-insider-preview-build-17704/).
- [Powercfg options: `/a` is discovery; `/energy` is an idle-system diagnostic](https://learn.microsoft.com/en-us/windows-hardware/design/device-experiences/powercfg-command-line-options).

## Optional vendor board-power discovery

`Invoke-AtmosVendorPowerProbe` is a separate explicit one-off function in the helper. It is not called by `Invoke-AtmosPowerProbe`, current collector startup metadata or any periodic sample loop. It uses only the existing System32 NVIDIA utility with fixed `--query-gpu=power.draw --format=csv,noheader,nounits`, a five-second timeout, hidden console and no shell. It never requests power limits, clocks, temperatures or driver changes.

The pure `ConvertFrom-AtmosNvidiaPowerDraw` parser caps output/row count, preserves `[N/A]` as unsupported/null rather than zero, distinguishes real numeric zero, rejects unexpected columns/non-finite/out-of-range values and never returns raw UUID/serial output. If a future GPU reports a number, it is explicitly an individual driver-reported GPU-board reading, not Atmos-attributed energy or whole-PC power. No cross-GPU total is invented.

Seven pure parser fixtures run without native calls or hardware sampling (`test-power-probe.ps1`, wrapped by `power-probe.test.mjs`). The present GT 710 remains unavailable, so running run2 power fields stay unchanged as null/unmeasured.

NVIDIA documents unsupported query values as `N/A` and limits readings by hardware/environment support: [official NVIDIA-SMI reference](https://docs.nvidia.com/deploy/nvidia-smi/index.html). Tool/NVML presence alone is not proof of power-sensor support.
