# Renderer Section 增長調查 — 2026-11-05

在進行中的 run1 長時間採樣中發現兩個 renderer 的 private bytes／handles 持續增加。本次調查沒有修改使用者設定、重啟／終止安裝版或操作 UI；只對已歸屬 Atmos 的精確 PID 讀取程序角色與 native handle 類型。

## 已確認的事實

高耗用的兩個程序都是 **renderer**：分別隸屬管理頁與桌布的 browser profile。真正的 GPU-process 是另一組程序，不能將 renderer 的高記憶體／handles 直接歸咎於 GPU 驅動快取。

run1 前 23 個樣本、約 330 秒的局部區間：

| 程序角色 | Handles 變化 | Private bytes 變化 |
| --- | ---: | ---: |
| 管理頁 renderer | 14,037 → 14,273（+236） | 839.55 → 849.86 MiB（+10.31） |
| 桌布 renderer | 14,007 → 14,242（+235） | 563.29 → 570.17 MiB（+6.88） |
| 管理頁 GPU-process | 淨變化 0 | +0.70 MiB |
| 桌布 GPU-process | 淨變化 0 | −0.03 MiB |

兩個 renderer 的 22 個相鄰區間都沒有 handle 下降，每 15 秒約增加 9–12 個。另兩次 native type 快照中，兩個 renderer 的 Section 分別 13,859 → 14,047、13,858 → 14,046；各增加 188，恰好等於各自總 handle 增量，其餘類型沒有變化。GPU-process 只有 37／30 個 Section，與這種持續增長不同。

這些是局部實測，不是完整兩小時結論。不同資料表的兩組快照不是相同時間窗，不能混用作精確速率。

## 候選原因與獨立 A/B

共用的 `bridge-client.js` `ack()` 約每 1.4 秒 POST 一次回報。原實作只檢查 HTTP status，沒有讀取或取消 JSON response body。這與 handles 增長頻率一致，但單靠頻率還不足以證明原因。

因此另外建立 **完全獨立的 headless Chrome、兩個全新 profile 與自行管理的 synthetic HTTP origin**，每一組只對 `/synthetic-body` 發出 600 次小型 JSON POST：

| A/B 組別 | Settled Section 增量 | Settled 總 handles 增量 |
| --- | ---: | ---: |
| 不讀 response body | +614 | +817 |
| 每次 `await response.json()` | +14 | +217 |
| 差異 | **+600** | **+600** |

因此在此 Chromium fixture，未消費 body 的每個回應會額外保留一個 Section。這是對 response-body lifetime 的因果比較，不是送假回報到 Atmos。合成端點沒有使用真實 Agent／wallpaper API、session 或設定；沒有連線安裝版。結束後關閉自己的 HTTP server 與完整 Chrome 子樹。

Private bytes 的組間增長差為 4,689,920 bytes；這項短時差異有啟動／GC 雜訊，不能宣稱它解釋了安裝版所有記憶體增長。Chrome 版本為 156.0.8078.75，另行觀察安裝版 WebView2 為 156.0.4314.58；必須在真正修正後重新實測安裝版，不能以合成 A/B 取代。

## 修正與驗收建議

正常、錯誤及重試路徑都應讀完或取消 fetch response body。對 ack 回應先消費 bounded JSON，再判斷成功／錯誤，不應建立新的 requestId 或偽造 renderer ack。

修正後保留同樣的 native host／renderer 分角色計量，以及 Section count 對照。至少重跑原訂長時間採樣，檢查 handles 是否仍線性增加，並將 private bytes 的暫態快取／GC 波動與持續增長分開。沒有完整實測前，不宣稱零洩漏。

## 重現入口

```text
node desktop-host/scripts/test-http-body-lifetime.mjs --exercise-synthetic
```

僅測試自身新建的 headless fixture，不驅動使用者瀏覽器、不使用 CDP／Computer Use，並停用 GPU，避免額外使用真實桌布 GPU renderer。

`inspect-process-handle-types.ps1 -OwnedPids ...` 只取得指定程序的 type/count。它使用本機 `DuplicateHandle(DUPLICATE_SAME_ACCESS)` 後查 `ObjectTypeInformation`；只關閉本地 duplicate，絕不使用 CLOSE_SOURCE。沒有查詢 object 名稱、檔案路徑或 kernel pointer。Extended handle information 是 Windows 診斷介面布局，其他 Windows／權限環境可能不支援，應回報錯誤而非猜測。

數值摘要：`desktop-host/evidence/renderer-resource-investigation.json` 與 `synthetic-http-body-ab.json`。原始 run1 samples 仍只留本機。

官方 API 依據：[DuplicateHandle](https://learn.microsoft.com/en-us/windows/win32/api/handleapi/nf-handleapi-duplicatehandle)、[NtQueryObject](https://learn.microsoft.com/en-us/windows/win32/api/winternl/nf-winternl-ntqueryobject)。
