# Windows 資源與 GPU 實測 — 2026-11-05（0.7.1）

**修正後安裝版已完成連續兩小時採樣，桌布目標在 481／481 個採樣點有新鮮相符回報，未觀察到原生掛載中斷。先前每次 ACK 增加 Section 的特定症狀未再出現；整棵程序樹與嵌入核心仍有 private bytes 增長警示，不能宣稱全部耗用問題已解決、已全面進入平台或零洩漏。**

本報告將成功收集、量測覆蓋、renderer 回報、原生狀態及仍待調查的增長分開記錄。所有數值以[匿名完整統計](../evidence/windows-soak-2026-11-05.json)為準；短時模式矩陣與最終回歸測試的收尾另列文末。

## 本輪範圍與執行狀態

本輪測試 Windows x64 安裝版 **0.7.1**，包含 ACK response body lifecycle 修正。真正安裝版由原生宿主掛載主螢幕桌布，管理頁保持開啟；這是「桌布＋管理頁＋嵌入核心及其後代」的總成本，不是只開桌布的隔離基準。

| 項目 | 實際資料／界線 |
| --- | --- |
| 修正後 run2 | 成功完成標記 `collected`；觀察區間達到原訂兩小時 |
| 台北起訖 | 2026-11-05 01:06:23 → 2026-11-05 03:06:23 |
| 計畫時長／實際涵蓋 | 7,200 秒／7,200.008 秒；收集器執行 7,200.655 秒 |
| 採樣數／預設間隔 | 481 筆／15 秒；480 個相鄰區間 |
| Windows／顯示環境 | Windows build 19045；單一主螢幕，由原生宿主提供實體邊界 |
| GPU／驅動 | NVIDIA GeForce GT 710；vendor 驅動 475.14（Windows 表示為 30.0.14.7514） |
| CPU 正規化 | 56 個邏輯處理器；同時報告單核心與整機百分比 |
| 工作負載 | 內建 future 場景、rain、intensity 100、無自選圖片；顯示元件計數 2，包含時鐘／日曆 |
| 程序範圍 | 15 個已記錄程序生命週期：宿主 1、Node 核心 1、WebView2 12、其他後代 1 |
| 設定變更 | 外觀、layout、host placement revision 變化均為 0；目標數始終為 1；記錄到的工作負載變化為 0 |
| 回報的有效政策 | 全部 481 筆為 `paused=false`、30 FPS cap、AC 供電、reason=active；有效政策變化為 0 |
| 遮蔽背景 | 9 筆回報 fullscreenCovered=true、472 筆為 false；背景狀態轉換 14 次，未造成有效 cap／pause 變化 |
| 人工操作 | 依使用者要求先做自動腳本；沒有本輪目視、鎖屏、登入或裝置插拔驗收 |

30 FPS 是回報的繪製上限，**不是量到的實際呈現 FPS**。設定及政策不變，也不能證明每一刻的實際 GPU 負載、外部資料或其他程式完全相同。

原始採樣、程序識別、私密路徑、session、服務位址、設定內容及完整系統紀錄只留本機。公開資料只使用審閱過的匿名聚合，不提交原始 samples 或私人設定。

## 已發現問題、修正與證據分界

0.7.0 的 run1 在採樣中發現管理頁與桌布 renderer handles／private bytes 增長。局部約 329.9 秒區間，兩個 renderer 的 handles 分別增加 236／235；另一組 native 類型快照中，兩者各增加 188 個 Section，恰好等於各自總 handles 增量。不同快照的時間窗不同，不混成單一精確速率，也不將 renderer 成本歸咎於另一個 GPU-process。

原 `bridge-client.js` 的 ACK 只檢查 HTTP status，未讀取或取消回應 body。獨立 headless Chrome A/B 對合成端點各送出 600 個請求，穩定後「不消費 body」相對「消費 JSON」多保留 **600 個 Section**。這證明該合成環境中的 body lifetime 因果關係；不能直接解釋安裝版全部記憶體增長。

