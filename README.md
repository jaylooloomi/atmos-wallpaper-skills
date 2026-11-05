# Atmos — 你的個人資訊桌面

Atmos 是可客製化的個人資訊桌面（Personal Dashboard）：把關心的資訊、生活安排與系統狀態，放進自己設計的動態桌布。動態桌布提供氛圍，元件匯集資料，JSON／Agent 介面負責自動化。採本機優先架構，保留既有桌布、日曆及管理能力。

## 最新功能

- 0.7.27 將上方操作列中文按鈕縮短為「套用、取消、停用、啟用、重查」，隱藏的重新套用／重試也不超過四字，完整語意保留於無障礙名稱與提示。[驗證](desktop-host/docs/SHORT-BUTTONS-2026-11-06.md)。

- 0.7.26 在「桌面元件 → 播放資訊」加入 YouTube 網址與播放按鈕，支援影片／分享／播放清單及起播時間，使用官方可見小播放器。切換設定卡片時仍可見，停止、頁面隱藏或關閉管理頁會停止；不需 OAuth。[功能與驗證](desktop-host/docs/YOUTUBE-PLAYER-2026-11-06.md)。

- 0.7.25 將語言選單移至上方操作列、桌布確認按鈕旁，移除原本預覽區的語言容器；保留切換事件、草稿與交易中鎖定行為。[驗證](desktop-host/docs/TOOLBAR-LANGUAGE-2026-11-06.md)。

- 0.7.24 簡化控制列：移除本機核心標題、一般草稿／同步提示與已掛載／設定版本文字，僅於處理中、失敗或衝突時顯示必要狀態，保留操作按鈕。[驗證](desktop-host/docs/QUIET-TOOLBAR-2026-11-06.md)。

- 0.7.23 精簡常駐技術提示：移除預覽螢幕資訊與比例說明、重複的分頁／宿主說明；保留實際比例計算、錯誤及儲存狀態，未偵測時由選單標示預設比例。[驗證](desktop-host/docs/HINT-CLEANUP-2026-11-06.md)。

- 0.7.22 移除控制列「說明」按鈕與展開文字，保留套用、取消、桌布與省電操作。[驗證](desktop-host/docs/TOOLBAR-HELP-REMOVAL-2026-11-06.md)。

- 0.7.21 移除管理頁右上角重複的核心同步角標；連線錯誤仍顯示於既有控制列，內部同步與桌布恢復保留。[驗證](desktop-host/docs/MANAGER-BADGE-REMOVAL-2026-11-06.md)。

- 0.7.20 將桌面元件改為三欄選取卡片，點選後於整個卡片區下方顯示單一設定區，取消元件收合箭頭；窄欄自動改為兩欄或一欄，保留開關與未儲存草稿。[驗證](desktop-host/docs/WIDGET-GRID-2026-11-06.md)。

- 0.7.19 以「桌布與系統設定」取代原螢幕保護分頁位置，集中既有系統控制；保留桌面元件與外觀字型分頁，不恢復螢幕保護功能。[驗證](desktop-host/docs/SYSTEM-SETTINGS-TAB-2026-11-06.md)。

- 0.7.18 將介面風格移至「桌面元件」分頁最上方，保留等寬與 2×2 卡片排列、既有切換及草稿行為。[驗證](desktop-host/docs/THEME-DESKTOP-TAB-2026-11-06.md)。

- 0.7.17 移除管理頁的外部 AI 指令產生區與原始技術診斷區，保留 JSON／CLI 與內部真實畫面回報。[驗證](desktop-host/docs/MANAGER-PANELS-REMOVAL-2026-11-06.md)。

- 0.7.16 依使用者要求移除螢幕保護功能：管理頁只保留桌面元件／外觀字型，停止提供 `.scr` 與啟用 API；升級會清理舊 Atmos 選取與檔案，其他 Windows 設定保留。[移除與驗證](desktop-host/docs/SCREENSAVER-REMOVAL-2026-11-06.md)。

- 0.7.15 修正外觀分頁「介面風格」額外縮排，與同欄設定卡片等寬，保留四張風格卡排列。[驗證](desktop-host/docs/THEME-WIDTH-2026-11-06.md)。

- 0.7.14 將 GitHub 熱門、Yahoo 新聞與 RSS 整合至桌面元件清單，與番茄鐘／播放資訊使用相同卡片，保留獨立儲存。[驗證](desktop-host/docs/FEED-WIDGET-LIST-2026-11-06.md)。

