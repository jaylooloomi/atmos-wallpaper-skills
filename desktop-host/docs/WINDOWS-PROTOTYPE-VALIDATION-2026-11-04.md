# Windows 單主螢幕原型實測

日期：2026-11-04（Asia/Taipei）。本次交付是 **Tauri 2＋Rust 的 Windows 單主螢幕原型**，不是安裝版或跨平台桌布引擎。宿主沒有 Lively 呼叫、IPC、程式碼或執行期相依。

## 環境與方法

- Windows 10 專業版 x64，10.0.19045；主螢幕 1920×1080，另接一個螢幕，但只對主螢幕掛載一個桌布視窗。
- Rust 1.99.1（MSVC）、Tauri 2.12.5、WebView2 Runtime 156.0.4314.58、Node 24.13.0、Python 3.14＋Pillow。
- 透過真實 Tauri／WebView2、Win32 原生視窗 metadata、實際 renderer ack、程序與 TCP port 檢查，配合使用者目視確認；**沒有使用 Computer Use、瀏覽器／桌面 UI 自動化，也沒有偽造 ack**。
- 原型使用獨立 store、WebView2 profile、動態 loopback port；既有 41873 管理服務、設定與日曆均保留。原始紀錄只留在本機 `%LOCALAPPDATA%\AtmosWallpaper\native-prototype-validation`。

## 結果

| 項目 | 結果與證據 |
| --- | --- |
| 實際桌布掛載 | 視窗 class 為 Tauri Window，native parent 為背景 WorkerW；實際位置與尺寸吻合主螢幕，並非一般前景無邊框視窗 |
| 桌布頁面載入 | 收到真實 wallpaper role、stateApplied=true 的 ack；沒有以管理頁或 HTTP 200 取代 renderer 證據 |
| 畫面與互動 | 使用者確認桌布在圖示後方、時鐘持續更新、桌面圖示可正常操作 |
| 正常退出 | stdin `exit`，exit code 0；自有視窗、WebView2 程序、核心程序與 port 清除，原靜態桌布路徑未改 |
| 重複啟動 | 第二個實例被拒絕，不另建桌布與核心程序 |
| 限時退出 | 15 秒情境正常退出，清理與還原檢查通過 |
| 核心程序故障 | 終止本次建立的 Node 子程序後，宿主偵測失敗並退出（code 2），清理與還原檢查通過 |
| 宿主強制終止 | OS 移除自有視窗；Job 清理核心程序，WebView2 程序及 port 亦清除。強制終止不會產生正常退出 journal，不能把它當成有執行 graceful cleanup |
| 既有桌面保留 | 四種情境均保留原有可見桌面視窗；原桌布在測試前、退出後均有真實持續 ack |
| 最終視覺還原 | 四種情境全部結束後，使用者再次確認「已正確還原，時鐘正常，沒有白畫面」 |

可重跑的自動化證據為 [windows-prototype.json](../evidence/windows-prototype.json)。它只描述自動檢查；目視結果獨立記在本報告。

## 本次發現與修正

1. **退出崩潰**：早期直接呼叫 DestroyWindow，會使 Tao 後續讀取失效 HWND 而 panic。改成先攔截退出，讓 Tauri 在事件迴圈內銷毀自己管理的視窗，再返回與記錄還原；正常退出／故障退出重新測通。
2. **白畫面與驗收缺口**：最初只確認自有視窗已清除及桌布路徑未變，但使用者回報全白。當時原有 Lively renderer 的最後一次 ack 早於此次測試約 16 小時。這不足以證明唯一根因，也不能把 metadata 通過當成視覺還原通過。
   - 為修復現況，透過官方 Lively CLI 僅重新載入原本主螢幕那張桌布，未改其他螢幕或管理服務；使用者確認原桌布與時鐘恢復。
   - 修改宿主：已有有效 WorkerW 時直接沿用，避免每次啟動重送 Explorer 建立桌面層訊息。此舉減少干擾既有桌面的機會，不宣稱已證明白畫面的唯一成因。
   - 擴充整合測試：原桌布在每個情境前後都必須有真實近期 wallpaper ack；最後再由使用者目視確認還原。新版四種情境及最終視覺確認皆通過。
   - 上述 Lively CLI 是修復既有環境的一次性操作；**沒有加入原型、啟動腳本或測試腳本**。原型不負責重新啟動其他廠牌的失效桌布播放器。
3. **清理檢查時序**：Windows Job 終止與 socket teardown 為非同步；測試改為在 10 秒上限內等待完全清理，不以 PID 剛消失的瞬間判定 port 必須已關閉。超過上限仍會失敗。
4. **測試隔離**：singleton 單元測試使用獨立名稱，避免與正在運作的原型互相干擾；實際重複啟動防護仍另由原生整合測試驗證。

## 回歸與可重建性

- `cargo test --locked`：8 passed、0 failed。
- `cargo clippy --locked --all-targets -- -D warnings`：通過；`cargo fmt --check`：通過。
- 現有 `scripts/verify.mjs`：產品 188 passed／2 skipped，共用核心 41 passed／2 skipped；prepare、獨立 draft/path review、CLI help 通過。合計 237 個 Rust／Node 測試通過、0 failed、4 skipped；另有四種原生整合情境通過。
- 從 Git index 乾淨匯出後，`npm ci --ignore-scripts`、web assets 重建及 Rust `cargo test --offline --locked`（使用本機已下載的 crate cache）通過。不把共用 crate cache 的離線重建說成無工具／無網路的全新 Windows 安裝測試。
- `Cargo.lock` 隨源碼提交。SOURCE-MANIFEST 使用 Git blobs 的 bytes／SHA-256／Git blob SHA；不收錄 target、dist、profile、store、raw logs 或 token，凍結 prototype manifest 保留。

## 尚未驗證與產品限制

- Windows 11、其他 Explorer／WorkerW 變體、無既有 WorkerW 的冷啟動、真正 Explorer 重啟、螢幕熱插拔、混合 DPI、多螢幕配置尚未實機驗證。
- macOS 只有未實作提示，沒有 macOS 桌布 adapter 或測試；Tauri 跨平台不代表桌布引擎跨平台。
- 本機不支援 S0／S1／S2／S3，不能宣稱真正睡眠恢復通過。本次沒有新增鎖屏或休眠測試。
- 沒有重新做 2 小時耗用測試，GPU／耗電未測，也不保證零洩漏。先前約 36 分鐘的 Lively 測量不能當成本次 Rust 宿主的數據。
- Node 與 Python/Pillow 仍為外部先決條件；沒有自包含安裝程式、簽章、自動更新或登入自啟。托盤入口已實作，本次沒有自動操作托盤 UI。
- 原桌布應先能正常運作；移除自己的層不等於能修復任意第三方播放器。原生 parent／尺寸、renderer ack 與目視呈現是三種不同證據。
