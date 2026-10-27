import {it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import {createPinia} from 'pinia'
import UpdateProgress from '../UpdateProgress.vue'
import {useUpdateStore} from '@/stores/update'
const hooks=vi.hoisted(()=>({progress:null as null|((event:any)=>void)}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:vi.fn(async()=>'{}')}))
vi.mock('@tauri-apps/api/updater',()=>({onUpdaterEvent:vi.fn(async()=>vi.fn())}))
vi.mock('@tauri-apps/api/event',()=>({TauriEvent:{DOWNLOAD_PROGRESS:'tauri://update-download-progress'},listen:vi.fn(async(_name:string,callback:any)=>{hooks.progress=callback;return vi.fn()})}))
it('never renders downloaded bytes over an inconsistent total or a false completed percentage',async()=>{
 Object.defineProperty(window,'__TAURI_IPC__',{value:vi.fn(),configurable:true})
 const pinia=createPinia(),store=useUpdateStore(pinia);await store.initialize();store.$patch({status:'installing',installPhase:'preparing'})
 const wrapper=mount(UpdateProgress,{global:{plugins:[pinia]}})
 hooks.progress?.({payload:{chunkLength:60,contentLength:100}});await flushPromises();expect(wrapper.text()).toContain('60 B / 100 B');expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBe('60')
 hooks.progress?.({payload:{chunkLength:60,contentLength:100}});await flushPromises();expect(wrapper.text()).toContain('120 B');expect(wrapper.text()).not.toContain('/ 100 B');expect(wrapper.text()).not.toContain('100%');expect(wrapper.get('[role="progressbar"]').attributes('aria-valuenow')).toBeUndefined();expect(wrapper.find('.indeterminate').exists()).toBe(true)
 wrapper.unmount();store.cleanup()
})
