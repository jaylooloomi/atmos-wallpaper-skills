# 0.7.3 設定區重疊修正

問題為舊版桌面布局仍把 component-section、widgetManager、componentInspector 與展開卡片設為零基底的可收縮 flex item；新增設定區後，外框可能比展開內容更矮，overflow-visible 內容因此蓋到後續設定。只增加按鈕 margin 無法處理這個高度問題。

修正讓右側設定依內容自然撐高，禁止設定區與卡片被壓縮至內容以下；同時移除對齊器每次寫入的 inline important 固定高度。移除展開欄位的固定最小高度、最大高度及巢狀捲動，使用既有頁面捲動。保留原本雙欄、順序、左側預覽、元件位置與草稿／套用規則，沒有直接改使用者設定。

## 自動回歸

使用獨立 headless Chromium CLI、合成宿主與私人測試資料目錄，不操作使用者瀏覽器或桌面 UI、不使用 Computer Use／CDP。

- 修正後：320、390、768、1000、1100、1280、1876 px，各測 natural／neon，14 個案例通過。
- 繁中：1280 px neon 額外案例通過；另測切換至天氣展開內容後，說明區高度保持 111.17 px、預覽與目標比例皆為 16:9，合計 16 個成功案例。
- 舊版對照（0.7.2 對齊程式及不含新 override 的樣式）：1280 px neon 明確得到 SETTINGS_PANEL_CONTENT_OVERLAP；componentInspector 及展開卡片的子內容超出外框，且存在巢狀捲動。因此新檢查可以抓到舊版問題，不只檢查場景卡片寬度。
- 檢查涵蓋設定區兄弟元素順序、子內容不逃出整個側欄與元件容器、展開欄位不產生巢狀捲動、螢幕保護區在 inspector 後方，以及既有水平溢出、卡片擠壓、工具列與預覽對齊檢查。桌面與窄版單元案例也檢查舊的固定高度會被清除。
- Node 完整回歸 445 通過、5 項平台／權限略過、0 失敗；Rust 48 通過、0 失敗。

[匿名排版證據](../evidence/panel-flow-layout-2026-11-05.json)。原始測試 profile／截圖留在本機，這是合成頁面的幾何驗證，不冒充人工目視驗收。

重現：先在 implementation-p4_2_5 執行 node scripts/prepare.mjs，再執行 node scripts/audit-layout.mjs --host-controls --native --panel-flow；可用 --panel-flow-quick --zh 執行單一繁中桌面案例。請將 ATMOS_LAYOUT_OUTPUT 設為本機私人測試輸出位置。

## 目前安裝版

已重建並安裝最終 0.7.3，逐檔比對安裝前後的 live 與設定檔及開機啟動偏好，原設定保持相同，私人備份留在本機。實際安裝版提供的 hifi.css 及 p422-ui.js 等檔案與最終建置一致，並有真正 WorkerW 掛載及新鮮 wallpaper／manager 回報，沒有由測試寫入 ACK。[安裝更新證據](../evidence/panel-flow-upgrade-2026-11-05.json)。

前一輪管理頁回報在 15 秒預算內尚未到達，沒有列為通過；稍後唯讀查核確認回報正常。最終腳本採用最多 60 秒的有界等待，並在所有 Chromium 幾何測試結束後驗證最終安裝包。安裝測試退出後，再用獨立命令觀察至少 18 秒，確認程序、視窗中繼資料及真實回報仍正常：[交接證據](../evidence/panel-flow-handoff-2026-11-05.json)。

安裝檔、私人設定備份、原始測試 profile 與日誌不加入 Git。沒有人工目視、鎖屏／睡眠、登入或長時間耗用驗收；這次只針對面板內容邊界、既有功能回歸與實際安裝更新。
