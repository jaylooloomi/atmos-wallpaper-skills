# Agent JSON 檔案監看

`--watch-json` 是明確啟用的本機命令，不是常駐服務或預設開機功能；它不啟動桌布、不刷新網頁，也不監看內部 `live.json`。既有 `--apply-json`、JSON Schema、CAS、requestId 與 durable receipt 規則完全沿用。

```powershell
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --watch-json 'D:\Atmos-input\change.json'
```

輸入必須是內部 store 之外的一個本機絕對路徑。檔案可在監看啟動後建立；其父資料夾必須已存在。僅輪詢指定檔案，不遞迴監看目錄，不執行任何設定內容或任意 shell。

預設每 250ms 讀取，上限 32KiB。相同 bytes 維持至少 750ms 才嘗試一次提交；檔案控制代碼前後與目前路徑的檔案識別、大小及修改時間必須一致。UTF-8（含 optional BOM）才會接受。

可用參數：

- `--poll-ms 100..5000`：輪詢間隔。
- `--settle-ms 500..10000`：穩定期間，必須至少為輪詢間隔兩倍。
- `--duration-ms 0..86400000`：0 表示持續執行；非零值提供腳本可預期的停止時間。
- `--store ABSOLUTE_LOCAL_DIRECTORY`：選用診斷／隔離測試 store；正式使用通常省略。

例如 10 分鐘後自動結束：

```powershell
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --watch-json 'D:\Atmos-input\change.json' --duration-ms 600000
```

直接 Node CLI 可用 Ctrl+C／SIGTERM 中止。原生執行檔持有專用 Windows kill-on-close Job；正常退出或父程序被終止後，監看子程序也結束。這個 Job 只包含本次監看子程序，不包含已運作的桌布宿主、管理頁或其他服務。原生 wrapper 先完成 Job 綁定才允許子程序開始讀取／提交。

## Agent 發布流程

1. 讀 `--get-state`，使用當前 revision 建立新意圖與唯一 requestId。
2. 先寫入同資料夾的暫存檔，寫完關閉後再以原子 rename／replace 發布到指定輸入檔。
3. 讀取 JSONL `result`；分別判斷 `committed` 與 `rendererAcknowledged`。監看不等待 renderer、不刷新畫面，也不偽造 ack。
4. 若回應不確定，呼叫 `--get-result REQUEST_ID`。相同 requestId 只能重送完全相同內容。
5. 發生 revision／request 衝突時，重新讀狀態並產生已調和的新意圖；監看器不擅自替換 expectedRevision 或 requestId。

穩定期間只能辨識檔案是否持續改動，無法知道「語法合法、但作者仍打算增加欄位」的內容是不是最終稿。因此 **原子替換是發布協定的一部分**，不能把 debounce 當成任意寫入方式都不會被提前讀取的保證。

## JSONL 與失敗政策

每一行是一個 JSON 物件，`apiVersion=1`：

- `watch-started`：已啟動，列出輪詢、穩定期間與時限；不暴露原始設定或私人路徑。
- `result`：既有交易的結果，包含 revision、receipt／ack 等欄位。
- `input-error`：檔案缺失、非一般檔案、大小或 UTF-8／JSON 錯誤。
- `command-error`：Schema、revision、requestId、存取或交易錯誤。
- `watch-stopped`：正常信號或時限停止。強制終止程序時無法保證輸出最後一行。

未變更的 bytes 只處理一次；相同內容的錯誤不會每次輪詢洗版。重新啟動監看器會再讀現有檔案，但舊 requestId 仍由核心 receipt 保護，不會重複提交。`STORE_LOCKED` 最多額外重試三次（250／500／1000ms）；仍失敗時等待新的輸入 bytes，避免無限重試。其他錯誤也等待內容變更。

一般的 `committed=true` 證明設定保存，不代表 OS 掛載或目視驗證。缺失、過期、manager-role 或其他 revision 的回報均不能作為 wallpaper ack。

## 自動驗證

`tests/agent-watch.test.mjs` 覆蓋部分寫入／原子替換、穩定期間、去重、非法 UTF-8／JSON、檔案大小、內部路徑拒絕、manager 競爭、receipt 重播、bounded busy 重試、刪除重建、有限時間真 CLI 子程序、取消及原生 readiness gate。這些是自動化功能測試，沒有宣稱人工桌面或鎖屏驗收。


## 多螢幕目標

獨立模式必須在 state／validate／apply／result／watch 命令帶 `--target display-<64 hex>`。目標必須已配置；revision 與 requestId receipt 都屬於該 scope。輸入檔仍禁止位於整個 owner store 及任何 targets 子目錄中。

重複模式共用外觀；`rendererAcknowledged` 必須每個已選目標的新鮮 wallpaper-role、targetId、session 與 revision/hash 都相符，不能只靠共享或 manager 回報。

監看啟動後會固定當時的 mapping；版面模式／目標清單變更後，`TARGET_SCOPE_CHANGED` 阻止繼續寫入。讀取新設定並調和後再啟動新 session。單螢幕模式省略 `--target`。
