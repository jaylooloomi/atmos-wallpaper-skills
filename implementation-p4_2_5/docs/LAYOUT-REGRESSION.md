# 排版回歸門檻

在 `implementation-p4_2_5` 執行：

```sh
npm run test:layout
npm run verify
```

`test:layout` 執行排版門檻；`verify` 執行原有完整回歸並包含排版門檻。`verify:unit` 只供快速診斷，其驗證摘要會標記 `layoutRequired=false`，不能代替發版驗證。

## 覆蓋內容

- 12 個情境：320～1876 CSS px，包含 1000/1001、1100/1101 斷點兩側及 1280×600 短視窗；繁中／英文。
- 每個情境切換 natural、neon、warm、minimal 四種風格，以及時鐘、日曆、天氣、待辦、計時器、系統資訊、媒體七個展開元件，合計 336 個正向狀態。
- 每個展開內容加入長文字，檢查側欄／卡片完整容納、控制項互不重疊、控制項不溢出、區塊順序、巢狀捲動及橫向溢出。
- 檢查螢幕保護按鈕至少 44px 高、至少 8px 間隔，並保留既有場景、預覽設定列與浮動套用工具列檢查。
- 每次先在合成頁面故意注入重疊樣式，必須確實測到 `CONTROLS_OVERLAP`。這是預期失敗的負向對照，不是產品失敗；若測不到也拒絕通過。

使用全新私人 profile 與合成核心／宿主，透過 Chromium CLI headless 執行，不操作使用者瀏覽器、桌面或已安裝 App，不寫入真實 ACK。原始輸出留在系統暫存目錄；公開摘要為 `evidence/layout-regression.json`。檢查開始時先清除舊通過狀態，失敗、缺少瀏覽器與逾時都記錄為非通過；未完成的矩陣不能沿用先前綠燈。

找不到瀏覽器時設定 `ATMOS_TEST_CHROME` 為 Chrome／Edge／Chromium 的可執行檔路徑。明確指定但無效的路徑會失敗，不會悄悄換成其他瀏覽器或略過。

## 封裝與維護

Windows `build-installer.mjs` 在封裝前自動執行同一門檻；`--reuse-payload` 另比對已測前端與快取前端的檔案清單及 SHA-256，避免測了新 CSS 卻封裝舊 CSS。

新增控制項、元件或支援尺寸時，同步擴充 `scripts/layout-regression-contract.mjs` 與 probe 覆蓋。門檻單元測試包含缺案例、重複狀態、逾時、缺瀏覽器、舊結果失效、封裝拒絕與過期 payload 偵測。

這些測試能擋下目前涵蓋的幾何退化，不能保證所有像素、裝置、縮放、字型與未來變更皆零跑版；不要把合成測試冒充人工或實機目視。

本機於 2026-11-05（台北）完成預設完整驗證：Node 450 通過、5 平台／權限略過、0 失敗；12 個情境／336 正向狀態通過，注入重疊的負向對照成功被拒絕。封裝缺瀏覽器拒絕與過期 payload 偵測測試亦通過。結果見 `evidence/verification.json` 及 `evidence/layout-regression.json`。
