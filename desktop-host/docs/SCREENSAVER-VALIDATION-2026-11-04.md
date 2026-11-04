# Windows 螢幕保護程式

Atmos 安裝版提供與桌布宿主分開的 `.scr` 生命週期，使用 Tauri／WebView2 呈現自己的視窗，沒有隱藏／替換原桌布播放器、沒有代替 Windows 鎖屏或認證，也沒有任何解鎖動作。

安裝程式在自己的安裝目錄產生 `Atmos.scr`。它與主程式共用已封裝的 Node／Python／WebView2，不要求額外安裝 runtime；`.scr` 與安裝包都不提交 Git。

管理頁的「螢幕保護程式」可設定重複、每螢幕獨立或跨螢幕，以及所有已連接螢幕／確切目標清單。這份設定使用獨立版本及 CAS／receipt，與平常桌布放置設定分開保存。空清單代表所有可辨識的已連接顯示器；明確清單內的離線目標保留，不套到另一個螢幕。

模式與螢幕設定本身不改 Windows 啟用狀態。只有按 **選為 Windows 螢幕保護程式** 才會對已核對為主程式同一份 binary 的固定 `Atmos.scr`，呼叫 Windows `InstallScreenSaver`。**開啟 Windows 螢幕保護程式設定** 只開固定的系統設定頁。閒置時間與是否在恢復時要求登入，仍由 Windows 管理。

選用前會將目前的 `SCRNSAVE.EXE` 值保存在本機 owner 綁定備份。解除安裝時，只有目前選擇仍為本安裝目錄的確切 Atmos.scr，才還原先前選擇；如果使用者已改選其他保護程式則不更動。沒有先前選擇時只移除 Atmos 自己的選擇值。這個清理不更動登入驗證、閒置時間或其他 App。

## 原生入口

```text
Atmos.scr /s
Atmos.scr /c
Atmos.scr /p HWND
atmos-wallpaper.exe --screensaver-command get
atmos-wallpaper.exe --screensaver-command update
atmos-wallpaper.exe --screensaver-settings
atmos-wallpaper.exe --screensaver-install
atmos-wallpaper.exe --screensaver-stop
atmos-wallpaper.exe --screensaver-unregister
```

- `/s`：建立自己的全螢幕視窗；讀取實際 session 的最後輸入資訊，偵測新輸入後退出。未知輸入狀態／核心故障也退出，不攔截鍵盤、滑鼠或阻止系統操作。
- `/c`：透過固定的同目錄主程式開啟 Atmos 管理設定。
- `/p HWND`：驗證存在且具有可用 client bounds 的父視窗，只將 Atmos 自己的子視窗掛入它；父視窗消失／程序身分改變即退出。不改父視窗樣式或內容。
- `--screensaver-stop`：透過安裝目錄識別的專用事件，要求該安裝的 full／preview 實例退出；不終止平常桌布宿主或其他播放器。
- `--screensaver-install`／`--screensaver-unregister`：具有前述 Windows 選擇副作用，只由明確使用者操作／解除安裝執行，沒有放進自動驗證腳本。

設定 update 讀取最多 16KiB JSON 標準輸入：

```json
{
  "schemaVersion": 1,
  "requestId": "screensaver.mode.001",
  "expectedRevision": 0,
  "screensaver": { "mode": "duplicate", "targets": [] }
}
```

使用者控制的 HTTP 介面只接受上述模式／目標 DTO；不接受父 HWND、執行檔路徑、任意命令或驗證設定。Windows 設定／選用動作也只有固定的 manager session／same-origin action。

## 多螢幕、聲音與回報

跨螢幕使用各實體螢幕自己的 tile 與共用畫布座標，保留負座標／不規則排列的幾何；主要元件錨定選定第一個目標（全螢幕清單預設主螢幕優先）。每個 tile 有自己的 target ID、投影版本與回報。

保護程式本身永遠無聲；完整 `/s` 實例存在時，透過專用原生 mutex 暫停 Atmos 桌布音訊。預覽不暫停平常的音訊。正常退出／故障後 mutex 由 OS 回收，原桌布依原政策恢復，避免兩份聲音疊播。

保護程式 renderer 使用獨立 session role／nonce，不具管理寫入權限。回報存入 `observations/screensaver-<ID>.json`，不能寫成 wallpaper ack。`screensaver-status.json` 分開列出 own native window、真正 matching renderer ack 與「presentation unknown」；ack 不是人眼看到的證據。

其 runtime log、WebView profile 與狀態檔使用專用名稱；排程寫入在保護程式 runtime 停用，避免同一套設定有兩個排程器。同一 store 的核心交易仍使用既有鎖。

## 自動測試與限制

已跑 `test-screensaver-native.mjs --exercise-native`：雙實體螢幕上的重複／獨立／跨螢幕各自啟動、反覆收到真正的 screensaver-role 回報、透過自己 stop 事件退出、清除自有視窗，並核對原本桌面視窗 metadata 未變，共 **7 項通過**。腳本沒有送出任何 ack，沒有模擬輸入。

另跑 `test-screensaver-preview.mjs --exercise-native`：以測試自行建立的隱藏 Win32 父視窗，確認真正 `/p HWND` 掛載及 screensaver ack，關閉該父視窗後確認預覽退出並清除自有視窗，**2 項通過**。這是正向原生預覽 fixture，不是 Windows 控制面板本身的人工驗收。測試 helper binary 只生成於本機驗證目錄，沒有提交。

原生單元測試涵蓋 `/s /c /p` 的大小寫／嚴格 HWND 與非法輸入、錯誤父視窗、設定 DTO、Windows 選用參數固定路徑與引用，以及解除安裝只還原自己仍被選用的條件。Node 測試涵蓋 manager-only API、same-origin、Schema、固定 native 命令及 Windows 動作只能明確呼叫。

Windows 自己的控制面板預覽、閒置自動啟動、實際鍵鼠退出手感、登入驗證、鎖屏／睡眠、實體插拔與混合 DPI 目視驗收仍未人工執行。GPU、耗電及長期耗用沒有由此測試取得。硬體不支援真正睡眠的限制仍有效。

參考：Microsoft [Screensaver Sample](https://learn.microsoft.com/en-us/previous-versions/windows/desktop/ms686421(v=vs.85))、[GetLastInputInfo](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-getlastinputinfo)、[SCRNSAVE.EXE／InstallScreenSaver](https://learn.microsoft.com/en-us/windows/win32/devnotes/scrnsave-exe)。
