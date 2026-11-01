# Windows CLI 與回復程序

**此文件是將來獲准執行時的操作說明，不表示已在Windows跑過。** 目前已驗的是cloud/POSIX及受控Windows分支mock。只讀盤點已有Node24.13.0、Python3.14.2/Pillow12.1.0；不用安裝新套件。

## 啟動前
- 先取得啟動本機服務的明確授權。改桌布另外需指定顯示器授權；兩者不可互推。
- 完整解壓保留 `implementation-p1`、`prototype-v8.1-frozen`、`implementation-p4` 的相對位置。
- 確認沒有其他程序占127.0.0.1:4173。占用就停，不殺未知程序／不靜默換port。
- Start-Atmos.cmd會使用 `%LOCALAPPDATA%\AtmosWallpaper\data`，不覆寫凍結原型/P3/cloud測試store。
- 同一份既有store跨版本先做backup；不把未知schema當舊版自動降級。

## 服務
在 `implementation-p4` 內執行 `Start-Atmos.cmd`（需已授權），前景保持開啟。
管理頁 `http://127.0.0.1:4173/manager`；桌布頁 `http://127.0.0.1:4173/wallpaper`。
正常browser先驗兩向交易，再考慮Lively宿主；這支cmd本身不啟Lively、不改桌布。
Ctrl+C只停止此服務。重啟會換記憶體nonce，舊頁需重新載入；不持久保存credential。

## CLI契約（所有佔位字都需使用真回傳值）
從 `implementation-p4` 開cmd，設定此shell的兩個值（不使用setx、不改全系統PATH）：

```bat
set "ATMOS_STORE=%LOCALAPPDATA%\AtmosWallpaper\data"
set "ATMOS_PYTHON=C:\Python314\python.exe"
node bin\atmos.mjs --help
node bin\atmos.mjs status --store "%ATMOS_STORE%"
```

P4 CLI共用P1命令schema：
- `init`：僅首次建立空store；已存在不重設。`status`：revision/desired/最後observed（要另判timestamp freshness）。
- `stage --request-id NEW_ID --file relative.png`：檔案只能在 `STORE\imports`。必須真raster解碼；不收URL/SVG/路徑越界/ADS/裝置名。回傳assetId，尚未改live。
- `validate --patch patch.json`：受限imports JSON只驗證，不提交。
- `preview --request-id NEW_ID --expected-revision CURRENT --patch patch.json`：建立候選，不改live。patch檔例：`{"visual":{"effect":"snow","intensity":70}}`。自訂圖可在CLI patch帶真stage assetId；HTTP不開此能力。
- `apply --request-id NEW_ID --expected-revision CURRENT --preview RETURNED_PREVIEW_ID`：原子提交；UI/桌布頁自動取新rev後驗圖paint。server提交不是OS成功。
- `cancel --request-id NEW_ID --preview RETURNED_PREVIEW_ID`：取消候選，不改live。
- `rollback --request-id NEW_ID --expected-revision CURRENT --target-revision RETAINED_PREVIOUS`：用保留history恢復desired，產生**新**單調revision，不倒退revision數字。
- `backup`：產生完整desired＋referenced raster驗hash備份，不帶observed造假。保留真回傳檔案位置，勿手改JSON。
- `restore --request-id NEW_ID --expected-revision CURRENT --backup backup.json`：備份檔由受控流程放imports，先驗schema1/hash/decode並只建立preview；再正常apply。未支援legacy migration／future schema。
- `export [--preview PREVIEW_ID]`：P1離線bundle作備援，不是P4每次live更新所需。
- `observe --report report.json`：離線renderer報告受限核對，仍hostunknown。P4 live ack由同源renderer自動送，不需手造report。
- `recover --request-id NEW_ID`：只有發生corrupt/missing live且有可證明備份才用；保存quarantine／單調watermark。不等於隨便刪檔重來。

同一邏輯請求失回應要用**同requestId、同payload、同expectedRevision**重試取得receipt；換payload必須新ID。同ID異資料拒絕，不以刪receipt繞過。stale revision應先讀新版，明確決定最小差異，不自動覆蓋別人。

## 故障與回復
1. **提交結果未知／503／busy**：UI保草稿與同ID重試；不要同時手改live.json。查status/receipt，確認原程序是否仍活著。不刪一個仍活躍的`.lock`。
2. **圖片錯誤**：不ack壞圖；保前一有效frame。用已驗asset或rollback，不能忽略SHA/decode使畫面假成功。
3. **資料損壞**：先保留整個產品store副本供診斷；走backup/restore或recover，不覆寫未知schema。Windows掉電durability仍未實測，不承諾與Linux目錄fsync等價。
4. **服務停止**：已提交desired留store；桌布頁顯離線提示，尚未驗Windows宿主會如何保畫面。恢復同產品服務後重新載入取得新nonce。
5. **若將來已更換Lively桌布**：事前記錄目標螢幕在Lively UI中的實際編號與原桌布library項；不可直接以Windows DISPLAY2當Lively monitor2。回復時在Lively只把該目標螢幕切回原項，其他螢幕不動，再停Atmos服務。此步尚未授權／尚未執行；不能寫死未知原桌布路徑或謊稱自動回復已通。
6. 不做解除安裝自動刪data；未授權不改開機服務、firewall、全系統PATH，也不kill所有Node/Lively。
