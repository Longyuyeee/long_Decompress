import {expect,test} from '@playwright/test'
for(const viewport of [{width:1440,height:900},{width:1024,height:720},{width:920,height:620}]){
 test(`clipboard1740 compact dual pane ${viewport.width}`,async({page},info)=>{
  test.setTimeout(90000)
  await page.setViewportSize(viewport)
  await page.addInitScript(()=>{
   const win=window as any;win.__copyCalls=[]
   win.__TAURI_IPC__=(message:any)=>{
    let result:any
    if(message.cmd==='load_app_settings')result=JSON.stringify({autoCheckUpdates:false,theme:'dark'})
    else if(message.cmd==='file_manager_locations')result=[{name:'资料',kind:'home',path:'C:/资料'},{name:'C:',kind:'drive',path:'C:/'},{name:'D:',kind:'drive',path:'D:/'}]
    else if(message.cmd==='list_files')result=[{name:'项目资料.zip',path:'C:/资料/项目资料.zip',isDir:false,size:1600,modified:1730000000000},{name:'目标文件.txt',path:'D:/目标文件.txt',isDir:false,size:10,modified:1730000000000}]
    else if(message.cmd==='get_archive_engine_capabilities')result={available:true,fullEngine:true,formats:[],browseExtensions:['zip'],nestedExtensions:['zip'],boundedPreviewFormats:['ZIP'],boundedTextPreviewFormats:['ZIP'],imagePreviewExtensions:['png'],textPreviewExtensions:['txt'],message:'ready'}
    else if(message.cmd==='browse_archive')result={format:'ZIP',revision:'r1740',totalFiles:2,totalDirectories:2,totalUncompressedSize:30,entries:[{path:'资料/说明.txt',name:'说明.txt',size:10,isDir:false},{path:'照片.png',name:'照片.png',size:20,isDir:false},{path:'空目录/',name:'空目录',size:0,isDir:true}]}
    else if(message.cmd==='copy_archive_entries'){win.__copyCalls.push(message);win.__copyPending=message;return}
    else if(message.cmd==='cancel_compression'){const pending=win.__copyPending;win[`_${pending.error}`]?.('复制已取消');win.__copyPending=null}
    else if(message.cmd==='take_pending_context_actions'||message.cmd==='load_history')result=[]
    else if(message.cmd==='is_context_menu_registered')result=true
    else if(message.cmd==='tauri'&&message.__tauriModule==='App')result='1.7.40'
    win[`_${message.callback}`]?.(result)
   }
  })
  await page.goto('/#/browser',{waitUntil:'domcontentloaded',timeout:60000});await expect(page.getByTestId('dual-pane-file-manager')).toBeVisible()
  await page.evaluate(async()=>{const path='/src/stores/app.ts';(await import(path)).useAppStore().openArchiveInBrowser('C:/资料/项目资料.zip')})
  const pane=page.locator('[data-file-pane="left"]');await expect(pane.locator('[data-entry-path="资料/"]')).toBeVisible()
  const before=await page.locator('.file-pane').evaluateAll(nodes=>nodes.map(node=>node.getBoundingClientRect().width));expect(Math.abs(before[0]-before[1])).toBeLessThan(3)
  await pane.locator('[data-entry-path="资料/"]').click();await pane.locator('[data-entry-path="空目录/"]').click({modifiers:['Control']})
  await pane.locator('[data-entry-path="资料/"]').click({button:'right'})
  const menu=page.getByTestId('archive-context-menu');await expect(menu).toBeVisible();await expect(menu).toContainText('2 个已选项目');await expect(menu.getByTestId('archive-context-copy-files')).toContainText('Ctrl+C')
  const box=await menu.boundingBox();expect(box!.x+box!.width).toBeLessThanOrEqual(viewport.width);expect(box!.y+box!.height).toBeLessThanOrEqual(viewport.height)
  await page.screenshot({path:info.outputPath('copy-menu.png')});await menu.getByTestId('archive-context-copy-files').click()
  await expect(pane.getByTestId('archive-copy-status')).toBeVisible();await expect(page.locator('.file-pane')).toHaveCount(2)
  expect(await page.evaluate(()=>(window as any).__copyCalls[0].entryPaths)).toEqual(['资料/','空目录/'])
  const overflow=await page.locator('.file-pane').evaluateAll(nodes=>nodes.map(el=>el.scrollWidth-el.clientWidth));expect(overflow.every(value=>value<=2)).toBe(true)
  await page.screenshot({path:info.outputPath('copy-progress.png')});await pane.getByTestId('archive-copy-status').getByRole('button',{name:'取消'}).click();await expect(pane.getByTestId('archive-copy-status')).toHaveCount(0)
  await pane.locator('[data-entry-path="资料/"]').press('Control+c');await expect(pane.getByTestId('archive-copy-status')).toBeVisible()
  await page.evaluate(()=>{const win=window as any;win[`_${win.__copyPending.callback}`](2)})
  await expect(pane.getByTestId('archive-copy-status')).toHaveCount(0);await expect(pane.locator('[data-entry-path="资料/"]')).toHaveClass(/selected/)
 })
}
