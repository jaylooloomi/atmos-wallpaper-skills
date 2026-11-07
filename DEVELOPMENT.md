# Atmos 開發與交付

## 日常修改

先確認 HEAD、工作區與 AGENTS.md。編輯維護中的 `implementation-p4_2_5/src/`／scripts，不直接修改 dist，也不改凍結原型。日常工作不自動升版、打包、安裝、變更真實設定、推送或部署。先前習慣性的 release 流程不是新任務的授權。

### 最新驗證政策（使用者於 2026-11-07 再次調整）

| 時機 | 驗證方式 |
| --- | --- |
| 日常開發／修改 | 相關針對性單元與整合測試；UI 加手機／桌面 quick，依實際行為選測試 |
| 一般 prepare／debug 編譯 | 不自動執行完整矩陣；仍跑與變更相關的測試 |
| Windows 安裝版／正式發版 | 自動完整 verify，全部通過才封裝；quick／unit／smoke 不得授權發版 |
| 使用者明確要求全面驗證 | 可手動執行 npm run verify |

不再因為跨元件／共用布局的日常修改就預設跑三分鐘矩陣。這不是取消測試：必須選取涵蓋此次改動的單元／整合／UI 檢查；修改測試器本身仍要驗證其正向與負向行為。完整 release gate 與幾何限制不變。

判斷本輪相對起始工作區的實際變更；保留先前已驗證但尚未 commit 的內容，不每次重跑。這不等於跨程序沿用 release 憑證。

小改例（implementation-p4_2_5 目錄）：

```powershell
npm run verify:quick -- --widget github --test tests/daily-feeds-client.test.mjs
```

`--widget` 指定受影響元件，`--test` 可重複指定且不可省略。quick 會 prepare 一次、跑指定測試，再以隔離 Chromium 檢查 390px 手機與 1280px 桌面（含選定元件、一般幾何／overflow／toolbar 對比）。不跑整個 720 天氣矩陣；報告明示 `releaseEligible:false`，不能據此宣稱已通過完整矩陣或可發版；跨元件修改需另選涵蓋全部改動的相關測試。`verify:unit` 和 `verify:smoke` 仍僅供診斷，不能單獨作 UI 完成證據。

完整 verify 保留 12 viewport、28 元件／主題、三分頁、feed／資訊模板、省電、YouTube fixture、720 天氣與重疊負控制。現在只 prepare 一次，測試與 layout 共用 read-only frontend snapshot，並檢查 source／dist／依賴／工具內容是否改變。來源指紋涵蓋工作區未提交內容，不只 HEAD。JSON `passed:true` 並非可重用憑證。

verify、layout、builder 都逐階段即時輸出。`verification.json`／`layout-regression.json` 保留 elapsedMs、timeout／中斷、來源／產物 hash 與工具摘要。原始 stdout/stderr 與 Chromium profile 留本機。

## Windows HTTP 開發（由操作者明確啟動）

需要本機既有 Node 與 Python/Pillow。先檢查選定 port 未被占用，使用獨立 ATMOS_STORE，不能指向安裝版私人資料。例：在 PowerShell、implementation-p4_2_5 目錄中：

```powershell
if (Get-NetTCPConnection -LocalPort 49173 -State Listen -ErrorAction SilentlyContinue) { throw 'Port in use' }
$env:ATMOS_STORE = Join-Path $env:LOCALAPPDATA 'AtmosWallpaper/dev-preview-store'
$env:ATMOS_PYTHON = 'C:\Python314\python.exe' # 先確認為本機既有 Python/Pillow
node scripts/prepare.mjs --assets-only
node src/windows-runtime.mjs --host 127.0.0.1 --port 49173 --strictPort --start-local-service
```

這會啟動 loopback 服務與初始化獨立測試 store；不會掛載 Windows 桌布。管理頁 `/manager`，唯讀桌布頁 `/wallpaper`。`npm run dev` 是官方 Sites managed preview，禁止偽造環境或移除平台 guard。

- 已存在檔名的 CSS／JS／HTML：prepare 後重新載入，現行 router 每次讀 bytes、no-store；先處理草稿。沒有自動 HMR 保證。
- 新增／刪除／改名 dist：目前 server 靜態清單在建立時掃描，需重啟服務。
- Node src／依賴：重啟服務載入模組；session 變更需頁面重新連線。
- Rust／宿主：重建 debug binary 並重啟開發宿主即可，不因此必然需要安裝器。
- `desktop-host/scripts/run-prototype.ps1` 的 debug 路徑會實際套用主螢幕，需明確授權；不是無副作用 HTTP 預覽。不要 kill 所有 node.exe，以自己的宿主生命週期收尾。

上述重載判斷依現行 router 原始碼，不宣稱已測完整 HMR。自動 UI 一律使用隔離 Chromium CLI fixture，不操作使用者 UI。

## 正式交付（另行明確要求）

優先使用單一入口，避免手動先 verify 再另起 build 重跑：

```powershell
# implementation-p4_2_5 目錄；會建置 NSIS，但不安裝、不升版、不 push
npm run release:windows
```

`node desktop-host/scripts/build-installer.mjs` 獨立入口會自行執行完整 verify；沒有活躍流程憑證不會跳過。兩個入口皆只 prepare／完整 layout 各一次，`prepare-payload` 直接封裝已驗 snapshot，不再 prepare。只執行一般 `npm run verify` 不會呼叫 bundler。

憑證只存在目前 Node 程序的私有 WeakMap。跨次快取刻意不做；舊 `--reuse-payload` CLI 明確拒絕。程式內同次產生的完整 payload 可經輸入／輸出全量驗證後重用；任意 report、舊 manifest 或序列化物件都不能授權。

完整輸入寫入 `payload-inputs.json`：來源／測試／prepare／封裝腳本、凍結來源、package／lockfile、runtime pins、來源 manifest、工具版本與內容指紋、已安裝 bundler 依賴。這份檔案本身也列入完整輸出 hash 清單。`payload-manifest.json` 保持原生 Rust 嚴格 schema 相容；所有輸出清單／大小／SHA-256 均核對，包含 runtime、依賴、授權與後端來源。多檔少檔、同大小改寫、symlink、驗證後改動都拒絕；失敗會撤銷本次可重用狀態。

`node desktop-host/scripts/verify-workflow-payload.mjs --run-payload` 是明確建立生成 payload 的功能驗證入口：完整 verify 後建立 payload／核對 manifest，不製作 NSIS、不安裝、不改真實設定。一般小改不執行這個入口。Tauri＋NSIS 暫合併計時，沒有實測時不得拆報時間或宣稱發版加速倍數。

## 維護與後續

P1 同流程去重、完整 payload manifest 與負向驗收已實作，量測／界線見 desktop-host/docs/WORKFLOW-IMPROVEMENT-2026-11-07.md 與 TODO。前後條件不同、quick 覆蓋較少，不能把 quick 耗時說成完整 verify 加速。

Chromium 有限並行／process 重用屬另一步實驗，不先對共享 core 做 Promise.all。來源轉換先依 WORKFLOW-P0 的來源圖定位；逐步補替換斷言與小模組，不改 frozen/provenance、不一次重寫產品。