0.7.1 對成功 ACK 完整消費 JSON 並核對 accepted／revision／來源／可見性界線；錯誤回應取消未讀 body。另加入單一進行中 ACK、五秒 timeout，並阻止舊畫面遲到回覆標記新畫面。正常 snapshot 的 ETag 唯讀查核保持穩定；未發現每輪 timestamp 造成圖片重載的證據。

run1 因已授權修正升級而失去原宿主，其標記為 **root-lost**：約 857.1 秒、58 個樣本，屬於中止／介入資料，不是兩小時通過。修正後另開新的 run2 完成這次連續兩小時；沒有將兩段拼接。兩次執行的起始累積狀態及持續時間不同，不用它們宣稱整體記憶體下降百分比。

相關證據：

- [renderer Section 調查](RENDERER-SECTION-INVESTIGATION-2026-11-05.md)
- [ACK response lifecycle 修正及六項回歸測試](ACK-RESPONSE-LIFECYCLE-2026-11-05.md)
- [合成 HTTP A/B 匿名統計](../evidence/synthetic-http-body-ab.json)
- [原安裝版局部匿名統計](../evidence/renderer-resource-investigation.json)
- [修正後 renderer 分角色檢查](../evidence/renderer-soak-checkpoints-2026-11-05.json)

另外的 200 次 headless HTTP lifecycle fixture 驗證成功／失敗回應確實消費或取消 body。這些端點與安裝版隔離，沒有真實 session、設定寫入或偽造桌布 ACK。合成測試與安裝版實測分開列示。

## 收集、覆蓋與完整性

收集器以確定的原生宿主及建立時間為起點，使用 Toolhelp 父子關係辨識後代；已證明歸屬且仍存活的後代，重新掛父後仍依程序身分保留。程序身分包含建立時間，不讓重用的 PID 繼承舊歸屬。不把其他 App 的 WebView2、收集器或整台機器程序混算成 Atmos。兩次輪詢之間完全建立並結束的短命程序仍可能漏掉。

| 項目 | 結果 |
| --- | --- |
| 時間／輸入 | 481／481 有效時間；無不遞增、無損壞 JSON 行、無截斷尾行 |
| 採樣間隔 | median 15.0002 秒、p95 15.0666 秒、max 15.2536 秒；無超出分析規則的長缺口 |
| 單筆收集時間 | median 568.59ms、p95 1,378.66ms、max 2,346.37ms；不是 Atmos 本身 CPU 用量 |
| CPU 完整區間 | 451／480 個；6,765.263 秒，占觀察時間 **93.9619%** |
| CPU 部分區間 | 29 個；434.745 秒，占 **6.0381%**，另列 matched lower bound |
| CPU 排除區間 | 長缺口、query failure、無效樣本、counter 不可用排除數均為 0；counter 回退為 0 |
| 程序快照 | 463／481 完整快照（96.2578%）；18 筆保守標為 partial，未混入主要 memory／handles 趨勢 |
| 程序身分 | 已記錄生命週期始終 15 個；觀察到新增、消失、PID 重用、晚發現的生命週期均為 0 |
| 原生一致性 | root 身分矛盾、session mismatch、native stale、錯誤／未來 timestamp、錯誤 root PID 均為 0 |

18 筆 partial 是候選子程序 counter 讀取嘗試的不確定紀錄；已知穩定生命週期仍有資料。收集器的廣泛 catch 未保存該候選的具體 native error／exception 類型，**無法判定是存取失敗、race、不可用 counter 或程序剛退出，也不能把它肯定描述成正常 helper 退出**。分析保守地將這些樣本與相鄰 CPU 區間列為部分覆蓋。

CPU 以累積 CPU 秒數除以相鄰單調時鐘間隔求差。單核心 100% 代表占滿一個邏輯處理器；整機百分比再除以 56。部分區間只報相符身分的已觀察下界，不當成全樹完整 CPU。方法及完整性規則見[採樣分析口徑](SOAK-ANALYSIS-METHOD-2026-11-05.md)。

## CPU 結果

以下為 451 個完整 cohort 區間；百分比是程序樹 CPU，不是整機總忙碌率。樹狀聚合可同時使用多個核心，單核心正規化 peak 超過 100% 並非無效值。

