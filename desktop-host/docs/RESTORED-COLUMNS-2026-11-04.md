# 恢復原本預覽／控制面板雙欄 0.3.4

日期：2026-11-04。

先前為了將欄位排成單排，把預覽設定移到整個雙欄上方，超出了使用者要求。現在恢復左側預覽、右側控制面板；預覽解析度／語言及其訊息留在左欄，右側控制面板上緣與左欄預覽設定區塊上緣一致。場景區仍保持先前要求的位置，在預覽上方。後續 UI 變更須遵守此結構，已記入根目錄 TODO。

欄位改採左欄內的彈性排列：寬螢幕標籤、輸入框、套用與語言同列；較窄左欄只在本欄內換行，不藉由移動整個區塊來強求單排。手機版也保留至少 60px 的寬高欄位，無額外水平捲動。未改使用者設定、交易或桌布宿主行為。

## 驗證

- 隔離 Chromium CLI：320×844 natural、390×844 neon、1280×900 natural、1876×1080 natural 皆通過。兩個桌面案例的控制面板與預覽設定上緣差為 0；1876px 全部控制項維持單排。窄螢幕不要求雙欄頂端對齊，而是正常上下堆疊。
- 額外檢查控制列確實位於 main 的左側預覽欄、場景順序、輸入框最小寬度、場景卡片溢出，以及浮動工具列頂部 12px／按鈕 hit test。
- 重新跑慢速載入、連續重整、圖片失敗及樣式失敗 fixture，並將首次可見位置條件更新為左側預覽欄，全部通過。這些都是隔離合成頁面，不操作使用者瀏覽器／桌面，也不當作實際目視確認。
- Chromium 虛擬時間預算避免超過既有 ack 時間容許範圍，未變更正式伺服器驗證、不偽造 ack；版面與載入證據不等同實機呈現證明。歷史 0.3.3 載入證據保留，本次輸出另存新目錄。
- 完整 Node 檢查：產品 221 passed／2 skipped、共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 通過，共 262 passed、4 skipped、0 failed。本次只有前端版面與安裝版本變更，沒有 Rust 行為變更。

證據：[restored-columns-layout/report.json](../../implementation-p4_2_5/evidence/restored-columns-layout/report.json)、[restored-columns-boot/report.json](../../implementation-p4_2_5/evidence/restored-columns-boot/report.json)。

## 提交限制

安裝檔、執行檔與生成目錄不提交。除了既有 target／payload／dist 忽略規則，新增 exe、msi、msix、msixbundle、appx、appxbundle 忽略規則；提交前另檢查暫存區路徑。文件可以記錄本機安裝包的檔名與雜湊，不能把安裝包本體加入 Git。

## 本機更新

已從 0.3.3 靜默升級到 0.3.4，已儲存設定 bytes 與登入啟動偏好相同。payload／內嵌執行環境自檢、安裝內容與來源生成結果比對、WorkerW 掛載，以及真正桌布／管理頁近期 ack 皆通過；沒有新的人工目視確認。[升級證據](../evidence/restored-columns-upgrade.json)。

安裝包僅留本機：Atmos Wallpaper_0.3.4_x64-setup.exe，292,189,070 bytes，SHA-256：`55e57dc9289be7699c57881267a6f5fda83bd049a6f40e5187e0532ec81e17e6`。自有安裝包未簽章。
