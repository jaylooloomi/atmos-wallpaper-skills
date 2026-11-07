import fs from 'node:fs';import path from 'node:path';import {inventory,digest,fileEntry,frontendInputs} from '../../implementation-p4_2_5/scripts/artifact.mjs';import {assertVerified} from '../../implementation-p4_2_5/scripts/workflow.mjs';import {spawnSync} from 'node:child_process';
import {atomicJson} from '../../implementation-p4_2_5/scripts/process-runner.mjs';
// Dependency injection creates an isolated test policy, never a production proof.
export function createPayloadPolicy({assertProof,readInputs,mappings}){
 const seals=new Map();
 function expected(proof,payload,context){for(const {source,target} of mappings(proof,context)){
  const a=fs.lstatSync(source);if(a.isSymbolicLink())throw Error('PAYLOAD_SOURCE_SYMLINK');
  if(a.isDirectory()){if(digest(inventory(source))!==digest(inventory(path.join(payload,target))))throw Error('PAYLOAD_SOURCE_MISMATCH')}
  else if(digest(fileEntry(source))!==digest(fileEntry(path.join(payload,target))))throw Error('PAYLOAD_SOURCE_MISMATCH');
 }}
 function seal(proof,payload,metadata={}){
  const key=path.resolve(payload);seals.delete(key);const manifest=path.join(key,'payload-manifest.json');if(fs.existsSync(manifest))fs.unlinkSync(manifest);
  try{const context=assertProof(proof);expected(proof,key,context);const inputs=readInputs(proof,context);atomicJson(path.join(key,'payload-inputs.json'),inputs);const files=inventory(key),value={...metadata,files};atomicJson(manifest,value);seals.set(key,{proof,inputsHash:digest(inputs),manifest:digest(fs.readFileSync(manifest)),filesHash:digest(files)});return value}catch(e){if(fs.existsSync(manifest))fs.unlinkSync(manifest);throw e}
 }
 function verify(proof,payload){const key=path.resolve(payload),seal=seals.get(key);try{
  const context=assertProof(proof);if(!seal||seal.proof!==proof)throw Error('PAYLOAD_CURRENT_RUN_PROOF_REQUIRED');
  if(digest(readInputs(proof,context))!==seal.inputsHash)throw Error('PAYLOAD_INPUT_CHANGED');
  if(digest(fs.readFileSync(path.join(key,'payload-manifest.json')))!==seal.manifest)throw Error('PAYLOAD_MANIFEST_CHANGED');
  if(digest(inventory(key,{exclude:['payload-manifest.json']}))!==seal.filesHash)throw Error('PAYLOAD_BYTES_CHANGED');expected(proof,key,context);return {matched:true,scope:'complete payload, current process only'};
 }catch(e){seals.delete(key);throw e}}
 return {seal,verify,invalidate(payload){seals.delete(path.resolve(payload))}};
}
const relativeFiles=['SOURCE-MANIFEST.json','implementation-p4_2_5/package.json','implementation-p4_2_5/package-lock.json','desktop-host/installer/runtime-lock.json','desktop-host/package.json','desktop-host/package-lock.json','desktop-host/Cargo.toml','desktop-host/Cargo.lock','desktop-host/tauri.conf.json','desktop-host/build.rs','.cargo/config','.cargo/config.toml','desktop-host/.cargo/config','desktop-host/.cargo/config.toml','rust-toolchain','rust-toolchain.toml','desktop-host/rust-toolchain','desktop-host/rust-toolchain.toml'];
function releaseTools(){const result={};for(const name of ['cargo','rustc']){const executable=path.join(process.env.USERPROFILE,'.cargo/bin',name+'.exe'),r=spawnSync(executable,['--version'],{encoding:'utf8',windowsHide:true,timeout:10000});if(r.status!==0)throw Error('RELEASE_TOOL_UNAVAILABLE');result[name]={version:r.stdout.trim(),...fileEntry(executable)}}return result}
export const payloadPolicy=createPayloadPolicy({
 assertProof:assertVerified,
 readInputs(proof,s){return {schemaVersion:1,artifact:s.identity,nativeResources:{icons:inventory(path.join(s.repo,'desktop-host/icons')),buildEnvironmentHash:digest(Object.fromEntries(['RUSTFLAGS','CARGO_ENCODED_RUSTFLAGS','RUSTUP_TOOLCHAIN','RUSTUP_HOME','CARGO_HOME','CC','CXX','CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER'].map(k=>[k,s.env[k]||null]))),ignoredOverrides:['TAURI_CONFIG','CARGO_TARGET_DIR','CARGO_BUILD_TARGET']},sourceInputs:frontendInputs(s.repo,{dependencies:false}),dependencies:{product:inventory(path.join(s.repo,'implementation-p4_2_5/node_modules')),desktop:inventory(path.join(s.repo,'desktop-host/node_modules'))},tools:{...s.tools,release:releaseTools()},files:Object.fromEntries(relativeFiles.map(p=>[p,fileEntry(path.join(s.repo,p))]))}},
 mappings(proof,s){
  const inputs=['implementation-p1/src/decode-image.py','prototype-v8.1-frozen/manifest.json','prototype-v8.1-frozen/assets','SOURCE-MANIFEST.json'];
  return [...inputs.map(p=>({source:path.join(s.repo,p),target:'app/'+p})),{source:s.artifact.dist,target:'app/implementation-p4_2_5/dist'}];
 }
});
