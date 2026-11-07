export async function executeReleasePlan({proof,reusePayload=false,assertProof,preparePayload,verifyPayload,bundle}){
 assertProof(proof);
 if(!reusePayload)await preparePayload(proof);
 assertProof(proof);verifyPayload(proof);
 await bundle(proof);
 assertProof(proof);verifyPayload(proof);
 return {passed:true,fullVerificationReusedWithinProcess:true};
}
