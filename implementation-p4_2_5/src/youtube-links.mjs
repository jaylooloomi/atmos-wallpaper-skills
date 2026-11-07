// Parse locally. Never fetch arbitrary input URLs or forward tracking parameters.
export function parseYouTubeLink(value){
 if(typeof value!=='string'||value.length>2048||/[\x00-\x1f\x7f]/.test(value))throw Error('YOUTUBE_URL_INVALID');
 let text=value.trim();if(!text)throw Error('YOUTUBE_URL_INVALID');
 if(/^[A-Za-z0-9_-]{11}$/.test(text))text='https://www.youtube.com/watch?v='+text;
 if(/^(?:www\.|m\.|music\.)?youtube\.com\//i.test(text)||/^youtu\.be\//i.test(text))text='https://'+text;
 let url;try{url=new URL(text)}catch{throw Error('YOUTUBE_URL_INVALID')}
 if(!['https:','http:'].includes(url.protocol)||url.username||url.password||url.port)throw Error('YOUTUBE_URL_INVALID');
 const host=url.hostname.toLowerCase(),short=host==='youtu.be';
 if(!short&&!['youtube.com','www.youtube.com','m.youtube.com','music.youtube.com','www.youtube-nocookie.com'].includes(host))throw Error('YOUTUBE_URL_INVALID');
 for(const key of ['v','list','t','start'])if(url.searchParams.getAll(key).length>1)throw Error('YOUTUBE_URL_INVALID');
 const parts=url.pathname.split('/').filter(Boolean);let videoId=null;
 if(short){if(parts.length!==1)throw Error('YOUTUBE_URL_INVALID');videoId=parts[0]}
 else if(parts.length===1&&parts[0]==='watch')videoId=url.searchParams.get('v');
 else if(parts.length===2&&['shorts','live','embed'].includes(parts[0])&&parts[1]!=='videoseries')videoId=parts[1];
 else if(!((parts.length===1&&parts[0]==='playlist')||(parts.length===2&&parts[0]==='embed'&&parts[1]==='videoseries')))throw Error('YOUTUBE_URL_INVALID');
 if(videoId!==null&&!/^[A-Za-z0-9_-]{11}$/.test(videoId))throw Error('YOUTUBE_URL_INVALID');
 const listId=url.searchParams.get('list');if(listId!==null&&!/^[A-Za-z0-9_-]{10,128}$/.test(listId))throw Error('YOUTUBE_URL_INVALID');
 if(!videoId&&!listId)throw Error('YOUTUBE_URL_INVALID');
 const raw=url.searchParams.get('start')??url.searchParams.get('t')??'0';let startSeconds;
 if(/^\d{1,6}$/.test(raw))startSeconds=Number(raw);
 else{const match=raw.match(/^(?:(\d{1,3})h)?(?:(\d{1,3})m)?(?:(\d{1,3})s)?$/);if(!match||!raw)throw Error('YOUTUBE_URL_INVALID');startSeconds=Number(match[1]||0)*3600+Number(match[2]||0)*60+Number(match[3]||0)}
 if(startSeconds>604800)throw Error('YOUTUBE_URL_INVALID');
 const watch=new URL(videoId?'https://www.youtube.com/watch':'https://www.youtube.com/playlist');if(videoId)watch.searchParams.set('v',videoId);if(listId)watch.searchParams.set('list',listId);if(startSeconds)watch.searchParams.set('t',String(startSeconds));
 return {videoId,listId,startSeconds,watchUrl:watch.href};
}
export function youtubeEmbedUrl(value,origin){
 const source=parseYouTubeLink(value),base=new URL(origin);
 if(base.origin!==origin||base.protocol!=='http:'||base.hostname!=='127.0.0.1'||!base.port)throw Error('YOUTUBE_ORIGIN_INVALID');
 const embed=new URL('https://www.youtube-nocookie.com/embed/'+(source.videoId||'videoseries'));
 for(const [key,item]of Object.entries({autoplay:'1',controls:'1',playsinline:'1',enablejsapi:'1',origin,...(source.listId?{list:source.listId,listType:'playlist'}:{}),...(source.startSeconds?{start:String(source.startSeconds)}:{})}))embed.searchParams.set(key,item);
 return embed.href;
}
