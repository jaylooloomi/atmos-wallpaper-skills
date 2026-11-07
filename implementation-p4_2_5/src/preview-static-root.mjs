// Explicit development override only; installed/native-host entry points keep their bundled UI.
import fs from 'node:fs';import path from 'node:path';import {validateLocalRoot} from './portable-store.mjs';
export function previewStaticRoot({requested,defaultRoot,nativeExecutable}){
 if(requested===undefined)return defaultRoot;
 if(nativeExecutable)throw Error('PREVIEW_STATIC_ROOT_NATIVE_DENIED');
 if(typeof requested!=='string'||!path.isAbsolute(requested))throw Error('PREVIEW_STATIC_ROOT_INVALID');
 validateLocalRoot(requested);let at=path.resolve(requested);while(true){if(fs.lstatSync(at).isSymbolicLink())throw Error('PREVIEW_STATIC_ROOT_LINK_DENIED');const parent=path.dirname(at);if(parent===at)break;at=parent}
 if(!fs.statSync(requested).isDirectory()||!fs.statSync(path.join(requested,'index.html')).isFile())throw Error('PREVIEW_STATIC_ROOT_INVALID');return fs.realpathSync(requested);
}
