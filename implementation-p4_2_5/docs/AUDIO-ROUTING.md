# 桌布音訊的邏輯螢幕政策

實作：`desktop-host/src/audio_routing.rs`、`audio.rs`、`src/audio-service.mjs` 與管理頁音訊區。

音訊有兩個獨立選擇：

- **實體播放裝置**：`audio-settings.json` 的 `audio.output`，選 Windows 預設或確切 WASAPI device ID。
- **聲音來自哪個桌布**：`audio-routing.json` 的 `routing`，選主螢幕、指定螢幕或所有已選螢幕。螢幕 ID 不能當成喇叭 ID。

新的政策預設為主螢幕。保留原本的啟用、靜音、音量、焦點、音源與實體裝置設定；不改 Windows 全域音量。

```text
atmos-wallpaper.exe --audio-routing-command get
atmos-wallpaper.exe --audio-routing-command update
```

update 由標準輸入讀取最多 16KiB 的嚴格 JSON；版本、requestId、CAS 與 receipt 屬於獨立命名空間：

```json
{
  "schemaVersion": 1,
  "requestId": "audio.route.001",
  "expectedRevision": 0,
  "routing": { "mode": "all", "target": null }
}
```

模式：`primary`／`all` 的 target 必須是 null；`display` 必須提供完整 `display-<64 lowercase hex>` ID。設定可以保留離線 ID；離線、不顯示或未選為桌布的指定目標不會自動改播其他螢幕。

管理頁使用經 session／same-origin／manager-role 保護的 `/api/host/audio/routing` GET／POST；不接受裝置路徑、命令或任意來源。與一般音訊設定分開 CAS，衝突後重新讀取並調和。

## 播放規則

原生宿主傳入自己管理的各桌布視窗、是否為主螢幕及其 appearance scope。`weather` 讀取音訊擁有者自己的天氣；指定 `rain`／`thunder` 仍只由符合政策且顯示中的桌布啟用。

主螢幕沒有 Atmos 桌布時，`primary` 保持安靜。`display` 目標離線／清除時安靜。`all` 使用所有顯示中的已選桌布，**相同音源種類只生成一份**；不同天氣先依來源去重，再以等權平均送入同一個有界混音 bus，不為每個 renderer 開另一個 WASAPI stream，避免重複音量。

`audio-status.json` 增加 routingRevision、routingMode、owners 與 mixPolicy。這是原生政策／串流狀態，不是人耳聽到的保證。

音量零、靜音、禁用、非桌面焦點（啟用 focus-only 時）、鎖屏／輸入桌面未知以及政策讀取失敗仍優先停止。模式或裝置改變不重置使用者偏好。

三秒測試音現在綁定正在運作的 host PID、audio session、設定 revision 與到期時間。沒有新鮮且仍運作的原生音訊宿主時拒絕要求；宿主重啟後不重播上一輪的短期測試意圖。

## 自動化驗證界線

Rust 測試涵蓋嚴格政策、獨立 CAS／receipt、主螢幕／指定目標不 fallback、隱藏目標、同音源去重、多來源平均、聲音有界與淡出、測試音 session／revision／expiry 綁定。Node 測試涵蓋固定 native 命令、JSON DTO、HTTP 權限與 appearance revision 不受影響。

這些測試不代表此次多螢幕聲音已由人耳驗收；先前 0.6.1 的單螢幕音量／靜音／焦點驗收紀錄維持原樣。
