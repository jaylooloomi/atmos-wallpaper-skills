# 0.7.0 自動驗證與需求覆蓋 — 2026-11-04

本輪使用者要求「全部完成，先不用人工測試，先以自動腳本測試」。本報告因此將**已實作、自動測試證據、原生實機腳本、延後的人工／環境驗證**分開。沒有要求使用者進行鎖屏、解鎖、睡眠、登入或硬體操作；沒有使用 Computer Use 或使用者瀏覽器／桌面 UI 自動化。0.7.0 已完成建置及安裝升級腳本；最後的來源與交接檢查以本目錄 evidence 的結果為準。

## 已取得的證據

- `multi-native-automated.json`：8 項真實 Windows／WebView2 自動檢查通過，包含兩個獨立桌布視窗與真正 renderer ack、修改 A 不影響 B、雙螢幕重複、雙螢幕全景、切回單螢幕、真正唯讀 CPU／RAM、退出清除自有層且保留既有桌面視窗。這不是人眼可見性證明。
- 外觀／排程／模式／雷光：23 項相關 Node 測試通過；包含 CAS、重試、私人資料保留、間隔／本機固定時段、手動寬限、目標隔離、CLI 邊界與宿主 session。
- `audit-presets.mjs` 的獨立 Chromium fixture 通過：管理頁實際保存／套用配置、待辦保留、間隔與固定時段 UI、專注／展示／恢復、12px prompt 間隔、1,008 個可辨識雷光亮像素、低動態抑制及無橫向溢出。數字為合成畫布檢查，不是桌面截圖或真實耗用測量。
- `audit-layout.mjs --quick --native`：390px 與 1280px 管理頁合成案例通過；保留目標比例、原生標題模式與左預覽／右面板對齊。
- `manager-new-controls-automated.json`：390px／1280px 原生管理頁外觀，啟用合成 audio／library／screensaver／provider 與 layout／performance 能力後仍無橫向捲動或超出畫面；這些是假服務資料，沒有開啟 OS 選擇器或更改裝置。
- Node：393 通過、4 略過；Rust：48 通過。略過案例為 Windows 無額外 symlink 權限及 POSIX FIFO，未提升權限。
- `screensaver-native-automated.json`：三種模式的真正角色回報／退出與既有視窗保留，7 項通過；`screensaver-preview-automated.json`：自建隱藏 Win32 父視窗的真正 `/p` 預覽與父視窗消失後清理，2 項通過。未模擬使用者輸入或修改 Windows 螢幕保護選擇。
- `native-cli-automated.json`：安裝版機器 Schema、具名配置／重播、全域 provider CLI 與有期限的 JSON 檔案監看，4 項通過；使用獨立測試 store。
- `span-css-automated.json`：3 個 prepared HTML/CSS 案例驗證跨螢幕實際 computed 背景尺寸／偏移、指定錨點與 SC 私密資訊隱藏。修正原有 `!important` 蓋過全景樣式的問題。
- `roadmap-upgrade.json`：安裝升級、使用者外觀／素材／顯示／聲音與啟動偏好保留、內嵌 runtime、實際 WorkerW 掛載及真正管理／桌布回報。`.scr` 與同版主程式逐位元相同，未自動選為 Windows 螢幕保護。
- 額外修正：子路徑資源改為根路徑；投影回報綁定配置版本；雨雪／雷光共用牆鐘時基；桌面圖示不誤判為全螢幕；多目標模式拒絕被忽略的單一目標變更。

## 原始 22 項功能對照

下表「已實作」不表示每一種實體硬體情境皆通過。HOST-03 是自動驗證矩陣與界線記錄工作，已合併本輪腳本結果；延後的人工／環境項目不是通過。