| 範圍 | 單核心 mean % | 單核心 p95 % | 單核心 peak % | 整機 mean % | 整機 p95 % |
| --- | --- | --- | --- | --- | --- |
| 整棵程序樹 | 77.56 | 85.45 | 103.43 | 1.3850 | 1.5259 |
| 原生宿主 | 6.44 | 7.92 | 12.19 | 0.1151 | 0.1414 |
| 嵌入 Node 核心 | 4.23 | 5.29 | 6.35 | 0.0755 | 0.0944 |
| 全部 WebView2 後代 | 66.89 | 73.86 | 89.58 | 1.1945 | 1.3189 |
| 其他已歸屬後代 | 0.00 | 0.00 | 0.00 | 0.0000 | 0.0000 |

29 個部分區間的整樹已觀察下界：單核心 mean **75.26%**、p95 **82.48%**；整機 mean **1.3439%**。全部 480 區間的 matched lower bound mean 為單核心 **77.42%**／整機 **1.3825%**；後者仍含部分覆蓋，不能取代上表完整區間的結果。

## 記憶體結果與仍在增長的部分

下列主要統計只使用 463 筆完整快照；18 筆 partial 的觀察值另留在 JSON 的 `partialSnapshotObserved`。單位 MiB（1,048,576 bytes）。

### Private bytes

| 範圍 | First MiB | Last MiB | Median MiB | p95 MiB | Peak MiB |
| --- | --- | --- | --- | --- | --- |
| 整棵程序樹 | 734.71 | 1,072.65 | 1,020.59 | 1,087.55 | 1,224.56 |
| 原生宿主 | 5.46 | 5.55 | 5.54 | 5.87 | 6.02 |
| 嵌入 Node 核心 | 58.19 | 166.20 | 139.66 | 177.52 | 180.69 |
| 全部 WebView2 後代 | 664.86 | 894.69 | 868.03 | 899.34 | 1,068.89 |
| 其他已歸屬後代 | 6.21 | 6.21 | 6.21 | 6.21 | 6.27 |

### Working set

程序 working set 相加可能重複計入共用頁面，不能稱為唯一實體 RAM。

| 範圍 | First MiB | Last MiB | Median MiB | p95 MiB | Peak MiB |
| --- | --- | --- | --- | --- | --- |
| 整棵程序樹 | 1,004.80 | 1,061.66 | 1,242.20 | 1,303.87 | 1,383.11 |
| 原生宿主 | 23.69 | 21.68 | 24.08 | 24.34 | 24.56 |
| 嵌入 Node 核心 | 80.86 | 123.08 | 145.97 | 157.82 | 161.18 |
| 全部 WebView2 後代 | 889.02 | 911.38 | 1,058.15 | 1,116.11 | 1,191.95 |
| 其他已歸屬後代 | 11.23 | 5.52 | 11.23 | 11.23 | 11.26 |

### 預先指定的五分鐘暖機分析

排除前五分鐘，再比較不重疊的開頭／結尾五分鐘中位數；穩態共 443 個完整樣本、約 6,900 秒。OLS 與 Theil–Sen 為描述性每小時斜率，不是外推保證或單憑斜率就能確診洩漏。

| 範圍 | 開尾窗中位數 MiB | 變化 MiB | OLS MiB/h | Theil–Sen MiB/h |
| --- | --- | --- | --- | --- |
| 整棵程序樹 | 901.07 → 1,076.09 | 175.01 | 88.84 | 81.54 |
| 原生宿主 | 5.46 → 5.55 | 0.09 | 0.09 | 0.07 |
| 嵌入 Node 核心 | 69.81 → 169.14 | 99.34 | 43.30 | 40.22 |
| 全部 WebView2 後代 | 818.80 → 895.63 | 76.84 | 45.45 | 40.67 |
| 其他已歸屬後代 | 6.21 → 6.21 | 0.00 | -0.00 | 0.00 |

**整樹 +175.01 MiB 與 Node 核心 +99.34 MiB 都觸發 `PRIVATE_MEMORY_GROWTH_REVIEW`。** WebView2 合計也增加 76.84 MiB，雖未跨過該組的相對門檻，仍不能稱為完全持平。全樹 peak 1,224.56 MiB 必須保留，不以較低的最後一筆取代峰值。

