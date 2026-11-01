import {validateDesired as oldValidate} from '../../implementation-p1/src/contract.mjs';
import {validateDesired,initialDesired,fail} from './contract.mjs';
export function validateSupportedDesired(value){if(value?.formatVersion===2)return validateDesired(value);if(value&&Object.hasOwn(value,'formatVersion'))fail('DESIRED_VERSION_UNSUPPORTED','unsupported desired format version');return oldValidate(value)}
export function normalizeLegacyDesired(value){const old=validateSupportedDesired(value);if(old.formatVersion===2)return old;const base=initialDesired();return validateDesired({...old,formatVersion:2,visual:{...base.visual,...old.visual,effect:old.visual.effect==='tornado'?'clear':old.visual.effect,paused:false,reduced:false,lightning:true},widgets:{...old.widgets,cards:Object.fromEntries(Object.entries(old.widgets.cards).map(([id,c])=>[id,{...c,font:'theme',fontSize:null,color:null}]))}})}
export function needsLegacyMigration(value){return !!value&&value.formatVersion!==2}
