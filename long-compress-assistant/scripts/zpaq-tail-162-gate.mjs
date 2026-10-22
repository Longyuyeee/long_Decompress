import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync,readdirSync,copyFileSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runZpaqTail162Gate({driver,bridge,application,root}){
 const base=path.join(root,'test-results/zpaq-tail-162');mkdirSync(base,{recursive:true})
 const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex')
 const state={version:'1.6.2',binarySha256:sha(application),work:path.join(base,randomUUID()),checks:[],layouts:[]};mkdirSync(state.work)
 const save=()=>writeFileSync(path.join(base,'desktop.json'),JSON.stringify(state,null,2))
 const find=s=>driver.wait(async()=>{for(const e of await driver.findElements(By.css(s)))if(await e.isDisplayed())return e;return false},30000,s)
 async function click(s){const e=typeof s==='string'?await find(s):s;await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000);await driver.wait(()=>driver.executeScript(e=>{const r=e.getBoundingClientRect();return !e.disabled&&e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))},e),15000);await e.click()}
 async function fill(s,value){const e=typeof s==='string'?await find(s):s;await driver.wait(()=>e.isEnabled(),30000);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);await e.sendKeys(value)}
 async function shot(name,selector){for(const[width,height]of[[1100,700],[760,520]]){
  await driver.manage().window().setRect({width,height});await find(selector);await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
  const geometry=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,panes:[...document.querySelectorAll('.file-pane')].map(e=>({width:e.getBoundingClientRect().width,height:e.getBoundingClientRect().height})),dialogs:[...document.querySelectorAll('[role="dialog"] .app-dialog')].filter(e=>e.getClientRects().length).map(e=>({overflow:e.scrollWidth-e.clientWidth,right:e.getBoundingClientRect().right}))}));assert.ok(geometry.overflow<=1&&geometry.dialogs.every(d=>d.overflow<=1&&d.right<=geometry.width+1),JSON.stringify(geometry));assert.equal(geometry.panes.length,2)
  const file=path.join(base,`${name}-${width}x${height}.png`);writeFileSync(file,Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,file,...geometry});save()
 }await driver.manage().window().setRect({width:1100,height:700})}
 const fixtures=path.join(root,'tests/fixtures/archives/zpaq-interrupted-162'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
 const tree=folder=>Object.fromEntries(readdirSync(folder,{withFileTypes:true}).flatMap(e=>{const f=path.join(folder,e.name);return e.isDirectory()?Object.entries(tree(f)).map(([p,h])=>[e.name+'/'+p,h]):[[e.name,sha(f)]]}))
 await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
 const queued=[]
 for(const c of manifest.cases){
  const folder=path.join(state.work,c.name);mkdirSync(folder);const members=c.parts.map(p=>{const f=path.join(folder,path.basename(p.file));copyFileSync(path.join(fixtures,p.file),f);assert.equal(sha(f),p.sha256);return f})
  await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[members[0]]]);await click('[data-testid="file-manager-open-archive"]')
  if(c.native)await click('[data-testid="zpaq-multipart-accept"]')
  await driver.wait(async()=>(await(await find('.pane-archive-location')).getAttribute('title')).includes(c.name),30000)
  if(c.encrypted){await fill('.file-manager .browser-toolbar input[type="password"]',manifest.password);await click('.file-manager .browser-toolbar button[title="重新读取"]')}
  await find('[data-testid="zpaq-interrupted-tail"]');assert.ok((await(await find('[data-testid="zpaq-interrupted-tail"]')).getText()).includes('旧快照'))
  assert.equal((await driver.findElements(By.css('.file-pane'))).length,2)
  state.checks.push({name:'diagnosis-'+c.name,members:members.length,oldSnapshots:3});save()
  if(c.name==='plain-single'||c.name==='key-native'){
   if(c.name==='plain-single')await shot('diagnosis','[data-testid="zpaq-interrupted-tail"]')
   await click('[data-testid="zpaq-recovery-open"]');const version=c.encrypted?1:2;const select=await find('[data-testid="zpaq-recovery-version"]');await select.findElement(By.css(`option[value="${version}"]`)).click()
   const output=path.join(folder,'retrieved');mkdirSync(output);await fill('[data-testid="zpaq-recovery-output"]',output)
   await driver.wait(async()=>await(await find('[data-testid="zpaq-recovery-confirm"]')).isEnabled(),30000)
   if(c.name==='plain-single')await shot('retrieve','[data-testid="zpaq-recovery-dialog"]')
   await click('[data-testid="zpaq-recovery-confirm"]');await find('[data-testid="zpaq-recovery-go-queue"]')
   const tasks=await bridge('extractionRecoveryTasks');assert.ok(tasks.every(t=>t.status==='pending'));const task=tasks.find(t=>t.output===output);assert.ok(task);queued.push({id:task.id,c,version,output,members});await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')))
  }
 }
 await click('[data-testid="nav-Decompress"]');await click('[data-testid="start-decompression-queue"]')
 for(const q of queued){
  const record=await driver.wait(async()=>(await bridge('taskHistory')).find(r=>r.id===q.id)||false,120000)
  assert.equal(record.status,'completed',JSON.stringify(record));assert.equal(record.executionConfig.zpaqSnapshot.version,q.version);assert.ok(record.logs.some(l=>l.message.includes('中断事务')&&l.message.includes('未取回')));assert.ok(!JSON.stringify(record).includes(manifest.password));assert.deepEqual(tree(record.outputPath),q.c.snapshots[q.version-1].expected)
  for(let i=0;i<q.members.length;i++)assert.equal(sha(q.members[i]),q.c.parts[i].sha256)
  state.checks.push({name:'retrieval-'+q.c.name,version:q.version,id:q.id,outputPath:record.outputPath,allHashes:true,originalsUnchanged:true,record});save()
 }
 state.completedAt=new Date().toISOString();save()
}
