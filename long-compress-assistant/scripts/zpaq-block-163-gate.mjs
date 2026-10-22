import assert from 'node:assert/strict'
import {createHash,randomUUID} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync,copyFileSync,existsSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

export async function runZpaqBlock163Gate({driver,bridge,application,root}){
 const base=path.join(root,'test-results/zpaq-block-163');mkdirSync(base,{recursive:true})
 const sha=p=>createHash('sha256').update(readFileSync(p)).digest('hex'),receipt=path.join(base,'desktop.json')
 const state=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{version:'1.6.3',binarySha256:sha(application),candidateOptimization:'app opt-level=1; CI and release unchanged',work:path.join(base,randomUUID()),checks:[],layouts:[]}
 if(state.binarySha256!==sha(application)){assert.equal(process.env.ZPAQ_BLOCK_163_RESUME_FROM,state.binarySha256,'A candidate change requires explicit evidence continuation');for(const row of [...state.checks,...state.layouts])row.binarySha256||=state.binarySha256;state.previousIdentities=[...(state.previousIdentities||[]),state.binarySha256];state.binarySha256=sha(application);state.continuationReason='Only FileIntegrity old-snapshot initialization guard changed; resume affected entry flows, retain completed diagnosis and layout evidence'}
 mkdirSync(state.work,{recursive:true})
 const save=()=>writeFileSync(receipt,JSON.stringify(state,null,2)),passed=name=>state.checks.some(c=>c.name===name),done=(name,evidence={})=>{state.checks.push({name,binarySha256:state.binarySha256,...evidence});save()}
 const find=s=>driver.wait(async()=>{for(const e of await driver.findElements(By.css(s)))if(await e.isDisplayed())return e;return false},30000,s)
 async function click(s){await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000);const e=typeof s==='string'?await find(s):s;await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await driver.wait(()=>driver.executeScript(e=>{const r=e.getBoundingClientRect();return !e.disabled&&e.contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))},e),15000);await e.click()}
 async function fill(s,value){const e=await find(s);await driver.wait(()=>e.isEnabled(),30000);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE);if(value)await e.sendKeys(value)}
 async function shot(name,selector){for(const[width,height]of[[1100,700],[760,520]]){
  await driver.manage().window().setRect({width,height});const target=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),target);await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
  const geometry=await driver.executeScript(()=>({width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,dialogs:[...document.querySelectorAll('[role="dialog"] .app-dialog')].filter(e=>e.getClientRects().length).map(e=>({overflow:e.scrollWidth-e.clientWidth,x:e.getBoundingClientRect().x,right:e.getBoundingClientRect().right}))}));assert.ok(geometry.overflow<=1&&geometry.dialogs.every(d=>d.overflow<=1&&d.x>=0&&d.right<=geometry.width+1),JSON.stringify(geometry));const file=path.join(base,`${name}-${width}x${height}.png`);writeFileSync(file,Buffer.from(await driver.takeScreenshot(),'base64'));state.layouts.push({name,file,binarySha256:state.binarySha256,...geometry});save()
 }await driver.manage().window().setRect({width:1100,height:700})}
 const fixtures=path.join(root,'tests/fixtures/archives/zpaq-damaged-163'),manifest=JSON.parse(readFileSync(path.join(fixtures,'manifest.json'),'utf8'))
 if(!state.parts){state.parts=manifest.cases.map(c=>{const folder=path.join(state.work,c.name);mkdirSync(folder);return c.parts.map(p=>{const f=path.join(folder,path.basename(p.file));copyFileSync(path.join(fixtures,p.file),f);assert.equal(sha(f),p.sha256);return f})});save()}
 async function choose(source,encrypted=false){
  await bridge('queueDesktopDialogSelections',[[source]]);await click('[data-testid="diagnostic-choose-archive"]')
  if(/part\d+\.zpaq$/i.test(source))await click('[data-testid="zpaq-multipart-accept"]')
  await driver.wait(async()=>(await(await find('[data-testid="archive-diagnostic-panel"] .archive-inputs ~ p')).getText()).includes(path.basename(path.dirname(source))),30000)
  if(encrypted)await fill('[data-testid="diagnostic-password"]',manifest.password)
  await click('[data-testid="start-archive-diagnosis"]')
 }
 const closeDetails=async()=>{await click(await driver.findElement(By.xpath('//*[@role="dialog"]//button[normalize-space(.)="关闭"]')));await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="zpaq-block-details"]'))).length,15000)}
 await driver.manage().setTimeouts({script:180000});await driver.manage().window().setRect({width:1100,height:700})
 await click('[data-testid="nav-FileIntegrity"]');await click('[data-testid="archive-diagnostic-mode"]')
 for(let i=0;i<manifest.cases.length;i++){
  const c=manifest.cases[i],name='diagnosis-'+c.name;if(passed(name))continue
  await choose(state.parts[i][0],c.encrypted);await find('[data-testid="zpaq-block-summary"]');assert.ok((await(await find('[data-testid="zpaq-block-summary"]')).getText()).includes('2 受影响'))
  assert.equal((await driver.findElements(By.css('[data-testid="start-archive-repair"]'))).length,0)
  if(i===0)await shot('summary','[data-testid="zpaq-block-summary"]')
  await click('[data-testid="zpaq-block-details-open"]');const entries=await driver.findElements(By.css('[data-testid="zpaq-block-list"] [role="listitem"]'));assert.equal(entries.length,2)
  const names=await Promise.all(entries.map(e=>e.findElement(By.css('strong')).getText()));assert.deepEqual(names.sort(),[...c.affected].sort());assert.ok((await(await find('[data-testid="zpaq-block-details"]')).getText()).includes('快照 3'))
  if(i===0)await shot('files','[data-testid="zpaq-block-details"]')
  await click('[data-testid="zpaq-block-blocks-tab"]');const failed=await driver.findElements(By.css('[data-testid="zpaq-block-list"] [data-state="failed"]'));assert.equal(failed.length,1)
  if(i===0)await shot('blocks','[data-testid="zpaq-block-details"]')
  await click('[data-testid="zpaq-block-files"]');assert.equal((await driver.findElements(By.css('[data-testid="zpaq-block-list"] [data-state="affected"]'))).length,2);await closeDetails();done(name,{affected:names,sharedBlock:c.targetBlock.id,noPartialExtraction:true,sourceNamesVisible:true})
 }
 if(!passed('older-snapshot-browser-entry')){
  const back=await driver.findElements(By.css('[data-testid="zpaq-diagnosis-browse"]'))
  if(back.length)await click(back[0]);else{await click('[data-testid="nav-ArchiveBrowser"]');await bridge('queueDesktopDialogSelections',[[state.parts[3][0]]]);await click('[data-testid="file-manager-open-archive"]');await click('[data-testid="zpaq-multipart-accept"]')}
  await driver.wait(async()=>!(await driver.findElements(By.css('[data-testid="archive-browse-cancel"]'))).length,30000)
  const passwordHelp=await driver.findElements(By.xpath('//div[contains(@class,"pane-read-error")]//button[contains(.,"填写密码")]'));if(passwordHelp.length)await click(passwordHelp[0]);else{const tools=await find('[data-testid="archive-pane-tools"]');if(await tools.getAttribute('aria-expanded')!=='true')await click(tools)}
  const input=await find('.file-manager .browser-toolbar input[type="password"]');if(await input.getAttribute('value')!==manifest.password){await fill('.file-manager .browser-toolbar input[type="password"]',manifest.password);await click('.file-manager .browser-toolbar button[title="重新读取"]')}
  const select=await find('[data-testid="zpaq-snapshot-select"]');await driver.actions().click(select).sendKeys(Key.END,Key.ENTER).perform()
  const d=String(manifest.cases[3].snapshots[0].date),date=`${d.slice(0,4)}-${d.slice(4,6)}-${d.slice(6,8)} ${d.slice(8,10)}:${d.slice(10,12)}:${d.slice(12)} UTC`
  await driver.wait(async()=>await(await find('[data-testid="zpaq-snapshot-select"]')).getAttribute('value')==='1'&&(await(await find('[data-testid="zpaq-snapshot-picker"] > span:first-of-type')).getText()).includes(date),30000,'Wait for committed snapshot metadata, not only the native select value')
  await click('[data-testid="zpaq-diagnose-current"]');await driver.wait(async()=>(await(await find('[data-testid="diagnostic-report"] h2')).getText()).includes('快照 1 检查通过'),30000)
  await click('[data-testid="zpaq-block-details-open"]');const rows=await driver.findElements(By.css('[data-testid="zpaq-block-list"] [role="listitem"]'));assert.equal(rows.length,Object.keys(manifest.cases[3].snapshots[0].expected).length);for(const row of rows)assert.equal(await row.getAttribute('data-state'),'verified');await closeDetails();done('older-snapshot-browser-entry',{snapshot:1,doesNotClaimWholeArchiveHealthy:true})
 }
 if(!passed('interrupted-tail-browser-recovery-entry')){
  const folder=path.join(state.work,'interrupted-tail');mkdirSync(folder,{recursive:true});const file=path.join(folder,'backup.zpaq');copyFileSync(path.join(root,'tests/fixtures/archives/zpaq-interrupted-162/plain-single/backup.zpaq'),file)
  await choose(file);await driver.wait(async()=>(await(await find('[data-testid="diagnostic-report"] h2')).getText()).includes('尾部更新中断'),30000);assert.equal((await driver.findElements(By.css('[data-testid="zpaq-block-summary"]'))).length,0);await click('[data-testid="zpaq-diagnosis-browse"]');await find('[data-testid="zpaq-interrupted-tail"]');done('interrupted-tail-browser-recovery-entry',{noEntryInference:true,noAutomaticRecovery:true})
 }
 for(let i=0;i<manifest.cases.length;i++)for(let j=0;j<state.parts[i].length;j++)assert.equal(sha(state.parts[i][j]),manifest.cases[i].parts[j].sha256)
 state.completedAt=new Date().toISOString();save()
}
