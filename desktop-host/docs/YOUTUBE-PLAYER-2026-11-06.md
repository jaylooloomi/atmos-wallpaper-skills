# YouTube 可見播放器（0.7.26）

在「桌面元件 → 播放資訊」貼上 YouTube 影片或播放清單連結，按「播放」。支援 watch、youtu.be 分享、shorts、live、music.youtube.com、embed 與起播時間；暫停、跳轉及音量使用官方播放器內建控制。小播放器固定在管理頁左下方，切換元件或分頁仍可見，停止／關閉會移除 iframe。頁面隱藏、關閉或視窗過小會停止，不會自行恢復播放。

播放器至少 200×200，視窗無法容納時要求放大，避免裁切。依 [YouTube 嵌入文件](https://developers.google.com/youtube/player_parameters) 使用官方 iframe，保留控制及廣告；依 [開發者政策](https://developers.google.com/youtube/terms/developer-policies) 不抽取音訊、不提供隱藏播放器。未載入外部 API script 至 Atmos 主頁，不需要 API key 或 OAuth。若自動播放被瀏覽器阻擋，使用者可按官方播放鍵；部分影片、地區、年齡或權利設定會限制嵌入。

此功能是管理頁內的可見播放器，不是獨立桌布背景音樂服務。關閉／隱藏管理頁會停止；URL 僅保留於本次頁面，不新增或改寫既有設定檔，也不於啟動時自動播放。音訊使用 Windows／WebView2 預設輸出，獨立於原有 CPAL 雨聲裝置選擇與桌布焦點政策。沒有宣稱與其他程式 Now Playing、歌名、進度或音訊路由已整合。舊 demo 設定保留為隱藏相容性綁定，停止顯示虛構曲目；桌面卡片改為中性的管理頁播放入口說明。

## 安全與來源識別

網址僅在本機解析白名單 YouTube 主機、11 字元影片 ID、播放清單 ID 及數字／hms 起播時間。拒絕憑證、非標準連接埠、任意主機、異常參數與超長內容；不向使用者輸入的任意 URL 發送請求。嵌入 URL 由固定 youtube-nocookie.com/embed 路徑產生，丟棄分享追蹤參數；不轉送 Atmos session。

僅管理頁放行該 iframe 來源；桌布仍 frame-src none，review 保留 self。主頁 script-src 與 connect-src 仍為 self／nonce，不引入遠端主頁程式碼。跨來源 iframe 保持 sandbox、無 top navigation 權限。依 [YouTube 來源識別要求](https://developers.google.com/youtube/terms/required-minimum-functionality)，iframe 使用 strict-origin-when-cross-origin，管理頁原 no-referrer 不變。以兩個本機來源的真 Chromium 測試證實 origin-only Referer 已送出：[標頭結果](../evidence/youtube-referrer-2026-11-06.json)。這不代表 YouTube 已接受某影片或實際聽音通過。

## 自動驗證

測試覆蓋連結正規化、追蹤參數剔除、URL 邊界、角色 CSP、尚未操作時無 iframe、重複換片僅一個 iframe、關閉／隱藏／過小停止、過期 iframe 回呼不影響新選擇，以及交易中停止鈕仍可使用。

Chromium 排版測試以隔離 HTTP fixture 將新 iframe 替換為初始空白頁，嚴格確認替換點並量測每個尺寸／元件／風格下的輸入、至少 200×200 播放器、視窗邊界、切換卡片仍可見及停止後 iframe 不殘留。新增 336 個播放器狀態覆蓋，不發送實際 YouTube 請求或偽造播放／渲染 ACK；保留既有所有覆蓋與刻意重疊負控制。

完整 verify 通過：產品 484 pass／3 skip、核心 41 pass／2 skip，共 525 pass／5 個既有情境跳過。12 種尺寸與 336 個新增播放器狀態通過，既有覆蓋及重疊負控制保留。封裝前第二輪完整 gate 亦通過。0.7.26 已更新安裝，10 份既有設定逐位元組保留，開機啟動與資訊省電偏好沿用；安裝／HTTP UI 資源（含 YouTube 模組與合併樣式）等於建置產物，桌布／管理頁真實 ACK 正常。見[升級結果](../evidence/youtube-player-upgrade-2026-11-06.json)。未進行人工聽音、實際影片解碼或完整 YouTube 線上播放驗證；不得把空白 iframe 或 HTTP 載入當成播放成功。使用命令列與隔離 Chromium，無使用者 UI 自動化；安裝檔、私人設定與原始紀錄不提交。

本機 NSIS 293,455,294 bytes，SHA-256 `65c82507bf516e5e27247c826e36365067950ffb45fb7dc02b378b9a5249d525`。
獨立 18 秒／6 次唯讀收尾採樣通過，同一宿主、原生桌布掛載及真實桌布／管理頁回報持續正常，見[收尾結果](../evidence/youtube-player-handoff-2026-11-06.json)。此證據屬 Atmos 宿主存活與渲染回報，不是 YouTube 播放成功或聽音證明。舊安裝檔已清理，僅保留 0.7.26。