### 三十分鐘後的補充視角

以下是額外排除前三十分鐘的描述性視角，共 344 個完整樣本、約 5,400 秒。它不能改寫預先指定的五分鐘規則，也不移除原本的警示。

| 範圍 | 開尾窗中位數 MiB | 變化 MiB | OLS MiB/h | Theil–Sen MiB/h |
| --- | --- | --- | --- | --- |
| 整棵程序樹 | 1,000.67 → 1,076.09 | 75.41 | 69.77 | 66.93 |
| 嵌入 Node 核心 | 134.48 → 169.14 | 34.67 | 34.07 | 32.30 |
| 全部 WebView2 後代 | 853.48 → 895.63 | 42.15 | 35.66 | 31.55 |

Node 核心在此較晚視窗仍增加 **34.67 MiB**，Theil–Sen 約 **32.30 MiB／小時**，仍觸發記憶體增長檢查。峰值後曾回落及部分區段較平，不能推論為所有程序已進入平台。現有聚合不能區分 GC、cache、WebView／驅動配置或其他保留來源；原因仍待進一步受控調查，沒有證據足以把增長直接宣布為無害或確診某一類洩漏。

## Handles、threads、Section 與程序生命週期

完整快照中的程序數始終為 15（WebView2 為 12）；沒有觀察到程序身分增生／重啟。這不排除輪詢之間未記錄到的短命程序或同一程序內的活動。

| 範圍 | Handles first → last | Handles median / p95 / peak | 開尾窗 handles Δ | Handles OLS / Theil–Sen 每小時 | Threads first → last | Threads median / p95 / peak | Threads OLS / Theil–Sen 每小時 |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 整棵程序樹 | 6,822 → 6,975 | 6,918 / 6,993.9 / 7,007 | 96 | 78.35 / 77.09 | 443 → 457 | 449 / 460.0 / 464 | 10.99 / 11.83 |
| 原生宿主 | 320 → 318 | 317 / 319.0 / 323 | 1 | 0.43 / 0.00 | 8 → 6 | 6 / 7.0 / 11 | -0.16 / 0.00 |
| 嵌入 Node 核心 | 230 → 248 | 229 / 251.0 / 270 | 20 | 14.02 / 11.68 | 11 → 14 | 10 / 14.0 / 15 | 2.86 / 0.00 |
| 全部 WebView2 後代 | 6,164 → 6,301 | 6,265 / 6,318.0 / 6,337 | 74 | 63.90 / 64.79 | 422 → 435 | 431 / 438.0 / 442 | 8.33 / 9.00 |
| 其他已歸屬後代 | 108 → 108 | 108 / 108.0 / 108 | 0 | 0.00 / 0.00 | 2 → 2 | 2 / 2.0 / 4 | -0.03 / 0.00 |

表中開尾窗變化及斜率均排除前五分鐘；first／last 是整段完整快照。整樹 handles 開尾窗中位數 **6,881 → 6,977（+96）**，尚未跨過此規則的 handle-growth 門檻，但正斜率及 threads 增長仍照實保留，不等同零資源增長。

修正後 renderer 的 Section 唯讀檢查，管理頁為早期 **46 → 結束 48（+2）**，桌布為 **37 → 37（0）**。這支持本次兩小時內未再看到先前約每次 ACK 保留一個 Section 的特定症狀；不能據此解釋其他 private bytes 增長或排除更慢的問題。分角色與 native checkpoint 證據見[匿名檢查摘要](../evidence/renderer-soak-checkpoints-2026-11-05.json)。

## GPU 結果

採樣使用同一份持續存在的 English PDH query，首次引擎值為 warmup，後續檢查狀態。每筆按當時已歸屬程序更新範圍；無效／缺失不補成零。

