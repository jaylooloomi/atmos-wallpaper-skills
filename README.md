# Atmos Wallpaper

Atmos 動態桌布管理介面、唯讀桌布 renderer 與本機 CLI。此倉庫是 **P4.2.5 / 0.4.2-5** 的可重建產品源碼，包含七類元件、四種風格、六個場景及必要素材。

## 最新功能

- 「全部顯示／全部隱藏」控制七類元件；只改顯示狀態，不重排已存位置。
- 元件設定使用真正行內展開，移除重複分隔線。
- 五語介面（繁中、簡中、英文、日文、韓文）、12／24 小時及秒數設定。
- 管理頁 `/manager` 的修改先成草稿，再按套用提交；`/wallpaper` 為唯讀顯示頁。
- 本機圖片匯入、備份、還原、版本遷移、離線匯出與受限 CLI。

## 目錄

```text
implementation-p4_2_5/    目前產品：src、scripts、tests、CLI、Windows 入口與技術文件
implementation-p1/       共用核心、解碼器與基礎回歸測試（仍為必要依賴）
implementation-p4_2_1/src/ 舊格式相容／遷移測試所需凍結實作
prototype-v8.1-frozen/   凍結前端基底、字體授權與原始場景素材
```

請保持上述相對路徑；不能只複製 `implementation-p4_2_5`。保留凍結基底 manifest 原貌；歷史交付／QA 私人筆記不在公開倉庫中。沒有收錄 runtime store、私密帳號資料、token、`node_modules` 或生成的 `dist`。

## 重建及測試

環境：**Node.js >= 22**、已安裝 **Python 3 + Pillow**。無 npm 套件依賴，不需要 `npm install`。請先自行備妥依賴；腳本不會代為安裝。

```sh
cd implementation-p4_2_5
npm run prepare -- --assets-only
npm test
node bin/atmos.mjs --help
```

`--assets-only` 只重建 `dist/`，不初始化 demo store。一般 `npm run prepare` 另會在缺少時建立本機 demo store。基礎核心回歸測試可從 `implementation-p1` 執行 `npm test`。

## Windows 本機啟動

此倉庫提供 Windows 入口，但**本次公開版本未重新進行 Windows 實機驗證**。請確認 Node、Python/Pillow 已存在並指向正確路徑。以下由你在自己的 Windows 命令提示字元執行：

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
- **仍為 demo／N/A**：Google／Microsoft 日曆 OAuth、真天氣服務、主機 CPU／RAM／網路／電量資料、Now Playing。示範事件不是私人行程。
- AI Prompt 只產生可複製文字，不會自動呼叫外部 AI、登入帳號或更新桌布。
- renderer ack 不代表 OS 桌布已套用；Windows 宿主、鎖屏、DPI、耗電及完整生命週期仍待實機驗證。
- scoped session token 用於防誤寫／能力檢查，不是惡意同源 JavaScript 的安全隔離。

詳見 [本版文件](implementation-p4_2_5/README.md)、[本機 CLI 與恢復流程](implementation-p4_2_5/docs/WINDOWS-CLI-AND-RECOVERY.md) 與 [全部顯示規則](implementation-p4_2_5/docs/P425-BULK-VISIBILITY.md)。

## 本次驗證（2026-11-01）

對公開倉庫的乾淨選取源碼重新執行：prepare 成功、產品測試 **167/167**、共用核心 **42/42**、CLI help 成功；生成的 35 個 dist 檔與正式凍結版本逐位元組一致。獨立正式 UI QA 另驗證七卡顯示／隱藏與草稿／套用、非顯示資料及 timer deadline 保留、行內展開／分隔線、五語工具列、Enter／Space，以及 390×844 和 1876×1080 畫面。這是本批影響範圍驗證，不代表所有歷史瀏覽器測試或全平台通過。未重跑 Windows 實機或真外部服務整合。

## 素材與授權

字體授權見 [FONT-LICENSE.txt](prototype-v8.1-frozen/assets/FONT-LICENSE.txt)。本次匯入未擅自新增整個專案的開源授權；公開可見不等於授予任意再授權。