- 0.7.13 精簡管理工具列：寬版單列、跟隨主題配色、統一按鈕／省電開關，說明預設收起；錯誤與儲存狀態仍顯示。[驗證](desktop-host/docs/COMPACT-TOOLBAR-2026-11-06.md)。

- 0.7.12 管理頁修正：元件展開改用一致 SVG；工具列初始保持正常版面，捲動吸頂後才顯示陰影。[修正與驗證](desktop-host/docs/UI-REFINEMENT-2026-11-06.md)。

- 0.7.11 資源最佳化：快照只保留有期限的 head 快取，時鐘、暫停計時與閒置狀態沿用文字節點；減少整頁翻譯／控制項掃描，並提供原生唯讀 heap／DOM 計數。[驗證、實測與限制](desktop-host/docs/MEMORY-VALIDATION-2026-11-05.md)。

- 資訊省電模式：背景靜態顯示、簡化合成，時鐘／行程／資訊更新保留；管理頁可即時切換。[模式與本機短測](desktop-host/docs/INFORMATION-SAVING-2026-11-05.md)。

- 個人資訊桌面：GitHub／Yahoo 每日資訊，加上自訂 RSS／Atom、更新頻率、來源狀態與三種資訊卡片配置範本。[定位、操作與 Agent 介面](desktop-host/docs/PERSONAL-DASHBOARD-2026-11-05.md)。

前端修改請執行 `implementation-p4_2_5` 的 `npm run verify`：已包含 12 情境／336 狀態的排版回歸，Windows 封裝亦強制執行。[排版測試與規則](implementation-p4_2_5/docs/LAYOUT-REGRESSION.md)。

- 「全部顯示／全部隱藏」控制七類元件；只改顯示狀態，不重排已存位置。
- 元件設定使用真正行內展開，移除重複分隔線。
- 五語介面（繁中、簡中、英文、日文、韓文）、12／24 小時及秒數設定。
- 管理頁 `/manager` 的修改先成草稿，再按套用提交；`/wallpaper` 為唯讀顯示頁。
- 本機圖片匯入、備份、還原、版本遷移、離線匯出與受限 CLI。
- Windows x64 0.7.11：Tauri 2＋Rust、獨立 WebView2、托盤與退出還原；安裝後預設啟動並套用主螢幕，可在管理頁選單一、獨立、重複或全景配置／清除 Atmos 層，提供宿主 JSON/CAS 與原生 Agent 教學；支援登入 Windows 後背景自動套用與管理頁即時開關。內含 Node／Python／Pillow 與離線 WebView2 安裝程式，不需另外安裝 Node、Python 或 Lively。[安裝、重建與限制](desktop-host/README.md)。

- 本機具名外觀配置／收藏、托盤快捷、專注／展示、間隔輪播與固定時段；保留私人待辦／行程來源與計時進度。
- JSON 檔案監看與外觀／排程／模式受限 CLI；明確 opt-in，不直接改 live.json、不建立開機 watcher。
- 原生音訊 owner 路由、FPS／全螢幕／電池動畫政策、引用審核與可恢復素材封存。螢幕保護已於 0.7.16 移除。
- 明確啟用的 Open-Meteo 模型天氣與 Windows 真 CPU／RAM；網速、GPU、耗電、Now Playing 尚未提供。
- 「真實資料來源」內可直接切換天氣元件顯示、查看資料取得／模型時間與離線狀態，並顯示及定位元件；顯示修改沿用草稿／套用流程，線上連線仍需獨立明確啟用。
- 遠方雲光／寫實分岔回閃、AI 指令與診斷自然接在左側預覽下方，保留原本雙欄與目標比例。

依使用者指示先做自動測試。0.7.1 修正未消費 ACK 回應造成的 renderer Section／handle 累積，已完成本機「主螢幕＋管理頁」兩小時 CPU／記憶體／handles／GPU 資源採樣，481 次全目標回報正常。記憶體增長診斷與計數器缺口仍如實保留，不宣稱零洩漏；瓦特／焦耳未測得，人工鎖屏／登入、物理插拔與其他硬體環境仍未驗證。GPU 目前僅提供診斷採樣，尚未串接元件資料源。[本機資源實測與限制](desktop-host/docs/WINDOWS-RESOURCE-VALIDATION-2026-11-05.md)、[完整功能／驗證矩陣](desktop-host/docs/AUTOMATED-ROADMAP-VALIDATION-2026-11-04.md)、[外觀與排程教學](implementation-p4_2_5/docs/APPEARANCE-AUTOMATION.md)。

