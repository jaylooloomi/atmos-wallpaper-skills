// Static machine-readable policy schemas. Reading this catalog never opens a store or network.
const draft='https://json-schema.org/draft/2020-12/schema';
const boolean={type:'boolean'},revision={type:'integer',minimum:0,maximum:Number.MAX_SAFE_INTEGER};
const requestId={type:'string',minLength:1,maxLength:80,pattern:'^[A-Za-z0-9][A-Za-z0-9._-]{0,79}$'};
const displayId={type:'string',pattern:'^display-[a-f0-9]{64}$'};
const targets=(minItems=0)=>({type:'array',minItems,maxItems:16,uniqueItems:true,items:displayId});
const object=properties=>({type:'object',additionalProperties:false,required:Object.keys(properties),properties});
const choices=(...values)=>({enum:values});
const update=(name,key,value)=>({$schema:draft,$id:'urn:atmos:host:'+name+':1',title:'Atmos '+name+' update v1',...object({schemaVersion:{const:1},requestId,expectedRevision:revision,[key]:value})});
export const hostSchemas={
 layoutUpdate:update('layout','layout',{oneOf:[object({mode:{const:'single'},targets:{...targets(),maxItems:0},askOnApply:boolean}),object({mode:choices('independent','duplicate','span'),targets:targets(1),askOnApply:boolean})]}),
 audioUpdate:update('audio','audio',object({enabled:boolean,muted:boolean,volume:{type:'integer',minimum:0,maximum:100},focusOnly:boolean,source:choices('weather','rain','thunder'),output:{oneOf:[{const:'system-default'},{type:'string',pattern:'^device-[a-f0-9]{64}$'}]}})),
 audioRoutingUpdate:update('audio-routing','routing',{oneOf:[object({mode:choices('primary','all'),target:{type:'null'}}),object({mode:{const:'display'},target:displayId})]}),
 performanceUpdate:update('performance','performance',object({fps:choices(5,15,30),pauseBattery:boolean,pauseFullscreen:boolean})),
 screensaverUpdate:update('screensaver','screensaver',object({mode:choices('independent','duplicate','span'),targets:targets()})),
 providerUpdate:update('providers','weather',object({enabled:boolean,city:choices('台北','東京','倫敦','紐約')})),
 libraryArchive:{$schema:draft,$id:'urn:atmos:host:library-archive:1',...object({schemaVersion:{const:1},requestId,expectedRevision:revision,auditHash:{type:'string',pattern:'^[a-f0-9]{64}$'}})}
};
export const hostContract={apiVersion:1,title:'Atmos host policy command schemas',schemas:hostSchemas,namespaces:{layout:{command:'--layout-command get|update',schema:'layoutUpdate'},audio:{command:'--audio-command get|update',schema:'audioUpdate'},audioRouting:{command:'--audio-routing-command get|update',schema:'audioRoutingUpdate'},performance:{command:'--performance-command get|update',schema:'performanceUpdate'},screensaver:{command:'--screensaver-command get|update',schema:'screensaverUpdate'},providers:{command:'--providers-command get|update',schema:'providerUpdate'},libraryArchive:{command:'--library-command audit|archive|result',schema:'libraryArchive'}},rules:['Schemas describe strict update input; read namespace state/revision before submitting. Namespace revisions are separate from appearance revisions.','Native commands validate connected display identities, filesystem ownership and current OS capability in addition to JSON Schema. Schema acceptance alone does not reserve a revision or prove a commit.','All global host namespaces reject --target; appearance presets/schedules use explicit target in independent mode. Provider commands do not fetch weather, sample system metrics or launch the host.','Library archive requires auditHash from a fresh audit. Retained references or inventory changes invalidate it. Archive retains copies; no permanent deletion or disk savings.','committed is persistence only; renderer acknowledgement, native attachment and OS visibility are separate.']};
