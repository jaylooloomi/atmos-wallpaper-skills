# 場景區與預覽工具列：排版驗證

2026-11-03。場景區位於預覽設定上方，這次發現並修正原有預覽解析度工具列的固定 470px 最小寬度與禁止換行：小螢幕原先會在工具列出現橫向捲動，現在解析度／語言面板依可用寬度換行，窄螢幕使用兩欄輸入與整列選單／套用按鈕。截圖檢查還發現英文的元件操作列會橫向捲動，一併改為換行；自然風格場景區則補齊與管理面板一致的底色／邊框。

使用已安裝 Chromium 的 headless CLI，啟動獨立本機 fixture 與合成 store。手機尺寸由同源 iframe 的真實 CSS viewport 設定，不是把桌面畫面縮小。未使用 Computer Use、瀏覽器 UI 自動化或使用者的 browser profile，未讀取私人日曆或修改實際桌布。

矩陣為 320、360、390、600、768、1000、1100、1101、1280、1440、1876 CSS px，四種風格，共 44 組；用英文長標籤驗證。手機高度 844，其餘 1080。逐組量測：

- viewport 與要求的寬度一致。
- 頁面 scrollWidth 沒有超過 clientWidth（允許 1px 捨入差）。
- 預覽設定工具列沒有額外橫向捲動。
- 所有可見且設定 auto／scroll overflow 的容器都沒有橫向內容溢出，包括元件操作列。
- 場景區在預覽解析度之前，主要區塊未超出 viewport。
- 六張場景卡保有至少 85px 寬、內容未橫向溢出；手機兩欄、中段三欄、桌面六欄。

結果見 [原始量測摘要](../evidence/layout/report.json)，及相同目錄的手機／桌面合成畫面。這是 Chromium 實際排版測試，不宣稱已涵蓋 iOS Safari／所有 Android 瀏覽器、所有字級與輔助功能設定。

重跑方式（需要已安裝 Chromium，不會自動安裝瀏覽器）：

```sh
npm ci --ignore-scripts
node scripts/prepare.mjs --assets-only
npm run audit:layout
```

可用 `ATMOS_TEST_CHROME` 指定已安裝的 Chromium 完整路徑。`node scripts/audit-layout.mjs --quick` 只跑 390px 與 1280px 的快速檢查。fixture 使用暫存 store、獨立 profile 與空日曆來源，不會連到使用者的 Atmos store。
