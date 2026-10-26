import { beforeEach, expect, it, vi } from 'vitest'
import { flushPromises, mount } from '@vue/test-utils'
import { createPinia, setActivePinia } from 'pinia'
import ArchiveOpenDialog from '../ArchiveOpenDialog.vue'
import { useAppStore } from '@/stores/app'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),open:vi.fn(),push:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/dialog',()=>({open:mocks.open}))
vi.mock('vue-router',()=>({useRouter:()=>({push:mocks.push})}))
const setup=async()=>{
 const pinia=createPinia();setActivePinia(pinia)
 const app=useAppStore();await flushPromises()
 app.enqueueArchiveOpen(['C:/Downloads/资料.zip'])
 const view=mount(ArchiveOpenDialog,{global:{plugins:[pinia],stubs:{Teleport:true}}});await flushPromises()
 return {app,view}
}
beforeEach(()=>{
 localStorage.clear();vi.clearAllMocks();mocks.open.mockResolvedValue(null);mocks.push.mockResolvedValue(undefined)
 mocks.invoke.mockImplementation(async(command:string)=>command==='load_app_settings'?'{}':command==='preview_extraction_output'?'C:/Downloads/资料':command==='is_context_menu_registered'?true:undefined)
})
it.each([['here',false],['subfolder',true]] as const)('asks once and queues explicit %s extraction using the chosen directory policy',async(action,subfolder)=>{
 const {app,view}=await setup()
 expect(view.findAll('.open-choice')).toHaveLength(4)
 expect(app.pendingContextActions).toHaveLength(0)
 await view.get(`[data-testid="archive-open-${action}"]`).trigger('click');await flushPromises()
 expect(app.pendingContextActions[0]).toMatchObject({files:['C:/Downloads/资料.zip'],extractToSubfolder:subfolder,openResultFolder:false})
 expect(mocks.push).toHaveBeenCalledWith('/decompress');expect(app.settings.archiveOpenBehavior).toBe('ask');view.unmount()
})
it('chooses a custom directory, remembers this action, and keeps the chosen directory exact for one archive',async()=>{
 const {app,view}=await setup();mocks.open.mockResolvedValueOnce('E:/自选目录')
 await view.get('[data-testid="archive-open-remember"]').setValue(true)
 await view.get('[data-testid="archive-open-choose-folder"]').trigger('click');await flushPromises()
 expect(app.pendingContextActions[0]).toMatchObject({outputPath:'E:/自选目录',extractToSubfolder:false})
 expect(app.settings.archiveOpenBehavior).toBe('choose-folder')
 expect(JSON.parse(localStorage.getItem('app-settings')!).archiveOpenBehavior).toBe('choose-folder');view.unmount()
})
it('cancelling the directory picker leaves the ask dialog available and starts nothing',async()=>{
 const {app,view}=await setup();await view.get('[data-testid="archive-open-choose-folder"]').trigger('click');await flushPromises()
 expect(app.pendingContextActions).toHaveLength(0);expect(app.pendingArchiveOpenFiles).toHaveLength(1)
 expect(view.find('[data-testid="archive-open-dialog"]').exists()).toBe(true);expect(mocks.push).not.toHaveBeenCalled();view.unmount()
})
it('a saved folder-picker action opens once; cancelling does not repeatedly reopen it',async()=>{
 const {app,view}=await setup();app.updateSettings({archiveOpenBehavior:'choose-folder'});await flushPromises()
 expect(mocks.open).toHaveBeenCalledTimes(1);expect(app.pendingArchiveOpenFiles).toHaveLength(0)
 expect(app.pendingContextActions).toHaveLength(0);expect(app.settings.archiveOpenBehavior).toBe('choose-folder');view.unmount()
})
it('a saved browse action enters the dual-pane browser without asking or extracting',async()=>{
 const {app,view}=await setup();app.updateSettings({archiveOpenBehavior:'browse'});await flushPromises()
 expect(app.pendingArchiveBrowserPath).toBe('C:/Downloads/资料.zip');expect(mocks.push).toHaveBeenCalledWith('/browser')
 expect(app.pendingContextActions).toHaveLength(0);expect(mocks.open).not.toHaveBeenCalled();view.unmount()
})
it('groups batch extraction under the chosen directory while allowing a single shared operation choice',async()=>{
 const {app,view}=await setup();app.enqueueArchiveOpen(['D:/Other/照片.zip']);mocks.open.mockResolvedValueOnce('E:/取回')
 await view.get('[data-testid="archive-open-choose-folder"]').trigger('click');await flushPromises()
 expect(app.pendingContextActions[0]).toMatchObject({files:['C:/Downloads/资料.zip','D:/Other/照片.zip'],outputPath:'E:/取回',extractToSubfolder:true});view.unmount()
})
