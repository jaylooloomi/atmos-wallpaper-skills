# 浮動工具列頂部留白 0.3.2

日期：2026-11-04。

管理頁的浮動套用工具列仍使用 sticky，吸附位置改為頂部安全區加 12px。保留既有圓角、背景與陰影；工具列最大高度預留頂部安全區與上下合計 24px，窄小視窗可在工具列內垂直捲動。不改變按鈕、交易、草稿或設定資料。

## 版面與回歸

隔離 Chromium 命令列合成 fixture：320×160 minimal、320×844 natural、390×844 neon、1280×900 natural，四組皆通過。捲動 600px 後工具列 top 均為 12px，套用按鈕 hit test 正常；工具列下緣保留至少 12px，沒有新增水平溢出。桌面預覽設定仍維持單排、場景順序正常。小視窗工具列高 116.5px，桌面高 46.5px。

證據：[toolbar-gap-layout/report.json](../../implementation-p4_2_5/evidence/toolbar-gap-layout/report.json)。此測試使用無安全區 inset 的合成 viewport；沒有實測瀏海手機的硬體 inset，也不將 CLI 圖片當成使用者桌面的目視確認。沒有 Computer Use 或使用者瀏覽器／桌面 UI 自動化。

完整 Node 檢查：產品 214 passed／2 skipped、共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 通過。此次沒有 Rust 行為變更，只有安裝版本更新；不重跑開機、睡眠、卸載或長時間耗用。

## 本機更新

已從 0.3.1 靜默升級到 0.3.2，已儲存設定 bytes 與登入啟動偏好保持相同。payload／內嵌執行環境自檢、安裝樣式比對、WorkerW 掛載與真正桌布／管理頁近期回報皆通過；沒有新的人工目視確認。見 [升級證據](../evidence/toolbar-gap-upgrade.json)。

安裝檔 Atmos Wallpaper_0.3.2_x64-setup.exe：292,187,429 bytes；SHA-256：`0925b84355e86723de6161e6aebc667b832346fd92f59444f81807c0571ef03e`。自有安裝包未簽章。
