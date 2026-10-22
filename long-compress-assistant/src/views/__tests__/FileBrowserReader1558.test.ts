import {beforeEach,it,expect,vi} from 'vitest'
import {mount} from '@vue/test-utils'
import {createPinia,setActivePinia} from 'pinia'
import FileBrowserView from '../FileBrowserView.vue'
import {useArchiveReadingStore} from '@/stores/archiveReading'
const mocks=vi.hoisted(()=>({browse:vi.fn(),pendingPath:'',options:{} as Record<string,unknown>}))
vi.mock('vue-router',()=>({useRouter:()=>({push:vi.fn()})}))
vi.mock('@/stores/app',()=>({useAppStore:()=>({get pendingArchiveBrowserPath(){return mocks.pendingPath},takeArchiveBrowserOptions:()=>mocks.options,takeArchiveBrowserPath:()=>{const path=mocks.pendingPath;mocks.pendingPath='';return path},openArchiveInBrowser:mocks.browse})}))
vi.mock('@/stores/compression',()=>({useCompressionStore:()=>({})}))
vi.mock('@/components/file-manager/DualPaneFileManager.vue',()=>({default:{props:['archiveRequest'],template:'<div data-testid="actual-default-files" :data-source="archiveRequest?.path"><slot name="header-actions"/></div>'}}))
vi.mock('@/views/ArchiveBrowserView.vue',()=>({default:{template:'<div data-testid="actual-archive-mode"/>'}}))
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());vi.clearAllMocks();mocks.pendingPath='';mocks.options={}})
it('reader1558 actual file-manager landing exposes restored reading records before any archive is open',async()=>{
 const snapshot={version:1,date:20261021010000},row={filePath:'C:/pages.zip.001',format:'ZIP',revision:'a'.repeat(64),snapshot,path:'pages/101.png',directory:'pages',scope:'directory' as const,fit:false,zoom:1.5,selected:['pages/2.png'],updatedAt:new Date().toISOString()};useArchiveReadingStore().save(row);setActivePinia(createPinia())
 const w=mount(FileBrowserView,{global:{stubs:{Modal:{props:['visible'],template:'<div v-if="visible"><slot/></div>'}}}});expect(w.find('[data-testid="actual-default-files"]').exists()).toBe(true);expect(w.find('[data-testid="reading-recent-open"]').exists()).toBe(false);await w.get('[data-testid="reading-history-open"]').trigger('click');expect(w.get('[data-testid="reading-recent-open"]').text()).toContain('pages/101.png');await w.get('[data-testid="reading-recent-open"]').trigger('click');expect(mocks.browse).toHaveBeenCalledWith(row.filePath,{focusEntryPath:row.path,entryAction:'read',resumeReading:true,zpaqSnapshot:snapshot,zipPasswordEncoding:undefined});expect(w.find('[data-testid="actual-default-files"]').exists()).toBe(true);expect(w.find('[data-testid="actual-archive-mode"]').exists()).toBe(false);w.unmount()
})

it('external archive requests retain the dual-pane shell and exact selected-entry options',()=>{
 mocks.pendingPath='C:/library/project.zip';mocks.options={librarySourceId:'project',focusEntryPath:'docs/readme.txt',entryAction:'preview',password:'session-only'}
 const w=mount(FileBrowserView,{global:{stubs:{Modal:true}}});expect(w.get('[data-testid="actual-default-files"]').attributes('data-source')).toBe('C:/library/project.zip');expect(w.find('[data-testid="actual-archive-mode"]').exists()).toBe(false);expect(mocks.pendingPath).toBe('');w.unmount()
})
