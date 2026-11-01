# Frozen core compatibility delta
P1 remains unchanged. P4 portable-core copies its transaction implementation with only:
1. Store import changed to portable-store.
2. contract and decoder relative locators adjusted for the new folder.
3. Python executable explicit option/env (argv array, no shell).
Portable-store inherits the same envelope/read/lock/commit/recovery implementation. Atomic writer adapts directory flush by actual platform; Windows explicitly reports weaker unverified power-loss durability, not hidden success equivalence.
No Windows program/server/host was started by cloud engineering. windows-runtime refuses non-win32. Start-Atmos.cmd is a future authorized launcher, not proof of successful execution.
