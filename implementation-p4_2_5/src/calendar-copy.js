(()=>{const rows=[
 ['ICS 日曆','ICS 日历','ICS calendar','ICS カレンダー','ICS 캘린더'],
 ['我的 ICS 日曆','我的 ICS 日历','My ICS calendar','自分の ICS カレンダー','내 ICS 캘린더'],
 ['匯入日曆','导入日历','Import calendar','カレンダーを読み込む','캘린더 가져오기'],
 ['訂閱日曆','订阅日历','Subscribe to calendar','カレンダーを購読','캘린더 구독'],
 ['立即更新','立即更新','Refresh now','今すぐ更新','지금 새로고침'],
 ['移除本機日曆','移除本机日历','Remove local calendar','ローカルカレンダーを削除','로컬 캘린더 제거'],
 ['尚未匯入日曆','尚未导入日历','No calendar imported','カレンダー未読み込み','가져온 캘린더 없음'],
 ['日曆處理中…','日历处理中…','Processing calendar…','処理中…','캘린더 처리 중…'],
 ['日曆資料已關閉','日历数据已关闭','Calendar data is off','カレンダーデータはオフです','캘린더 데이터가 꺼져 있습니다'],
 ['未來 7 天沒有行程。','未来 7 天没有日程。','No events in the next 7 days.','今後7日間に予定はありません。','향후 7일간 일정이 없습니다.'],
 ['請在日曆設定匯入檔案或貼上訂閱網址。','请在日历设置导入文件或粘贴订阅网址。','Import a file or add a subscription in calendar settings.','設定でファイルを読み込むか購読URLを入力してください。','설정에서 파일을 가져오거나 구독 URL을 추가하세요.'],
 ['行程過多，只顯示前 200 筆。','日程过多，仅显示前 200 条。','Showing the first 200 events.','最初の200件を表示しています。','처음 200개 일정만 표시합니다.'],
 ['日曆名稱（選填）','日历名称（选填）','Calendar name (optional)','カレンダー名（任意）','캘린더 이름 (선택 사항)'],
 ['不用登入。匯入 .ics 檔，或訂閱 Google／Outlook 提供的 ICS 連結。','无需登录。导入 .ics 文件，或订阅 Google／Outlook 提供的 ICS 链接。','No sign-in needed. Import an .ics file or subscribe to a Google/Outlook ICS link.','ログイン不要。.icsファイルの読み込み、またはGoogle/OutlookのICSリンクを購読できます。','로그인 없이 .ics 파일을 가져오거나 Google/Outlook ICS 링크를 구독하세요.'],
 ['日曆內容已儲存；請按上方「套用變更」保存顯示模式。','日历内容已保存；请点击上方“应用更改”保存显示模式。','Calendar saved. Apply changes above to save its display mode.','保存しました。上部の適用ボタンで表示モードを保存してください。','저장되었습니다. 상단의 변경 사항 적용으로 표시 모드를 저장하세요.']
 ];for(const values of rows)window.AtmosI18nCatalog.messages[values[0]]=Object.fromEntries(['zh-TW','zh-CN','en','ja','ko'].map((locale,i)=>[locale,values[i]]));})();
