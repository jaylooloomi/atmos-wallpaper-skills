# Native process-tree soak analysis method — 2026-11-05

`desktop-host/scripts/analyze-soak.mjs` analyzes the local raw files produced by the current native-host collector. It does not launch, change or control the app. The older Lively collector and the short multi-screen host/Node check are not evidence of whole-tree 0.7 resource use: WebView2 browser, renderer, GPU and utility processes must be counted as owned descendants as well.

## Collector contract

`metadata.json` declares `schemaVersion`, `startedAt`, `plannedDurationSeconds`, `sampleIntervalSeconds`, `logicalProcessors`, `root: {pid, startedAt}`, the native `hostSessionId` and `expectedTargetIds`. Optional workload notes stay local and are not copied wholesale to the public analysis.

Each `samples.jsonl` record has `at`, monotonic `monotonicSeconds`, `collectionDurationMs`, `rootAlive`, `processes`, `processErrors`, `native` and `health`. A failed process query uses `processes: null`, never an empty success list. Process rows include PID, parent PID, creation timestamp, a collector-assigned group (`host`, `runtime`, `webview`, `other`), cumulative `cpuSeconds`, private/working-set bytes, handles and threads. `native` contains the observed host session/capture timestamp and target attachment reports. `health.perTarget` includes current desired revision/hash and genuine renderer acknowledgement state. Optional GPU/power records must identify their source, units and process attribution before they are treated as resource measurements.

The collector must start from the known Atmos PID and its creation time, discover descendants through parent identity and retain validated child identities across reparenting. A reused PID must not inherit ownership without matching creation time. The analyzer checks the declared root against the samples, but cannot reconstruct processes that were never sampled or independently prove the collector's original ancestry decisions.

## CPU accounting

CPU deltas use adjacent monotonic sample times, not wall-clock time. Identity is PID plus creation time. Newborn or disappeared identities, missing cumulative counters and counter regressions are not joined to an unrelated prior value. Intervals greater than `max(30 seconds, 3 × nominal sample interval)` and process-query failures are excluded from CPU statistics.

The report separates complete-cohort intervals from partial-cohort **matched-process lower bounds**. The latter omit work done by newly observed/exited processes and cannot be read as the total CPU used in a churn interval. CPU coverage includes excluded gaps, failed queries, partial durations and identity-pair counts. Short-lived children created and terminated entirely between polls are not measurable with this polling collector.

One-core 100% means one logical processor was fully busy. Whole-machine percent divides by the declared logical processor count. Means are duration-weighted; CPU percentiles are weighted by interval duration. Group totals include all observed owned WebView2 processes, not just the Atmos host and Node runtime.

## Memory, handles and process churn

For each group and the whole tree, the report gives first/last, median, p95 and peak private MiB, summed working-set MiB, handles, threads and process count. Summed process working sets can double-count shared pages; they are not unique physical RAM used by Atmos.

Primary memory/handle/thread/process-count statistics and steady-state windows use complete owned-tree snapshots only. Incomplete observations are separately reported as `partialSnapshotObserved`; they cannot form a low opening baseline or a false growth slope. The first five minutes are excluded from steady-state growth analysis by default. Opening/closing five-minute medians are compared only when their windows do not overlap. Both ordinary least-squares and robust Theil–Sen slopes are reported per hour. The latter uses all points up to 120, otherwise an evenly spaced 120-point subset. Peaks and slopes remain visible even when endpoints look stable.

Process inventory records the number of unique observed identities, PID reuse, newly/disappeared identities between complete snapshots, and processes first discovered after the start. Incomplete snapshots do not create fictitious exit/restart events. These are observed transitions, not a guarantee that every process birth/exit was captured.

## Diagnostic review rules

These thresholds are review aids, not universal pass/fail specifications or leak diagnoses:

- At least 30 minutes of steady samples, disjoint endpoint windows, positive Theil–Sen slope and private-memory median growth greater than `max(32 MiB, 10% of opening median)` raise a memory-growth review flag.
- The same time/slope conditions and handle growth greater than `max(100, 10% of opening median)` raise a handle-growth review flag.
- An increasing closing process-count median by more than one, with positive slope, raises a process-count review flag.
- Sampling gaps, incomplete process coverage, missing/reused root identity, native session mismatch, prolonged native attachment uncertainty and acknowledgement outages are reported independently.
- A 15-second acknowledgement outage is highlighted; observed nearby lock/unlock/suspend/resume events are attached as context, not converted into a claimed successful recovery.

A stable two-hour run with no configured flags cannot rule out a slow leak. Increasing memory can also reflect cache warming, a changed scene, a newly opened manager, device changes or renderer restarts. The report counts target revision transitions so a changing workload is not described as fixed. A claimed-positive acknowledgement must also have a finite `lastAckAt`: the source uses four-second freshness, with an additional one-second read tolerance and at most one second of small future-clock skew accepted and counted. Missing, stale or clearly future timestamps stay unknown with separate counts. Native capture timestamps use `native.observedAtMs` when valid within the collection window; otherwise the sample end timestamp is the reference. Session/PID/root-identity contradictions cannot produce positive native acknowledgement evidence.

## Run and progress commands

```powershell
node desktop-host/scripts/analyze-soak.mjs C:\absolute\local\measurement-directory --partial
node desktop-host/scripts/analyze-soak.mjs C:\absolute\local\measurement-directory
```

Without `--partial`, a successful `complete.json` marker (`collected`, `completed` or `complete`) is required. Failed/cancelled markers can be examined with `--partial` and remain explicitly unsuccessful. The default output is local `analysis-summary.json`; `--output ABSOLUTE_FILE` selects a different aggregate destination. A partial/truncated JSONL tail remains visible as a parsing review flag. Output does not echo raw sample lines, PIDs, target IDs, session IDs, origin URLs, store paths, process names, renderer IDs, hashes or arbitrary metadata notes. Raw files stay local; only a reviewed aggregate should be staged publicly.

This analysis does not measure human desktop visibility, power/energy, frame rate, physical hotplug, lock/login success or hardware sleep by implication. Validated GPU analysis excludes warm-up, frame errors, invalid/out-of-range counter instances and incomplete owned-process inventories from the primary statistics, even if a malformed frame claims `available=true`; exclusion counts remain visible. It reports the busiest accounted physical engine percent from owned-process counter instances, plus accounted dedicated/shared process memory. It does not sum independent engine types or present this as whole-GPU percentage. System adapter dedicated/shared memory is reported separately as whole-machine context, never as Atmos attribution. Power and energy remain unmeasured when the available probe exposes null watts/joules; GPU utilization is not converted into power.


## Coverage required in the final report

Report the successful completion marker and observed span separately; an ongoing `--partial` analysis is not a completed two-hour result. Include complete/partial CPU-duration fractions, excluded query/counter/time gaps, full vs partial memory sample counts, process births/exits/PID reuse, GPU eligible/warm-up/error/incomplete-inventory exclusions, native session/root/timestamp anomalies, and per-target acknowledgement freshness/unknown/stale/future counts. When the collector omits renderer IDs, distinct renderer count is `null` (unavailable), not zero; per-sample renderer-row counts remain usable.

Session/power event registration availability is separate from observed event counts. Zero captured events does not establish that no lock/suspend occurred and does not constitute a successful lock/sleep test. Malformed event lines are disclosed separately. Original and partial raw logs remain local, and no observer/collector workload is added to Atmos process totals.
