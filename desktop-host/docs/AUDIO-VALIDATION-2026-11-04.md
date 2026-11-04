# 原生音訊與實體輸出 — Windows 0.6.1

## 行為與選型

採 CPAL 0.18.2 的 Windows WASAPI 輸出；Rust 原生執行，管理頁 WebView 不播放音訊，也不依賴瀏覽器自動播放權限。選用現代 WASAPI 與安全串流所有權，避免自行管理舊 waveOut 的原始緩衝區生命週期。CPAL 授權原文隨安裝包放在 licenses 目錄。

雨聲、強風與遠雷由本專案原創程序合成，不是假資料狀態，也不是第三方錄音。持續產生訊號，沒有固定短音檔的循環接縫；這一版不聲稱達到現場錄音的寫實程度。雨／雷源可明確選擇；weather 跟隨目前桌布的手動天氣特效，晴天／下雪無聲，不代表串接真實氣象服務。

預設停用且靜音。主音量 0–100、靜音、啟用與桌面焦點偏好立即保存；只調整 Atmos 產生的樣本增益，不改 Windows 全域音量或其他 App。增益平滑切換，無聲政策會關閉串流。正常退出先停止音訊執行緒、釋放串流，再關閉核心與視窗；程序被終止時由 OS 回收其音訊資源。

桌面焦點由原生前景視窗／桌面圖示層判斷；一般資料夾、管理頁與桌布 WebView 自己的 focus 都不算桌面。OpenInputDesktop／GetUserObjectInformationW 唯讀判斷輸入桌面；未知／鎖定時停止，不切換桌面、不進行解鎖。自有桌布視窗清除／不可用、設定錯誤、裝置失敗與零音量都停止正常播放。每個宿主只有一個音訊引擎，跨螢幕來源政策另列 AUD-03。

播放裝置可以跟隨 Windows 預設，或明確選擇原生列出的輸出。只保存 opaque device ID，不以友善名稱猜測；指定裝置缺失時報錯／重試，不偷偷改送其他裝置。虛擬線路也是有效輸出，但不等同喇叭。裝置名稱只在本機管理頁呈現，不寫入公開驗證報告。

三秒測試音只由使用者主動按鈕／明確 CLI 呼叫啟動，需啟用且取消靜音。它暫時忽略桌面焦點限制以便在管理頁測試；靜音、鎖屏與不可用桌布仍優先。它不修改保存的音源／偏好，也不是自動開機測試。

## 介面與資料

`audio-settings.json` 是獨立 schema/revision，與畫面、螢幕與素材庫設定分開。採嚴格型別／欄位驗證、CAS、requestId、上一份備份及原子保存。pending receipt 以最後提交 requestId 綁定設定指標；下次修改前先完成前一筆回覆，重試不重複加版本。

```text
atmos-wallpaper.exe --audio-help
atmos-wallpaper.exe --audio-command get
atmos-wallpaper.exe --audio-command update
atmos-wallpaper.exe --audio-command probe
atmos-wallpaper.exe --audio-command diagnose
atmos-wallpaper.exe --audio-command test
```

update 從 stdin 接收最多 16384 bytes 的 JSON，所有 audio 欄位均必填：

```json
{"schemaVersion":1,"requestId":"audio.update.001","expectedRevision":0,"audio":{"enabled":false,"muted":true,"volume":50,"focusOnly":false,"source":"weather","output":"system-default"}}
```

先 get 最新版本與裝置清單；output 可以是 system-default 或清單中的確切 device ID。衝突須重讀協調；相同 requestId 只重送相同意圖。probe 只送數位靜音，不能證明人耳聽到；diagnose 只讀取預設端點與 Atmos 自己工作階段的音量／靜音／峰值，不錄音、不改混音器。test 從 stdin 接收 schemaVersion=1 與 expectedRevision，建立三秒的暫時請求，只有明確使用者意圖才呼叫。

HTTP `/api/host/audio` 與 `/api/host/audio/test` 沿用同源、session 及管理角色保護；不接受任意路徑、URL、shell、時長或音訊檔案。狀態區分保存版本與宿主套用版本；過期／已退出程序的狀態不算目前播放證據。streamOpen／playing 是引擎狀態，不等於喇叭已發聲。

## 除錯與驗證

0.6.0 的靜音裝置探測通過，但使用者回報設定保存後無聲。唯讀 Core Audio 診斷確認 Atmos 工作階段未靜音、音量 1.0，收到約 0.0794 非零峰值，系統預設端點未靜音／音量約 0.58。這只能證明 Windows 收到訊號，未記為人耳通過。

0.6.1 補上明確播放裝置選擇、實際名稱與三秒測試音後，使用者確認測試音、雨聲、音量、靜音及 Win+D／Alt+Tab 桌面焦點切換正常。首次失敗與後續成功均保留在證據，沒有把測試音或靜音探測冒充原本已通過。

- Node：298 通過、4 略過；Rust：24 通過。包含政策、增益／數位靜音、CAS／重播、裝置 ID 邊界、API 權限與測試音請求邊界。
- 4 種控制 fixture：桌面、手機、英文、裝置錯誤；沒有橫向溢出。這是合成 Chromium 測試，不是實體裝置證據。
- 安裝版 7 項探測／政策／API 檢查、原生退出音訊執行緒／串流關閉、Shell 重啟及獨立交接後存活檢查通過。
- 原本套件版本更新誤改相依鎖定版本已修復；新增指定 Atmos package 的版本工具與回歸測試，禁止整個 Cargo.lock 全域替換。
- 原畫面、素材庫位置、螢幕與啟動偏好維持；音訊測試後保留使用者選擇。

證據：desktop-host/evidence/audio-{upgrade,installed,lifecycle,handoff,human-confirmation,output-diagnosis}.json 與 implementation-p4_2_5/evidence/audio-controls/。

未宣稱已測：實體拔除／重新接回、藍牙重連、新一輪音訊鎖屏、重新登入、真正睡眠、GPU／耗電或長時間播放；這些保留於 HOST-03。本機韌體不支援真正睡眠。沒有 Computer Use、使用者瀏覽器或桌面 UI 自動化，沒有麥克風／桌面音訊錄製。

依據：[CPAL 0.18.2](https://docs.rs/cpal/0.18.2/cpal/)、[CPAL Windows WASAPI backend](https://github.com/RustAudio/cpal/blob/master/src/host/wasapi/mod.rs)、[Microsoft OpenInputDesktop](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-openinputdesktop)。

最終安裝包：292575045 bytes，SHA-256 `1b80d1675d36f9f57364d05e52db2ce6b9f38adeccf2e70fbbc91f85d00930a5`；未簽章、僅留本機，未提交。
