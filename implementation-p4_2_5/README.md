# Atmos P4.2.5 — 全部顯示／隱藏與分隔線修正

已新增 ICS 匯入／訂閱，首次執行請先 `npm ci --ignore-scripts`。操作與資料隱私見 [ICS-CALENDAR.md](docs/ICS-CALENDAR.md)。以下版本歷史中「日曆僅示範」描述不包含本次新增功能；Google／Microsoft OAuth 仍未實作。

沿已交 P4.2.4 增量修正；歷史 P4、P4.1、P4.2、P4.2.1、P4.2.2、P4.2.3 與原型保持凍結。P4.2 功能驗收不代表已符合確認圖，本批按使用者確認圖修正。所有新功能先是草稿，按「套用」才經版本／requestId／CAS 寫入同一本機核心。管理頁 `/manager`，只讀顯示頁 `/wallpaper`。

## 本批最新修正
新增「全部顯示」，工具列順序為位置→復原→全部顯示→全部隱藏→整理位置。顯示／隱藏對稱控制7元件，僅visibility改變；其他位置、風格、待辦、timer資料保留，正式版需套用才提交。去除內部operations與legacy卡重複分隔線，保外框與所有設定。非幾何render僅投影不回寫位置；整理仍可明確改位。詳見 `docs/P425-BULK-VISIBILITY.md`。

## P4.2.4 保留修正
移除指定尺寸成功提示，保留可讀的錯誤驗證。恢復單欄真inline accordion，active body緊接其header、後續headers下移、Tab順序一致。props至少280px且僅其內捲；空間不足只擴非canvas說明區，保持比例與底齊。原Hide all/Arrange移到Position/Restore右側，不再有footer。刪空legacy wrapper，兩垂直gap同15px。詳見 `docs/P424-READABILITY-AND-INLINE.md`。

## 前批保留修正
依使用者真圖及1672px真重現修正右panel底多10px；改直接對說明卡實際bottom，保等比預覽。解析度4控件單row、語言右外獨立panel，窄屏局部橫捲。Accordion header含switch一致focus可見提示。詳見 `docs/P423-ALIGNMENT-CORRECTION.md`。

## 時間與最新捲動規則
時制 12／24、顯示秒數獨立設定，預設24＋秒，locale時段與固定外框、不因tick寫核心。內層format4明確兼容升級。控制panel不捲；七個header可達，僅active props內捲；「全部隱藏／整理位置」已按最新要求移至元件toolbar，不留footer。本版已改回單欄真正行內展開；仍同單activeId與獨立switch。詳見 `docs/P423-CLOCK-AND-SCROLL.md`。

## 保留的前批小改
可見元件彼此 LEFT-to-LEFT、TOP-to-TOP、RIGHT-to-RIGHT 對齊；不同卡寬使用真右邊緣。單軸最近候選、穩定 tie-break、6 CSS px 吸附／10 px 輔助線；自由拖離，不移動其他卡。完整契約見 `docs/P422-EDGE-CONTRACT.md`。資料格式／草稿／套用與皮膚不變，本批未連 Windows。

## 保留的前批視覺修正
自然管理介面改淺 sage／米白卡／olive 控制；Atmos 標頭、四風格橫幅、真六場景縮圖與直排 Prompt。日曆連結移入日曆卡，移除獨立 AI 連接按鈕；核心診斷改可展開而非裸 JSON。英文長標籤與系統拖曳標題、SVG chevron、mono 八字槽、可見卡片左邊緣吸附均有本批修正。詳細閾值／字框策略見 `docs/P421-CHANGELOG.md`。

## 保留的既有實作
- zh-TW／zh-CN／en／ja／ko 系統介面、狀態、可存語言與日期星期。語言不改時區；時鐘預設固定八格 HH:mm:ss，另可選 12／24 小時與顯示秒數開關（四組合），每秒按當前時間重算，tick 不寫 desired。待辦及播放資訊原文不翻、不外傳；目前日曆為合成示範事件，示範標題可本地化，未連真私人行程。
- 七類元件：日期時間、日曆、天氣、待辦、番茄鐘、系統資訊、播放資訊。右約 510px（最新 1.5 倍要求覆蓋原圖 340px）；點標題單一卡內展開，開關只顯隱。舊獨立數值／示範資料區已整合，各元件保原操作節點與草稿功能，沒有新拖曳素材庫或 tabs。
- 每元件安全系統字體堆疊／文字大小／文字顏色；theme＋null 預設跟皮膚，手動 override 換 theme 保留。「恢復主題預設」僅清文字 override，保位置與內容。色票、原生自訂選色與 #RRGGBB；提示可能對比不足，不假稱背景影像對比已量測，不強改顏色。不提供背景／邊框／陰影設定，不下載字體。
- 七類 X/Y、水平／雙軸置中、方向鍵、拖曳邊界、中心線與輕吸附。日曆原 Y 是卡片頂緣、widget X/Y 是左上角；「置中」按實際卡片尺寸計算幾何中心，不偽裝所有卡都必為 50/50。時鐘為中心錨點。原 S/M、clockSize、calendarWidth 能力保留，不新增任意拉框 resize。
- 主「位置」入口統一開所有已啟元件手柄；桌布頁無新增編輯入口。四 theme 只改皮膚，不重排已存文字 override 與座標。天氣「晴天／下雨／下雪／颱風／打雷」enum 不改；P4.1 移除龍捲風及固定 motion 設定保留。

