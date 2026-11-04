# Windows GPU 可觀測性與計量口徑 — 2026-11-05

本機可以透過 Windows GPU 效能計數器量到 Atmos 的 GPU 引擎使用率與程序記憶體帳務，不能再將 GPU 一概列為「無法測量」。本文件是可行性與短時點測，不是兩小時耗用結論，也不是耗電／溫度測量。

## 本機查核結果

- Windows build 19045；顯示卡 `NVIDIA GeForce GT 710`，驅動 `30.0.14.7514`，CIM 回報狀態 OK。
- 存在 `GPU Engine`、`GPU Process Memory` 與 `GPU Adapter Memory` 的 PDH／CIM 計數器。
- 本次程序樹快照：一個 Atmos native host，合計 15 個屬於它的程序，其中 12 個 WebView2 程序。只查 native host 自己會漏掉真正工作的 GPU child。
- 2026-11-04 16:46:13 UTC（台北 11-05 00:46:13）的 PDH 點測：20 個有效的 owned GPU engine instance，分屬 10 個實體引擎鍵；最大引擎聚合為 **28.4983%**。程序帳務 dedicated **247,484,416 bytes**、shared **15,007,744 bytes**，沒有無效的 owned instance／query error。
- 同期另一個獨立 Get-Counter 查核也取得 24 個有效的 owned engine／memory sample。這是能力交叉確認，兩次查詢的時間窗不同，不用它們相減或當成同一張畫面的固定數值。
- 程序樹包含目前宿主及其管理／桌布 WebView 後代，並未將每個 WebView2 程序都算入，也沒有隔離成「只含桌布、管理頁關閉」的基準。

沒有額外安裝驅動、修復計數器、變更宿主設定、建立背景收集器、使用 UI 自動化或啟動 ETW tracing。沒有推送原始程序命令列、私人路徑或完整機器診斷。

## 提供給長時間收集器的入口

`desktop-host/scripts/probe-gpu.ps1` 可被 dot-source，使用一份持續存在的 English PDH query：

```powershell
. .\desktop-host\scripts\probe-gpu.ps1
Initialize-AtmosGpu
try {
    # ownedPids 由外層收集器取得，必須包含此 Atmos host 的真正後代。
    $sample = Read-AtmosGpu -OwnedPids ([int[]]$ownedPids)
} finally {
    Close-AtmosGpu
}
```

Initialize 只呼叫一次；Read 沒有 sleep、CIM 探查、程序掃描或重新建立 query。第一次 Read 明示 warmup，下一個正常採樣間隔才使用引擎百分比。初始化查詢／編譯與本機一次 warm Read 的延遲分開：本次 warm Read 牆鐘約 64.27ms，這不是收集器 CPU 使用率的測量。

Windows 本地化計數器名稱不影響 English PDH 路徑。使用 `PDH_FMT_DOUBLE | PDH_FMT_NOCAP100`，檢查每個 sample 的 CStatus，只接受 VALID_DATA／NEW_DATA。缺失、warmup、無效或超出合理百分比的資料回傳 null／available=false，不以 0 偽裝成功，也不把異常數字 clamp 成成功。

外層必須在每個採樣點更新程序樹並防止 PID 重用／父程序消失的錯誤歸屬。helper 不會把同名的其他 App WebView2、DWM 或整台機器程序偷偷加入 Atmos。

## 百分比與記憶體定義

`engine.busiestEnginePercent`：依 LUID／physical adapter／engine ID 將同一時間窗、已歸屬 Atmos 的各程序數值先相加，再取引擎最大值。**不加總 3D、Copy、Video Decode 等不同引擎當成整張 GPU 的百分比**；引擎可以並行，也可能共用底層硬體。

`processMemory.accountedDedicatedBytes`／`accountedSharedBytes`：已選程序的 GPU 帳務加總。跨程序共用 surface 可能被重複計入，不能稱作唯一實體 VRAM 用量。Microsoft 也記錄 Windows 10 部分情境的程序 dedicated counter 會顯示不正確增長，不能單靠這條線判定顯存洩漏。

`systemAdapterMemory` 是全機 adapter 的總量，與 Atmos 程序樹值分開。它包含其他 App，不是 Atmos 使用量。DWM 的合成成本也不能在目前這種程序歸屬法中全部算給 Atmos；若要歸因必須另做受控基準／專門 tracing，而非把全機數字貼成 Atmos 成本。

## 環境與驗證限制

- 驅動必須暴露支援的 WDDM GPU 計數器；存在類別名稱不等於有有效的程序樣本，必須看本次數值與狀態。
- RDP／虛擬 GPU 可能有不同的 host／guest 範圍。RemoteFX 的影格、編碼或網路計數器不能當成 App 的 GPU Engine 百分比。沒有可歸屬的 GPU sample 時清楚回報不可用。
- GPU Engine 百分比不代表耗電、溫度、電池續航、實際呈現 FPS 或目視流暢。這次沒有取得這些量。
- 點測不能證明長期穩定、零洩漏，或其他 GPU／驅動也有相同耗用。

`test-gpu-probe.ps1` 有 10 個純合成聚合案例，覆蓋精確 PID、同引擎聚合／跨引擎最大值、valid zero 與 missing、warmup、無效覆蓋、超界值、query error、程序帳務／全機分離及空程序樹。`implementation-p4_2_5/tests/gpu-probe.test.mjs` 在 Windows 會自動執行它；這些測試不會初始化 PDH、操作視窗或偽造實機採樣。

官方依據：[Microsoft GPU 引擎與記憶體口徑](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/)、[程序 GPU 記憶體計數器已知限制](https://learn.microsoft.com/en-us/troubleshoot/windows-client/performance/gpu-process-memory-counters-report-wrong-value)、[PDH 格式化陣列與 NOCAP100](https://learn.microsoft.com/en-us/windows/win32/api/pdh/nf-pdh-pdhgetformattedcounterarrayw)、[PDH 狀態碼](https://learn.microsoft.com/en-us/windows/win32/perfctrs/pdh-error-codes)、[RDP graphics 計數器範圍](https://learn.microsoft.com/en-us/azure/virtual-desktop/remotefx-graphics-performance-counters)。
