# Windows 桌布宿主與恢復驗證（2026-11-03）

狀態：**使用者要求停止測試；僅有約 36 分鐘的部分耗用結果，未完成兩小時驗證。管理頁服務保留。**

使用者要求不使用 Computer Use。本次僅透過官方 Lively 命令列、Atmos 核心／HTTP、Windows 程序計數器與事件紀錄；畫面正常與時鐘更新由使用者親自確認。

## 實際桌布套用

- 已安裝 Lively 2.2.1.5（Microsoft Store package 1.0.164.0），WebView2 播放器。
- 先讀現有 WallpaperLayout，確認 Lively index 2 對應 Windows DISPLAY2 主螢幕；不是假設兩種編號相同。
- 原先 4173 的服務提供不同版本，保留原程序與 store。另啟目前儲存庫版本於 127.0.0.1:41873，以獨立 store 測試。
- 使用官方 `setwp --file PROJECT --monitor 2`，程序參數確認實際播放器載入 `/wallpaper`，1920×1080；服務收到 wallpaper 角色的有效 revision／資產 hash 回報。
- 使用者確認實際桌布正常，時鐘會更新。renderer ack 本身仍不宣稱 OS 視覺正確，產品 API 的 hostVisibility 仍保留 unknown。

## 服務重啟恢復修正

舊版在服務重啟換 session 後只顯示「請重新載入」，不能自行恢復。新增唯讀 wallpaper 的自動重載與跨重載退避（1 秒起，上限 60 秒），manager 不自動重載，保留未提交草稿。

真實 Lively 中停止／重啟服務後，HTTP trace 觀察到 401 → 自動 GET /wallpaper → 讀取新 session → 有效 snapshot／資產 → 真實 POST /api/ack；從第一個 401 到有效 ack 約 1.6 秒，無手動重載指令介入該輪成功測試。播放器也維持原實例，rendererId 換新。過程中修正 browser native timer 被錯誤 receiver 呼叫的問題，並加入相應回歸測試。

追加 online、pageshow 及恢復可見時立即同步；平常輪詢保留。測試服務可用 `ATMOS_TRACE_REQUESTS=1` 記錄路徑／狀態碼，不記錄 session、headers 或 request body。正式耗用測試關閉 trace。

## 鎖屏與睡眠

- 鎖屏／解鎖：使用者已親自操作並確認桌布與時鐘正常。
- 此環境 SystemEvents 尚未記錄到 SessionLock／SessionUnlock；Security event log 無讀取權限，沒有修改稽核設定或權限。因此鎖屏結果屬人工確認，並非完整獨立 OS 事件證據。
- 使用者回覆已操作睡眠／喚醒，桌布與時鐘正常。然而進一步執行 `powercfg /a` 顯示此測試環境韌體不支援 S0 低電源閒置、S1、S2、S3，僅列出休眠與快速啟動。`powercfg /lastwake` 為 0；System log 沒有測試期間的 Kernel-Power 42／107 或 Power-Troubleshooter 1，取樣最大間隔也沒有達到要求的 30 秒。
- 因此**真正 OS 睡眠／喚醒恢復未能在此環境驗證**，不可把人工畫面恢復確認當作系統進入過睡眠的證據。沒有擅改韌體／Hypervisor／安全設定，也沒有改用未授權的休眠或關機。需要支援睡眠的環境補測。

## 兩小時耗用

正式開始：台北時間 2026-11-03 01:12:17，預計 03:12:17 完成。負載為海岸場景、下雪強度 100、全部元件啟用，獨立測試 store revision 1。

紀錄器 `scripts/measure-windows-host.ps1` 每約 10 秒收集服務與實際播放器／WebView2 子程序的累計 CPU、private bytes、working set、handles、threads、播放器存在與健康回報。這是取樣間隔加上收集耗時，不是保證精確 10 秒。原始資料留在使用者 LocalAppData，不提交原始程序／個人宿主設定。

結束後以 `scripts/read-windows-lifecycle-events.ps1` 查 OS 事件，再以 `node scripts/summarize-windows-host.mjs ABSOLUTE_MEASUREMENT_DIRECTORY` 產生摘要。CPU 分一核心及整機百分比，記憶體成長比較暖機兩分鐘後前／後十次中位數。尚未量測 GPU、FPS、電池或瓦數；兩小時也不代表已證明不存在任何長期洩漏。

部分結果：158 次取樣、觀察跨度約 2172 秒，未發現 HTTP 錯誤、播放器消失或失去有效 ack。服務 CPU P95 約為整機 0.025%；桌布／WebView2 約為整機 0.599%（本機 56 邏輯處理器；一核心分別約 1.41%／33.53%）。桌布程序樹 private bytes 峰值約 467 MiB，暖機後前後中位數成長約 85.84 MiB，handles 中位數成長約 1670；這些增長尚未證明收斂，不能宣稱長時間耗用或無洩漏已通過。原始程序資料留本機，公開統計見 `../evidence/windows-host-partial-2026-11-03.json`。使用者要求保留管理頁，因此未停止 41873 服務或恢復舊桌布。

## 可重跑驗證

設定既有 Python/Pillow 的 `ATMOS_PYTHON`，在本版目錄執行 `node scripts/verify.mjs`。新增 session recovery 與量測摘要回歸測試，不以模擬測試冒充實際鎖屏／睡眠測試。

官方命令來源：[Lively Command Line Controls](https://github.com/lively-community/lively/wiki/Command-Line-Controls)。本次官方 wiki 連結的 utility 下載自 v2.0.4.0 release；zip SHA256 `39B74812B6C736A45EB7AB5037AA953930BA34413E7F2D730E73068BC9B5763D`。WindowsApps 主程式受到 ACL 限制時不繞過 ACL，改用官方獨立 utility。
