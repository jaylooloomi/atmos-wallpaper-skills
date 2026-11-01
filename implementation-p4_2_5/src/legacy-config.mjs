import {validateDesired as oldValidate} from '../../implementation-p1/src/contract.mjs';
import {validateDesired as validateV3} from './state-v3.mjs';
import {validateDesired as validateV2} from './state-v2.mjs';
import {validateDesired,initialDesired,fail} from './contract.mjs';
export function validateSupportedDesired(value){if(value?.formatVersion===4)return validateDesired(value);if(value?.formatVersion===3)return validateV3(value);if(value?.formatVersion===2)return validateV2(value);if(value&&Object.hasOwn(value,'formatVersion'))fail('DESIRED_VERSION_UNSUPPORTED','unsupported desired format version');return oldValidate(value)}
export function normalizeLegacyDesired(value){const old=validateSupportedDesired(value);if(old.formatVersion===4)return old;const base=initialDesired();return validateDesired({...old,formatVersion:4,visual:{...base.visual,...old.visual,lockScene:true,effect:old.visual.effect==='tornado'?'clear':old.visual.effect,paused:false,reduced:false,lightning:true},widgets:{...old.widgets,cards:Object.fromEntries(Object.entries(old.widgets.cards).map(([id,c])=>[id,{...c,font:c.font||'theme',fontSize:c.fontSize??null,color:c.color??null}]))}})}
export function needsLegacyMigration(value){return !!value&&value.formatVersion!==4}
