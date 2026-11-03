# 管理頁浮動套用工具列 0.2.3

日期：2026-11-04。

管理頁的「本機核心控制／套用變更／取消草稿」工具列使用 CSS sticky。原位保留文件流，向下捲動到頂端後吸附於安全區頂端，並顯示於預覽與元件之上。保留原本不透明背景，增加陰影；窄屏文字與按鈕換行，極小視窗時工具列內容可垂直捲動。桌布頁仍隱藏控制列，不更動交易、草稿或位置設定。

## 版面與回歸

透過隔離 Chromium 命令列合成 fixture，檢查英文 320px natural、390px neon、1280px natural；不連接或自動操作使用者的瀏覽器／桌面。三組皆捲動 600px 後工具列 top=0，套用按鈕的 hit test 仍落在工具列，工具列與頁面沒有水平溢出。窄屏工具列高 116.5px，桌面高 46.5px。場景卡片及原有預覽工具列檢查也通過。

證據：[sticky-toolbar-layout/report.json](../../implementation-p4_2_5/evidence/sticky-toolbar-layout/report.json)。這是三組針對性檢查，不當成重新通過所有歷史版面案例或人工目視確認。

完整 Node 檢查：產品 204 passed／2 skipped，共用核心 41 passed／2 skipped；prepare、獨立 draft／path review、CLI help 均通過。Rust 12 passed。沒有新增平台依賴，也不重跑開機、睡眠、卸載或長時間耗用測試。

## 安裝更新

本機已靜默升級到 0.2.3，已儲存設定與登入啟動偏好保留。payload／內嵌執行環境自檢通過，安裝後的樣式與來源生成結果相同；原生 WorkerW 掛載、真實桌布及管理頁近期 ack 皆正常。未取得新的人工目視確認。見 [升級證據](../evidence/sticky-toolbar-upgrade.json)。

安裝檔 Atmos Wallpaper_0.2.3_x64-setup.exe：292,171,918 bytes；SHA-256：`40761306192689fc532e1eb460cdc0f474bd33078ebae6499aa59c1be1c5c67e`。自有安裝包未簽章。
