# 安裝版移除重複頁首 0.3.5

日期：2026-11-04。

安裝版已有「Atmos Wallpaper 管理」原生視窗標題，因此隱藏網頁內「Atmos／本機核心控制」整排頁首，收回其佔用空間。保留原本的頁首 DOM 供主題與翻譯程式更新，但不在原生管理視窗顯示；單獨瀏覽器仍顯示頁面標題。雙欄位置、控制面板與預覽設定頂端對齊、場景順序及浮動工具列 12px 留白不變。

## 實作邊界

原生管理視窗透過 Tauri initialization_script 設定一個純外觀標記；bridge 只對 manager 角色且 boolean true 的標記加上 body 屬性，CSS 僅隱藏該情況的直接頁首。桌布模式不套用此標記。F5 的新文件仍由原生初始化腳本設定，並沿用現有首次顯示閘門；沒有新增 HTTP 權限、IPC capabilities 或 OS 操作介面。標記不能作為認證或權限判斷。

## 驗證

- 新增兩項測試，涵蓋原生 manager、一般 browser、wallpaper、非 boolean 標記、CSS 作用範圍、既有 DOM 與原生初始化設定。
- 隔離 Chromium CLI 在 320×844 natural、390×844 neon、1280×900 natural、1876×1080 natural 分別測試原生標記與一般瀏覽器，八組均通過：原生頁首 display=none、height=0；一般瀏覽器保留頁首。各組均無新增水平溢出，桌面雙欄頂端對齊、寬螢幕單排與浮動工具列 hit test 正常。
- 這些是合成 fixture 的標記與 CSS 測試，不假稱操作了使用者的 native WebView 或取得人工目視確認。沒有 Computer Use 或使用者瀏覽器／桌面 UI 自動化。
- 完整 Node：產品 223 passed／2 skipped、共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 通過。Rust 13 passed、clippy `-D warnings` 與 fmt check 通過，合計 277 passed、4 skipped、0 failed。

證據：[native-manager-layout/report.json](../../implementation-p4_2_5/evidence/native-manager-layout/report.json)、[browser-brand-layout/report.json](../../implementation-p4_2_5/evidence/browser-brand-layout/report.json)。

安裝檔與 runtime／生成目錄只留本機；提交前檢查暫存區，不將 exe／msi／msix 等安裝包加入 Git。未重跑真正登入、睡眠、卸載、多螢幕、GPU 或長時間耗用。

## 本機更新

已從 0.3.4 靜默升級到 0.3.5，已儲存設定 bytes 與登入啟動偏好相同。payload／內嵌執行環境自檢、安裝的 HTML／bridge／樣式與來源生成結果比對、WorkerW 掛載與真正桌布／管理頁近期 ack 均通過。未取得新的人工目視確認；上述標記外觀由隔離 fixture 檢查。[升級證據](../evidence/native-manager-upgrade.json)。

安裝包只留本機：Atmos Wallpaper_0.3.5_x64-setup.exe，292,189,384 bytes，SHA-256：`75404963107fd8ca00455b31c96f53841f666954138d86522439532d298bf22d`。自有安裝包未簽章。
