# Windows 驗證紀錄：2026-11-02

環境：Windows、Node 24.13.0、Python 3.14、Pillow 12.1.0。所有寫入與服務測試使用獨立暫存 store，未使用正式資料或變更 OS 桌布。

## 修正

- P1 原子寫入在 Windows 不再對目錄執行不支援的 fsync；仍對檔案執行 fsync，仍先寫暫存檔再 rename；POSIX 保留目錄 fsync。Windows 的目錄持久化保證仍與 POSIX 不同。
- P1 圖片解碼支援既有的 `ATMOS_PYTHON`、constructor 選項與 Windows `python` 預設，避免只能使用 POSIX 的 `python3` 名稱。
- 螢幕讀取使用 Windows 系統目錄中的 PowerShell，不依賴 PATH。實際偵測到兩個 1920×1080、100% 縮放螢幕，選取主螢幕 DISPLAY2。
- P1 測試明確區分無權建立的檔案 symlink 與 POSIX FIFO，並增加可獨立執行的 Windows junction 拒絕測試。
- 修補獨立草稿檢查的過期 VM fixture，及兩個檢查缺少 evidence 目錄時的寫入失敗。
- Git 固定 LF，避免 Windows checkout 改寫凍結基底／fixture 的 bytes，破壞完整性雜湊。

## 自動檢查

`node scripts/verify.mjs`：prepare、產品測試、核心測試、獨立草稿檢查、獨立路徑檢查、CLI help 全部成功。

| 測試 | 通過 | 失敗 | 跳過 |
| --- | ---: | ---: | ---: |
| 產品（含新增 Windows 真實 HTTP runtime 測試） | 166 | 0 | 2 |
| 共用 P1 核心 | 41 | 0 | 2 |

兩套各跳過檔案 symlink（目前權限不足）及 POSIX FIFO（Windows 不適用）。Windows junction 拒絕測試實際通過，未調整系統權限。測試涵蓋圖片驗證、不可變資產、草稿／套用、重試與衝突、備份／還原／復原、舊資料遷移、離線匯出、時鐘與日期、五語、位置與比例、bulk visibility、角色與輸入邊界。

新增 HTTP 測試啟動真正的 Windows 服務，確認 manager／wallpaper 入口、CSP、不同 session token、未驗證讀取拒絕、wallpaper 寫入拒絕、manager 寫入、相同 request replay、磁碟 revision、靜態 JS、超大 request body 拒絕，最後關閉程序並清除該測試的暫存 store。

## 瀏覽器與 Windows 實機

- 本機 loopback 服務正常啟動；manager 與唯讀 wallpaper 同步同一個 revision。
- 全部顯示、全部隱藏、草稿取消與套用；套用後重新整理仍保留設定。
- 四種風格、六個場景、五種程式天氣均透過真實 UI 操作；風格及場景等待非同步載入完成。
- 五語切換；12 小時制、隱藏秒數可提交並在 wallpaper 顯示。
- 待辦新增、完成、刪除、還原；番茄鐘開始、暫停；服務停止重啟後保留已完成待辦、暫停剩餘時間及其他設定（r3）。
- 使用專案 review iframe 檢查 1876×1080 與 390×844；截圖保存在 evidence。直接 manager 與 wallpaper 未觀察到 console error／warning。
- review 包裝頁的瀏覽器工具記錄出現 MutationObserver 的 Node 類型錯誤；該頁只載入 review.js，錯誤未阻止 iframe 渲染，直接產品頁未重現。此項保留為瀏覽器檢測工具／包裝頁限制，未據此宣稱所有瀏覽器零錯誤。

## 尚不能宣稱通過的範圍

Google／Microsoft OAuth、即時天氣、CPU／RAM／電量／網路、Now Playing 仍是產品明示的 demo／N/A，並未新增真整合。未操作 Lively 或其他 Windows 桌布宿主；OS 套用、鎖屏、休眠恢復、多螢幕實際桌布、長時間資源消耗及所有瀏覽器相容性尚未完成驗證。測試全綠不等於能保證任何環境永遠沒有問題。

機器可讀結果：`../evidence/verification.json`；完整測試輸出：本機 `../evidence/*.log`。
