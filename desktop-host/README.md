# Atmos Wallpaper Windows 安裝版 0.2.0

Tauri 2＋Rust、Windows x64、單主螢幕桌布。保留現有管理頁、場景、元件與 ICS 日曆，由自己的 WebView2 宿主套用，**不使用 Lively**。

## 安裝與操作

執行 `Atmos Wallpaper_0.2.0_x64-setup.exe`，完成頁預設勾選啟動 Atmos；按完成後自動套用主螢幕桌布。安裝器包含 Node、隔離的 Python＋Pillow，以及官方離線 WebView2 安裝程式；一般使用者不必另外安裝 Node、Python 或 Lively，也不需要 Rust 開發工具。

- 在目前使用者範圍安裝，預設 `%LOCALAPPDATA%\Atmos Wallpaper`；不新增系統 PATH 或 Windows Service。
- 系統匣「開啟管理頁」調整設定，按「套用變更」保存；再次啟動 Atmos 捷徑也會開啟現有管理頁。
- 系統匣「退出並還原桌布」移除 Atmos 自有層，顯示原桌布。正式啟動沒有原型的十分鐘期限。
- 使用 Windows「應用程式與功能」解除安裝。更新／解除安裝會先正常關閉這份 Atmos，不以通用程序名稱終止 Node、Python 或其他桌布軟體。
- 使用者資料在 `%LOCALAPPDATA%\io.atmos.wallpaper\store`，診斷在同層 `runs`。預設解除安裝保留資料；GUI 的「刪除應用程式資料」選項才會移除對應資料目錄。
- 本版沒有設定登入自啟；安裝完成啟動與登入自啟是不同功能。
- 新安裝不會自動搬移舊開發 store，也不會關掉已在使用的舊管理服務；可另以管理頁的備份／還原搬移設定，ICS 來源須另行匯入。

安裝包尚未做發行者程式碼簽章，Windows 可能顯示未識別發行者。Microsoft WebView2 子安裝程式本身有有效 Microsoft 簽章。安裝包附 `SHA256SUMS.txt`，雜湊不能取代發行者簽章。

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
```

`--check-install` 驗證 payload、內嵌 Node／Python／Pillow，不套用桌布。`--quit` 使用同使用者、同 session、同執行檔路徑的原生 event 關閉自身實例，不暴露網頁 shell／檔案權限。

```powershell
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
node scripts/test-installer.mjs --install-and-apply
```

整合測試會實際安裝、套用、重裝、解除安裝，最後重新安裝並保留 Atmos 運作。為避免覆寫既有使用者安裝，測試預設拒絕已有的 Atmos 安裝。原生四情境測試另見 `scripts/test-windows-host.mjs --apply-to-primary`，可用 `ATMOS_HOST_EXE` 指定 release／已安裝執行檔；執行前須先退出 Atmos。兩者只用 CLI、真實 renderer ack、Win32 metadata 與程序／port 檢查，不操作瀏覽器或桌面 UI。

如需觀察既有 Atmos 桌布在測試前後的回報，可設定 `ATMOS_BASELINE_PORT`；這只做唯讀健康檢查，不是產品相依。沒有既有管理服務也可以測安裝版。Git clone 的封裝採追蹤檔案列表；來源 ZIP 沒有 `.git` 時，改採 SOURCE-MANIFEST 的已發佈檔案列表。

保留 `scripts/run-prototype.ps1 -RunSeconds 600` 的開發模式：明確指定既有 Node／Python 與獨立測試 store，期限 5..3600 秒，不代表安裝版仍需系統執行環境。

## 原生桌面與驗證界線

- 真正掛到背景 WorkerW，驗證 native parent 與主螢幕實體尺寸；不以一般無邊框視窗替代。
- 沿用有效 WorkerW，只有缺少時才要求 Explorer 建立背景層。WorkerW 是非穩定公開合約，無法宣稱適用所有 Shell 版本。
- 不覆寫 Windows 桌布設定，不改其他程式視窗樣式／可見性；退出只移除自己的層。原本就故障的其他播放器仍須自行恢復。
- 自有本機核心先通過啟動 gate，再放入 kill-on-close Job；宿主退出／被終止時清理自有核心。WebView2 亦有真實清理檢查。
- 真實 renderer ack 與 native 掛載分開記錄；初始化無 ack 會失敗退出。啟動完成後遇到鎖屏／遮蔽節流只記錄 stale，不因短期無 ack 永久退出；不把 stale 當成畫面正常。
- 管理頁與桌布角色仍分離，未授予網頁任何原生檔案／shell capabilities；導覽限制於自己的 loopback 頁面。

Windows 10 x64 build 19045 已實測；詳見 [安裝版報告](docs/WINDOWS-INSTALLER-VALIDATION-2026-11-04.md) 與 [先前原型報告](docs/WINDOWS-PROTOTYPE-VALIDATION-2026-11-04.md)。Windows 11、全新無 WebView2 的 VM、混合 DPI／多螢幕、Explorer 重啟、真正睡眠、GPU／耗電與長時間測試仍有未驗證項。macOS 桌布 adapter 尚未實作。
