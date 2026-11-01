# P4.2.1 系統英文漏字與窄面板可讀性稽核

日期：2026-11-01 16:40 UTC。只讀 frozen P4.2／clone P4.2.1 的正式來源與 P4.2 QA 記錄；沒有改 catalog、runtime、inspector、CSS 或主程式，沒有啟動服務／瀏覽器。本報告不是五張最新使用者圖片的逐像素審查；圖片規格由主代理另行確認。

## 1. 已確證而非猜測的漏字

### P0：色票 aria-label 原語言殘留
- `src/inspector.js`：`t('文字顏色') + ' ' + color` 建立色票名稱。
- 正式 `src/i18n-catalog.js` 沒有 `文字顏色 {color}` pattern。靜態提案已補不代表正式 app 已補。
- 用正式 catalog＋locale.js 的隔離 VM 純函式重現：locale=en，`t('文字顏色 #FFFFFF')` 與 `t('文字顏色 #B8D9DF')` 仍完整中文。
- 最小修法：新增五語完整 pattern，建構／refresh 都呼叫 `t('文字顏色 {color}', {color})`。保留色碼原值；不要將文字色碼當使用者自然語言翻譯。語言切回測所有 4 色票。

### P0：新增元件拖曳手柄的裝飾前綴阻斷名稱翻譯
- 既有 widgets renderer 手柄可見內容形如 `↕ 天氣 · 拖曳／方向鍵`。
- 現 pattern `{name} · 拖曳／方向鍵` 將 `↕ 天氣` 整段當 name；對它再 t 不等於單獨的 `天氣`。
- 正式 runtime 純重現：英文結果為 `↕ 天氣 · drag / arrow keys`。這是可見系統字，不是應保留的原文。
- 最小修法：箭頭獨立 aria-hidden span，或使用完整 `↕ {name} · 拖曳／方向鍵` 模板並明確翻譯系統 name。不能泛刪前綴或翻譯任意 `{text}`。
- 檢查 5 個 optional widgets＋時鐘／日曆手柄的可見名及 aria，跨語言來回不殘留。

### P1：raw renderer JSON 直接暴露在主畫面
- `src/bridge-client.js` 直接 `document.body.append(detail)`，detail 是 `#bridgeEvidence` pre。
- `locale.js` 刻意保護 pre／#bridgeEvidence，正確地不改機器資料；因此含中文 note 與英文機器 key 的 JSON 在主頁可見。
- 最小修法：預設收合的 native details「診斷資訊」容器，使用者明確開啟後才顯示原 JSON；顯示一條已翻譯安全摘要即可，不改 observed/hash/source 數據。wallpaper 模式繼續不顯示。
- 不應為消除中文而翻譯 JSON、亂動 host=unknown 或把可驗證欄位刪掉。

## 2. 已確認翻譯正常的動態句（避免重修退步）
正式 locale runtime 的 isolated VM（非 browser）輸出：
- `電量 N/A · 未收到 host 資料` → `Battery N/A · No host data received`
- `電量 N/A · 合成固定樣本／虛構網卡` → `Battery N/A · Fixed synthetic sample / fictional adapter`
- `還有 2 項 · 本機資料（需套用）` → `2 more items · Local data (apply to save)`
- `台北 · 晴朗 · 示範` → `Taipei · Clear · Demo`
- `工作 24:00 · 計時中` → `Work 24:00 · running`
- `目前：未來霓虹（可切換）。` → `Current: Future neon (changeable).`
- `日期與時間顯示` → `Show Date and time`
- `核心 r9 已同步 · host未知` → `Core r9 synchronized · host unknown`

單次英文成功仍不等於所有異常狀態／全部語言已驗。P4.2 QA 真驗已覆蓋五語切換、原文保留、Prompt 英文、日期／metadata；不應再把那些歷史「早期漏字」誤列為目前全部未修。

## 3. 英文容易撐版／裁字的實碼條件
以下是 CSS 風險，不偽稱全部已在 P4.2.1 browser 重現；待收到五張圖後按精確規格修。

1. **三欄屬性 grid**：`.inspect-fields` 固定 3 等欄，右面板 340px，扣 panel／卡片 padding 後單欄約 80px。`Horizontal position X %`／`Vertical position Y %`、`Follow theme` placeholder 更長，容易多行標籤與裁切輸入 placeholder。建議沿批准圖：位置兩欄、font-size 獨立或同語意行；label 可正常換行，數字／hex 值不能被截斷。勿僅把字縮到極小。
2. **heading 同行擠壓**：`.inspect-heading` flex，`Desktop widgets` 和 `Opening another card collapses the previous one` 同行；small 10px 且無明確換行策略。後者是輔助說明，可放下一行，不搶主標題與開關寬度。
3. **theme status 固定高度**：`banner.css` `.theme-status` height/min/max 全 34px 且 overflow:hidden。英文切風格結果句在窄屏可超兩行而被裁。應依內容 auto/min-height 或將冗長成功提示精簡；四 themes 幾何仍共用同規則。
4. **色票行競爭**：四顆固定 20px 色票＋color picker＋hex input 同 flex row。兩 input 共用 flex:1，`Follow theme` placeholder 會被裁；hex 需要至少完整 #RRGGBB 的可見寬度。明確固定 picker 小寬度，給 hex 足夠字符寬；不要藏可用控制。
5. **按鈕長文**：`Restore theme defaults` 與 `Uses theme by default` 同 `.inspect-reset` flex；`Center horizontally`／`Center X/Y` 同行。允許文字自然換行與一致最小高度；必要依批准圖改兩行，不用 overflow:hidden。
6. **長 source／metadata／錯誤**：host ID、來源、預覽尺寸與錯誤 safe code 需要 overflow-wrap:anywhere；不可把 manager viewport 冒稱 host data 來縮文案。
7. **狀態浮條**：`#bridgeStatus` fixed top/right max-width85vw，可能覆蓋長語言的頁頭操作。按批准版面整合至狀態區；保草稿／提交失敗可見，不靜默隱藏。

## 4. 保護原文與色彩界線
- 保留 `[data-user-content]`、待辦正文、media title/artist、輸入與 textarea 的原文保護。即使使用者待辦寫了「文字顏色 #FFFFFF」也不能套 UI 模板。
- calendar 合成 demo 固定系統文案可翻；未來真事件內容必須標 data-user-content，不因解掉舊 `.event` blanket guard 而被誤翻。
- 城市 option canonical value 先固定、顯示文字再翻，P4.2 已修；不得重構後退回隱含 value。
- 色彩本輪只按批准的淺 sage 自然風格；其餘三種現有皮膚不改語意／幾何。任何個別 widget 手選 font／size／color override 繼續跨 theme 保留。

## 5. 最小回歸契約
- 英文 fresh load＋zh-TW→en→ja→en：所有 7 卡、兩種拖手柄、色票、狀態、tooltip／aria 無殘中文系統字；允許原文內容保留。
- 340px panel 與390 viewport：長英文標籤可讀、完整 hex 可見、展開一張／switch 不展開、單一panel scroll不疊下方控制。
- 主頁預設不展 raw JSON；鍵盤可開診斷，原機器資料未改；wallpaper 無診斷入口。
- 4 themes 同 geometry、自然只按批准新色，手動 override 和 draft/apply/取消保持。
- 正式主源打包後驗，而非只驗 static-preview 修補版。
