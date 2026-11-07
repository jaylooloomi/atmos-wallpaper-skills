# 系統資訊卡 C／D 槽容量（2026-11-07，尚未發版）

保留現有上下行動畫／速率，移除兩格內框與背景，C、D 槽細容量條放在下方並上下排列。超過 80% 已用顯示橙色、90% 顯示紅色；仍有文字百分比，不只依靠顏色。使用 GiB 表示二進位容量。

## 資料與設定

Rust `--disk-info` 唯讀查詢固定 C、D 根目錄；只接受本機固定磁碟，缺失、網路／可移除／光碟等不支援類型或 API 失敗回傳 unavailable，不偽造容量。不掃描檔案、不使用 Node 或啟動桌布。依 Windows 的 GetDriveTypeW 與 GetDiskFreeSpaceExW；freeBytes／totalBytes 是目前使用者可用容量，可能受配額影響。

來源：[GetDiskFreeSpaceExW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdiskfreespaceexw)、[GetDriveTypeW](https://learn.microsoft.com/en-us/windows/win32/api/fileapi/nf-fileapi-getdrivetypew)。

資料服務沿用 `/api/providers/system`，附加 disks／disksSampledAtMs；磁碟獨立 30 秒快取、並行讀取合併，CPU／網速仍約 2 秒更新。失敗清掉舊容量，不把過期快取顯示成即時值。純 Node 預覽未連原生宿主時沒有真實容量。

顯示開關使用既有畫面設定交易：`changes.widgets.metrics.diskC`／`diskD`，在管理頁依既有套用流程儲存。新設定預設開啟；舊四項 metrics 設定可原樣驗證、讀取不改 bytes／hash；只有明確修改 metrics 時才補入磁碟預設欄位，原本的數值與元件位置不重設。全部六項關閉仍拒絕。Agent schema 已描述新欄位。舊版 App 不理解新欄位，勿把新寫入的設定直接交回舊版。

## 幾何與省電

依實際可捲動內容與卡片內可用高度計算增量，保持左／上錨點，不寫入或重排其他元件。增量以桌布底部與水平相交的既有元件為上限；空間不足時保留原卡片高度並允許卡內垂直捲動，沒有橫向捲軸。既有位置若本來重疊，不偷偷替使用者重排，只限制新增高度。關閉容量顯示時恢復原尺寸。省電只停動畫，容量資訊照常更新。

## 自動證據與界線

- 原生 `cargo test --locked` 53 項通過；debug `--disk-info` 在空 PATH／無效 Node 選項環境實際讀到本機 C／D 有效容量。真實 Node provider → 原生 CLI → 30 秒快取串接也測通；未啟動桌布。
- 產品測試 562 項：559 通過、3 既有跳過。最初直接跑產品測試漏設 ATMOS_PYTHON，已以明確 Python 路徑重跑；全關閉 metrics 的測試也擴為六項，沒有取消驗證。
- targeted＋桌面／手機 quick：17 項相關測試及兩個 viewport 通過，確認管理頁有 C／D 勾選控制。
- 實際 renderer／CSS 的 24 組主題、寬度 224／280／360、字級 13／16 合成案例通過，涵蓋獨立開關、未知值、省電更新、邊界與碰撞限制、捲動與原高恢復。原上下行正常／低動態測試仍通過。
- `verify-disk-dashboard.mjs --preview` 提供明確標示合成資料的動畫版型，不冒充本機監測、不產生任何 live ACK。

後續已整合至 0.7.35：完整 verify 通過並保留 11 份設定檔更新安裝版，安裝後原生容量、真實 provider 快取與 31 秒後刷新核對通過；詳見 [發版紀錄](UI-BATCH-0.7.35-2026-11-07.md)。仍無人工目視驗證，不把原生資料或 ACK 當成實際可見性證明。
