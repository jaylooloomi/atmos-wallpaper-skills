export function runtimeOptions(argv,env={}){
 const values={};for(let i=0;i<argv.length;i++){const key=argv[i];if(key==='--strictPort'){if(values.strict)throw Error('DUPLICATE_ARGUMENT');values.strict=true;continue}if(!['--host','--port'].includes(key)||values[key]!==undefined||!argv[i+1]||argv[i+1].startsWith('--'))throw Error('INVALID_RUNTIME_ARGUMENT');values[key]=argv[++i]}
 const host=values['--host']||'127.0.0.1';if(host!=='127.0.0.1')throw Error('LOOPBACK_REQUIRED');const raw=values['--port']??env.PORT;if(!raw||!/^\d+$/.test(raw))throw Error('PORT_REQUIRED');const port=Number(raw);if(port<1||port>65535)throw Error('PORT_INVALID');if(env.PORT&&values['--port']&&Number(env.PORT)!==port)throw Error('PORT_CONFLICT');return {host,port};
}
