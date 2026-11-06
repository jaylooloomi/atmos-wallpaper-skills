// Build-time Node only. The installed executable embeds these data catalogs.
import fs from 'node:fs';
import {fileURLToPath} from 'node:url';
import {agentHelp,commandSchema} from '../../implementation-p4_2_5/src/agent-contract.mjs';
import {hostContract} from '../../implementation-p4_2_5/src/host-contract.mjs';
import {appearanceHelp} from '../../implementation-p4_2_5/src/appearance-cli.mjs';
const root=fileURLToPath(new URL('../src/cli-catalogs/',import.meta.url));
const catalogs={'agent-help':{apiVersion:1,ok:true,guide:agentHelp},'agent-schema':commandSchema,'host-schema':hostContract,'appearance-help':appearanceHelp};
if(!process.argv.includes('--check'))fs.mkdirSync(root,{recursive:true});
for(const [name,value] of Object.entries(catalogs)){
 const file=root+name+'.json',text=JSON.stringify(value,null,2)+'\n';
 if(process.argv.includes('--check')){if(fs.readFileSync(file,'utf8')!==text)throw Error('CLI_CATALOG_STALE: '+name);}
 else fs.writeFileSync(file,text);
}
console.log(JSON.stringify({catalogs:Object.keys(catalogs).length,checked:process.argv.includes('--check')}));
