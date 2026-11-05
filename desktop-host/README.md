# Atmos Wallpaper Windows 0.7.14

Tauri 2＋Rust、Windows x64，提供單目標、各螢幕獨立、重複與全景桌布配置（新安裝預設跟隨主螢幕）。保留現有管理頁、場景、元件與 ICS 日曆，由自己的 WebView2 宿主套用，**不使用 Lively**。

0.7.0 新增具名配置／輪播／固定時段、專注與展示、跨螢幕音訊 owner、FPS／全螢幕／電池政策、真 CPU／RAM、opt-in 模型天氣、素材引用封存、JSON watch 與 `.scr`。功能與自動／人工界線見 [功能驗證矩陣](docs/AUTOMATED-ROADMAP-VALIDATION-2026-11-04.md)。0.7.1 修正 ACK response body 的資源生命週期，並完成本機兩小時程序樹與 GPU 採樣；[結果與限制](docs/WINDOWS-RESOURCE-VALIDATION-2026-11-05.md) 保留記憶體增長與不完整計數器覆蓋，不以單元測試推定安裝版或零洩漏。

0.7.4 採用方案 A 分類分頁：桌面元件、螢幕保護、外觀字型；切換不提交草稿，保留既有儲存與 Windows 螢幕保護設定操作。[分類分頁與驗證](docs/SETTINGS-TABS-2026-11-05.md)。

0.7.5 在上方浮動控制列加入「停用並恢復原桌布」、「重新啟用 Atmos」與實際宿主狀態。停用移除所有 Atmos 桌布層，保留管理頁與既有設定；開機啟動偏好獨立保留。見 [控制與驗證](docs/WALLPAPER-CONTROLS-2026-11-05.md)。

0.7.6 新增預設開啟的 GitHub 每日熱門與 Yahoo 最新新聞元件，各顯示 5 筆；每日抓取、持久快取、離線提示與獨立位置設定。[來源、更新規則與驗證](docs/DAILY-FEEDS-2026-11-05.md)。

0.7.7 將定位落實為「個人資訊桌面」，加入自訂 RSS／Atom、來源管理與資訊卡片配置範本；管理視窗改用新定位名稱，保留安裝目錄與應用程式身分。[操作、界線及驗證](docs/PERSONAL-DASHBOARD-2026-11-05.md)。

0.7.8 新增「資訊省電模式」：上方與效能設定可切換，背景靜態顯示、簡化合成，資訊及設定同步保留；關閉後沿用原特效與 FPS。見 [模式、驗證與資源界線](docs/INFORMATION-SAVING-2026-11-05.md)。

0.7.10 降低快照完整歷史解析、時鐘 Intl 格式器及 DOM 更新的配置壓力；省電政策不再重複清空已暫停的 Canvas，未展開診斷只保留最新證據。包含保留設定的安裝更新、最終時鐘產物與舊版本增長紀錄，見 [記憶體驗證及長測](docs/MEMORY-VALIDATION-2026-11-05.md)。不以 private bytes 單獨診斷 JS 洩漏，不保證任意負載零增長。

0.7.11 減少閒置整頁翻譯／控制項掃描，暫停的番茄鐘及未變動來源文字不再替換節點。既有 `--host-command get` 的 `runtime.rendererMemory` 提供每分鐘原生 heap／DOM 計數與時間戳；只查詢兩個固定 CDP 方法，不開除錯連接埠、不執行任意程式或強制 GC，未知及失敗不填零。原生計數與程序 private bytes 要分開判讀，見同份驗證報告。

## 安裝與操作

0.7.3 修正展開元件內容超出容器、蓋到螢幕保護設定的高度衝突，改為自然內容高度；包含能重現舊版錯誤的幾何回歸與 16 個修正後案例。見 [設定區重疊修正](docs/PANEL-FLOW-FIX-2026-11-05.md)。

0.7.2 已將天氣元件顯示開關、資料更新狀態與「顯示並定位元件」整合到資料來源區，並拉開螢幕保護按鈕間距。本機已完成保留設定的安裝更新、實際資源比對及原生回報檢查，見 [更新驗證](docs/WEATHER-CONTROLS-UPGRADE-2026-11-05.md)。

執行 `Atmos Wallpaper_0.7.11_x64-setup.exe`，完成頁預設勾選啟動 Atmos；按完成後依已保存目標套用桌布，新安裝預設跟隨主螢幕。安裝器包含 Node、隔離的 Python＋Pillow，以及官方離線 WebView2 安裝程式；一般使用者不必另外安裝 Node、Python 或 Lively，也不需要 Rust 開發工具。