## 目錄

```text
implementation-p4_2_5/    目前產品：src、scripts、tests、CLI、Windows 入口與技術文件
desktop-host/            Tauri 2＋Rust Windows 多螢幕安裝版、封裝工具與原生整合測試
implementation-p1/       共用核心、解碼器與基礎回歸測試（仍為必要依賴）
implementation-p4_2_1/src/ 舊格式相容／遷移測試所需凍結實作
prototype-v8.1-frozen/   凍結前端基底、字體授權與原始場景素材
```

請保持上述相對路徑；不能只複製 `implementation-p4_2_5`。保留凍結基底 manifest 原貌；歷史交付／QA 私人筆記不在公開倉庫中。沒有收錄 runtime store、私密帳號資料、token、`node_modules` 或生成的 `dist`。

## 重建及測試

環境：**Node.js >= 22**、已安裝 **Python 3 + Pillow**。ICS 日曆新增 npm 依賴；首次請在產品目錄執行 `npm ci --ignore-scripts`，之後再建置與測試。請先自行備妥依賴；腳本不會代為安裝。

```sh
cd implementation-p4_2_5
npm ci --ignore-scripts
npm run prepare -- --assets-only
npm test
node bin/atmos.mjs --help
```

`--assets-only` 只重建 `dist/`，不初始化 demo store。一般 `npm run prepare` 另會在缺少時建立本機 demo store。基礎核心回歸測試可從 `implementation-p1` 執行 `npm test`。

## Windows 本機啟動

以下是既有 Windows 本機網頁服務入口；它本身不會套用 OS 桌布。新增的原生宿主另見 [desktop-host](desktop-host/README.md)。請確認 Node、Python/Pillow 已存在並指向正確路徑。以下由你在自己的 Windows 命令提示字元執行：

```bat
cd implementation-p4_2_5
node scripts\prepare.mjs --assets-only
set "ATMOS_STORE=%LOCALAPPDATA%\AtmosWallpaper\data"
set "ATMOS_PYTHON=C:\你的既有Python路徑\python.exe"
node src\windows-runtime.mjs --host 127.0.0.1 --port 4173 --strictPort --start-local-service
```

開啟 `http://127.0.0.1:4173/manager`；桌布顯示 URL 為 `http://127.0.0.1:4173/wallpaper`。服務僅監聽 loopback，按 Ctrl+C 停止。`Start-Atmos.cmd` 預設使用 `C:\Python314\python.exe`，使用前請檢查並調整成實際路徑。腳本不安裝工具、不建立自動啟動、不改防火牆，也不會替你設定 Lively 或 OS 桌布。

`npm run dev` 是受保護的官方 Sites managed preview 入口，不是一般 Linux／macOS 桌面啟動指令；不要移除其 guard 或偽造環境旗標。

## 真實功能與示範界線

- 真實：本機設定與草稿／套用、版本衝突檢查、圖片驗證、素材呈現、備份／還原、格式遷移、renderer 連線與離線匯出。
- **仍為 demo／N/A**：Google／Microsoft 日曆 OAuth、網速、GPU／電量／耗電資料及 Now Playing。示範事件不是私人行程；模型天氣需明確啟用（免費 hosted API 僅非商業、資料 CC BY 4.0），CPU／RAM 在 Windows 安裝宿主可取得真值。
- AI Prompt 只產生可複製文字，不會自動呼叫外部 AI、登入帳號或更新桌布。
- renderer ack 與原生視窗掛載分開報告，兩者都不等於人眼可見性證明；鎖屏、真機混合 DPI、耗電與完整硬體／登入生命週期仍有未驗證項。
- scoped session token 用於防誤寫／能力檢查，不是惡意同源 JavaScript 的安全隔離。

詳見 [本版文件](implementation-p4_2_5/README.md)、[本機 CLI 與恢復流程](implementation-p4_2_5/docs/WINDOWS-CLI-AND-RECOVERY.md) 與 [全部顯示規則](implementation-p4_2_5/docs/P425-BULK-VISIBILITY.md)。

## 本次驗證（2026-11-01）

