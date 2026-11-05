# 系統資訊實測與 TrafficMonitor 調研（0.7.28）

## 原因

CPU／RAM 本來已有原生 Windows 來源，先以已安裝版本 CLI 連續讀取證實數值與時間變動；截圖中的「系統資訊 · 真 host 未接」與固定 N/A 情境則是遺留控制介面，沒有反映真正 provider。上傳／下載原本確實未接，provider 當時只覆寫 CPU／RAM，且忽略 metrics 顯示選項。不是所有數值都是假資料，但舊介面會誤導。

## 參考來源

本輪查看 TrafficMonitor master 的固定提交 `930f17533d6098989ebad62f210aa97d75ef174b`：

- [AdapterCommon.cpp](https://github.com/zhongyang219/TrafficMonitor/blob/930f17533d6098989ebad62f210aa97d75ef174b/TrafficMonitor/AdapterCommon.cpp)：讀取介面流量計數器，對應網卡。
- [TrafficMonitorDlg.cpp](https://github.com/zhongyang219/TrafficMonitor/blob/930f17533d6098989ebad62f210aa97d75ef174b/TrafficMonitor/TrafficMonitorDlg.cpp)：定時更新，對輸入／輸出位元組作差，依實際經過時間計算速率；支援連線選擇及更新。
- [CPUUsage.cpp](https://github.com/zhongyang219/TrafficMonitor/blob/930f17533d6098989ebad62f210aa97d75ef174b/TrafficMonitor/PdhHardwareQuery/CPUUsage.cpp)：使用 PDH，失敗回退到 GetSystemTimes。
- [LICENSE](https://github.com/zhongyang219/TrafficMonitor/blob/930f17533d6098989ebad62f210aa97d75ef174b/LICENSE)：Anti 996。只研究資料取得方法，沒有複製或打包其程式碼、DLL、皮膚或驅動。

## 修正與界線

Atmos 以自己的 Rust 宿主與 Windows SDK 實作。CPU 保留 GetSystemTimes 差值，RAM 來自 GlobalMemoryStatusEx。新增 [GetIfTable2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/nf-netioapi-getiftable2) 的 [MIB_IF_ROW2](https://learn.microsoft.com/en-us/windows/win32/api/netioapi/ns-netioapi-mib_if_row2) 64 位元 InOctets／OutOctets，使用單調時鐘計算 B/s、KiB/s、MiB/s。SDK binding 保留結構對齊，每次使用 FreeMibTable 釋放。

優先加總運作中的實體介面，排除 loopback／tunnel／filter；無實體介面時使用運作中的非 loopback 介面並以 scope 明示。以 LUID 比對兩次採樣集合；變動、重設、溢位或無可用介面時回傳 unavailable，不偽造零值或尖峰。此為介面傳輸速率，不是 ISP 測速；VPN／虛擬網卡與加總範圍可能與 TrafficMonitor 手動選擇不同。這版沒有網卡手動選擇或歷史流量統計。

CPU 仍有 primary processor group 界線（最多 64 顆邏輯處理器的系統包含全部）；沒有宣稱數值與 PDH Processor Utility 或工作管理員完全一致。GPU、溫度、耗電仍未提供，不載入硬體感測器驅動，也不要求管理員權限。

前端改為真實即時面板，移走舊 systemScenario 到隱藏相容性綁定；保留原 checkbox 與儲存事件。指標選項實際控制桌面卡片項目，面板顯示即時數值與更新時間。不再呈現虛構 CPU／RAM 樣本、舊情境選單及過時 host 提示。資料不可用顯示 N/A，不拿 demo 補值。

CPU／RAM／網路讀取獨立於天氣，每兩秒更新、服務共用兩秒快取與單一 pending 工作；天氣請求即使延遲，也不阻塞系統資訊。未顯示卡片且未選取系統設定時不採樣，頁面隱藏不額外抓取；只保留最新樣本，沒有無限歷史陣列。設定、桌布、啟動與省電偏好不自動修改。

## 驗證

Rust 三項 telemetry 單元測試通過，涵蓋 CPU 差值、64 位元計數、反向／重設／介面集合變更。debug 實機連續四筆 CPU 約 8.71～11.55%，網路下載約 3,012～30,140 B/s、上傳約 5,717～27,017 B/s，時間持續前進，使用一張實體介面。此為包裝前診斷，最後安裝版結果另列，沒有把模擬數值當實機。

Node 測試涵蓋兩秒快取、網路資料驗證、移除舊介面、保留原綁定、metrics 選項、零設定寫入及慢天氣不阻塞系統更新。排版矩陣新增即時面板存在及數值不溢出檢查；原有全部覆蓋與負控制保留。

完整 verify 重跑通過：產品 489 項（486 通過、3 項既有跳過），核心 43 項（41 通過、2 項既有跳過），draft／path／CLI 及 12 種視窗尺寸完整 Chromium 排版矩陣通過；Rust 全部 52 項通過。矩陣保留 28 種元件／風格狀態、三分頁、三種資訊版型及省電／YouTube 空白 frame 幾何檢查，刻意重疊的負控制確實被拒絕。

第一次完整測試曾有既有素材封存 worker 測試失敗（預期 completed、實際 failed）；未變更測試或門檻，該檔獨立重跑 14 項及第二次完整 verify 皆通過。未確定首次失敗原因，原始失敗紀錄留本機，沒有當作首次全綠或保證不再發生。

0.7.28 安裝更新通過，10 份原設定逐位元保留，開機啟動與資訊省電偏好保持；實際提供的 JS／CSS 與測過的打包內容一致。新宿主掛載與 genuine wallpaper／manager ACK 通過，另以獨立命令確認宿主存活 18 秒。這些不是人眼可見性證明。

安裝版認證 API 連續 6 筆真實採樣，6 個不同時間，跨 14.448 秒：CPU 7.68～17.75%，RAM 使用 65.76～66.23 GiB，下載 1147～60983 B/s，上傳 700～81393 B/s，scope 為 active-physical-interfaces。確認實機資料會更新；這是短期資料鏈路驗證，並非兩小時長測、效能 benchmark 或全部硬體相容性證明。

公開統計：[升級](../evidence/live-system-upgrade-2026-11-06.json)、[存活與 ACK](../evidence/live-system-handoff-2026-11-06.json)、[即時資料](../evidence/live-system-samples-2026-11-06.json)。本機未簽章安裝檔 293,366,781 bytes，SHA-256 `3d42b2a6643ebcae94cfca76205ba1a2ed17825b59c6caaaab448b1f771c3e53`；升級成功後清除舊 0.7.27 安裝檔，保留最新版本。無 Computer Use 或使用者瀏覽器／桌面 UI 自動化；原始紀錄、私人設定及安裝檔不提交。
