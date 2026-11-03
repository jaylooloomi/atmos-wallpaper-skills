# Windows 安裝版 0.2.0 實測

日期：2026-11-04（Asia/Taipei）。本次交付為 **Windows x64 NSIS 安裝版**，安裝完成頁預設啟動並自動套用單主螢幕，不再要求一般使用者安裝 Node、Python 或 Lively。

## 產物與封裝

- 檔名：`Atmos Wallpaper_0.2.0_x64-setup.exe`。
- 大小：292,145,369 bytes（約 278.61 MiB）；包含 215,017,168 bytes 的官方離線 WebView2 子安裝程式。
- SHA-256：`2e8dbc1ecb5733b10d77c1058eb2bf057bb39580bfeb160ce7c668f84eb82b52`。
- 自有安裝包 `NotSigned`；內嵌 Microsoft WebView2 安裝程式簽章為 `Valid`、發行者 Microsoft Corporation。雜湊不能替代自有發行者簽章。
- Tauri 2.12.5、Rust 1.99.1 MSVC；封裝 Node 24.22.2、Python 3.14.10 embeddable、Pillow 13.0.0、production npm 依賴、凍結素材、生成的 web assets 及授權聲明。
- payload manifest：843 個檔案、179,652,274 bytes（不含 manifest 本身）。下載來源與 SHA-256 固定在 `installer/runtime-lock.json`；啟動會檢查已封裝檔案。這是損壞偵測，不是防竄改簽章。
- 宿主 PE dependencies 檢查只包含 Windows 系統 DLL／UCRT，不另要求 VC Runtime 安裝。Python／Pillow 所需 DLL 隨官方嵌入式套件及 wheel 封裝。

## 實測環境與方法

Windows 10 專業版 x64，10.0.19045，主螢幕 1920×1080。透過實際 NSIS `/S /R`、程式原生 CLI、Win32 metadata、真實 renderer ack、程序與 port 檢查及使用者目視確認；沒有 Computer Use、瀏覽器／桌面 UI 自動化或偽造 ack。

測試主機本來已有全域 Node、Python 與 WebView2，沒有移除它們或變更全機設定。使用最小 PATH，核對實際執行中的 Node image 確實位於安裝目錄，並直接使用已安裝的 Node／Python 跑功能與回歸；**不是全新 VM 無依賴安裝測試**。WebView2 缺少時的離線安裝分支有封裝與簽章證據，但本機因已有 Runtime 而未走該分支。

## 實際安裝／更新／解除安裝

| 情境 | 結果 |
| --- | --- |
| 靜默安裝＋自動套用 | `/S /R` 完成後自行啟動；未傳 Node／Python 路徑，仍成功載入自有 WorkerW 桌布及真實 wallpaper ack。預設持續運作，無十分鐘期限 |
| 安裝內容自檢 | `--check-install` 驗證 payload 與內嵌執行環境；實際核心 Node image 路徑位於安裝目錄 |
| 管理頁 | `--manager` 透過同使用者原生 event 開啟真正管理視窗，收到 manager role 的真實 ack |
| 損壞檢查 | 刻意修改已安裝 CSS，`--check-install` 以 code 1 拒絕；還原原始 bytes 後再驗證通過 |
| 執行中重裝 | 舊宿主透過原生關閉協定正常退出並還原，舊 Node／port 清除；新實例自行啟動。`live.json` bytes 與合成資料保留，無關 Node sentinel 存活 |
| 執行中解除安裝 | 自有程序與 port 清除，程式檔移除；預設保留使用者資料，無關 Node sentinel 存活；原本桌布仍持續回報 |
| 最終重新安裝 | 再次自行套用，保留安裝版運作；使用者確認「正常，桌布、時鐘與圖示都正常」，無白畫面 |

安裝後預設目錄為 `%LOCALAPPDATA%\Atmos Wallpaper`，資料為 `%LOCALAPPDATA%\io.atmos.wallpaper`；本次沒有停用既有 4173／41873 服務，沒有呼叫 Lively CLI，也沒有把舊開發 store 或私人日曆包進安裝檔。

