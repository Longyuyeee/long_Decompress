import assert from 'node:assert/strict'
import {createHash} from 'node:crypto'
import {mkdirSync,readFileSync,writeFileSync} from 'node:fs'
import path from 'node:path'
import {By,Key} from 'selenium-webdriver'

// Final presentation/keyboard pass only. Previously passed archive matrices are reused.
export async function runReleaseUi160Gate({driver,bridge,application,root}) {
 const base=path.join(root,'test-results/release-ui-1.6.0');mkdirSync(base,{recursive:true})
 const report={startedAt:new Date().toISOString(),binarySha256:createHash('sha256').update(readFileSync(application)).digest('hex'),checks:[],layouts:[],screenshots:[]}
 const save=()=>writeFileSync(path.join(base,'result.json'),JSON.stringify(report,null,2))
 async function find(s){return driver.wait(async()=>{for(const e of await driver.findElements(By.css(s)))try{if(await e.isDisplayed())return e}catch{}return false},15000,s)}
 async function click(s){const e=await find(s);await driver.executeScript('arguments[0].scrollIntoView({block:"center",inline:"nearest",behavior:"instant"})',e);await driver.wait(()=>driver.executeScript('const r=arguments[0].getBoundingClientRect();return arguments[0].contains(document.elementFromPoint(r.x+r.width/2,r.y+r.height/2))',e),10000);await e.click()}
 async function nav(name){await driver.manage().window().setRect({width:1100,height:800});await click(`[data-testid="nav-${name}"]`);await driver.wait(()=>driver.executeScript(name=>document.querySelector(`[data-testid="nav-${name}"]`)?.getAttribute('aria-current')==='page',name),10000)}
 async function shot(name,sizes=[[1100,800],[760,520]]){for(const [width,height]of sizes){await driver.manage().window().setRect({width,height});await driver.executeScript(()=>document.activeElement?.blur());await driver.wait(()=>driver.executeScript(()=>document.getAnimations().filter(a=>a.effect?.getTiming().iterations!==Infinity).every(a=>a.playState!=='running')),10000)
  const layout=await driver.executeScript(()=>{const page=document.querySelector('.app-main>div>div'),nav=document.querySelector('.app-sidebar>nav'),labels=[...document.querySelectorAll('.sidebar-nav-label')],dialog=[...document.querySelectorAll('[role="dialog"] .app-dialog')].find(e=>e.getClientRects().length),r=dialog?.getBoundingClientRect();return{width:innerWidth,height:innerHeight,overflow:document.documentElement.scrollWidth-innerWidth,pageOverflow:page?page.scrollWidth-page.clientWidth:0,navScrollbar:getComputedStyle(nav).scrollbarWidth,clippedLabels:labels.filter(e=>e.scrollWidth>e.clientWidth+1).map(e=>e.textContent),dialog:r?{x:r.x,y:r.y,width:r.width,height:r.height}:null}})
  assert.ok(layout.overflow<=1&&layout.pageOverflow<=1,JSON.stringify({name,...layout}));assert.deepEqual(layout.clippedLabels,[]);assert.equal(layout.navScrollbar,'none');if(layout.dialog){const r=layout.dialog;assert.ok(r.x>=0&&r.y>=0&&r.x+r.width<=layout.width+1&&r.y+r.height<=layout.height+1,JSON.stringify(layout))}
  const file=`${name}-${width}x${height}.png`;writeFileSync(path.join(base,file),Buffer.from(await driver.takeScreenshot(),'base64'));report.layouts.push({name,...layout});report.screenshots.push(file);save()
 }}
 await driver.manage().setTimeouts({script:120000});await bridge('reset');await nav('Settings');await click('[data-theme="obsidian"]')
 const expected={SourceComparison:['d','/compare','Ctrl+Shift+D'],ArchiveLibrary:['l','/library','Ctrl+Shift+L'],Workflows:['w','/workflows','Ctrl+Shift+W']}
 for(const [name,[key,route,shortcut]]of Object.entries(expected)){
  await nav('Settings');const entry=await find(`[data-testid="nav-${name}"] .sidebar-shortcut`);assert.equal(await entry.getText(),shortcut)
  await driver.executeScript(()=>document.activeElement?.blur());await driver.actions().keyDown(Key.CONTROL).keyDown(Key.SHIFT).sendKeys(key).keyUp(Key.SHIFT).keyUp(Key.CONTROL).perform()
  await driver.wait(()=>driver.executeScript(route=>location.hash.endsWith(route),route),10000);report.checks.push({shortcut,route})
 }
 await nav('ArchiveLibrary');await click('[data-testid="library-search-tab"]');await click('[data-testid="library-query"]')
 await driver.actions().keyDown(Key.CONTROL).keyDown(Key.SHIFT).sendKeys('d').keyUp(Key.SHIFT).keyUp(Key.CONTROL).perform();assert.equal(await driver.executeScript(()=>location.hash.endsWith('/library')),true);report.checks.push('shortcuts-do-not-navigate-while-typing')
 for(const name of ['Decompress','Compress','SpecialCompression','ArchiveBrowser','SourceComparison','Workflows','ArchiveLibrary','Vault','FileIntegrity','History','Settings']){
  await nav(name)
  if(name==='Decompress'||name==='Compress')await bridge('seedResponsiveWorkspace',name==='Compress'?'compression':'decompression','completed',true)
  if(name==='ArchiveBrowser'){const archive=path.join(root,'test-results/compact-modules-ui/fixtures/项目资料-交付归档.zip');await bridge('queueDesktopDialogSelections',[archive]);await click('[data-testid="file-manager-open-archive"]');await find('.file-manager .browser-page')}
  await shot(name)
 }
 assert.match(await (await find('[data-testid="sidebar-version-badge"]')).getText(),/1\.6\.0/)
 await shot('themes')
 await click('[data-settings-section="formats"]');await shot('format-counts');await click('.settings-format-details:last-of-type>summary');await find('[data-testid="settings-password-extraction-formats"]');await shot('password-formats',[[760,520]])
 await nav('SpecialCompression');for(const mode of ['video','pdf']){await click(`[data-testid="compression-mode-${mode}"]`);await find(`[data-testid="${mode}-compression-workspace"]`);await shot('special-'+mode)}
 await nav('Compress');await click('[data-testid="open-global-compression-settings"]');await shot('compression-options',[[760,520]]);await click('[role="dialog"] .app-dialog>button')
 await bridge('clearTasks');await bridge('previewUpdateDownload',50*1024*1024,100*1024*1024,true);await find('[data-testid="update-progress"]');await shot('update-progress',[[760,520]]);await bridge('previewUpdateStatus','RESET')
 report.checks.push('all-eleven-main-pages-and-media-modes-wide-narrow','password-format-disclosure-and-readable-text','shared-compression-options-and-update-progress','all-navigation-labels-readable-without-scrollbar')
 report.completedAt=new Date().toISOString();save();console.log(JSON.stringify({checks:report.checks.length,layouts:report.layouts.length,binarySha256:report.binarySha256}))
}