## 手動預覽尺寸與鎖定場景
風格與場景說明整區已移除，四風格及六場景保留；切風格不再自動換場景。新設定拒 lockScene=false；舊資料保原始 bytes 並明示遷移。解析度列可選 auto／常用尺寸／custom，寬高 320–7680 整數；「套用預覽尺寸」先成草稿，既有「套用變更」才寫核心。manual 明示非偵測、非 Windows 解析度操作，CSS preview px 是等比衍生值，不能輸入冒充 target。語言唯一入口移至此工具列右側。

## 真目標比例，不把管理視窗當螢幕
只讀 `monitor-adapter.mjs` 可在明確 Windows runtime 啟動時讀 monitor GDI ID、physical pixel resolution、scale；本輪未在 Windows 執行。資料快取於 runtime 啟動，螢幕配置變更需重啟讀取。GDI ID 不是 Lively monitor index，也不是永久硬體 ID。
- manager preview 是完整 logical canvas 等比 contain；右面板展開不拉長畫布。scale 已知時 logical CSS 尺寸=physical/scale；未知時不宣稱已知 CSS 尺寸。
- 無結果時明示未偵測，使用 1920×1080 預設比例；不是從 manager viewport 推定真螢幕。
- 管理頁在手機仍預覽「目標畫布」的 desktop namespace；只有目標本身窄時才用 mobile namespace。畫布含已存獨立 widget tray，縮放的是整體，不裁掉卡片。
- metadata 在畫布外，分開顯示 target physical／scale／來源與實際 preview CSS 尺寸。fixture 永遠標測試資料，不冒充 host。螢幕 metadata 不等於已安裝或 OS wallpaper 已顯示；ack 的 hostVisibility 仍 unknown。
- 官方 managed dev 可用 `ATMOS_MONITOR_FIXTURE` 指定本機合成 JSON 做 QA；正式啟動必須清除。`tests/fixtures/monitor-4x3.json` 是 1600×1200@125% 合成案例。

## 字級與邊界
clock/calendar 手動字級 12–160px，其他五卡 12–48px；null 跟皮膚。字體 family 僅 theme/system/sans/serif/mono/rounded，顏色僅 null 或 #RRGGBB，拒 CSS 表達式/URL。時計八格不因秒數跳寬。
極大日曆字級保留實際 requested px，採對應行高與可換行內容、受場景邊界限制的文字框，超量時卡內捲動；不假稱同時看全，也不暗縮字。若桌布宿主穿透輸入而不能捲動，請在管理頁選較小字級。一般 widget 主內容亦可在固定卡內捲動。字體設定不改編輯手柄／操作按鈕的 UI 字級。

## 格式與相容
外層 store／backup／commands schemaVersion 仍 1；內層 desired 明確 formatVersion=4。格式 2／3 先用各自凍結 validator 驗證與備份，再升級格式 4；固定 lockScene=true 保原場景，加入 previewMode/Width/Height。新欄位以 typed validator 共用於 CLI、renderer、提交、備份；未支援的版本或值拒絕。
舊無 formatVersion 資料先依原 schema 驗原 hash，保存原 live bytes／歷史，再新 revision 加 locale、文字屬性、日曆 X；原強度、待辦、timer deadline 等保留。舊 tornado→clear／固定 flags 沿 P4.1。備份還原先 preview，不寫 live，apply 才提交；rollback/recover 能辨識舊／新格式，未知版本不靜默降級。
**升級前停止舊服務；不同產品版本不可同時操作一個 store。**

## 重建、CLI、離線顯示
需要既有 Node >=22 與 Python/Pillow，無 npm install。
```sh
npm run prepare
npm test
node bin/atmos.mjs --help
```
CLI 完整用法見 `docs/WINDOWS-CLI-AND-RECOVERY.md`。Cloud 僅官方 Sites managed daemon；不自行另起 listener。Windows 便攜碼未在此輪執行、不安裝依賴、不改現在桌布。
本版離線 export 使用 format4 validator／renderer／locale／appearance，無 session、HTTP bridge、live 寫入。report 仍須 artwork bytes SHA256＋decode＋相符狀態後才能產生；離線下載 report 不是 OS ack，也不是即時橋接。

## 驗證與限制
工程測試 167 項（cloud POSIX，保留左邊緣 10 項，新增三邊候選 11 項及 preview/migration 9 項）；本批真 browser 影響／視覺驗收另列 qa-p4_2_5。前版獨立 core 25 項、monitor fixture/guard 10 項是歷史 P4.2 證據，不充作本批新跑。不把純測、歷史 248 或前版 Windows 57/8 混作本版全平台通過。
真 Google/Microsoft OAuth、天氣服務、host CPU/RAM/流量/電量、NowPlaying 仍 demo/N/A。沒有新增帳號、權限、外部翻譯、網路字體、Windows 遠控或自動換桌布。scoped token 是防誤寫能力檢查，不是惡意同源 JS sandbox。Windows fsync 分支不含 directory flush；宿主生命週期、鎖屏、DPI、耗電和完整 Windows 行為仍需另批實機驗證。