GUI 安裝完成頁的「啟動」預設勾選，已核對實際生成的 NSIS script；未自動操作安裝器 UI。靜默安裝透過官方 `/R` 明確要求啟動，沒有 `/R` 的 IT 靜默部署不會擅自啟動。此版不設定登入自啟。

## 已安裝功能與相容性

- 使用已安裝 Node 24.22.2、內嵌 Python／Pillow，於最小 PATH 下產生及匯入 PNG／JPEG／WebP，含中文及空白檔名；資產套用、備份檔讀取與還原預覽通過。
- ICS 匯入使用 production `ical.js`／`luxon` 與真實 worker；Windows DPAPI 加密保存，重新建立 calendar service 後仍能解密並讀出合成事件。
- 另用舊開發環境 Pillow 12.1.0 建立三種圖片來源的備份，再用內嵌 Pillow 13.0.0 還原；資產 SHA-256 與 bytes 完全保留。這是合成案例相容性測試，不代表所有歷史圖片皆已窮舉。
- 新增 `-B` 避免圖片子程序產生 bytecode cache，並設定 `windowsHide` 避免 console 閃現。內嵌 Python 由 `._pth` 隔離；開發模式保留合法的 user-site Pillow，沒有強制 `-I` 破壞既有 CLI。凍結交易流程比對只排除這個明確的子程序啟動差異，原始 fixture 雜湊仍固定。

## 回歸與證據

- Rust：10 passed、0 failed；clippy `-D warnings` 與 fmt check 通過。
- Node 產品：188 passed、2 skipped；共用核心：41 passed、2 skipped。使用已安裝 Node 24.22.2 與已安裝內嵌 Python 重跑；既有開發 Python 也另跑通。
- 合計 239 個 Rust／Node 測試通過、0 failed、4 skipped；prepare、獨立 draft／path review、CLI help 通過。
- release 宿主另重跑四種原生情境：正常退出與單實例、限時退出、核心故障、強制終止；WebView2、核心與 port 清理、原桌面保留及原桌布前後真實 ack 均通過。
- 六項安裝整合檢查通過，最後安裝版保留運作。
- 從 Git index 乾淨匯出來源 ZIP，218 個發佈檔案的 bytes／SHA-256／Git blob SHA 均通過；沒有 target、payload、node_modules、dist 或私人 runtime data。來源 ZIP 以 manifest fallback 重建出與工作版完全相同的 payload manifest，並以既有 crate cache 通過離線 Rust 測試。這不是全新 VM 測試。
- 自動檢查見 [windows-installer.json](../evidence/windows-installer.json)、[windows-prototype.json](../evidence/windows-prototype.json)、[runtime-compatibility.json](../evidence/runtime-compatibility.json)；目視確認記於本報告，不把 ack 當成 OS 視覺證據。
- 原始診斷、WebView2 profiles、合成 store 與本機路徑留在本機，沒有提交 Git。安裝二進位另提供 SHA256SUMS，不放入來源 manifest。

## 修正與尚未涵蓋

本次補強初始化期間的退出同步：先建立可關閉 event，再驗證 payload，若更新／解除安裝已提出退出要求，便在建立 renderer 前結束。更新與解除安裝只關閉同路徑的 Atmos，不按通用程序名稱清除 Node／Python。

測試工具另修正 NSIS 子程序繼承輸出管線造成的等待、同版本重裝時舊 journal 被誤判為新實例，以及備份 API 回傳檔案參照的正確讀取；修正後整輪重跑通過。

尚未涵蓋：真正升版到不同版本（已測同版本重裝）、全新無 WebView2 VM、Windows 11、所有 WorkerW 變體／冷啟動、多螢幕／混合 DPI、Explorer 重啟、真正睡眠、GPU／耗電與長時間測量。本機仍不支援 S0／S1／S2／S3，不宣稱睡眠恢復通過。不保證零洩漏；先前 Lively 的部分長測不能移作本版數據。macOS adapter、程式碼簽章、自動更新與登入自啟仍未實作。
