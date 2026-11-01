# P4.2 五語系統資源盤點

## 交付與 API
`src/i18n-catalog.js` 宣告 `window.AtmosI18nCatalog`，純資料、無網路呼叫、無 DOM mutation。
- `locales`: `zh-TW`, `zh-CN`, `en`, `ja`, `ko`。
- `localeNames`: 五種語言的原生名稱。
- `messages[source][locale]`: 明確系統字串的翻譯。source 通常為繁體中文；少數既有 exception 使用原英文 source。
- `patterns[sourceTemplate][locale]`: 帶 `{revision}`、`{name}`、`{text}` 等 placeholder 的整句。整句翻譯後才插入參數，勿逐字翻譯／拼接不同語序。
- `protectedContent` / `formatting`: 原文保護與日期格式約定。

範例：`messages['水平置中']['ja']` → `水平方向の中央`；`patterns['元件 {index}/{count} · {name}']['en']` → `Widget {index}/{count} · {name}`。
只可在明確系統欄位呼叫翻譯，再以 textContent/安全 DOM 寫入。禁止將原始 placeholder 當 HTML 注入。不要盲掃整個 DOM：使用者也可能寫出與 UI 字串完全相同的待辦。

## 已提供覆蓋
- 主題名與副標、場景、天氣（晴天／下雨／下雪／颱風／打雷），不改 enum。
- 七類元件標題、顯隱、位置、中心、尺寸、字體、字級、文字色與恢復風格預設。
- 草稿／提交／衝突／重試／連線／素材校驗／revision 狀態及動態模板。
- 待辦管理系統動作、番茄鐘各階段、天氣城市顯示名與 demo 狀態。
- 系統資料／電量 N/A／媒體未連線等真實能力邊界。
- 主要 aria-label、placeholder、手柄、pager 文字。
- 目標螢幕、解析度、縮放、來源、本機宿主、未偵測及預設比例預覽。明示 manager viewport 不代表 host display。
- 外部 AI 入口、複製成功／失敗、隱私界線與新的完整 prompt 系統段落、狀態模板。

## 不翻譯的內容
- 使用者待辦文字，包括以 `{text}` 放入「完成」aria-label 的參數。
- 日曆事件標題／描述、音樂歌名／藝人，連合成 demo metadata 也視作內容原文保留。
- 使用者輸入的外部 AI 請求、路徑、雜湊、requestId、revision、單位／技術代碼。
- 城市 canonical value 仍台北／東京／倫敦／紐約，只翻譯 option 顯示文字，不改值。
- 不呼叫翻譯服務；沒有內容外傳。

## 日期與時間
日期／星期使用 `Intl.DateTimeFormat(locale, {month:'long',day:'numeric',weekday:'short'})`（格式可按既定版面調整）。語言切換不設定新 timeZone、不改時間模型。時鐘維持固定八格 HH:mm:ss，只翻譯其系統 aria-label。

## 必須由主實作整合／驗收的界線
字典存在不等於全 app 已翻完。本子任務不改 app.js、widgets.js、bridge-client.js 或 HTML，尚未對真 DOM 做每語覆蓋／overflow 驗收。
1. 所有 renderer 更新與 status tick 要走明確翻譯節點，不能只在初載翻一次。
2. 新 inline inspector 實作若新增文案，需加入字典，不能默認回退即算完成。
3. 原 source 有隱藏的 B–E 舊布局字串，已明確不恢復，未花資源翻譯那些移除的入口。
4. 原 prototype prompt 仍有「CLI／MCP／手動圖片匯入均未實作」過時句，**不可繼續展示或只翻譯**。改用已提供的新系統段落：外部 AI 不會自動連線，先核實工具與授權、僅用確認的受限介面，不臆造命令。
5. server validation raw message 不應直接當已翻 UI；UI 可按 error code 配上本語安全摘要並保留 code。資源目前覆蓋 renderer 常見錯誤，未逐條翻譯所有底層 CLI 檔案系統診斷。
6. 新 schema migration 的 exact notice 文案若改變，需同步新增；已包含舊 tornado/fixed-settings notice。

## 本子任務驗證
- Node syntax check 通過。
- 每個 messages/patterns 條目均有五語非空字串。
- 所有 pattern 翻譯與 source 的 placeholder 集合完全相同。
- 沒有以全 DOM replacement 或外部翻譯 API 操作使用者內容。
- 這些是資源完整性純測，不是 browser 全語測通。

## 最後唯讀整合檢視（15:51 UTC）
資源目前 339 exact messages、37 patterns；已按最新 inspector.js 補齊其字體／顏色／位置／七類 switch aria 文案。
`locale.js` 的 messages/patterns object 讀法相容。下列仍需主實作處理：
- 動態 pattern 的 `{name}`／`{phase}` 若是系統名稱，先明確翻譯；`{text}`（待辦）原封保留，不能一律翻參數。
- `generatedPrompt` 是 textarea、會被 DOM 保護略過，應在 builder 對系統段落呼叫 t，再拼使用者原文。
- `document.title` 不在 body TreeWalker 中，需明確翻譯。
- `monitor.source` 若 UI 顯示 unavailable 等機器枚舉，需顯示已翻人類名而保留 API 原值；不得偽報已偵測。
- 未知 backend error 保留安全 code；不能保證任意拼接的底層英文診斷都翻譯。
- 清天氣按鈕應由主源明確改為「晴天」；沒有把一般「無」全域翻成「晴天」，避免誤譯其他 None。

## Prompt 模組（15:54 UTC）
新增 `src/prompt-template.js`：`window.AtmosBuildPrompt(request,state,positionKeys,names,themeCatalog)`。positionKeys 可為函式或物件，日曆讀 calX/cal，時鐘讀 x/y。只讀白名單 visual 欄位，原文 request 只沿既有敏感字串遮蔽，不序列化整個 state，不含使用者待辦／事件／音樂。
25 個純契約斷言（5 語 × 原文保留／憑證遮蔽／未知 state 不輸出／XY 數值／參數替換）通；英文 prompt 除指定原文 request 外無漢字。另 locale runtime mock 15 斷言通；皆非 browser 驗證。
城市 select 的 option 若未設 value，翻譯顯示文字會連隱含 value 一起改。已提醒主實作：翻譯前固定 canonical value，避免 Taipei 等顯示值寫進 schema。
