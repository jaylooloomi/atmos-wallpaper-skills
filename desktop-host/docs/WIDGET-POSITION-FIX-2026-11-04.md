# 元件位置保存修正 0.2.2

日期：2026-11-04。修正天氣等可拖曳元件在「套用變更」後回到初始位置。

## 原因與行為

預覽自動排版是暫時投影，未改寫 JSON 的 desktopZone 與手動位置。拖曳又從原始座標起算；套用後重新載入時，自動排版覆蓋手動座標。

現在明確移動時，從目前顯示位置起算，固定桌面所在區域及自動排版座標，再把位置送入既有草稿／JSON 交易。已存在的手動座標不再被自動排版覆蓋。套用、重新載入及重啟會沿用保存位置。

方向鍵、座標欄位與置中使用同一套處理。桌面操作保留窄屏座標，窄屏操作保留桌面座標；單純顯示／隱藏與縮放仍只投影，不改寫原始幾何。取消拖曳恢復原幾何，沒有位移的點擊不建立草稿。

## 自動驗證

新增六項合成 DOM 回歸測試，執行生成的完整 widgets.js，經真正 DraftState、ControlCore、JSON 設定交易與重新建立 renderer fixture，檢查桌面、獨立溢出區、窄屏、縮放比例、連續 pointermove、取消／零位移、方向鍵、座標欄位及舊版 custom 座標。不是瀏覽器／桌面 UI 自動化，也不當作人工目視證據。

完整 Node 驗證：產品 204 passed／2 skipped，共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 通過。Rust 12 passed，clippy `-D warnings` 與 fmt check 通過。共 257 passed、4 skipped、0 failed。

本次不重跑先前的開機、卸載、睡眠、多螢幕、GPU 或長時間耗用驗證。凍結 prototype 不變，修正在生成流程與獨立來源模組。

## 本機安裝更新

已將既有安裝靜默升級為 0.2.2 並開啟管理頁。升級前後已儲存的 live.json bytes 相同，登入啟動偏好相同；payload 與內嵌執行環境自檢通過，已安裝 widgets.js 與本次來源生成結果逐位元組一致。新的宿主掛載到 WorkerW，真實桌布與管理頁皆收到近期 ack。這些不等同人工拖曳目視確認；本次未取得新的人工確認。見 [升級證據](../evidence/widget-position-upgrade.json)。

安裝檔：Atmos Wallpaper_0.2.2_x64-setup.exe，292,172,099 bytes，SHA-256：`99ca89fa35bab12110b5d80cba7c91fcb6b56d946bb78ff22e10f7a5eff19944`。自有安裝包未簽章。
