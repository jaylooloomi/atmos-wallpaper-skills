import {parentPort,workerData} from 'node:worker_threads';
import {parseCalendar} from './calendar-parse.mjs';
try{parentPort.postMessage({ok:true,result:parseCalendar(workerData.text,workerData.options)})}catch(e){parentPort.postMessage({ok:false,code:e.code||'ICS_INVALID'})}
