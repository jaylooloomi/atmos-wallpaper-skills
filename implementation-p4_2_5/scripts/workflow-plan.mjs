// This engine can be exercised with synthetic dependencies. It cannot mint a
// production release proof; only workflow.mjs owns that WeakMap.
export async function executePlan({prepare,capture,assertArtifact,checks,layout,unitOnly=false,record=()=>{}}){
 let artifact;const results=[];try{
  await record('start');await prepare();artifact=await capture();
  for(const check of checks){await assertArtifact(artifact);const result=await check.run(artifact);results.push({name:check.name,...result});await assertArtifact(artifact);if(!result.passed)throw Error('WORKFLOW_CHECK_FAILED:'+check.name)}
  let layoutResult=null;if(!unitOnly){await assertArtifact(artifact);layoutResult=await layout(artifact);if(!layoutResult.passed||layoutResult.viewports!==12||layoutResult.componentThemeStates!==336||layoutResult.weatherCardStates!==720||layoutResult.negativeControl?.expectedOverlapRejected!==true)throw Error('WORKFLOW_LAYOUT_INCOMPLETE');await assertArtifact(artifact)}
  const result={passed:true,diagnosticOnly:unitOnly,artifact,results,layout:layoutResult};await record('complete',result);return result;
 }catch(error){await record('failed',{passed:false,error:error.message,artifact,results});throw error}
}