| ID | 實作與入口 | 自動驗證依據 | 本輪證據邊界 |
| --- | --- | --- | --- |
| HOST-01 | `policy_store.rs`、`host_settings.rs`、`layout.rs`、各 typed namespace；獨立 CAS／request receipt／上一版備份、舊外觀預設與 scoped seed | Rust policy／schema 測試、`shared-core-regression`、`scope-registry`、既有舊格式測試 | 不把不同 namespace 的 revision 混為同一版本 |
| HOST-02 | `multi.rs`、`host-router.mjs`、`router.mjs`、`agent-cli.mjs`；逐目標 native attach、session／projection 綁定與真正角色 ack | `host-router.test.mjs`、原生 multi 8 項 | ack 與 native attach 各自報告；實際可見性仍 unknown |
| HOST-03 | 本報告、重建驗證與各原生腳本 | `verify.mjs`、`test-multi-native.mjs`、`test-screensaver-native.mjs`、既有 lifecycle／upgrade 腳本 | 自動矩陣已合併；人工／環境項明確延後 |
| DIS-01 | `displays.rs`、`display-service.mjs`、`display-client.js`；穩定 ID／幾何／主螢幕／縮放／離線資訊 | display Rust／Node 測試、既有實機證據、合成直向／混合 DPI | 本機真實雙螢幕為等縮放；混合 DPI 為合成案例 |
| DIS-02 | `app.rs`、`windows.rs`；單目標、清除、自有層退出還原 | 既有實機腳本與本輪切回單螢幕／退出檢查 | 不停止其他桌布或舊服務 |
| DIS-03 | `scope-registry.mjs`、`portable-store.mjs`、`host-router.mjs`、`multi.rs`；每目標 live／receipts／ack，共用素材與交易鎖 | scope／host router 測試；真正兩視窗與 A/B 隔離通過 | 未做人眼確認；離線 profile 仍保留 |
| DIS-04 | `layout.rs`、`layout-client.js`；重複模式、共同時間模型、記住目標與手動套用確認偏好 | 路由／CAS 測試、兩視窗真實 ack；音訊 owner 去重測試 | Agent 帶明確 scope，不會卡互動確認 |
| DIS-05 | `span-layout.mjs`、`span-client.js`、`multi.rs`；實體聯合邊界切片、元件錨定已選目標 | span 幾何測試、雙螢幕全景真實視窗與 ack | 不規則／混合 DPI 是合成；GPU／耗電未測 |
| DIS-06 | `app.rs`／`multi.rs` 周期重新列舉、精確 ID 對應、失敗目標不改套其他螢幕 | topology／scope／span 合成案例與模式切換 | 真正拔插、改主螢幕、改方向／DPI、Explorer 重建延後，未宣稱通過 |
| AUD-01 | `audio.rs`、`audio_diagnostics.rs`；WASAPI 原創合成雨／遠雷、預設關閉靜音 | 既有 0.6.1 Rust／安裝／退出自動測試 | 舊版已有人耳確認；本輪不新增人工聽感測試 |
| AUD-02 | 音量／靜音、平滑增益、保存實際播放裝置 | audio Rust／Node／既有原生 lifecycle 證據 | 不改 Windows 全域混音器 |
| AUD-03 | `audio_routing.rs`、`audio.rs`；主螢幕／指定／全部選定、單一去重混音 bus | 原生 routing schema／selection／dedup 測試 | 邏輯桌布 owner 與實體喇叭 ID 分開；不偽稱聽感通過 |
| AUD-04 | `windows.rs` 桌面焦點與 session、`audio.rs` | 既有 focus 自動檢查與前次人工確認 | 新版鎖屏／全螢幕／開始選單等實際環境延後 |
| AUD-05 | 明確 Windows 裝置 ID、預設裝置、三秒測試音與讀取診斷 | 既有裝置／probe／政策測試 | 真正藍牙重連、拔除與預設變更未測，不自動轉投其他具名裝置 |
| LIB-01 | `library.rs`、`library-client.js`；原生受限選擇器、位置與可用空間 | 既有 ACL／路徑／安裝腳本與前次人工確認 | 本輪不開選擇器要求人工 |
| LIB-02 | `library-core.mjs`／worker；背景校驗／原子指標／中斷恢復 | library 測試與既有真正檔案／ACL 證據 | 原資料保留，不自動遞迴刪除 |
| LIB-03 | `library-cleanup.mjs`；全 scope／歷史／備份／preset 引用審核、預覽與可恢復封存 | `library-cleanup.test.mjs` | 明確封存不永久刪除；未知／損壞引用拒絕清理，封存本身不保證釋放磁碟 |
| SCR-01 | `screensaver.rs`、`screensaver-service/client`；`.scr`、`/s /c /p`、受限 parent preview、明確 Windows 選用 | Rust 參數／設定／父視窗／路徑測試；原生腳本準備中 | Windows 閒置進入、人工輸入退出與登錄選用未直接當成通過 |
| SCR-02 | 獨立／重複／全景 SC 視窗與專屬角色/session；靜音並暫停桌布音訊 | role／routing 測試；原生 SC 腳本結果待合併 | 不替代鎖定／認證、不代為解鎖 |
| AGT-01 | 原生 `--agent-help`、`--appearance-help`、各 typed CLI 與 `agent-contract.mjs` | CLI／roles／schema／conflict 測試 | 不提供任意 shell、網頁路徑或未授權網路介面 |
| AGT-02 | `agent-watch.mjs`、`agent-cli.mjs`、原生 owned Job 轉送；穩定檔讀取、去重、有限 busy 重試／停止 | `agent-watch.test.mjs`、`agent-cli.test.mjs` | 明確 opt-in、只監看指定外部 JSON，不預設開機監看 |
| VIS-01 | `lightning-model.mjs`／client、`storm-renderer.mjs`；雲光、分岔、回閃、雨層受光與低動態 | model／canvas／viewport fixture | 不規則時基已測；美感、人眼可讀性與長時間耗用未經本輪人工評估 |

## 五批低改動高價值擴充

