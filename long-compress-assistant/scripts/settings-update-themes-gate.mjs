import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {existsSync,mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import path from 'node:path'
import {By} from 'selenium-webdriver'

export async function runSettingsUpdateThemesGate({driver,bridge,application,root,forwardContextAction}) {
 const base=path.join(root,'test-results/settings-update-themes');mkdirSync(base,{recursive:true})
 const finalOnly=process.argv.includes('--settings-update-themes-final')
 const report={startedAt:new Date().toISOString(),binarySha256:createHash('sha256').update(readFileSync(application)).digest('hex'),checks:[],layouts:[],screenshots:[],updateTransport:'Synthetic chunks through real Tauri event channel; installer not launched'}
 const save=()=>writeFileSync(path.join(base,finalOnly?'final-result.json':'result.json'),JSON.stringify(report,null,2))
 async function find(selector){return driver.wait(async()=>{for(const e of await driver.findElements(By.css(selector)))try{if(await e.isDisplayed())return e}catch{}return false},15000,selector)}
 async function click(selector){const e=await find(selector);await driver.executeScript('arguments[0].scrollIntoView({block:"center",inline:"nearest",behavior:"instant"})',e);await driver.wait(async()=>await e.isEnabled()&&await driver.executeScript('const r=arguments[0].getBoundingClientRect();return arguments[0].contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))',e),10000);await e.click()}
 async function shot(name,width=1100,height=700){await driver.manage().window().setRect({width,height});await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
  const layout=await driver.executeScript(()=>{const page=document.querySelector('.settings-view'),modal=[...document.querySelectorAll('[role="dialog"] .app-dialog')].filter(e=>e.getClientRects().length).at(-1),r=modal?.getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,pageOverflow:page? page.scrollWidth-page.clientWidth:0,modal:r?{x:r.x,y:r.y,width:r.width,height:r.height,overflow:modal.scrollWidth-modal.clientWidth}:null}})
  assert.ok(layout.overflow<=1&&layout.pageOverflow<=1,JSON.stringify(layout));if(layout.modal){const r=layout.modal;assert.ok(r.x>=0&&r.y>=0&&r.x+r.width<=layout.width+1&&r.y+r.height<=layout.height+1&&r.overflow<=1,JSON.stringify(layout))}
  const file=`${name}-${width}x${height}.png`;writeFileSync(path.join(base,file),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});report.screenshots.push(file);save()
 }
 await driver.manage().setTimeouts({script:120000})
 await bridge('reset')
 await driver.manage().window().setRect({width:1100,height:700})
 await click('[data-testid="nav-Settings"]')
 if(finalOnly){
  report.previousFunctionalEvidence='result.json (earlier build; themes, download event transport, actual X hide/restore passed before the cancel selector timed out)'
  await click('[data-theme="obsidian"]');await shot('final-theme-obsidian');await shot('final-themes-narrow',760,520)
  await driver.manage().window().setRect({width:1100,height:700});await click('[data-settings-section="formats"]')
  await shot('final-format-counts');await shot('final-format-counts-narrow',760,520)
  await click('.settings-format-details:first-of-type>summary');await find('[data-testid="settings-readable-extensions"]')
  assert.ok((await (await find('[data-testid="settings-readable-extensions"]')).getText()).includes('ZIP'))
  await click('[data-settings-section="behavior"]');await bridge('seedActiveTask')
  const toggle=await find('[data-testid="close-to-tray-switch"]');if(await toggle.getAttribute('aria-checked')==='true')await toggle.click()
  await driver.wait(async()=>!(await bridge('desktopBehaviorState')).close_to_tray,10000)
  await click('.window-titlebar button[aria-label="关闭"]');await find('[role="dialog"] .app-dialog')
  assert.equal(await bridge('isWindowVisible'),true);assert.match(await (await find('[role="dialog"]')).getText(),/后台/)
  await click('[role="dialog"] .app-dialog>div:last-child button:first-child');await bridge('clearTasks');await click('[data-testid="close-to-tray-switch"]')
  report.checks.push('final-theme-icon-format-disclosures-and-legible-count-labels','tray-disabled-close-keeps-existing-active-task-confirmation')
  report.completedAt=new Date().toISOString();save();console.log(JSON.stringify({layouts:report.layouts.length,binarySha256:report.binarySha256}));return
 }
 for(const mode of ['glacier','forest','nebula','obsidian']){
  await click(`[data-theme="${mode}"]`)
  const state=await driver.executeScript(()=>({theme:localStorage.getItem('app-theme'),settings:JSON.parse(localStorage.getItem('app-settings')),root:document.documentElement.className,scheme:document.documentElement.dataset.colorScheme,background:getComputedStyle(document.documentElement).getPropertyValue('--bg-main'),texture:getComputedStyle(document.querySelector('.app-main')).backgroundImage}))
  assert.equal(state.theme,mode);assert.equal(state.settings.theme,mode);assert.ok(state.root.includes(`mode-${mode}`));assert.notEqual(state.texture,'none');report.checks.push({theme:mode,...state,settings:{theme:state.settings.theme,accentColor:state.settings.accentColor}})
  await shot(`theme-${mode}`)
 }
 await shot('themes-narrow',760,520)
 await driver.manage().window().setRect({width:1100,height:700})
 await click('[data-settings-section="formats"]');await find('[data-testid="settings-extraction-count"]')
 const counts=await driver.executeScript(()=>({readable:Number(document.querySelector('[data-testid="settings-extraction-count"]').textContent),create:Number(document.querySelector('[data-testid="settings-creation-count"]').textContent),password:Number(document.querySelector('[data-testid="settings-password-count"]').textContent),extensions:document.querySelector('[data-testid="settings-readable-extensions"]').textContent.trim().split(' · '),outputs:document.querySelector('[data-testid="settings-creation-formats"]').textContent.trim().split(' · '),passwords:document.querySelector('[data-testid="settings-password-extraction-formats"]').textContent.trim().split(' · ')}))
 assert.equal(counts.readable,counts.extensions.length);assert.equal(counts.create,counts.outputs.length);assert.equal(counts.password,counts.passwords.length);assert.equal(new Set(counts.extensions).size,counts.readable);assert.ok(counts.readable>37&&counts.create>16&&counts.password>3);report.checks.push({formatCounts:counts})
 await shot('live-format-counts');await shot('live-format-counts-narrow',760,520)
 await bridge('previewUpdateDownload',26214400,104857600,true)
 await find('[data-testid="update-progress"][data-phase="downloading"]')
 await driver.wait(async()=>(await find('[role="progressbar"]')).getAttribute('aria-valuenow').then(v=>v==='25'),10000)
 await bridge('previewUpdateDownload',13107200,104857600)
 await driver.wait(async()=>(await find('[role="progressbar"]')).getAttribute('aria-valuenow').then(v=>v==='37'),10000)
 await shot('update-downloading');await shot('update-downloading-narrow',760,520)
 await bridge('previewUpdateDownload',4096,null,true)
 await driver.wait(async()=>(await find('[role="progressbar"]')).getAttribute('aria-valuenow').then(v=>v===null),10000)
 await shot('update-unknown-length',760,520)
 await bridge('previewUpdateStatus','DOWNLOADED');await find('[data-testid="update-progress"][data-phase="installing"]');await shot('update-installing',760,520)
 await bridge('previewUpdateStatus','ERROR');await find('[role="dialog"] button');assert.match(await (await find('[role="dialog"]')).getText(),/下载中断/)
 await bridge('previewUpdateStatus','RESET');report.checks.push('native-update-channel-known-unknown-length-install-phase-error')
 await driver.manage().window().setRect({width:1100,height:700});await click('[data-settings-section="behavior"]')
 const toggle=await find('[data-testid="close-to-tray-switch"]');if(await toggle.getAttribute('aria-checked')!=='true')await toggle.click()
 await driver.wait(async()=>Boolean((await bridge('desktopBehaviorState')).close_to_tray),10000)
 // Click the actual title bar X. The old appWindow.close path destroyed this WebView.
 await click('.window-titlebar button[aria-label="关闭"]')
 await driver.wait(async()=>!(await bridge('isWindowVisible')),10000)
 const marker=path.join(base,'restored.marker');forwardContextAction('--desktop-e2e-restore',[marker])
 await driver.wait(()=>existsSync(marker)&&readFileSync(marker,'utf8')==='visible',15000)
 assert.equal(await bridge('isWindowVisible'),true)
 assert.equal(await (await find('[data-testid="close-to-tray-switch"]')).getAttribute('aria-checked'),'true')
 report.checks.push('actual-titlebar-x-hides-with-live-webview-and-native-restore')
 await shot('tray-restored')
 await bridge('seedActiveTask');await click('[data-testid="close-to-tray-switch"]')
 await driver.wait(async()=>!(await bridge('desktopBehaviorState')).close_to_tray,10000)
 await click('.window-titlebar button[aria-label="关闭"]');await find('[role="dialog"] .app-dialog')
 assert.equal(await bridge('isWindowVisible'),true);assert.match(await (await find('[role="dialog"]')).getText(),/后台/)
 await click('[role="dialog"] .app-dialog>div:last-child button:first-child');await bridge('clearTasks');await click('[data-testid="close-to-tray-switch"]')
 report.checks.push('tray-disabled-close-keeps-existing-active-task-confirmation')
 report.completedAt=new Date().toISOString();save();console.log(JSON.stringify({formatCounts:{readable:counts.readable,create:counts.create,password:counts.password},layouts:report.layouts.length,binarySha256:report.binarySha256}))
}
