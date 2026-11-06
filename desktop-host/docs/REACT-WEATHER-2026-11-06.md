# React 天氣卡片（0.7.31）

## 實作與來源

天氣卡片局部導入 React 19.3.1／React DOM 19.3.1，esbuild 0.28.2 建置單一 production IIFE（約 229 KB 未壓縮）；外層桌面管理、Tauri／Rust 與交易架構不變。`@meteocons/svg`、`@meteocons/svg-static` 各固定 0.1.0，使用 fill 風格各 13 張圖示；建置及資源套件放 devDependencies，安裝 App 不需要自行安裝 React／Node，也不在執行時從 CDN 下載圖示。

- [Meteocons 官方 React 整合](https://github.com/basmilius/meteocons/blob/70dfb1d6e30dc9e791cfb0e4c5b5e5e60e972aa0/packages/docs/src/pages/docs/frameworks.mdx)。
- [Meteocons MIT 授權](https://github.com/basmilius/meteocons/blob/70dfb1d6e30dc9e791cfb0e4c5b5e5e60e972aa0/packages/svg/LICENSE)，Copyright 2020-present Bas Milius；npm 圖示包沒有附 LICENSE 檔，本專案保留上述官方完整 MIT 文字，與 React／React DOM／scheduler 的完整授權一同複製至 dist/licenses 並隨安裝及離線匯出發布。
- [React createRoot 局部整合](https://react.dev/reference/react-dom/client/createRoot#rendering-a-page-partially-built-with-react)。
- [Open-Meteo API／WMO 定義](https://open-meteo.com/en/docs#weather_variable_documentation)。

依經驗證的 `weather_code` 映射晴、局部多雲、陰、霧、毛毛雨、凍雨、雨、雪及雷雨；不是依溫度或使用者所選背景效果判斷。WMO 3 顯示陰天雲層，0 晴天依新增的 current.is_day 顯示太陽或月亮；1／2 也對應日夜圖示。允許官方表列 97 雷雨碼；不合法的日夜值拒絕，舊資料缺日夜欄位用 null／中性圖示，不假造白天。卡片標示模型天氣，沒有把模型當成本機觀測。

畫面有大字溫度、天氣／城市、濕度、模型資料時間、來源授權及小型溫度計。溫度計僅作 −30～50°C 的視覺示意，超出刻度時填滿／清空，但數字仍顯示實際溫度。長描述省略、完整描述留 title；極窄卡片的完整更新時間留資料來源 title。未啟用、讀取中、不可用與過期快取各自顯示，不退回假天氣數字。

## 省電、資料與生命週期

資訊省電、OS 低動態、隱藏頁面及原生暫停，會改載入無 SMIL／CSS 動畫的 svg-static；不是只用 CSS 暫停外部 SVG。溫度計在靜態模式不播放高度過渡，資訊仍沿用既有每五秒讀取、十五分鐘天氣快取及失敗退避。沒有新增資料輪詢／RAF／元件計時器；沒有改城市、線上資料 opt-in、開機啟動或效能設定。

每個真實天氣 body 只建立一個 React root，重用 DOM 更新；移除或重建卡片時在既有 paint 中 sweep／unmount，清掉 Map 內的脫離 root。離線匯出也攜帶 React bundle、必要 SVG 與授權，以靜態不可用狀態呈現，不匯出私人來源設定、不發 API 請求或宣稱有即時天氣。

## 驗證

單元涵蓋所有 WMO 分類、日夜／未知資料、溫度計界限、靜態與過期狀態、文字跳脫、選用 SVG 和鎖版套件原始 bytes 一致、SVG 不含 script／外部 URL，靜態版本不含動畫，完整授權隨包發布。Provider 測試驗證 is_day 請求／驗證與保留 opt-in、快取；停用不再退回 demo。

真實 production React bundle 在兩個獨立 Chromium CLI profile（正常、強制低動態）檢查六種條件／日夜圖示、所有圖片可解碼、省電後仍更新溫度及溫度計、恢復／隱藏靜態、80 次 root 替換後清到零，沒有 API 請求。這些是合成資料測試，不是桌面實機天氣或 renderer ACK。見 [瀏覽器統計](../evidence/weather-react-browser-2026-11-06.json)。

保留原本 12 種 viewport／336 元件風格／三分頁／資訊模板／省電／YouTube 空白 frame 及重疊負控制；新增每 viewport 60 天氣條件／寬度／主題案例，共 720 案例，每案驗證一般圖示、靜態時資料更新、React root 清理與原本 S 卡片高度不裁切。首次更嚴格高度檢查抓到極窄卡片 footer 擠出，已緊湊排列並保留完整時間提示，沒有放寬高度門檻。

最終完整 verify 通過：產品 495 項（492 通過、3 項既有跳過），核心 43 項（41 通過、2 項既有跳過），draft／path／CLI 與完整排版矩陣通過；React 文字 ownership 保護標記補上後，三項天氣卡片測試及兩種 Chromium profile 再通過。正式打包前也重跑同一完整排版門檻，720 天氣案例均無裁切／溢出／root 遺留，負控制保留且有效。

五個隔離離線匯出另啟用天氣卡片，確認 React 及本機 SVG 可載入，離線天氣是靜態 unavailable、沒有虛構數字，也沒有 API 請求；見 [離線統計](../evidence/weather-react-offline-2026-11-06.json)。

已更新目前安裝的 0.7.31，10 份設定逐位元保持、開機啟動與資訊省電偏好不變。實際提供的 weather-react.js 與測過的 bundle 一致；26 張 SVG 位元組與 MIME 正確，13 張靜態檔均無動畫，五份完整第三方授權存在。透過安裝版認證 API 取得真實可用天氣、布林日夜欄位與可識別 WMO 分類，沒有改城市或 opt-in；只輸出檢查布林值，不公開使用者城市或原始天氣回應。新宿主掛載、genuine wallpaper／manager ACK 與獨立 18 秒存活檢查通過。

[升級](../evidence/weather-react-upgrade-2026-11-06.json)、[安裝版資料／資源驗證](../evidence/weather-react-installed-2026-11-06.json)、[宿主存活](../evidence/weather-react-handoff-2026-11-06.json)。本機未簽章安裝檔 293,463,771 bytes，SHA-256 `5ff75f0803929e79ba95ebee34dddb32138a2c59ba713e1d0a7f886546cd416e`；成功更新後清理舊 0.7.30 安裝檔，保留最新版本。SVG 正常模式的時序由 WebView2 處理，並非由 Canvas FPS 限制節流；省電／暫停／低動態則使用無動畫檔。沒有 Computer Use 或使用者瀏覽器／桌面 UI 自動化；安裝檔、私人設定、原始紀錄與測試 profile 不提交。圖示靜態及元件清理測試不是 GPU／耗電基準或兩小時長測。