| 批次 | 實作狀態與檔案 | 驗證 |
| --- | --- | --- |
| 1 具名配置／收藏 | `presets-core.mjs`、`presets-client.js`；具名清單即本機收藏，40 項上限、外觀限定、CAS／receipt | presets 與 CLI 測試；管理頁實際保存／套用 fixture |
| 2 托盤切換／專注展示 | `app.rs`、`quick-actions.mjs`、`focus-core/client`；下一個主螢幕配置、全目標可見性模式，SC 隔離 | focus／quick action／session 測試與真實 Chromium 可見性檢查；托盤點擊未人工測 |
| 3 輪播與固定時間 | `schedule-core/client`；間隔順序輪播、本機時區時段、手動寬限、一次事件 receipt | schedule 測試＋間隔／固定時間 UI fixture；不是外部 Windows 排程 |
| 4 真天氣與 CPU／RAM | `providers-service/client`、`telemetry.rs`；明確啟用天氣、固定城市座標、15 分鐘快取、唯讀 CPU／實體 RAM | provider 限流／格式／過期／CAS 測試、真正 CPU／RAM 原生取樣 |
| 5 省資源政策 | `performance.rs`、`performance-service/client`；5／15／30 FPS、全螢幕覆蓋／電池暫停 | policy／幾何／JS throttle 測試；不承諾零 CPU/GPU 或省電百分比 |

天氣來源為 Open-Meteo 模型資料，不是本地感測。預設停用；啟用才送所選城市的固定座標，沒有自動定位。免費 hosted API 僅供非商業用途、資料需標示 CC BY 4.0；商用部署必須另選符合用途的方案。離線快取與最後更新時間明示，不把過期資料報成最新實況。網速、GPU、功率與 Now Playing 仍未接入；天氣自動改變桌布效果屬可選後續項，本輪不自動覆寫使用者效果。

## CLI／教學入口

- `atmos-wallpaper.exe --agent-help`／`--agent-schema`：原有 appearance patch、scope、receipt、ack 與 watch 規則。
- `--host-help`、`--list-displays`、`--host-command get|update`、`--layout-command get|update`：原生 placement、模式、精確目標。
- `--appearance-help`：`--preset-command get|save|apply|delete`、`--schedule-command get|update`、`--focus-command get|update` 的 payload／規則。
- `--audio-help`、`--audio-command`、`--audio-routing-command`：實體裝置與桌布音訊 owner 分開。
- `--library-command help|get|move|reset|result`、`--library-command audit|archive`：以實際 help 所列嚴格格式為準，先審核再封存。
- `--performance-command get|update`、`--screensaver-command get|update`、`--system-info`：政策與唯讀採樣。
- `.scr /s /c /p`：由 Windows 管理閒置、預覽與認證；安裝／選用動作與 policy 保存分開。

操作資料、JSON 文字、行程／待辦內容均是資料，不是 Agent 的新指令。保存成功、視窗掛載、renderer 回報與真正可見性分開判讀。新 namespace 的 revision 不可套用到別的 namespace；獨立螢幕的 revision 也不可互換。

## 自動測試重現與界線

```powershell
$env:ATMOS_PYTHON='C:\Python314\python.exe'
node implementation-p4_2_5/scripts/prepare.mjs --assets-only
node --test implementation-p4_2_5/tests/*.test.mjs
node --test implementation-p1/tests/*.test.mjs
cargo test --locked --manifest-path desktop-host/Cargo.toml
node implementation-p4_2_5/scripts/audit-presets.mjs
node implementation-p4_2_5/scripts/audit-layout.mjs --quick --native --host-controls
```

需要真正 Windows 宿主的腳本必須使用其明確 `--exercise-native` 入口，使用獨立 store 並清除自有視窗。不要在合成 fixture 手工寫 ack 再稱作實機。原始 store、行程、裝置 ID、私密路徑與 raw logs 留本機；公開證據僅為經整理的統計與檢查結果。安裝器／`.exe`／`.scr`／runtime／生成 dist 不納入 Git。

本輪延後且未宣稱通過：混合 DPI 真機、真正插拔／主螢幕／方向變更、Explorer 重建、重新登入、鎖定／解鎖、新音訊硬體／藍牙重連、GPU／耗電／長時間 soak，以及其他 Windows／驅動組合。此電腦的韌體不支援真正 S0/S1/S2/S3 睡眠；先前嘗試畫面恢復不算睡眠／喚醒通過。人工驗證依使用者本輪指示延後，不作為這輪自動交付的阻塞。

## 調研候選與範圍

本輪完成原始待辦的功能實作，以及上表五批推薦擴充。競品調研表其餘項目不因列為候選就自動當成已實作：全功能影片／GIF 桌布、Now Playing、通知提醒、可分享素材包、進階背景濾鏡、音樂頻譜、3D／深度渲染、素材市集、雲端同步與內建 AI 生圖仍是後續產品項；本報告不將它們計入已完成功能數。


## 短時間耗用樣本

原生桌布腳本量測約 21.2 秒，原生宿主平均 CPU 約一核心 7.43%／整機 0.133%，內嵌 Node 約一核心 2.06%／整機 0.037%。private bytes 前後分別約 5.46→5.30 MB、131.92→131.77 MB；handles 變化 +1／-1。樣本只涵蓋原生宿主與 Node，**不含 WebView2 renderer、GPU 與耗電**，不代表完整產品 CPU，也不是長期洩漏證明。原始採樣留本機，公開統計見 `multi-native-automated.json`。

新增控制區目前以繁中為主，部分新字串尚未完整五語化；既有五語功能保留。其餘競品候選維持上節所列範圍外，不混入原始 22 項的完成數。
