# 開發流程改善報告 — 2026-11-07

> 後續政策更新：使用者已要求日常開發只跑相關測試及 UI quick，完整 verify 改為僅正式安裝器／發版自動觸發（或明確要求）。下方耗時仍為當時實測，原「跨元件也 full」政策已由 DEVELOPMENT.md 的最新規則取代。

## 結果

使用者已於 2026-11-07 同意分層驗證，規則已落實到 AGENTS.md、DEVELOPMENT.md 與可執行入口。

| 項目 | 改善前 | 改善後 | 解讀 |
| --- | ---: | ---: | --- |
| 局部 UI 小改完成等待 | 167.1 秒（完整 verify） | 17.2 秒（quick） | 少 149.9 秒，約 89.7%；覆蓋不同，並非完整 verify 快九倍 |
| 完整 verify | 167.1 秒（P0 平均） | 175.7 秒（最終實測） | 沒有證據完整驗證加速；加入更多測試、來源／依賴／工具 hash，計時範圍也不同 |
| 同一次完整交付 prepare | 4 次 | 1 次 | 消除重複生成；測試與封裝沿用同次 snapshot |
| 同一次完整交付 layout gate | 2 次 | 1 次 | 保留完整矩陣與負控制，省掉第二次同內容 gate |
| 真實 payload 建立 | 未受控量測 | 16.4 秒 | 2021 檔、184534906 bytes；兩次完整核對通過，未建置 NSIS |

小改數字是新舊「完成流程」等待時間比較；不是同覆蓋率競速。P0 兩輪 166.652／167.536 秒，未清空 OS/browser cache。新流程也未清空，背景負載未控制。舊計時不含工具預檢，新完整計時包含，不能據此做嚴格性能回歸／加速結論。

## 真正完成的修改

- 局部文字／樣式／單一元件：指定受影響測試，加手機／桌面 quick；缺少測試參數會失敗，不會無測試宣告成功。共用布局、renderer prepare、核心／router、設定格式、跨元件及 release 仍 full verify。
- 一般 verify 不升版、不打包、不安裝、不推送。明確的 release:windows 入口才在同一 Node 程序中銜接驗證與封裝；獨立 builder 沒有憑證會自行完整驗證。
- prepare 產物轉為 read-only snapshot。瀏覽器 fixture 與 payload 讀同一 snapshot；既有單元讀取工作 dist 亦受 byte 相等檢查。來源、prepare／probe／contract／fixture／lockfile、依賴、工具、工作 dist 與 snapshot 改動都拒絕沿用。
- 憑證只存在目前程序私有 WeakMap；序列化物件、passed:true 或舊 report 不能授權封裝。同次 payload 全量驗證可重用；舊跨次 --reuse-payload CLI 明確拒絕。
- 完整輸入記錄放在 payload-inputs.json，並列入輸出 hash；包含實際產品／共用核心來源、凍結來源、source manifest、資源、依賴、runtime pins、腳本、工具／flags 指紋。payload-manifest.json 保持原生 Rust 嚴格 schema 相容。輸出檔案清單、大小與 SHA-256 完整核對，不只 dist。
- 失敗／timeout／中斷撤銷成功狀態；stdout/stderr 即時輸出、原始 bytes 留本機。測試涵蓋缺瀏覽器、負控制、覆蓋不足、子程序失敗、來源／產物變動、多檔／少檔、同大小改寫、symlink、未確認結果與 quick 不可發版。
- 開發重載已以隔離 fixture 測試：既有檔案取得新 bytes，新／刪檔需重建索引；Node 模組要重啟程序。不誤稱完整 HMR。

## 自動驗證

產品 545 項：542 通過、3 個既有 skip；核心 43 項：41 通過、2 個既有 skip。完整 12 viewport／336 元件主題／三分頁、1008 feed、1008 資訊模板、336 省電、336 YouTube、720 天氣及重疊負控制通過。quick 指定 GitHub 相關 14 項測試、兩個 viewport 通過；另外的單 viewport smoke 亦通過，但仍僅供診斷。

真實 payload 產出與兩次完整校驗通過，嵌入 Python 3.14.10／Pillow 13.0.0 可載入。沒有製作 NSIS、安裝、修改真實設定或操作使用者 UI。不能宣稱整體發版快幾倍，也不把 fixture 當實機桌布目視結果。

## 使用入口

在 implementation-p4_2_5 目錄中：

- 小改：npm run verify:quick -- --widget github --test tests/daily-feeds-client.test.mjs（換成真正受影響元件／測試；--test 可重複）。
- 共用／高風險：npm run verify。
- 明確要求發版：npm run release:windows（只產生安裝器，不自動安裝／升版／推送）。

跨次快取與 Chromium 並行仍不做；避免一次增加過多變動。剩餘 P3 轉換錨點斷言／模組整理是後續小步維護，不是本次快取／去重尚未完成。

公開原始數據：../evidence/workflow-improvement-2026-11-07.json、workflow-p1-payload-2026-11-07.json；原始日誌、profile、payload 和安裝器不進 Git。