- 在目前使用者範圍安裝，預設 `%LOCALAPPDATA%\Atmos Wallpaper`；不新增系統 PATH 或 Windows Service。
- 系統匣「開啟管理頁」調整設定，按「套用變更」保存；再次啟動 Atmos 捷徑也會開啟現有管理頁。
- 系統匣「退出並還原桌布」移除 Atmos 自有層，顯示原桌布。正式啟動沒有原型的十分鐘期限。
- 使用 Windows「應用程式與功能」解除安裝。更新／解除安裝會先正常關閉這份 Atmos，不以通用程序名稱終止 Node、Python 或其他桌布軟體。
- 使用者資料在 `%LOCALAPPDATA%\io.atmos.wallpaper\store`，診斷在同層 `runs`。預設解除安裝保留資料；GUI 的「刪除應用程式資料」選項才會移除對應資料目錄。
- 新增目前使用者的 **Windows 登入後自動啟動並套用桌布**，首次安裝／首次升級此功能預設開啟。管理頁「Windows 啟動」可開關，立即生效，不需要按桌布「套用變更」。關閉後重啟／重裝仍保留選擇。
- 登入啟動只在背景套用，不彈出管理頁；若程式已運作，不重建桌布、不搶焦點。這是登入目前帳戶後啟動，不是在登入前或 Windows 鎖定畫面顯示桌布。
- 系統匣退出只停止這次執行，不取消下次登入啟動；解除安裝則移除屬於這份安裝的啟動項目。開關選擇留在 HKCU 的 Atmos 偏好中，供重裝保留。
- 新安裝不會自動搬移舊開發 store，也不會關掉已在使用的舊管理服務；可另以管理頁的備份／還原搬移設定，ICS 來源須另行匯入。

安裝包尚未做發行者程式碼簽章，Windows 可能顯示未識別發行者。Microsoft WebView2 子安裝程式本身有有效 Microsoft 簽章。安裝包附 `SHA256SUMS.txt`，雜湊不能取代發行者簽章。

開關讀寫 Windows 目前帳戶的 `Run` 登錄，並非只改畫面或瀏覽器儲存。Windows「啟動應用程式」與組織政策仍可能覆蓋登錄；程式不修改 StartupApproved 或繞過政策。介面以「已登錄登入啟動」描述狀態，不把它當成已觀測到真正登入啟動。

## 從源碼重建安裝程式

以下僅是**開發／重建**先決條件：Rust stable MSVC、Microsoft C++ Build Tools＋Windows SDK、Node >=22、Python（解壓縮用標準函式庫）。`Cargo.lock`、npm lock 及 `installer/runtime-lock.json` 鎖定依賴；下載的執行環境會核對 SHA-256。

```powershell
cd implementation-p4_2_5
npm ci --ignore-scripts
cd ../desktop-host
npm ci --ignore-scripts
npm run build:windows
```

建置腳本預設使用 `C:\Python314\python.exe`，可由 `ATMOS_BUILD_PYTHON` 指定既有 Python；Git 可用 `ATMOS_GIT` 指定。下載快取位於 `%LOCALAPPDATA%\AtmosWallpaper\installer-build-cache`。建置會下載約 200 MiB 的官方離線 WebView2 安裝程式；使用者安裝時不需再下載此元件。

輸出在 `target/release/bundle/nsis/`。生成的 `payload`、`target`、安裝包、使用者資料與原始測量紀錄不提交 Git；發佈二進位檔應另附對應雜湊。第三方授權放在安裝內容的 `payload/licenses` 及各執行環境目錄。

payload 只包含已追蹤的產品源碼、凍結素材、生成的 web assets、production npm 依賴、官方 Node／Python／Pillow；不複製本機 store、帳號或設定。啟動時核對 payload 內容與必要檔案；內容損壞會拒絕套用。這是完整性檢查，不是不可竄改的簽章機制。

## 診斷與測試

已安裝的程式提供以下原生 CLI，不需要 UI 自動化：

```powershell
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --check-install
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --inspect-desktop
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --manager
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --quit
& "$env:LOCALAPPDATA\Atmos Wallpaper\atmos-wallpaper.exe" --autostart status
```

