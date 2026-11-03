# 浮動工具列提示文字對比 0.3.7

日期：2026-11-04。

使用者指出切換風格後「未提交草稿只在本頁；外部資料仍為示範。host未知。」難以閱讀。工具列本身有固定的淺色背景，但 small 元素繼承了深色頁面的淺色次要文字，造成低對比。

## 修正

提示文字使用工具列專用深色 `#435463`、opacity=1、11px 字級及 1.5 行高，取消文字陰影。不改文案、按鈕功能、其他區域的主題色或布局。維持原生標題、雙欄頂端對齊、說明卡片自然高度與浮動工具列 12px 頂部間距。

## 驗證

隔離 Chromium CLI：320×160 minimal、390×844 neon、1280×900 natural、1280×900 warm、1876×1080 minimal，五組皆通過，包含很矮的視窗與手機版換行。

各 fixture 除實際載入的風格之外，另在同一頁同步切換 natural → neon → warm → minimal → natural，讀取實際 computed style，完成後還原原風格；不提交設定，也不偽造桌布 ack。以瀏覽器解析的色彩／透明度合成後計算文字與背景的相對亮度對比，檢查標題、狀態、提示及可用狀態的所有工具列按鈕。工程測試門檻為 4.5:1；所有測量的最低值約 6.94:1，提示文字在所有風格維持相同對比。停用控制項不是這項可用按鈕對比門檻的對象。

同時檢查手機及小視窗不產生新水平捲軸、工具列不超出視窗、按鈕 hit test、預覽欄與控制面板頂端對齊，以及說明卡片自然高度。[證據](../../implementation-p4_2_5/evidence/toolbar-contrast-layout/report.json)。對比為 computed style 的色彩計算，不等同所有顯示器／字型光柵化的人工可讀性認證。

完整 Node：產品 225 passed／2 skipped，共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 通過，共 266 passed、4 skipped、0 failed。沒有 Rust 行為變更，只有安裝版本更新。

沒有 Computer Use 或使用者瀏覽器／桌面 UI 自動化；此為隔離合成版面測試，沒有新的人工目視確認。未重跑真正登入、睡眠、卸載、多螢幕、GPU 或長時間耗用。安裝檔只留本機，提交前檢查暫存區，禁止安裝包、runtime 及生成目錄加入 Git。

## 本機更新

已從 0.3.6 靜默升級到 0.3.7，已儲存設定 bytes 與登入啟動偏好相同。payload／內嵌執行環境自檢、安裝內容與來源生成結果比對、WorkerW 掛載及真正桌布／管理頁近期 ack 皆通過。沒有新的人工可讀性確認。[升級證據](../evidence/toolbar-contrast-upgrade.json)。

安裝檔只留本機：Atmos Wallpaper_0.3.7_x64-setup.exe，292,189,943 bytes；SHA-256：`b43377d5b9fa7d43b6473bee447971530d7b942e5d80e2154de4ce30dd882da1`。自有安裝包未簽章。