| 覆蓋項目 | 結果 |
| --- | --- |
| 有 GPU 資料的樣本 | 481 |
| 有效 owned engine 樣本 | **454／481（94.3867%）**；warmup 1 筆 |
| Frame／query error | 8 筆；保留在 coverage，未放入主要統計 |
| 部分程序樹樣本 | 18 筆；即使有可讀 GPU 值，仍排除於主要歸屬統計 |
| 明確無效／超界 instance | engine invalid、out of range、memory invalid 均為 0 |
| 額外排除的 available 值 | engine 18 筆、程序 GPU memory 26 筆；以實際有效樣本分母為準，不把可能重疊的排除數任意相加 |
| 有效程序 GPU memory | 455／481（94.5946%）；memory 可於首筆引擎暖機時已有有效帳務 |
| 有效全機 adapter memory | 473 筆；只作全機背景，不能歸因 Atmos |

| 指標 | Median | p95 | Peak | 定義／界線 |
| --- | --- | --- | --- | --- |
| Busiest accounted engine | 45.22% | 54.04% | 66.89% | 同一實體 adapter／engine 內先合計已歸屬程序，再取各 engine 最大值；不是整張 GPU 百分比 |
| Accounted dedicated | 241.43 MiB | 281.35 MiB | 349.86 MiB | 程序帳務合計，不是唯一實體 VRAM |
| Accounted shared | 13.62 MiB | 15.64 MiB | 16.58 MiB | 共用 surface 可能重複計入 |
| 全機 adapter dedicated | 1,572.76 MiB | 1,753.88 MiB | 1,857.09 MiB | 包含其他程式，只作背景 |
| 全機 adapter shared | 231.85 MiB | 300.23 MiB | 382.31 MiB | 包含其他程式，只作背景 |

有效 engine 最低觀察值為 31.82%；有效樣本均涵蓋 10 個實體 engine 鍵。不要把並行的 3D／Copy／Decode 引擎百分比相加當成整張 GPU 使用率。

程序 dedicated 帳務在五分鐘暖機後的開尾窗中位數為 **253.30 → 236.49 MiB（−16.81）**，shared 為 **13.00 → 13.80 MiB（+0.80）**。這些值有共用帳務及 Windows 10 已知 counter 限制，不能單靠曲線診斷顯存洩漏。GPU-process 的 private bytes 也與這組 GPU 記憶體帳務不同，不混用。

DWM 合成成本未完整歸因到 Atmos。GPU 引擎百分比不等於耗電、溫度、呈現 FPS 或目視流暢度；來源摘要沒有 GPU mean，這裡只報已提供的 median／p95／peak。詳細定義與官方來源見[GPU 可觀測性](GPU-OBSERVABILITY-2026-11-05.md)。

## Renderer 回報、原生狀態與事件

- 單一桌布目標的新鮮相符回報為 **481／481**：negative／unknown／stale／future／missing timestamp 均為 0；沒有觀察到 ACK outage 或 native attachment outage。
- 回報年齡 median **683ms**、p95 **1,323ms**、peak **1,805ms**。回報核對 live revision／hash、目標及投影；原生檢查另外核對宿主 session 與實體 bounds。
- 聚合共記錄 962 列 renderer 報告、每筆峰值 2 列；來源未保留 renderer ID，獨立 renderer 總數為 unavailable／null，不能把它寫成 0，也不能單由列數證明獨立程序或管理頁 freshness 的完整分母。
- 採樣點間隔約 15 秒；未觀察到缺口不等於排除兩次輪詢之間的短暫問題。renderer 聲明及原生掛載均不等於本人看見桌布正常。
- Session／power 事件註冊可用；本段捕獲 SessionLock、SessionUnlock、Suspend、Resume **各 0 次**，沒有 events JSONL。這不證明事件絕未發生，也不構成鎖屏／睡眠恢復通過。另行系統事件查核與其查詢範圍由收尾附註補充，不能由通道可讀推論實際恢復。

## 警示審閱與結論

