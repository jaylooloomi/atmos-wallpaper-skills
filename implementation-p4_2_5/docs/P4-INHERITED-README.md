# Atmos P4 — 桌布顯示入口與 Windows 相容準備

## 範圍
`/manager`（`/` alias）保P3正式控制面板；`/wallpaper`只顯示真場景、clock/calendar與選中widgets，不含管理panel/AI/prompt/主題卡/交易列/證據JSON。兩頁共用同一core/snapshot，不另造權威狀態。主題只換材質，不重排已存值。顯示頁的邊界clamp與tray是只讀投影，不寫core。

當桌面空間足夠時場景填滿viewport；已有tray配置時沿用獨立槽，不強塞五卡互疊；390維持場景＋可切換元件槽，非硬塞成單屏假截圖。連線正常時顯示頁隱藏小狀態；失聯仍保上一已驗圖並顯示小錯誤提示。沒有OS安裝／桌布已設成功的宣稱。

## 本輪安全界線
server為manager與wallpaper產生不同記憶體nonce，wallpaper token只可讀snapshot/art/health與送ack，POST commands server回403 ROLE_DENIED；一般API不回token，wallpaper HTML不含manager token。非navigation的瀏覽器fetch bootstrap被拒，無fetchmetadata探針只回無token文字。

**這不是對抗惡意同源JavaScript的安全sandbox。** 同origin頁面本來共享來源信任；scoped token用於防誤寫/能力檢查，不承諾同源惡意code隔離。兩頁只載受控本地程式、CSP無外部JS、不注入匯入HTML、素材必須驗hash/raster。若要對抗不可信同源code需另個origin/通道設計，本輪未宣稱完成。

`/api/health.renderers`列出最多16個活躍renderer的server-derived role/revision/hash/新鮮度，方便核對兩頁；hostVisibility仍unknown，不能把browser ack當桌面host驗證。

## 重建／測試
現有Node >=22、Python/Pillow用於圖片stage。無npm install。

```sh
npm run prepare
npm test
```

Cloud只能root用官方Sites managed daemon；同P3 `scripts.dev --host 127.0.0.1 --port PORT --strictPort`。`/review-wallpaper.html`提供真1672/390 iframe，不改state。

## Windows入口：已備碼，尚待明確啟動授權與實機驗證
桌機只讀盤點（root提供）：Win10Pro22H2 build19045.5737、Node24.13.0、`C:\Python314\python.exe`3.14.2／Pillow12.1.0。Lively Store套件1.0.164.0／EXE2.2.1.5；兩1920×1080，Windows DISPLAY2為主。**Lively monitor編號未必等於DISPLAY2，必須UI對應後才可指定。**

`Start-Atmos.cmd`只在獲准後執行：
- 使用既有Node及上述已盤點Python，不下載／安裝任何依賴。
- 必要時從凍結baseline重建dist（assets-only，不初始化cloud測試store）。
- 本產品store=`%LOCALAPPDATA%\AtmosWallpaper\data`。
- Windows-only runtime只綁127.0.0.1:4173，埠占用就錯、不另找埠、不曝LAN。
- 前景服務；Ctrl+C只關本產品server。沒有服務註冊、自動開機、改防火牆、自動啟Lively或改桌布。
- 管理URL `/manager`、顯示URL `/wallpaper`。啟動本地服務與修改指定螢幕桌布是兩個不同動作，需分別有授權。

不應在cloud執行此Windows入口；它會明確拒絕。不得偽造managed環境變數繞過原P3 guard。欲使用其他Python路徑先唯讀確認，再改本產品cmd的顯式設定；不拼接shell下載指令。

## Portable core差異（不改凍結P1/P3）
`src/portable-core.mjs`是P1交易core的compat副本，僅替換Store import／decoder路徑和顯式Python可配置；狀態schema、preview/apply、receipt、CAS與recovery不改。`bin/atmos.mjs`沿相同命令。圖片decoder仍同P1 Pillow程式。

`portable-store.mjs`保檔案fsync→rename，POSIX再目錄fsync；Windows分支**不提供目錄flush**，狀態明示durability字串，不能宣稱與Linux相同掉電durability。rename失敗不先刪目標、不靜默降成非原子覆寫。Windows實際rename/占用/防毒/故障需實機驗；目前只有受控io mock與cloud測試。

相對路徑拒ADS冒號、Windows裝置保留名、尾點/空格、反斜線、越界/絕對路徑；Windows store要求明確本機drive，不收UNC。既有symlink檢查保留；Windowsjunction/實際權限仍待實機驗，不拿Linuxsymlink測冒充。

## 未完成／不得誇大
Windows本機啟動、Lively掛載、指定螢幕、鎖屏/睡眠/全螢幕、圖示穿透、IME、多顯示器/DPI與耗電尚待授權後實機驗證。Calendar/weather/system/NowPlaying仍demo/N/A；P4顯示模式不使這些服務變真。

工程測試目前65項：42個凍結P1 core測試**實際對P4 compatibility core/CLI在POSIX重跑**，14個draft/commands測試，4個role/bootstrap測試，5個portable路徑/fsync/來源差異測試。Windows branches是受控mock/validator，不是Windows實測。真雙頁/桌布模式驗收另列qa-p4，不能混成65個真browser測試。
