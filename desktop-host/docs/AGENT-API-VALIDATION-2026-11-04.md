# Agent JSON API 0.3.0 驗證

日期：2026-11-04。Windows 10 x64 build 19045，既有安裝原地升級。使用命令列、原生安裝與真正 renderer 回報；沒有 Computer Use 或使用者瀏覽器／桌面 UI 自動化。

## 介面與範圍

提供 `--agent-help`、`--agent-schema`、`--get-state`、`--validate-json`、`--apply-json`、`--get-result`。同帳戶本機原生命令啟動固定內嵌 Node 模組，沿用管理頁的 commandPatch／executeCommand、交易鎖、CAS、持久 receipt 與回復規則。支援固定等待上限與原生 JSON exit code，不接受腳本或任意系統操作，不新增 HTTP 寫入權限。

操作文件：[AGENT-API.md](AGENT-API.md)。教學、規則及 Schema 可由已安裝執行檔自行輸出，桌布未執行時也可讀取，不需要管理頁 session、Node／Python 外部安裝或 Lively。此階段提供明確提交，不啟用檔案監看。

## 自動與實機結果

- 新增 10 項 Node 測試，涵蓋介面探索、Schema 欄位覆蓋、子程序 JSON／exit code、預檢、部分合併、receipt 重試、版本／ID 衝突、BOM／非法 UTF-8／超大檔案、內部 store／非一般檔案拒絕、非法與跨欄位設定、等待逾時、鎖定錯誤及壁紙角色回報。測試 fixture 的人工 ack 僅屬合成單元測試，沒有當成實機證據。
- 產品 214 passed／2 skipped，共用核心 41 passed／2 skipped，prepare、獨立 draft／path review、CLI help 通過。Rust 13 passed，clippy `-D warnings`、fmt check 通過。合計 268 passed、4 skipped、0 failed。
- 真正安裝版原生命令讀取 help 與 Schema；以隔離 store 實測預檢不提交、提交後無桌布 ack、相同封包 replay 不增版、舊 revision／不同資料重用 ID 被拒絕、非法值 exit 2、未知 receipt found=false。
- 對本機正在執行的桌布，以原生命令提交一次 intensity 變更，收到匹配 revision/hash 的真正 wallpaper-role ack；查詢 receipt 亦確認已提交與回報。之後以新交易還原，收到新的真正桌布 ack，最終 desiredHash 與原設定相同。沒有偽造這兩次實機 ack。
- Windows PowerShell 5 的文件範例管線已實跑，設定 UTF-8 後可捕捉／解析 JSON 並保留城市中文字串。
- 實機整合共五項檢查皆通過，證據：[agent-api.json](../evidence/agent-api.json)。原始輸入、store、logs 留本機，不推送私人資料。

## 升級與限制

0.2.3 → 0.3.0 靜默升級保留當下 live.json bytes 與開機啟動偏好；升級後 payload／執行環境完整性自檢、樣式比對、WorkerW 原生掛載，以及管理頁／桌布近期回報通過。[升級證據](../evidence/agent-upgrade.json)。後續實機交易測試會增加 revision／receipt，因此最終檢查還原的是 desiredHash，而不是整份 store bytes。

桌布角色 observations 獨立保存，管理頁回報不會覆蓋它。只接受 4 秒內且 revision/hash 等於目前設定的 wallpaper 回報；過期、未來時間、錯誤角色、缺檔／壞檔或版本被取代都不宣稱已確認。committed=true 仍只證明持久提交；rendererAcknowledged 不代表 OS／目視呈現，hostVisibility 維持 unknown。本次沒有新的人工目視確認，也不重跑開機、睡眠、卸載、多螢幕／DPI、GPU 或長時間耗用。

安裝包 Atmos Wallpaper_0.3.0_x64-setup.exe：292,186,862 bytes；SHA-256：`3735a9476389203dfd0521fd8aac3bbefe7f8abaf6ed4bb5a8b43eecfe798c54`。自有安裝包未簽章。