| Analyzer flag | 本次處理／仍待釐清 |
| --- | --- |
| PRIVATE_MEMORY_GROWTH_REVIEW — all | 保留 +175.01 MiB 的主分析警示；晚視窗仍增加 +75.41 MiB，沒有宣稱全樹持平 |
| PRIVATE_MEMORY_GROWTH_REVIEW — runtime | 保留 +99.34 MiB；三十分鐘暖機補充仍 +34.67 MiB 且有警示；來源待受控調查 |
| PROCESS_COVERAGE_PARTIAL | 18 個不確定候選 counter 樣本；CPU 29 個相鄰部分區間另列下界，不當成產品程序重啟或確定 helper 退出 |
| GPU_COUNTER_COVERAGE_PARTIAL | 8 個 frame error 及 18 個部分程序樹樣本等排除如上；有效 engine 454／481，不用零填缺口 |

沒有觀察到 root 消失、已記錄程序身分增生、相符 ACK 缺口、原生掛載中斷或 timestamp／session 矛盾。**兩小時已完整收集且有可用 CPU／GPU 資源統計；特定 ACK／Section 累積症狀已有修正後實機支持，但記憶體增長警示仍存在，因此此結果不是所有耗用問題皆通過的證明。**

## 功耗／能量未測得

本輪可量的是 CPU／記憶體／handles／程序數及 GPU 資源，**沒有取得有效瓦特、焦耳或電池續航量測**。481 個樣本的 power.available 均為 false，沒有功率或能量分析。

本機為 AC 供電且無系統電池；原生 EMI 枚舉找到零個目前存在的能源計介面。Power Meter／Energy Meter 名稱雖註冊，實際讀值不可用。既有 NVIDIA 工具／NVML 存在，但 GT 710 的功率讀值回報不支援；這些都不能當成零瓦特。單次 vendor 探查未加入週期採樣或改寫 run2 起始 metadata。

Windows Power Usage 分級考慮 CPU／GPU／磁碟活動，E3 也可能使用軟體估算模型；它們不能直接冒充硬體量測。watts／joules 保持 null，沒有執行伴隨長測的 `powercfg /energy`、提權、驅動安裝、GPU 功率限制／時脈調整或系統電源設定變更。

能力查核、pure parser 測試及官方依據見[功耗能力報告](POWER-CAPABILITY-2026-11-05.md)。

## 短時模式矩陣與最終回歸測試

完整兩小時資料先結束並封存，再以同一安裝版宿主測以下三種情境。管理頁保持開啟，沒有調低使用者的 FPS 設定來降低量測值。

| 情境 | 實際涵蓋秒數／樣本 | 全目標 ACK | 整機 CPU 平均／完整時間覆蓋 | GPU 最忙引擎 median／p95／peak | GPU 有效覆蓋 |
| --- | --- | --- | --- | --- | --- |
| 單螢幕 thunder100 | 120.65／29 | 29／29；1 目標 | 1.478%／96.63% | 45.28%／52.62%／57.27% | 28／29（96.55%） |
| 雙螢幕 duplicate rain100 | 120.82／29 | 29／29；2 目標 | 1.868%／83.64% | 78.71%／82.46%／83.20% | 26／29（89.66%） |
| 雙螢幕 span rain100 | 122.47／30 | 30／30；2 目標 | 1.924%／86.92% | 83.37%／86.93%／87.70% | 27／30（90.00%） |

三段皆完成真正原生掛載、相符回報、配置／工作負載與至少 80% CPU／GPU／記憶體覆蓋門檻；每段仍有 `partialMeasurement=true`，保留所有覆蓋缺口與分析旗標。請求採樣間隔為 2 秒，但內建 Windows PowerShell 的收集成本使實際多數間隔約 4 秒，以真正單調時間計算 CPU，沒有假設請求間隔已達成。GPU 是時間窗內的引擎活動，不代表捕捉短閃電瞬間的呈現峰值。短測不能當作每模式兩小時、實際 FPS、視覺美感或零洩漏證明；本機雙螢幕的 GPU 引擎負載也明顯較高。

兩次前置 harness 問題均已修復並保留本機失敗紀錄：第一輪未載入 `System.Net.Http` 組件，第二輪先抓 heartbeat 再做較慢政策查詢，造成本地假 stale。修正為明確載入組件、政策／拓樸先讀與 heartbeat 最後讀；未放寬 5 秒時效或 PID／session 檢查。兩輪都安全還原，第三輪才是上表的成功結果。延遲讀取、真正 stale、並行修改與中斷恢復有獨立 fixture。

