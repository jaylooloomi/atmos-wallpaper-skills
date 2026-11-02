# ICS 日曆：不需要應用程式註冊或登入

重新整理管理頁，展開「日曆」。原先尚未實作的 Google／Microsoft 登入卡片已換成 ICS 匯入／訂閱。

1. 匯入：選取 `.ics` 檔案，按「匯入日曆」。ZIP 需要先解壓，單一 ICS 上限 1 MiB。
2. 訂閱：貼入 `https://` 或 `webcal://` ICS 網址，可填自己的日曆名稱，按「訂閱日曆」。webcal 會使用 HTTPS 下載。
3. 成功後會自動選到「我的 ICS 日曆」，成為顯示設定草稿。按頁面上方「套用變更」，桌布頁就會顯示這個來源。
4. 顯示今天起七天內的行程，包含進行中／跨日的行程、全天、重複規則、排除日期與改期。最多顯示 200 個實例。UTC、浮動本機時間、IANA 時區和 ICS 內的 VTIMEZONE 可解析；無法辨識的時區會明確拒絕，不當作已正確匯入。

同時只保留一個來源。匯入檔案是當下快照；重新匯入才會改變內容。訂閱在頁面開啟且服務運作時每 15 分鐘嘗試更新，也可按「立即更新」。瀏覽器通常每 30 秒讀取最新內容；長期關閉所有頁面不會另開背景排程。

匯入、訂閱、立即更新與移除會立即更改本機來源，不受上方「取消草稿」回滾；顯示模式仍需套用。新來源解析失敗會保留上次成功資料並顯示錯誤。移除只清除 Atmos 本機來源，不會刪除原日曆上的行程。這是唯讀日曆，不會新增、刪改或邀請他人。

## 取得檔案或連結

- Google Calendar：在電腦網頁版匯出日曆，解壓後匯入 ICS；也可從該日曆設定的整合區取得 iCal 格式秘密網址。不要貼一般日曆網頁或 HTML 嵌入網址。[Google 官方匯出說明](https://support.google.com/calendar/answer/37111)
- Outlook：可以匯出 ICS，或在服務允許的情況下取得發佈日曆的 ICS 訂閱連結。公司政策可能禁止分享／發佈。[Microsoft 官方說明](https://support.microsoft.com/en-us/outlook/introduction-to-publishing-internet-calendars)

私密連結如同密碼，持有者可讀取它公開提供的內容；尤其「發佈日曆」不等於私人 OAuth 登入。你可以只匯入檔案，不必發佈日曆。連結只填在本機管理頁，請勿貼到聊天、issue 或 Git。

## 資料與權限

來源存於目前 store 的 `calendar/connection.bin`。Windows 使用目前 Windows 帳號的 DPAPI 加密 ICS 內容與訂閱 URL；其他 OS 使用受限檔案權限，不宣稱相同的磁碟加密。URL 不會傳給前端或桌布，行程不送 AI、不加入控制設定的備份或離線 renderer 匯出；若移動電腦／Windows 帳號，需要重新匯入或訂閱。

管理頁 session 才能改來源；桌布 session 只能讀取正規化的行程。所有寫入需要正確 Origin 與 JSON。HTTPS 下載拒絕本機、內網、保留 IP、帳密 URL 和非標準連接埠；DNS 位址會驗證並固定，重新導向每一跳重新驗證。回應有時間及大小上限。

解析在有記憶體／時間上限的 worker 中執行，過密或過久的重複規則可能被拒絕；這是明確的支援限制。失敗不會覆蓋上一份成功來源。

## 開發與驗證

本版新增 production dependencies `ical.js`、`luxon`，及僅測試使用的 `linkedom`。從乾淨 checkout 在本版目錄執行：

```sh
npm ci --ignore-scripts
node scripts/verify.mjs
```

Windows 仍需 Node >=22 及 Python/Pillow，設定 `ATMOS_PYTHON` 完整路徑。測試涵蓋 recurrence／EXDATE／改期、跨時區與日期邊界、全天、取消、錯誤資料與工作上限、DPAPI 實際讀回、來源保存與移除、SSRF／DNS 固定／重新導向、role／origin、真正 Windows HTTP 匯入大於 32 KiB 的檔案，以及 DOM 的匯入草稿和安全文字渲染。沒有使用 Computer Use 或瀏覽器 UI 自動化；DOM 測試不宣稱真人瀏覽器的視覺 QA。

另實際下載並解析 Google 公開台灣節日 ICS（120,533 bytes）；當次未來七天沒有節日是有效空結果。未連線使用者的私人 Google／Microsoft 帳號，未把公開來源預先替使用者訂閱。

解析套件授權：ICAL.js MPL-2.0、Luxon MIT；依鎖定的 npm 套件各自隨附 license，保留上游授權。
