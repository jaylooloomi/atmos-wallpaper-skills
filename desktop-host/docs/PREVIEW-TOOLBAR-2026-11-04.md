# 預覽設定單排控制列 0.3.1

日期：2026-11-04。

將預覽解析度、寬度、高度、套用按鈕與語言放在場景下方、雙欄主內容上方的整頁控制列。桌面寬度下，標籤與欄位在同一水平列，不再上下分兩層。保留原本的控制項、事件、語言翻譯與設定交易。窄螢幕解析度獨立一行、寬高並排、按鈕獨立一行，避免欄位／按鈕擠壓或橫向捲動。

## 驗證

隔離 Chromium 命令列合成 fixture：320px natural、390px neon、1280px natural，三組皆通過。1280px 的標籤文字與其欄位垂直中心一致，所有輸入框、選單與按鈕位於同一水平列；手機版沒有頁面、工具列或場景卡片水平溢出，並額外驗證寬高輸入框至少 60px，避免只剩一兩個字元的寬度。場景仍在預覽設定上方，原本套用變更工具列的頂端吸附與 hit test 檢查仍通過。

證據：[preview-toolbar-layout/report.json](../../implementation-p4_2_5/evidence/preview-toolbar-layout/report.json)。檢查只使用隔離合成頁面，不操作使用者瀏覽器／桌面；不當作全歷史版面案例或人工目視確認。

完整 Node 驗證：產品 214 passed／2 skipped，共用核心 41 passed／2 skipped，prepare、獨立 draft／path review、CLI help 通過。本次沒有 Rust 行為變更，只有安裝版本更新；沒有重跑開機、睡眠、卸載或長時間耗用。

## 本機更新

已從 0.3.0 靜默升級到 0.3.1，已儲存設定 bytes 與開機啟動偏好相同。payload／內嵌執行環境自檢、安裝樣式與本次生成結果比對、WorkerW 掛載、真正桌布與管理頁近期回報皆通過。未取得新的人工目視確認。[升級證據](../evidence/preview-toolbar-upgrade.json)。

安裝檔 Atmos Wallpaper_0.3.1_x64-setup.exe：292,188,019 bytes；SHA-256：`c0a4cca6c4b3e589dfbcbf9bcc64cd326f5884365b26b2b17d3a75fe42462e9b`。自有安裝包未簽章。
