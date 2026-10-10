import {readFile, lstat, realpath, writeFile} from 'node:fs/promises';
import path from 'node:path';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {fileURLToPath} from 'node:url';
import {checkMaintenanceAdmission} from '../../tauri/scripts/native-perf-maintenance-contract.mjs';
import {describeAuthAttribution} from '../../tauri/scripts/native-perf-auth-contract.mjs';

const stage = path.dirname(fileURLToPath(import.meta.url));
const load = async (file) => JSON.parse(await readFile(file,'utf8'));
const digest = async (file) => createHash('sha256').update(await readFile(file)).digest('hex');
const frozen = await load(path.join(stage,'runtime-package.json'));
assert.equal(await digest(frozen.exe), frozen.sha256);
assert.equal(frozen.resources.length,94);
for (const resource of frozen.resources) assert.equal(await digest(resource.copy),resource.sha256);
const samples = [];
for (const size of [100,5000]) {
  const root = path.join(stage,`auth${size}`);
  const group = await load(path.join(root,'native-perf-auth-attribution-results.json'));
  const journey = await load(path.join(root,'native-perf-sdk-journey-results.json'));
  const receipt = await load(path.join(stage,`native-auth-${size}.receipt.json`));
  assert.equal(group.diagnosticOnly,true);
  assert.equal(group.performanceMetrics,null);
  assert.equal(group.samplesRequested,3);
  assert.equal(group.samples.length,3);
  assert.equal(journey.samples.length,3);
  assert.equal(receipt.sourceUnchanged,true);
  assert.equal(receipt.pdfiumOverrideInherited,false);
  assert.deepEqual(group.samples.map((s)=>s.index),[1,2,3]);
  assert.deepEqual(journey.samples.map((s)=>s.index),[1,2,3]);
  for (const item of group.samples) {
    const expected = path.join(root,`sample-${String(item.index).padStart(3,'0')}`);
    const source = journey.samples.find((s)=>s.index===item.index);
    assert.equal(path.resolve(item.root).toLowerCase(),expected.toLowerCase());
    assert.equal(path.resolve(source.root).toLowerCase(),expected.toLowerCase());
    const names = ['native-perf-auth-attribution.json','native-perf-maintenance-admission.json'];
    const artifacts = {};
    for (const name of names) {
      const file = path.join(expected,name);
      const stat = await lstat(file);
      assert(stat.isFile() && !stat.isSymbolicLink());
      assert(stat.size <= (name.includes('maintenance') ? 4194304 : 262144));
      assert.equal((await realpath(file)).toLowerCase(),file.toLowerCase());
      artifacts[name] = {value:await load(file),sha256:await digest(file)};
    }
    const auth = artifacts[names[0]].value;
    const diagnostic = describeAuthAttribution(auth,source.owned,source.bound);
    const admission = checkMaintenanceAdmission(artifacts[names[1]].value,auth,source.owned,source.bound);
    assert.deepEqual(diagnostic,item.diagnostic);
    assert.deepEqual(admission,item.maintenance);
    const accepted = receipt.exitCode===0 && group.success===true && journey.success===true &&
      item.journeySuccess===true && item.success===true && source.success===true &&
      diagnostic.frontend.flows.length===2 && diagnostic.frontend.flows.every((f)=>f.outcome==='finished') &&
      admission.failures.length===0;
    samples.push({objects:size,index:item.index,accepted,journeySuccess:source.success,
      authenticationFlowOutcomes:diagnostic.frontend.flows.map((f)=>f.outcome),
      maintenanceFailures:admission.failures,registeredActivities:admission.registeredActivities,
      runId:admission.runId,pid:admission.pid,
      artifactSha256:Object.fromEntries(names.map((n)=>[n,artifacts[n].sha256]))});
  }
}
assert.equal(samples.length,6);
assert.equal(new Set(samples.map((s)=>s.runId)).size,6);
const result = {schemaVersion:1,task:'RF-1104',scope:'windows-native-maintenance-regression',
  diagnosticOnly:true,performanceMetrics:null,
  accepted:samples.every((s)=>s.accepted),sampleCount:6,
  exeSha256:frozen.sha256,sourceManifestSha256:frozen.sourceManifestSha256,
  rf312TaskClosed:false,oldHomeTimeoutRootCauseEstablished:false,samples};
await writeFile(path.join(stage,'native-validation.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx'});
process.stdout.write(JSON.stringify(result,null,2)+'\n');
process.exitCode = result.accepted ? 0 : 1;