對公開倉庫的乾淨選取源碼重新執行：prepare 成功、產品測試 **167/167**、共用核心 **42/42**、CLI help 成功；生成的 35 個 dist 檔與正式凍結版本逐位元組一致。獨立正式 UI QA 另驗證七卡顯示／隱藏與草稿／套用、非顯示資料及 timer deadline 保留、行內展開／分隔線、五語工具列、Enter／Space，以及 390×844 和 1876×1080 畫面。這是本批影響範圍驗證，不代表所有歷史瀏覽器測試或全平台通過。未重跑 Windows 實機或真外部服務整合。

## Windows 重跑驗證（2026-11-02）

在 `implementation-p4_2_5` 執行 `node scripts/verify.mjs`，可依序重建素材、執行產品及共用核心測試、獨立草稿／路徑檢查及 CLI help；結果寫入 `evidence/verification.json`，各項完整輸出寫入被 Git 忽略的 `.log`。Windows 請先設定 `ATMOS_PYTHON` 為已安裝的 Python 完整路徑，並確認 Pillow 可用。

本次 Node 24.13.0、Python 3.14、Pillow 12.1.0：產品測試 166 通過、2 跳過；核心測試 41 通過、2 跳過，均無失敗。跳過項為缺少建立檔案符號連結權限，以及 Windows 不適用的 POSIX FIFO；目錄 junction 拒絕另有實際通過案例。詳細修正、瀏覽器檢查與驗證界線見 [Windows 驗證紀錄](implementation-p4_2_5/docs/WINDOWS-VERIFICATION-2026-11-02.md)。

## 素材與授權

字體授權見 [FONT-LICENSE.txt](prototype-v8.1-frozen/assets/FONT-LICENSE.txt)。本次匯入未擅自新增整個專案的開源授權；公開可見不等於授予任意再授權。

## ICS 日曆（2026-11-03）

已實作免登入的 ICS 檔案匯入與 HTTPS／webcal 訂閱，顯示未來七天行程。開啟管理頁的日曆面板，匯入或訂閱後按「套用變更」。Google／Microsoft OAuth 仍未實作。操作、私密連結注意事項與依賴安裝見 [ICS 日曆說明](implementation-p4_2_5/docs/ICS-CALENDAR.md)。

本次耗用測試依使用者要求停止，僅取得約 36 分鐘資料，未完成兩小時、也未證明無洩漏；鎖屏由本人確認恢復，真睡眠受韌體不支援限制。[實機結果](implementation-p4_2_5/docs/WINDOWS-HOST-VALIDATION-2026-11-03.md)。

## 場景排版與獨立宿主規劃

場景區位於預覽解析度上方；預覽／語言與元件工具列在空間不足時換行，不依賴橫向捲軸。[實際 Chromium 排版量測](implementation-p4_2_5/docs/LAYOUT-VERIFICATION-2026-11-03.md) 可用 `npm run audit:layout` 重跑。

[Lively 功能盤點與 Atmos 獨立宿主規劃](implementation-p4_2_5/docs/LIVELY-AND-STANDALONE-HOST.md) 說明如何保留目前網頁並新增自己的 Windows 桌布層。這是已完成的原始碼研究與開發規劃；目前尚未提供不依賴 Lively 的原生宿主／安裝器。

## Agent JSON 控制

Windows 0.3.0 提供原生命令讀取狀態、JSON 預檢／提交與 receipt 查詢，並以 --agent-help 和 --agent-schema 自我描述。[操作教學、格式與規則](desktop-host/docs/AGENT-API.md)。不需點擊管理頁，不直接改寫內部 live.json。

## 後續功能規劃

多螢幕、放置模式、音訊、素材目錄與 Agent 後續擴充的盤點、執行順序及驗收條件，統一維護於 [TODO](TODO.md)。未勾選項目尚未實作。


## 0.7.0 自動交付（2026-11-04）

原始待辦功能與五批推薦擴充已實作；Node 393、Rust 48 項通過，另有 4 項平台能力略過。真正 Windows 桌布 8、螢幕保護 7、正向 `/p` 預覽 2、安裝版 CLI 4 項通過。安裝升級保留既有外觀、素材位置、聲音、顯示器及登入啟動偏好；原生層、renderer ack 與可見性分開記錄。安裝包／`.scr`／runtime 不提交 Git。新控制區部分字串仍以繁中為主。

本輪依要求未安排人工測試；GPU／電力、長期耗用與實體裝置／登入生命週期等未驗證項保留在[完整矩陣](desktop-host/docs/AUTOMATED-ROADMAP-VALIDATION-2026-11-04.md)。