成功輪結束後，原 desiredHash、layout、placement、同宿主持續存活、真實 ACK 及原生桌面視窗中繼資料還原檢查全部為 true。交易 revision／history／receipt 會正常前進，不宣稱整份 store 位元組不變；這仍不是人工目視還原確認。[完整匿名模式摘要](../evidence/resource-matrix-2026-11-05.json)。

Node 最終 `scripts/verify.mjs` 為 **449 項：444 通過、5 略過、0 失敗**（產品 403 通過／3 略過，核心 41 通過／2 略過）。三項 file-symlink 案例因本帳號無建立權限略過，兩項 POSIX FIFO 案例因 Windows 不支援略過；junction 與 Windows named-pipe／device-path 負面案例有實際執行。prepare、draft review、path review、CLI help 均成功；詳見 [`verification.json`](../../implementation-p4_2_5/evidence/verification.json)。Rust 最終 `cargo test --locked` 為 **48 通過、0 失敗、0 略過**，沒有將舊版數量當成本輪重跑。

矩陣 runner 完全結束後，以獨立命令再觀察 18 秒，確認同一安裝版宿主、可見自有視窗及新鮮 manager／wallpaper 回報持續存在；[獨立交接證據](../evidence/resource-handoff-2026-11-05.json)。這是程序與中繼資料檢查，不是人工目視或長測的替代。

## 系統事件與重現入口

完整起訖範圍內，System Display 4101、Application Error 1000、Application Hang 1002、Windows Error Reporting 1001 四類查詢皆可讀，符合查詢條件的事件均為零。不可讀時應回 null／unavailable；本次不是用不可讀冒充零。Display 4101 只算全機背景，不歸因給 Atmos；程式事件須符合 fault PID 與精確建立時間，不能單憑程式名或 Event System.Execution PID 歸屬。[匿名事件摘要](../evidence/windows-resource-events-2026-11-05.json)。沒有事件紀錄仍不等於人工鎖屏／睡眠恢復已通過。

以下為開發驗證工具，原始資料僅寫本機 AppData 下明確的新目錄；安裝版使用者不需要為此另裝開發工具。

```powershell
$run = "$env:LOCALAPPDATA\AtmosWallpaper\new-resource-run"
powershell -NoProfile -File desktop-host/scripts/measure-installed-soak.ps1 -Run -DurationSeconds 7200 -SampleSeconds 15 -OutputDirectory $run
node desktop-host/scripts/analyze-soak.mjs $run
powershell -NoProfile -File desktop-host/scripts/read-soak-events.ps1 -RunDirectory $run
```

短時矩陣使用 `node desktop-host/scripts/test-resource-matrix-installed.mjs --run --output <新的私人絕對目錄>`；它會暫時變更自有桌布，持久化復原意圖並以 CAS 還原。強制中斷後須以同目錄 `--recover` 明確恢復；宿主/session 或並行 revision 已改變時拒絕覆寫。不要直接修改 `live.json`，也不要把原始目錄或 preimage/journal 推上 Git。

## 尚未人工驗收與其他限制

- 使用者先前確認過其他版本的桌布、時鐘、圖示及音訊；不能直接挪作 0.7.1 本輪人工驗收。此次沒有操作使用者的瀏覽器／桌面 UI，也未要求人工鎖屏、重新登入、裝置插拔或切換 DPI。
- 韌體不支援 S0／S1／S2／S3，無法將本機列為真正睡眠／喚醒通過。休眠可用不代表已測；沒有改做休眠取代睡眠，也不代為解鎖或認證。
- 只涵蓋本機 Windows／GPU／驅動及所述工作負載；不能泛化到所有場景、多螢幕長測、其他裝置、混合 DPI、熱插拔或其他平台。
- 管理頁保持開啟及日常其他程式會影響背景；沒有做受控 Atmos 開／關耗電基準，也未完整歸因 DWM 成本。
- 即使較長觀察也不能保證零洩漏；本次既有增長警示尤其不能抹去。原始 logs、設定、命令列、session、私人素材與日曆資料保持本機。
