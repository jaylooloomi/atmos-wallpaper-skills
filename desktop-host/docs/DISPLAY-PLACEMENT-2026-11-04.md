# 單目標螢幕與宿主設定 — Windows 0.4.0

## 本批提供

- 原生 `EnumDisplayMonitors` / `GetMonitorInfoW` / `QueryDisplayConfig` / `DisplayConfigGetDeviceInfo` 讀取顯示器。明確使用每螢幕 DPI context 取得實體邊界，讀取縮放與旋轉；同型號螢幕按裝置連線路徑的 SHA-256 區分，不保存 GDI 編號為身份。
- 管理頁新增相對位置配置圖、跟隨主螢幕／選定單一螢幕、立即套用、清除與重新偵測。既有左預覽／右控制面板不變；支援五種介面語言與窄螢幕。
- 同一個自有桌布視窗在目標間移動。清除會隱藏 Atmos 自有層、保留目標；退出會移除自有視窗。未修改 Windows 原桌布設定、其他播放器或其他螢幕視窗。
- 目標離線、重複身份、無法辨識或拓樸讀取失敗時不猜測替代螢幕。原設定保留；同一個可靠 ID 回來後重新掛載。連接埠改變可能需要本人重新選擇。硬體鏡像共用 GDI source 時，不提供不可靠的實體目標選擇。
- 預覽的宿主目標資料會更新；拓樸資料納入 snapshot ETag，不必改畫面 revision。管理頁草稿不因螢幕資訊更新而被重設。

## 宿主設定與 Agent

宿主設定保存在 store 的 `host-settings.json`，與 appearance `live.json`、歷史、receipt 分開。沒有宿主文件的舊 store 使用啟用／跟隨主螢幕預設，不改寫舊畫面資料。更新採獨立 revision/CAS、嚴格欄位與值驗證、每 store 核心 mutex、檔案 flush + Windows 原子替換；保留上一份設定。未授權任意命令、URL、HTML 或網頁指定檔案路徑。

本批的 schema v1 僅承載 placement；素材庫與音訊政策仍在後續批次，不提供虛假的設定欄位。新版本或未知欄位明確拒絕，不進行隱性降版。診斷模式的宿主設定固定使用該獨立 store，不能意外寫入正式安裝設定。

原生命令：

```text
atmos-wallpaper.exe --host-help
atmos-wallpaper.exe --list-displays
atmos-wallpaper.exe --host-command get
atmos-wallpaper.exe --host-command update
```

`update` 從 stdin 讀取最多 16384 bytes 的 JSON，例如：

```json
{"schemaVersion":1,"expectedRevision":0,"placement":{"enabled":true,"target":"primary"}}
```

先讀 `get` 取得最新宿主 revision，明確選擇 `primary` 或 `--list-displays` 回報的 selectable ID。衝突不自動重試；結果不確定時重讀並協調。沒有本批宿主 receipt；不得宣稱具有外觀交易 API 的冪等 requestId 行為。停用使用 enabled=false，保留既有 target。診斷可加 `--store ABSOLUTE_LOCAL_DIRECTORY`，不得直接改內部 JSON。

退出碼 0 表示操作完成；2 為錯誤，解析 JSON `error.code`。保存設定、目前宿主是否掛載、renderer ack 與人眼可見性是四種不同證據。`runtimeFresh` 為五秒內狀態，檢查 `settingsRevision` 與目標是否匹配；過期狀態不證明成功。原 Agent appearance v1 入口與 schema 保持相容，`--agent-help` 指向新宿主教學。

HTTP `/api/host/displays` 使用既有同源、管理角色與 session 保護。原生可執行檔與診斷 store 路徑由宿主注入，不來自網頁請求。Wallpaper 角色不能讀寫這個管理入口。

## 驗證與界線

- 277 項 Node 測試通過，4 項略過；19 項 Rust 測試通過。
- 獨立 Chromium CLI 控制區 fixture：桌面、320px 中文、320px 英文、離線目標及設定已保存但宿主尚未套用的 pending 狀態均通過。包含負座標、直向與混合 DPI 的合成配置圖；這些合成案例不能充當實體混合 DPI／直向通過。
- 真正安裝版雙螢幕、指定目標／清除／還原、設定衝突、強制終止／正常退出與重啟、真實 renderer ack 的結果，以 `desktop-host/evidence/display-host-validation.json` 為準。
- 安裝完整性、設定保留與啟動偏好結果，以 `desktop-host/evidence/display-upgrade.json` 為準。安裝包僅留本機，不提交。
- 不宣稱已完成多螢幕同時獨立配置、重複／跨螢幕、音訊、素材庫或螢幕保護程式。
- 實體熱插拔、改主螢幕、混合 DPI／方向切換、Explorer 重建、新一輪人工畫面確認、重新登入、GPU／耗電及長時間測量仍需另外驗證。本機韌體不支援真正睡眠，不能以模擬替代。

原生 API 依據：[QueryDisplayConfig](https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-querydisplayconfig)、[DISPLAYCONFIG_TARGET_DEVICE_NAME](https://learn.microsoft.com/en-us/windows/win32/api/wingdi/ns-wingdi-displayconfig_target_device_name)。裝置路徑代表本次連線的可辨識身份，不保證換連接埠後仍相同。

## 測試交接回歸

第一輪整合測試在執行期間通過，但測試工具退出時，直接啟動的最後一個 GUI 程序被工具的程序生命週期清理。使用者回報看到 Windows 原桌布後，原生檢查確認 runtime 已過期、Atmos 視窗消失。這次回報不能記為目視通過。

測試重新啟動改由 Windows Shell 啟動一般應用程式，避免把交付後的 GUI 留在測試 runner 的程序 Job 中；再於 runner 已退出後，以另一個獨立命令連續觀察 18 秒，確認同一宿主、實體掛載與 renderer 回報持續存在。結果見 `desktop-host/evidence/display-handoff.json`。這是短時間程序存活回歸，並非長時間耗用測試。原生 runtime 子程序仍保留自身 kill-on-close Job，並未放寬核心清理。

管理頁也會比對保存設定的 revision 與宿主目前套用的 settingsRevision；尚未匹配時顯示等待宿主，不把上一目標的成功掛載算成新設定已生效。

人工確認：使用者先回報 Windows 原桌布，修正測試交接方式後，於本聊天明確回覆「已顯示 Atmos，時鐘與圖示正常」。此確認涵蓋目前主螢幕畫面；不擴張成實體熱插拔、其他 OS 或長期穩定性證據。

最終本機安裝包：292451497 bytes，SHA-256 `efcb498edca0931eab945fba05028bd84c62794123b781c4ff1190951b2078a7`；未簽章、僅留本機。
