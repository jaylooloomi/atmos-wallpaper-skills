// Transport boundary for the native-host service. Keep business logic, session
// roles and store transactions in the bridge; a replacement runtime must pass
// the same wire-contract fixtures before it replaces this adapter.
import http from 'node:http';

export function createLocalServer({bridge,origin,trace=()=>{}}){
 const server=http.createServer(async(req,res)=>{
  try{
   let length=0;const parts=[];
   for await(const part of req){
    length+=part.length;
    if(length>(req.url==='/api/calendar/import'?2*1048576:32768)){
     res.writeHead(413,{'Content-Type':'application/json'});
     res.end('{"error":{"code":"BODY_TOO_LARGE"}}');return;
    }
    parts.push(part);
   }
   const headers=new Headers();
   for(const [k,v] of Object.entries(req.headers))if(v!==undefined)headers.set(k,Array.isArray(v)?v.join(','):v);
   const body=Buffer.concat(parts);
   if(['GET','HEAD'].includes(req.method)&&body.length){res.writeHead(400);res.end();return;}
   const request=new Request(new URL(req.url,origin),{method:req.method,headers,...(body.length?{body}:{})});
   const response=await bridge.handle(request,{rawPath:req.url});
   trace({kind:'http',at:Date.now(),path:new URL(req.url,origin).pathname,status:response.status});
   res.writeHead(response.status,Object.fromEntries(response.headers));
   res.end(Buffer.from(await response.arrayBuffer()));
  }catch{
   res.writeHead(500,{'Content-Type':'application/json'});
   res.end('{"error":{"code":"LOCAL_OPERATION_FAILED"}}');
  }
 });
 server.headersTimeout=10000;server.requestTimeout=15000;
 return server;
}
