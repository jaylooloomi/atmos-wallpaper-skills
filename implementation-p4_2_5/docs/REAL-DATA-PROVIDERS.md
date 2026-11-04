# Real weather and native system providers

These read models are separate from appearance `desired`. Sampling does not write demo fields, manufacture renderer acknowledgement or prove OS visibility. The manager and desktop widgets display real source/time labels when data is available and report unavailable/stale honestly otherwise.

## CPU and physical RAM

`atmos-wallpaper.exe --system-info` uses Windows `GetSystemTimes` snapshots separated by approximately 500ms and `GlobalMemoryStatusEx`. Kernel time includes idle time, so busy percent is `(kernel + user - idle) / (kernel + user)` over the interval. The actual interval is included in `sampleWindowMs`. Reset/invalid counters are rejected. On machines with more than 64 logical processors, this API covers the primary processor group, not necessarily the whole machine; the response states that scope. This is not per-process Atmos CPU.

The Node backend launches only this fixed native command, shares in-flight sampling and caches readings for five seconds. Samples older than 15 seconds are unavailable. No admin rights, process enumeration, microphone, global mixer changes or hardware sensor drivers are used. Network throughput, GPU, temperature and power remain unavailable; no synthetic numbers are substituted.

Sources: [GetSystemTimes](https://learn.microsoft.com/en-us/windows/win32/api/processthreadsapi/nf-processthreadsapi-getsystemtimes), [GlobalMemoryStatusEx](https://learn.microsoft.com/en-us/windows/win32/api/sysinfoapi/nf-sysinfoapi-globalmemorystatusex).

## Weather

Online weather is **disabled by default**. Enabling the manager setting explicitly sends the selected city's coordinates to `https://api.open-meteo.com/v1/forecast`. This version offers Taipei, Tokyo, London and New York; it never requests device geolocation or accepts arbitrary provider URLs. Provider city is a separate setting from legacy demo-city controls. The source is forecast/model data, not a local sensor observation.

Requests use fixed fields, UTC Unix timestamps, six-second timeout, reject redirects, omit credentials and cap the response at 64KiB. Results refresh on demand after 15 minutes; failures back off for at least one minute. Retained stale data is labelled with model time/cache age and discarded for display after 24 hours. City/enable changes invalidate old in-flight results. No background fetch happens while disabled, and no effect auto-switching is silently enabled.

The free hosted API is for non-commercial use. Before commercial distribution, configure an appropriate licensed provider/customer endpoint rather than assuming the free endpoint can be shipped commercially. Data attribution links to Open-Meteo with CC BY 4.0; the free hosted-service conditions are distinct from the data licence. See [official API documentation](https://open-meteo.com/en/docs) and [current service conditions](https://open-meteo.com/en/pricing).

## API

All requests require the local session nonce and same-origin fetch metadata:

- `GET /api/providers/settings`: separate settings/revision and native availability.
- `GET /api/providers/system`: current native sample or unavailable.
- `GET /api/providers/weather`: current model/cache/error state; may fetch only after explicit opt-in.
- `POST /api/providers/settings`: manager role, Origin and JSON content type required. Strict payload: `schemaVersion`, `requestId`, `expectedRevision`, `weather: {enabled, city}`. Durable receipts/CAS use a separate namespace; no arbitrary URL/path/shell.

Tests cover opt-in, request bounds/fixed endpoint, caches/backoff/stale data, city-switch races, strict CAS/replay, invalid native values, authorization and DOM data/source rendering. Fixtures are automated synthetic cases, not proof of hardware sensors, network uptime or user-perceived desktop visibility.

## Agent discovery and explicit provider policy CLI

`--host-schema` emits the static JSON Schema catalog for layout, audio, audio routing, performance, screensaver, providers and library archive, without opening a store. `--providers-command get|update [--store DIR]` reads or updates only the global owner provider settings; `--target` is rejected even in independent-display mode. Updates read bounded UTF-8 JSON stdin and use the same strict payload/CAS/receipts as the manager. Neither command launches a host, fetches weather nor samples telemetry. The running host performs only opted-in reads.

Example stdin: `{"schemaVersion":1,"requestId":"weather.enable.001","expectedRevision":0,"weather":{"enabled":true,"city":"台北"}}`. Read current revision first. Native availability is explicitly not probed by this settings-only CLI.
