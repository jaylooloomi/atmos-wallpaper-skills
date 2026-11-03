# Windows 登入啟動 0.2.1 驗證

日期：2026-11-04，Asia/Taipei。環境為 Windows 10 x64 build 19045、主螢幕 1920×1080。全程使用命令列、原生 API／系統登錄、真實桌布 ack 及使用者確認；沒有 Computer Use 或瀏覽器／桌面 UI 自動化。

## 行為與權限

- 首次安裝／首次升級此功能預設開啟，透過目前帳戶的 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`，值名稱 `AtmosWallpaper`，登錄帶引號的已安裝執行檔與 `--autostart-launch`。不使用管理員、系統服務、額外 Node／Python 安裝或 Lively。
- 依 Windows Run 登錄的長度限制驗證指令，路徑含空白／中文可正確引用；拒絕相對路徑、引號與過長路徑。[Microsoft Run 登錄文件](https://learn.microsoft.com/windows/win32/setupapi/run-and-runonce-registry-keys)
- 登入啟動會等待 Explorer 桌面就緒（最多 60 秒，可被自身退出命令取消），套用桌布而不開管理頁；已存在實例時直接結束，不搶焦點。初始化失敗記錄本機 `startup-errors.log`，不在登入時彈出管理頁。
- 管理頁右側「Windows 啟動」開關直接讀寫原生設定，立即生效，不改 visual desired、revision 或草稿。切換失敗會顯示錯誤並重讀登錄；非安裝版顯示不可用；桌布頁不建立控制也不能呼叫此 API。
- HTTP 寫入需要 manager session、同來源、正確 Fetch Metadata 與 JSON，僅接受一個 boolean `enabled`。後端只可啟動固定已安裝宿主的固定命令，不接受網頁提供的路徑／命令；並序列化管理頁寫入。原生回傳的執行檔路徑不送到管理頁。
- 關閉選擇存在 `HKCU\Software\AtmosWallpaper\StartupEnabled`，重裝不會擅自開回來；一般程式啟動也不會重新登錄。退出程式保留登入設定；解除安裝移除屬於自身執行檔的 Run 項目，保留選擇供重裝使用。
- 狀態描述的是目前 Run 登錄，不是實際登入呈現。Windows「啟動應用程式」或組織政策仍可覆蓋；沒有修改 StartupApproved 或繞過其限制。

## 實機結果

第一次從已安裝 0.2.0 升級到 0.2.1，已確認預設建立登入登錄並保留 `live.json`。後續修正測試工具對「登錄值不存在」的讀取方式，完整測試以重設 Atmos 自己的新偏好為起點重跑，不變更其他啟動項目。

| 情境 | 證據／結果 |
| --- | --- |
| 安裝初始化 | 實際 NSIS `/S /R` 後，新功能未設定時預設開啟；管理 API、原生 status 與 Windows Run 值一致 |
| 管理頁開／關 | 真正 HTTP POST 能新增／刪除 Run 值；桌布角色被 403 拒絕；桌布設定 bytes 不變 |
| 登錄指令背景執行 | 結束現有實例後，解析並執行登錄中的同一條指令，收到新實例真實 wallpaper ack，沒有 manager renderer；重複啟動不重建實例 |
| 關閉選擇保留 | 關閉後，普通重新啟動及同版本重裝都維持關閉，沒有重新建立 Run 值 |
| 解除安裝 | 在開啟狀態解除安裝，Run 項目移除、自有核心結束，使用者資料保留；無關 Node sentinel 存活 |
| 再安裝 | 恢復原先開啟的選擇，保留設定，自動套用；其他 Windows 啟動項目前後比對一致 |
| 最終目視 | 使用者確認「看得到，開關與排版正常，桌布時鐘正常」；最後保留已安裝、開關開啟的 0.2.1 |

上述是**登錄與真實指令執行測試，沒有真正重開機／登出／重新登入**，不將它標成完整開機實測。自動證據：[windows-startup.json](../evidence/windows-startup.json)。

## 回歸與版面

- Node：產品 198 passed／2 skipped，共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 通過。
- Rust：12 passed、0 failed；clippy `-D warnings` 與 fmt check 通過。合計 251 個 Rust／Node 測試通過、0 failed、4 skipped。
- 新增前端狀態讀取、立即儲存、錯誤回復、外部狀態更新、非安裝版及桌布不顯示控制測試；另有 native 命令白名單、路徑過濾、寫入序列化、角色／來源／JSON 防護測試。
- 390px neon 與 1280px natural 的英文合成 fixture，透過隔離 Chromium CLI 檢查，沒有卡片／頁面超寬或可見橫向捲軸。此為兩組快速回歸，不稱作重新完成全 44 組；原有 44 組歷史證據保留。新增結果：[startup-layout/report.json](../../implementation-p4_2_5/evidence/startup-layout/report.json)。
- 新增模組對舊有長駐開發管理服務採可選載入，不因其靜態路由尚未知道新檔案而中止原本管理頁；安裝版的 payload 完整性檢查仍保留。

## 安裝產物與界線

`Atmos Wallpaper_0.2.1_x64-setup.exe`：292,172,774 bytes；SHA-256：`8d35022ff684ba7ef748283c9e2c7e306a17738f70ed96ed9afdfb071282a1e5`。內含 Node 24.22.2、Python 3.14.10／Pillow 13.0.0、846 個 payload 檔案及官方離線 WebView2。自有安裝包仍未簽章。

未驗證真正 Windows 登入、Task Manager 停用／政策覆蓋、無 Explorer 的完整等待路徑、全新 VM、Windows 11、多螢幕／DPI、睡眠、GPU／耗電及長時間耗用。既有 4173／41873 服務與私人設定保留；原始測試資料、帳戶啟動項目及 profiles 不推送 Git。不保證零洩漏，也不把先前其他宿主的實測當成本版本數據。
