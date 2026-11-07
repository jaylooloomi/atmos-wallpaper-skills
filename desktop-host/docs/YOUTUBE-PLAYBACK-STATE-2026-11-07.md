# YouTube 播放與預覽狀態

原卡片為固定文字，iframe onload 也不能確認播放。現在使用官方 IFrame Player API 的 onReady／onStateChange／onAutoplayBlocked／onError。只有 state=1 時顯示播放中與音柱；載入、緩衝、暫停、結束、嵌入受限與未知各自顯示。按播放會建立可見播放器並嘗試 playVideo，自動播放限制仍由 YouTube／瀏覽器決定。

- 官方文件：https://developers.google.com/youtube/iframe_api_reference
- 保留可見且至少 200×200 的實際影片播放器，不藏影片來抽取音軌；卡片音柱是播放狀態動畫，不是音訊頻譜量測。
- 載入 SDK 僅在使用者按播放後；manager CSP 僅新增官方 /iframe_api 與 /s/player/ script 路徑，其他頁面不允許，connect-src、API session／origin 檢查不變。
- 僅目前 frame／generation 的官方 callback 生效，舊事件不更新新影片；停止／隱藏／離頁／視窗過小都移除播放器並停止動畫。卡片重建時還原目前狀態，動態文字保護且以 textContent 寫入。
- 省電模式下背景仍暫停；播放中的 5 個 CSS transform 音柱作狀態指示。系統 prefers-reduced-motion 時靜態，暫停／結束時移除動畫。
- 9 項單元及 CSP 測試、手機／桌面 quick、320px 全元件 smoke、正常／減少動態 CSS 狀態 fixture 通過。geometry fixture 會移除 iframe src 與 API attach，不請求 YouTube、不產生播放回報。單元 provider callbacks 是契約 mock，不代表真正串流或 live renderer ACK。

未實播使用者指定影片、未人工確認聽感或嵌入權限；禁止嵌入（101／150）、無法識別（153）或瀏覽器阻擋時提供提示與 YouTube 開啟連結。未更新已安裝 App，沒有改動真實設定、升版或 push。
