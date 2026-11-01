export * from './state.mjs';
import {createHash} from 'node:crypto';import {canonical} from './state.mjs';
export const hash=value=>createHash('sha256').update(typeof value==='string'||Buffer.isBuffer(value)?value:canonical(value)).digest('hex');