`--check-install` 驗證 payload、內嵌 Node／Python／Pillow，不套用桌布。`--quit` 使用同使用者、同 session、同執行檔路徑的原生 event 關閉自身實例，不暴露網頁 shell／檔案權限。

`--autostart enable`／`disable` 是原生設定操作；管理頁只透過有 manager session、同來源驗證的白名單 API 呼叫這兩種操作。桌布角色不具備這項權限。啟動偏好獨立於桌布設定，不包含於設定草稿、備份或離線匯出。

```powershell
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
node scripts/test-installer.mjs --install-and-apply
node scripts/test-startup.mjs --configure-login-startup
```

整合測試會實際安裝、套用、重裝、解除安裝，最後重新安裝並保留 Atmos 運作。為避免覆寫既有使用者安裝，測試預設拒絕已有的 Atmos 安裝。原生四情境測試另見 `scripts/test-windows-host.mjs --apply-to-primary`，可用 `ATMOS_HOST_EXE` 指定 release／已安裝執行檔；執行前須先退出 Atmos。兩者只用 CLI、真實 renderer ack、Win32 metadata 與程序／port 檢查，不操作瀏覽器或桌面 UI。

如需觀察既有 Atmos 桌布在測試前後的回報，可設定 `ATMOS_BASELINE_PORT`；這只做唯讀健康檢查，不是產品相依。沒有既有管理服務也可以測安裝版。Git clone 的封裝採追蹤檔案列表；來源 ZIP 沒有 `.git` 時，改採 SOURCE-MANIFEST 的已發佈檔案列表。

登入啟動測試需要已有的安裝，會重設 Atmos 自己的新功能偏好作為測試起點、切換實際登錄、重裝／解除安裝，最後保留已安裝且啟動開關開啟的程式。它會執行登錄中的同一條指令，**不重開機、不登出，也不將此模擬呼叫標成真正 Windows 登入實測**。

保留 `scripts/run-prototype.ps1 -RunSeconds 600` 的開發模式：明確指定既有 Node／Python 與獨立測試 store，期限 5..3600 秒，不代表安裝版仍需系統執行環境。

## 原生桌面與驗證界線

- 真正掛到背景 WorkerW，驗證 native parent 與主螢幕實體尺寸；不以一般無邊框視窗替代。
- 沿用有效 WorkerW，只有缺少時才要求 Explorer 建立背景層。WorkerW 是非穩定公開合約，無法宣稱適用所有 Shell 版本。
- 不覆寫 Windows 桌布設定，不改其他程式視窗樣式／可見性；退出只移除自己的層。原本就故障的其他播放器仍須自行恢復。
- 自有本機核心先通過啟動 gate，再放入 kill-on-close Job；宿主退出／被終止時清理自有核心。WebView2 亦有真實清理檢查。
- 真實 renderer ack 與 native 掛載分開記錄；初始化無 ack 會失敗退出。啟動完成後遇到鎖屏／遮蔽節流只記錄 stale，不因短期無 ack 永久退出；不把 stale 當成畫面正常。
- 管理頁與桌布角色仍分離，未授予網頁任何原生檔案／shell capabilities；導覽限制於自己的 loopback 頁面。

Windows 10 x64 build 19045 已實測；詳見 [登入啟動報告](docs/WINDOWS-STARTUP-VALIDATION-2026-11-04.md)、[0.2.0 安裝版報告](docs/WINDOWS-INSTALLER-VALIDATION-2026-11-04.md)、[先前原型報告](docs/WINDOWS-PROTOTYPE-VALIDATION-2026-11-04.md) 與 [0.7.1 兩小時資源／GPU 報告](docs/WINDOWS-RESOURCE-VALIDATION-2026-11-05.md)。真正重開機／重新登入、Windows 11、全新無 WebView2 的 VM、混合 DPI／熱插拔、Explorer 重啟、真正睡眠、瓦特／焦耳與其他負載長測仍有未驗證項。GPU 診斷不代表已有 GPU 元件資料源；macOS 桌布 adapter 尚未實作。

0.2.2 修正天氣等元件拖曳後套用會回原位，並保留方向鍵、座標欄位與取消拖曳行為。見 [位置修正驗證](docs/WIDGET-POSITION-FIX-2026-11-04.md)。

0.2.3 讓管理頁套用工具列在捲動時固定於頂端，窄屏自動換行。見 [浮動工具列驗證](docs/STICKY-TOOLBAR-2026-11-04.md)。

