// Inspect the normal installer without installing or executing input SFX files.
import assert from 'node:assert/strict'
import {execFileSync} from 'node:child_process'
import {readFileSync,mkdirSync} from 'node:fs'
import {createHash} from 'node:crypto'
import path from 'node:path'
export function inspectSfxInstaller(installer,destination,expectedVersion='1.5.5'){
  mkdirSync(destination,{recursive:true})
  const seven=path.resolve('src-tauri/resources/archive-engine/7z.exe'),sha=b=>createHash('sha256').update(b).digest('hex')
  const listing=execFileSync(seven,['l','-slt','-sccUTF-8',installer],{encoding:'utf8',maxBuffer:8*1024*1024})
  const paths=[...listing.matchAll(/^Path = (.+)\r?$/gm)].map(m=>m[1].trim()),wanted=['archive-engine/licenses/UnRAR-License.txt','archive-engine/SOURCE.md'],members=new Map()
  for(const name of wanted){const matches=paths.filter(p=>p.replaceAll('\\','/').endsWith(name));assert.equal(matches.length,1,name);members.set(name,matches[0])}
  const mains=paths.filter(p=>p.replaceAll('\\','/').split('/').pop()==='Long解压.exe');assert.equal(mains.length,1)
  execFileSync(seven,['x','-y','-o'+destination,installer,...members.values(),mains[0]],{maxBuffer:4*1024*1024})
  const notices=[]
  for(const[name,member]of members){const bytes=readFileSync(path.join(destination,member));assert.equal(sha(bytes),sha(readFileSync(path.join('src-tauri/resources',name))),name);notices.push({name,bytes:bytes.length,sha256:sha(bytes)})}
  const main=path.join(destination,mains[0]),version=execFileSync('powershell.exe',['-NoProfile','-Command',`(Get-Item -LiteralPath '${main.replaceAll("'","''")}').VersionInfo.ProductVersion`],{encoding:'utf8'}).trim();assert.equal(version,expectedVersion)
  return{mainProductVersion:version,mainBytes:readFileSync(main).length,mainSha256:sha(readFileSync(main)),notices,installationExecuted:false,productionSfxRuntimeExecuted:false,note:'Native SFX functionality is verified by the separately identified desktop E2E binary; this receipt checks the ordinary packaged main executable and exact UnRAR notice/source disclosure.'}
}
