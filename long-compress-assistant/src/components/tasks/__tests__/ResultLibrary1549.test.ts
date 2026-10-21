import { beforeEach,afterEach,expect,it,vi } from 'vitest'
import { createPinia,setActivePinia } from 'pinia'
import { mount,flushPromises } from '@vue/test-utils'
import ResultActions from '../ResultActions.vue'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),navigate:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('vue-router',()=>({useRouter:()=>({push:mocks.navigate})}))
vi.mock('@/stores/app',()=>({useAppStore:()=>({})}))
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());vi.clearAllMocks();mocks.invoke.mockResolvedValue([])})
afterEach(()=>localStorage.clear())
it('library1549 explicitly adds an archive result and opens the library without auto-indexing',async()=>{const view=mount(ResultActions,{props:{path:'D:/new.zip'}});await view.get('[data-testid="result-add-library"]').trigger('click');await flushPromises();expect(mocks.invoke).toHaveBeenCalledWith('library_add',{paths:['D:/new.zip']});expect(mocks.navigate).toHaveBeenCalledWith('/library');expect(mocks.invoke.mock.calls.some(call=>call[0]==='library_index')).toBe(false);view.unmount()})
it('library1549 keeps an unavailable result reason beside its actions without navigation',async()=>{mocks.invoke.mockRejectedValueOnce(Error('来源不存在'));const view=mount(ResultActions,{props:{path:'D:/new.zip'}});await view.get('[data-testid="result-add-library"]').trigger('click');await flushPromises();expect(view.get('[role="alert"]').text()).toContain('来源不存在');expect(mocks.navigate).not.toHaveBeenCalled();view.unmount()})
it('library1549 does not offer archive-library indexing for extracted folders or media results',()=>{for(const props of [{path:'D:/out',directory:true},{path:'D:/photo.png'},{path:'D:/video.mp4'}]){const view=mount(ResultActions,{props});expect(view.find('[data-testid="result-add-library"]').exists()).toBe(false);view.unmount()}})
