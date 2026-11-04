# 素材庫位置與安全搬移 — Windows 0.5.0

本批提供匯入圖片素材的獨立目錄、原生資料夾選擇、開啟資料夾、使用預設位置、容量及離線狀態、背景搬移工作與同使用者 Agent CLI。內建場景仍是安裝包的唯讀程式資源，不搬移執行環境、交易 store、私密日曆、WebView profile 或 logs。

預設目錄是 store 同層的 `<store 名稱>-library`，與交易資料分開。空的舊素材庫會遷移到預設目錄；已有素材的舊庫保留相容讀取，使用者選擇預設／新位置時才搬移。自選位置會建立 `Atmos Wallpaper Library` 專用子資料夾。僅支援本機固定或卸除式磁碟；UNC、映射網路磁碟、安裝／store 目錄、符號連結／junction 及未知擁有者拒絕。

每個素材庫含擁有者標記與內容雜湊索引。搬移工作使用獨立程序及原 store 交易鎖，逐檔複製、驗證雜湊、檢查完整 PNG／metadata 配對、重新核對來源後，才原子切換設定。原資料保留，沒有遞迴刪除。磁碟滿、權限拒絕、檔案占用、資料不一致或中斷時，未切換的原位置仍可使用；部分目的檔可以在相同請求重試時核對後續傳。

程序突然終止時，既有 dead-PID lock 機制可恢復，避免留下「主程序仍活著、工作執行緒已死」的永久鎖。切換前先保存 pending receipt／intent；切換後回覆遺失時，可以重建已提交結果，不再次加 revision。素材庫有自己的 revision／requestId；外觀與螢幕 placement revision 維持原語意。

離線時保留指定路徑，不另建空庫或改寫其他位置。已套用且解碼的桌布保留上一有效畫面；管理頁可以使用明示的替代預覽開啟修復控制，但不送出原素材已套用的 ack。磁碟接回後可再次取得原圖；原 desired、assetId、歷史與備份不改變。

目前採保守的「全部保留」政策，包含歷史、預覽、備份及尚未使用的匯入素材；顯示素材數、容量與可用空間。未提供垃圾回收／自動清除。LIB-03 的完整參照清理功能仍未完成，不因離線／容量介面已完成就勾選。

## Agent 與 HTTP 邊界

執行 `atmos-wallpaper.exe --library-command help` 讀取機器教學，或 `--agent-help` 取得入口。`get` 讀取狀態；`move`／`reset` 從 stdin 讀取嚴格 JSON；`result` 查詢 requestId。可帶 `--store ABSOLUTE_LOCAL_DIRECTORY` 做獨立診斷。JSON 最多 32768 bytes，UTF-8，拒絕額外欄位。

```json
{"schemaVersion":1,"requestId":"library.move.001","expectedRevision":0,"path":"D:\\Wallpapers\\Atmos Wallpaper Library"}
```

先讀最新 library revision，重試只用相同 requestId／原封不動的輸入；衝突應重讀並協調。`committed` 是保存結果，不能代替目前磁碟可用性或桌布目視證據。

HTTP `/api/host/library` 僅管理角色、同源與 session 可使用。HTTP 不接受任意路徑，只接受選擇／預設／開啟動作；目錄由 Rust 原生選擇器或既有設定決定。背景工作回傳 202，管理頁查詢真實進度與失敗，不假裝同步完成。選擇器是使用者主動按鈕後操作，不是自動化桌面。

## 驗證

- Node：290 通過、4 略過；Rust：20 通過。
- 13 項素材庫測試包含相容讀取、中文／空白路徑、備份不變、CAS／requestId、寫入拒絕／磁碟滿故障注入、複製中斷、真正子程序突然終止、切換後 receipt 恢復、離線／接回與 HTTP 權限。
- 5 種獨立 Chromium 控制 fixture：桌面、手機、英文長文字、離線、複製進度，沒有橫向溢出。
- 實際產生的管理頁搭配離線自訂素材：可開啟修復區、明示替代場景、保留原 assetId，沒有寫入偽造 ack。見 `implementation-p4_2_5/evidence/library-offline-manager/report.json`。
- 已安裝版的原生 CLI、內嵌 Node/Python/Pillow、真正 Windows ACL 拒絕、搬移／還原／離線、已保存畫面與素材 ID／備份核對，見 `desktop-host/evidence/library-installed.json`。
- 安裝／設定與開機偏好保留見 `desktop-host/evidence/library-upgrade.json`。原生選擇器的人工結果另記，不能以注入選擇器替代。

沒有 Computer Use、使用者瀏覽器或桌面 UI 自動化。未聲稱已測實體拔除、硬體磁碟滿、映射網路磁碟或長期耗用。所有安裝包留本機，原始私人路徑、設定與紀錄不提交。

依據：[Microsoft SHBrowseForFolderW](https://learn.microsoft.com/en-us/windows/win32/api/shlobj_core/nf-shlobj_core-shbrowseforfolderw)、[Node.js 檔案系統](https://nodejs.org/api/fs.html)。

安裝包：292463716 bytes，SHA-256 `d7f45c291043ce9876b5945977bdc9af3a7b162975c762032d160e8157e2d84e`；未簽章、僅留本機。

人工驗證：使用者已在本聊天明確確認「選擇器、搬移、新位置與桌布時鐘都正常」。未公開其選定路徑或其他私人設定。
