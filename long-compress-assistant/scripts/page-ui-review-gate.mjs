import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

// UI-only sweep. Operations are staged in an isolated profile; no archive matrix is repeated.
export async function runPageUiReview({driver,bridge,application,root}) {
 const base=path.join(root,'test-results/page-ui-review');mkdirSync(base,{recursive:true})
 const reviewContent=process.argv.includes('--page-ui-review-content-only')
 const reviewFinal=process.argv.includes('--page-ui-review-final')
 const binarySha256=createHash('sha256').update(readFileSync(application)).digest('hex'), receipt=path.join(base,reviewContent?'content-result.json':reviewFinal?'final-result.json':'result.json')
 const report=existsSync(receipt)?JSON.parse(readFileSync(receipt,'utf8')):{binarySha256,checks:[],layouts:[],screenshots:[]}
 assert.equal(report.binarySha256,binarySha256,'Retain a separate receipt per candidate')
 const save=()=>writeFileSync(receipt,JSON.stringify(report,null,2)+'\n')
 const done=name=>{report.checks.push(name);save();console.log('[page-ui] '+name)}
 const passed=name=>reviewFinal&&(reviewContent||name!=='special-compression-three-modes-and-settings')||report.checks.includes(name)
 const find=selector=>driver.wait(async()=>{try{for(const e of await driver.findElements(By.css(selector)))if(await e.isDisplayed())return e;return false}catch(e){if(e.name==='StaleElementReferenceError')return false;throw e}},15000,'Missing '+selector)
 async function click(selector,twice=false){const e=await find(selector);await driver.executeScript(e=>e.scrollIntoView({block:'center',behavior:'instant'}),e);await driver.wait(async()=>await e.isEnabled()&&await driver.executeScript(e=>{const r=e.getBoundingClientRect(),hit=document.elementFromPoint(r.x+r.width/2,r.y+r.height/2);return e===hit||e.contains(hit)},e),10000,'Obscured '+selector);if(twice)await driver.actions().doubleClick(e).perform();else await e.click()}
 async function fill(selector,value,enter=false){const e=await find(selector);await e.sendKeys(Key.chord(Key.CONTROL,'a'),Key.BACK_SPACE,value);if(enter)await e.sendKeys(Key.ENTER)}
 const nav=async name=>{await driver.manage().window().setRect({width:1100,height:700});await click(`[data-testid="nav-${name}"]`)}
 const close=()=>click('[role="dialog"] .app-dialog > button')
 async function shot(name,anchor,sizes=[[1100,700],[760,520]]) {
  for(const [width,height]of sizes){if(report.layouts.some(r=>r.name===name&&r.width===width))continue
   const current=await driver.manage().window().getRect();if(current.width!==width||current.height!==height)await driver.manage().window().setRect({width,height});const e=await find(anchor)
   await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
   const layout=await driver.executeScript(anchor=>{
    const e=document.querySelector(anchor),main=document.querySelector('.app-main'),nav=document.querySelector('.app-sidebar>nav'),modal=[...document.querySelectorAll('[role="dialog"]')].filter(e=>e.getClientRects().length).at(-1),card=modal?.querySelector('.app-dialog')||modal,r=card?.getBoundingClientRect()
    const root=main.querySelector('.settings-view,.module-page,.history-view,.password-vault,.integrity-view,.special-compression-view,.file-manager,.compression-view,.decompress-view')
    return {width:innerWidth,height:innerHeight,documentOverflow:document.documentElement.scrollWidth-innerWidth,pageOverflow:root?root.scrollWidth-root.clientWidth:0,headerHeight:root?.querySelector('header')?.getBoundingClientRect().height,navScrollbar:getComputedStyle(nav).scrollbarWidth,anchorHeight:e?.getBoundingClientRect().height,modal:r?{x:r.x,y:r.y,width:r.width,height:r.height,overflow:card.scrollWidth-card.clientWidth}:null}
   },anchor)
   assert.ok(layout.documentOverflow<=1&&layout.pageOverflow<=1,JSON.stringify({name,...layout}));assert.equal(layout.navScrollbar,'none')
   if(layout.modal)assert.ok(layout.modal.x>=0&&layout.modal.y>=0&&layout.modal.x+layout.modal.width<=layout.width+1&&layout.modal.y+layout.modal.height<=layout.height+1&&layout.modal.overflow<=1,JSON.stringify({name,...layout}))
   const file=`${reviewContent?'content-':reviewFinal?'final-':''}${name}-${width}x${height}.png`;writeFileSync(path.join(base,file),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});report.screenshots.push(file);save()
  }
  await driver.manage().window().setRect({width:1100,height:700})
 }
 const fixtures=path.join(root,'test-results/compact-modules-ui/fixtures'), archive=path.join(fixtures,'项目资料-交付归档.zip'), folder=path.join(fixtures,'项目资料（本地文件夹）')
 await driver.manage().setTimeouts({script:120000})
 if(reviewFinal&&(await driver.findElements(By.css('[data-testid="pending-draft-recovery"] button'))).length>1)await click('[data-testid="pending-draft-recovery"] button:nth-of-type(2)')
 if(!passed('settings-six-categories-and-hover-help')){
  await nav('Settings');await click('.settings-theme-options button:nth-child(2)');await click('.settings-accent-options button[title="emerald"]')
  for(const section of ['appearance','accessibility','processing','behavior','updates','formats']){await click(`[data-settings-section="${section}"]`);await shot('settings-'+section,`[data-settings-panel="${section}"]`)}
  await click('[data-settings-section="appearance"]');await driver.manage().window().setRect({width:760,height:520});const help=await find('[data-settings-panel="appearance"] .help-trigger');await driver.executeScript(e=>e.scrollIntoView({block:'nearest',behavior:'instant'}),help);await driver.actions().move({origin:await find('.workspace-heading h1')}).move({origin:help,duration:200}).pause(300).perform();await find('[role="note"]');await shot('hover-help','[role="note"]',[[760,520]])
  await driver.actions().move({origin:await find('.workspace-heading h1')}).perform();await driver.wait(async()=>!(await driver.findElements(By.css('[role="note"]'))).length,10000)
  done('settings-six-categories-and-hover-help')
 }
 if(!passed('decompression-inline-and-compression-configuration')){
  await nav('Decompress');await shot('decompression','.decompress-view')
  if(!(await driver.findElements(By.css('[data-task-id]'))).length){await bridge('queueDesktopDialogSelections',[[archive]]);await click('.decompress-view [data-testid="dropzone-file"]');await find('[data-task-id]')}
  await shot('decompression-tasks','[data-testid="decompression-workspace-shell"]')
  await nav('Compress');await shot('compression','.compression-view')
  await click('[data-testid="open-global-compression-settings"]');await shot('compression-settings','[role="dialog"] .app-dialog');await close()
  await click('[data-testid="open-archive-conversion"]');await shot('conversion-dialog','[data-testid="archive-conversion"]');await close()
  done('decompression-inline-and-compression-configuration')
 }
 if(!process.argv.includes('--page-ui-review-rest')&&!passed('special-compression-three-modes-and-settings')){
  await nav('SpecialCompression')
  for(const mode of ['image','video','pdf']){
   await click(`[data-testid="compression-mode-${mode}"]`);await find(`[data-testid="${mode}-compression-workspace"]`)
   assert.equal((await driver.findElements(By.css(`#special-workspace-actions [data-testid="${mode==='pdf'?'pdf-toggle-batch-settings':mode+'-toggle-global-settings'}"]`))).length,1,'Actions belong in page header')
   await shot('special-'+mode,`[data-testid="${mode}-compression-workspace"]`)
   await click(`[data-testid="${mode==='pdf'?'pdf-toggle-batch-settings':mode+'-toggle-global-settings'}"]`);await shot(mode+'-settings','[role="dialog"] .app-dialog',[[760,520]]);await close()
  }
  done('special-compression-three-modes-and-settings')
 }
 if(!passed('browser-archive-and-preview-with-compact-header')){
  await nav('ArchiveBrowser');await fill('[data-testid="file-manager-path-input-left"]',folder,true);await find('[data-file-pane="left"] .file-row')
  await bridge('queueDesktopDialogSelections',[archive]);await click('[data-testid="file-manager-open-archive"]');await find('[data-testid="archive-pane-browser"] .browser-row')
  await shot('dual-pane-archive','.file-manager');done('browser-archive-and-preview-with-compact-header')
 }
 if(!passed('comparison-sources-and-results')){
  await nav('SourceComparison');await bridge('queueDesktopDialogSelections',[folder]);await click('[data-comparison-folder="left"]');await driver.wait(()=>driver.executeScript(()=>/\d+ 项/.test(document.querySelector('[data-comparison-source="left"]')?.textContent||'')),15000)
  await bridge('queueDesktopDialogSelections',[archive]);await click('[data-comparison-zip="right"]');await find('[data-testid="comparison-results"]');await shot('comparison-results','[data-testid="comparison-results"]');done('comparison-sources-and-results')
 }
 if(!passed('workflow-editor-results-automation-and-rule-dialogs')){
  await nav('Workflows');await click('[data-testid="workflow-edit-tab"]');await shot('workflow-editor','[data-testid="workflow-plan-name"]')
  await click('[data-testid="workflow-new-open"]');await shot('workflow-templates','[data-testid="workflow-templates"]',[[760,520]]);await close()
  await click('[data-testid="workflow-runs-tab"]');await shot('workflow-results','[data-testid="workflow-run-details"]')
  await click('[data-testid="workflow-directory-tab"]');await shot('workflow-automation','[data-testid="workflow-automation-panel"]')
  await click('[data-testid="workflow-schedule-new"]');await shot('schedule-dialog','[role="dialog"] .app-dialog',[[760,520]]);await close()
  await click('[data-testid="workflow-rule-new"]');await shot('directory-rule-dialog','[data-testid="workflow-rule-dialog"]',[[760,520]]);await close()
  done('workflow-editor-results-automation-and-rule-dialogs')
 }
 if(!passed('library-search-sources-and-retrieval')){
  await nav('ArchiveLibrary');await click('[data-testid="library-search-tab"]');await fill('[data-testid="library-query"]','项目说明');await find('.library-hit');await shot('library-search','[data-testid="library-query"]')
  await click('[data-testid="library-select-page"]');await click('[data-testid="library-batch-retrieval"]');await shot('library-retrieval','[data-testid="library-retrieval"]',[[760,520]]);await close()
  await click('[data-testid="library-sources-tab"]');await shot('library-sources','.source-summary');done('library-search-sources-and-retrieval')
 }
 if(!passed('vault-list-statistics-and-password-form')){
  await nav('Vault');await shot('vault-list','.password-vault');await click('[data-testid="vault-analytics-trigger"]');await shot('vault-statistics','[data-testid="vault-analytics-modal"]',[[760,520]]);await click('[data-testid="vault-analytics-modal"] header button')
  await click('.password-vault header button[aria-label="添加"]');await shot('password-entry','[role="dialog"] .app-dialog',[[760,520]]);await close();done('vault-list-statistics-and-password-form')
 }
 if(!passed('integrity-three-modes-and-history')){
  await nav('FileIntegrity');await shot('integrity-calculate','.integrity-calculate-grid');await click('[data-testid="checksum-verify-mode"]');await shot('integrity-verify','[data-testid="checksum-choose"]')
  await click('[data-testid="archive-diagnostic-mode"]');await shot('integrity-archive','[data-testid="archive-diagnostic-panel"]')
  await nav('History');await shot('history-list','.history-view');await click('[data-testid="history-open-statistics"]');await shot('history-statistics','[data-testid="history-statistics-content"]',[[760,520]]);await close();done('integrity-three-modes-and-history')
 }

 if(reviewFinal&&!reviewContent&&!report.checks.includes('final-affected-copy-and-main-content-layouts')){
  await nav('Settings');await click('[data-settings-section="processing"]');await shot('settings-processing','[data-settings-panel="processing"]')
  await nav('Compress');await click('[data-testid="open-global-compression-settings"]');await shot('compression-settings','[role="dialog"] .app-dialog');await close()
  await click('[data-testid="open-archive-conversion"]');await shot('conversion-dialog','[data-testid="archive-conversion"]',[[760,520]]);await close()
  await nav('Decompress');if(!(await driver.findElements(By.css('[data-task-id]'))).length){await bridge('queueDesktopDialogSelections',[[archive]]);await click('.decompress-view [data-testid="dropzone-file"]');await find('[data-task-id]')};await shot('decompression-tasks','[data-testid="decompression-workspace-shell"]')
  await nav('Workflows');await click('[data-testid="workflow-edit-tab"]');assert.equal((await driver.findElements(By.css('.workspace-heading .help-trigger'))).length,1);await shot('workflow-editor','[data-testid="workflow-plan-name"]')
  await nav('ArchiveLibrary');await click('[data-testid="library-search-tab"]');await fill('[data-testid="library-query"]','项目说明');await find('.library-hit');await shot('library-search','[data-testid="library-query"]')
  await nav('FileIntegrity');await click('.integrity-mode-switch button:first-child');await bridge('queueDesktopDialogSelections',[[path.join(folder,'文档/项目说明.txt'),path.join(folder,'文档/合同草稿-需要核对.txt')]]);await click('[data-testid="checksum-calculate-choose"]');await click('[data-testid="checksum-calculate-start"]');await find('[data-testid="checksum-calculation-entry"][data-status="success"]');await shot('integrity-results','.result-panel')
  done('final-affected-copy-and-main-content-layouts')
 }

 if(reviewContent&&!report.checks.includes('compact-populated-results-and-library-heading')){
  await nav('ArchiveLibrary');await click('[data-testid="library-search-tab"]');await fill('[data-testid="library-query"]','项目说明');await find('.library-hit');await shot('library-search','[data-testid="library-query"]')
  await nav('FileIntegrity');await click('.integrity-mode-switch button:first-child');await bridge('queueDesktopDialogSelections',[[path.join(folder,'文档/项目说明.txt'),path.join(folder,'文档/合同草稿-需要核对.txt')]]);await click('[data-testid="checksum-calculate-choose"]');await click('[data-testid="checksum-calculate-start"]');await find('[data-testid="checksum-calculation-entry"][data-status="success"]');await driver.wait(async()=>!(await driver.findElements(By.css('.toast-container>*'))).length,6500).catch(()=>{});await shot('integrity-results','.result-panel')
  await nav('History');await shot('history-compact-list','.history-view');const more=await find('[data-testid="history-record-row"] [aria-label="更多结果操作"]');await more.click();await find('[data-testid="history-record-row"] [data-testid="result-generate-checksums"]');await shot('history-more-actions','[data-testid="history-record-row"]',[[760,520]])
  await nav('SourceComparison');await bridge('queueDesktopDialogSelections',[folder]);await click('[data-comparison-folder="left"]');await driver.wait(()=>driver.executeScript(()=>/\d+ 项/.test(document.querySelector('[data-comparison-source="left"]')?.textContent||'')),15000);await bridge('queueDesktopDialogSelections',[archive]);await click('[data-comparison-zip="right"]');await find('[data-testid="comparison-results"]');await shot('comparison-compact-results','[data-testid="comparison-results"]')
  await nav('Workflows');await click('[data-testid="workflow-directory-tab"]');await click('[data-testid="workflow-schedule-new"]');await shot('schedule-title-help','[role="dialog"] .app-dialog',[[760,520]]);await close()
  await nav('ArchiveLibrary');await click('[data-testid="library-search-tab"]');await fill('[data-testid="library-query"]','项目说明');await find('.library-hit');await click('[data-testid="library-select-page"]');await click('[data-testid="library-batch-retrieval"]');await shot('retrieval-heading-help','[data-testid="library-retrieval"]',[[760,520]]);await close()
  done('compact-populated-results-and-library-heading')
 }
 report.completedAt=new Date().toISOString();save();console.log(JSON.stringify({checks:report.checks.length,layouts:report.layouts.length,binarySha256}))
}
