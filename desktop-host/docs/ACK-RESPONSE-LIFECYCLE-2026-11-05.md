# ACK response lifecycle 修正 — 2026-11-05

本文件記錄原始碼修正與自動化驗收，對應 [renderer Section 調查](RENDERER-SECTION-INVESTIGATION-2026-11-05.md)。所有程序以角色呈現；不收錄實際 PID、session、服務位址或原始採樣資料。

## 觀察與因果界線

run1 的約 329.9 秒局部區間，管理頁 renderer handles 增加 236、桌布 renderer 增加 235，約每 1.4 秒增加一個。另一組 native handle-type 快照顯示兩個 renderer 各增加 188 個 Section，恰好等於各自總 handle 增量，其他類型沒有變化。這兩組資料時間窗不同，不混用為同一精確速率。

當時場景／revision／效能政策保持同一狀態。額外的五次唯讀 snapshot 檢查為首次完整回覆、其後四次未變更回覆，ETag 相同；沒有發現 timestamp 每輪改變 ETag 或每 700ms 重載圖片的證據。正常新圖片路徑也會撤銷上一份 object URL。

原 `ack()` 約每 1.4 秒送出一次回報，但只檢查 HTTP status，未消費 response body。獨立 synthetic Chromium A/B 的 600 次請求，在穩定後產生 **600 個額外 Section 的組間差異**，支持該 fixture 中未消費 body 的因果關係。這不等於已證明它解釋安裝版所有 private bytes 增長，也不能代替真正 WebView2 的修正後長測。數據見 [合成 A/B 摘要](../evidence/synthetic-http-body-ab.json) 及 [匿名實機局部摘要](../evidence/renderer-resource-investigation.json)。

## 程式修正

`implementation-p4_2_5/src/bridge-client.js`：

- 成功 ACK 先完整讀取 JSON，驗證 `accepted=true`、正確 revision、`source=local-bridge` 與 `hostVisibility=unknown`，才更新本頁確認狀態。
- ACK 的失敗回應會取消尚未消費的 body；snapshot 的認證失敗、其他 HTTP 失敗，以及 artwork HTTP 失敗亦有相同處理。
- ACK 同時只允許一個進行中請求，並設五秒 timeout，避免停滯時堆積重疊請求。
- 舊畫面的遲到回覆不會將新畫面標成已確認；任何錯誤仍保留「未確認」界線。

這次沒有更改 ACK 的角色授權、核心 revision、投影驗證或提交內容，也沒有偽造 renderer 回報。Node HTTP 正常以完整 Buffer 結束回覆；未發現未結束的 HTTP framing 是主要來源的證據。

## 自動化測試

`implementation-p4_2_5/tests/bridge-response-lifecycle.test.mjs` 六項測試直接對產品原始碼注入合成 DOM／fetch，使用真正 `Response`、`ReadableStream`、`bodyUsed` 與 cancel callback，涵蓋：

1. 成功回覆的消費與 accepted／revision／來源／可見性驗證。
2. 失敗 ACK body 取消。
3. 單一進行中請求與 timeout signal。
4. 非法或過期畫面回覆不更新 ACK 狀態。
5. snapshot 認證／HTTP 錯誤取消 body。
6. artwork HTTP 錯誤取消 body，維持無 ACK。

`implementation-p4_2_5/scripts/audit-response-lifecycle.mjs` 使用全新獨立 profile、headless Chrome 與自行管理的合成 HTTP origin，完成 200 次成功 body 消費及兩個失敗拒絕案例。它不連安裝版，不操作使用者瀏覽器、不使用 Computer Use／CDP，不建立真實桌布 ACK；結果只證明該 fixture 的協定與 body lifecycle。

## 安裝版長測

原 run1 因已授權修正升級而終止，保留 `root-lost`／介入註記，不能算兩小時通過。0.7.1 的獨立 run2 已完成 7,200.008 秒、481 個樣本，481 次全目標新鮮 ACK 均正常。桌布／管理頁 renderer 的 Section 由早期 37／46 到結束 37／48，未重現原先每次 ACK 的累積模式。

這不代表全部資源都沒有問題：全程序樹及核心的 private bytes 增長診斷仍存在，計數器也有部分覆蓋缺口，不宣稱耗用下降比例或零洩漏。完整數字、後段波動與限制見 [兩小時資源報告](WINDOWS-RESOURCE-VALIDATION-2026-11-05.md) 和 [修正後角色快照](../evidence/renderer-soak-checkpoints-2026-11-05.json)。