0.3.0 新增本機 Agent JSON API：--agent-help、--agent-schema、--get-state、--validate-json、--apply-json、--get-result。使用內嵌執行環境與既有交易核心，保存與真實桌布回報分開判讀。[完整教學與規則](docs/AGENT-API.md)。

Agent 實機整合測試（會暫時變更強度再以交易還原）：node scripts/test-agent-installed.mjs --apply-and-restore。結果與界線見 [0.3.0 驗證報告](docs/AGENT-API-VALIDATION-2026-11-04.md)。

0.3.1 將預覽解析度、寬高、套用按鈕與語言改為桌面單排控制列；窄屏仍自動換行，避免擠壓。[版面驗證](docs/PREVIEW-TOOLBAR-2026-11-04.md)。

0.3.2 為浮動套用工具列加入頂部 12px 留白與安全區偏移，小視窗保留底部空間。[間距驗證](docs/TOOLBAR-GAP-2026-11-04.md)。

0.3.3 修正 F5 重新載入時短暫露出舊版佔位畫面：等待設定、圖片、字型與版面就緒後一起顯示，失敗保留可重試提示。[重新載入驗證](docs/RELOAD-BOOT-2026-11-04.md)。

0.3.4 恢復左預覽／右控制面板的原本雙欄佈局，控制面板與預覽解析度區塊頂端貼齊；寬螢幕欄位並排，窄視窗在左欄內換行。[還原佈局驗證](docs/RESTORED-COLUMNS-2026-11-04.md)。

0.3.5 安裝版管理視窗隱藏重複的 Atmos 頁首與說明列，使用原生視窗標題；單獨瀏覽器開啟仍保留頁面標題。原雙欄與對齊不變。[頁首驗證](docs/NATIVE-MANAGER-HEADER-2026-11-04.md)。

0.3.6 說明卡片依文字內容決定高度，不再被右側面板撐高；預覽比例與雙欄頂端對齊保留。[說明卡片驗證](docs/COMPACT-EXPLANATION-2026-11-04.md)。

0.3.7 修正浮動工具列提示文字在深色風格下對比不足：使用獨立深色文字、完整不透明度與 11px 字級。[文字對比驗證](docs/TOOLBAR-CONTRAST-2026-11-04.md)。

0.3.8 換用原創 Atmos 圖示：山形 A、流動地平線與日光時鐘；應用程式、系統匣、安裝與解除安裝圖示使用同一來源。[圖示設計與驗證](docs/APP-ICON-2026-11-04.md)。

0.3.9 新增極光山景與品牌軌道 loading，使用自包含 SVG／CSS、真實載入階段、低動態與錯誤重試狀態，不刻意延長等待。[動畫設計與驗證](docs/ANIMATED-LOADING-2026-11-04.md)。

0.4.0 新增單目標螢幕選擇、相對配置圖、立即套用／清除、獨立宿主 JSON/CAS 與原生 Agent 教學。目前同時維持一個桌布視窗。[功能與驗證](docs/DISPLAY-PLACEMENT-2026-11-04.md)；[後續批次](docs/ROADMAP-EXECUTION-2026-11-04.md)。

0.5.0 加入匯入素材的獨立目錄、原生資料夾選擇、背景複製／逐檔核對／原子切換、離線修復與 Agent 素材庫命令。保留所有原資料，不自動刪除。[素材庫驗證與限制](docs/LIBRARY-VALIDATION-2026-11-04.md)。

0.6.1 加入 Rust/CPAL/WASAPI 原創合成音源、總音量／靜音、桌面焦點政策、實際播放裝置選擇與主動三秒測試音。使用者已確認雨聲、音量／靜音及焦點切換。[音訊驗證與限制](docs/AUDIO-VALIDATION-2026-11-04.md)。版本更新請使用 `node scripts/set-version.mjs --write X.Y.Z`，避免誤改相依版本。

0.7.12 修正元件展開箭頭、標頭密度及工具列僅捲動後吸頂的外觀，保留目前布局與設定。[驗證說明](docs/UI-REFINEMENT-2026-11-06.md)。

0.7.13 將管理工具列在寬版合為一列，使用目前主題配色與一致按鈕，保留捲動後吸頂與各設定保存邊界。[驗證](docs/COMPACT-TOOLBAR-2026-11-06.md)。

0.7.14 將每日資訊元件整合進一般桌面元件清單，統一外觀並保留既有儲存邊界。[驗證](docs/FEED-WIDGET-LIST-2026-11-06.md)。
