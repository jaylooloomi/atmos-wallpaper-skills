# 五種粒子特效（0.7.30）

## 開源調研與取捨

查閱以下固定版本的 MIT 開源程式及 LICENSE：

- [tsParticles presets](https://github.com/tsparticles/presets/tree/ae866a538fa40b4b06f8a2b8d4b168d134bf6ba7)：[Firefly](https://github.com/tsparticles/presets/blob/ae866a538fa40b4b06f8a2b8d4b168d134bf6ba7/presets/firefly/src/options.ts) 的透明度／尺寸變化、[Fireworks](https://github.com/tsparticles/presets/blob/ae866a538fa40b4b06f8a2b8d4b168d134bf6ba7/presets/fireworks/src/options.ts) 的發射／分裂／重力／拖尾、[Stars](https://github.com/tsparticles/presets/blob/ae866a538fa40b4b06f8a2b8d4b168d134bf6ba7/presets/stars/src/options.ts) 的緩慢漂移／閃爍。[MIT 授權](https://github.com/tsparticles/presets/blob/ae866a538fa40b4b06f8a2b8d4b168d134bf6ba7/LICENSE)。
- [Sakura](https://github.com/jhammann/sakura/blob/b78b3f4cbbe2fcf218b6926fea8d2742ce6d54d6/src/sakura.js)：花瓣大小、風向、旋轉及生命週期；原實作以 DOM／CSS／排程管理花瓣。[MIT 授權](https://github.com/jhammann/sakura/blob/b78b3f4cbbe2fcf218b6926fea8d2742ce6d54d6/LICENSE)。

參考視覺及運動原理，在 Atmos 既有 Canvas 中獨立實作；沒有複製或打包上述引擎、程式碼、選項檔或圖片，沒有新增 CDN、npm runtime、WebGL 引擎或網路請求。這是參考開源效果後的整合實作，不宣稱裝入 tsParticles／Sakura 原套件。

## 功能

| 選項 | JSON visual.effect | 視覺 |
| --- | --- | --- |
| 流星雨 | meteor | 藍青色高速斜向拖尾 |
| 螢火蟲 | firefly | 黃綠光點、漂移與呼吸閃爍 |
| 櫻花雨 | sakura | 粉色花瓣、風向飄落與翻轉 |
| 煙火秀 | fireworks | 火箭升空、多色徑向爆炸、重力與消退 |
| 星際漩渦 | galaxy | 藍紫星點組成旋轉的螺旋星系 |

與原本晴天、下雨、下雪、颱風、打雷同一選單，使用原強度與套用交易。新增 SVG 圖示及五種語言標籤；目前格式 4、CLI 與 agent schema 接受新 effect，格式 2／3 歷史驗證維持原貌。不改現有場景、位置、音訊與效能偏好，新粒子效果沒有新增聲音。

只使用既有主動畫迴圈、Canvas 與 FPS 限制，沒有新增 RAF／粒子計時器／事件歷史陣列；以時間直接計算位置，每幀最多 400 個粒子描述，依畫面大小與強度調整。資訊省電模式、原生暫停、隱藏頁面與 OS 低動態偏好會停止粒子；強度零不繪製。資訊卡片更新不受影響。較華麗仍可能增加 CPU／GPU 負荷，粒子上限不是耗電或零洩漏證明。

## 驗證

單元測試涵蓋五種效果的嚴格設定驗證、低／高解析度、以兩小時及極長時間作為時間輸入時的有限且有界幾何（不是實際長測）、動畫位移、強度零、異常輸入、Canvas state 還原，以及隱藏／省電／低動態／暫停不繪製。另在獨立臨時 store 實際透過 agent JSON 套用五次，確認 effect 保存與 revision 從 0 前進至 5，沒有改使用者設定，沒有虛造 renderer ACK。

隔離 Chromium Canvas 預覽已檢查五種實際繪製形狀；不是使用者 App 截圖或效能實測。完整排版矩陣新增五個按鈕、SVG、溢出與 Canvas draw smoke 檢查，保留原有所有矩陣覆蓋及負控制。

完整 verify 通過：產品 491 項（488 通過、3 項既有跳過）、核心 43 項（41 通過、2 項既有跳過），draft／path／CLI 及 12 種尺寸完整矩陣通過。離線載入遮罩修正後，三項粒子／匯出單元再次通過。

檢查發現離線匯出未攜帶特效腳本及純月曆，且保留了不能自行結束的 Boot Loading；本輪一併補上本機腳本、載入順序與成功後移除遮罩。實際以五個全新隔離 Chromium CLI profile 分別開啟五個離線匯出，確認正確 effect、Canvas draw、純月曆、遮罩移除與零日曆／host API 請求。探針等待離線 report 腳本載入訊號，避免虛擬時間與圖片解碼的競態；缺少訊號有實際十秒上限而非跳過。見 [離線驗證](../evidence/offline-particles-2026-11-06.json)。沒有連接使用者瀏覽器或偽造桌布 ACK。

0.7.30 已更新目前安裝 App，10 份設定逐位元保留，開機啟動、資訊省電與原選用 effect 不變；新 particle-client.js 及 UI 實際提供內容與建置完全一致。原生掛載、genuine wallpaper／manager ACK 與獨立 18 秒程序存活檢查通過。安裝版 agent schema 包含五個 effect，逐一只讀 validate-json 通過，desired revision／hash 未變，沒有替使用者切換效果。

首次安裝版只讀驗證曾回傳 exit 4（CLI 契約定義為 STORE_LOCKED、可重試）；初版檢查器未保留 error.code，已改善錯誤碼診斷。未繞過鎖、未自動重試不明提交，後續兩次完整只讀驗證連續通過；原始非成功結果不列為通過，也不宣稱不會再遇到忙碌狀態。

[升級](../evidence/particle-effects-upgrade-2026-11-06.json)、[安裝版 JSON 驗證](../evidence/particle-agent-installed-2026-11-06.json)、[宿主存活](../evidence/particle-effects-handoff-2026-11-06.json)。本機未簽章安裝檔 293,371,254 bytes，SHA-256 `2157d24f8f3eb69c61af9fb322bae5cad35cccdab629af0647700b327f4dbea6`；升級成功後清除舊 0.7.29 安裝檔，保留最新版本。實機 GPU／耗電及新效果的兩小時長測未執行，資訊省電偏好保持原值，沒有人工目視驗證五種桌布。禁止 Computer Use 與使用者瀏覽器／桌面 UI 自動化；安裝檔、私人設定及原始紀錄不提交。
