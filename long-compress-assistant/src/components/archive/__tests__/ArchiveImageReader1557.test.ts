import {createPinia,setActivePinia} from 'pinia'
import {beforeEach,afterEach,it,expect,vi} from 'vitest'
import {mount,flushPromises} from '@vue/test-utils'
import ArchiveImageReader from '../ArchiveImageReader.vue'
const mocks=vi.hoisted(()=>({invoke:vi.fn(),callback:null as any,unlisten:vi.fn()}))
vi.mock('@tauri-apps/api/tauri',()=>({invoke:mocks.invoke}))
vi.mock('@tauri-apps/api/event',()=>({listen:vi.fn(async(_event,callback)=>{mocks.callback=callback;return mocks.unlisten})}))
const picture=(path:string)=>({path,name:path.split('/').pop()!,size:10,isDir:false,compressedSize:null,modified:null,crc:null,encrypted:false})
const props={filePath:'C:/pages.zip',entries:['pages/1.png','pages/2.png','pages/10.png','other/3.png','other/bad.svg'].map(picture),initialPath:'pages/2.png',directory:'pages',revision:'version',password:'session-only',encoding:'cp936' as const,supported:true,imageExtensions:['png']}
const preview=(entryPath:string)=>({entryPath,dataUrl:'data:image/png;base64,AA==',width:100,height:120,byteSize:10,mimeType:'image/png'})
beforeEach(()=>{localStorage.clear();setActivePinia(createPinia());mocks.invoke.mockReset();mocks.unlisten.mockClear();mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?preview(args.entryPath):undefined);vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:reader'),revokeObjectURL:vi.fn()})})
afterEach(()=>vi.unstubAllGlobals())
it('reader1557 component wires scope, page jump, keyboard, zoom and return location to the same source context',async()=>{
 const w=mount(ArchiveImageReader,{props,attachTo:document.body,global:{stubs:{Teleport:true}}});await flushPromises();expect(w.get('[data-testid="reader-total"]').text()).toBe('/ 3');expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/2.png');expect(mocks.invoke).toHaveBeenCalledWith('read_archive_image',expect.objectContaining({filePath:'C:/pages.zip',entryPath:'pages/2.png',password:'session-only',zipPasswordEncoding:'cp936',revision:'version'}));await w.get('[data-testid="reader-page"]').setValue(3);await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/10.png');await w.get('[role="dialog"]').trigger('keydown',{key:'ArrowLeft'});await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/2.png');await w.get('[data-testid="reader-scope"]').setValue('archive');await flushPromises();expect(w.get('[data-testid="reader-total"]').text()).toBe('/ 5');await w.get('[data-testid="reader-original"]').trigger('click');await w.get('[data-testid="reader-zoom-in"]').trigger('click');expect(w.get('[data-testid="reader-zoom"]').text()).toBe('125%');await w.get('[data-testid="reader-fit"]').trigger('click');expect(w.get('[data-testid="reader-zoom"]').text()).toBe('适应');await w.get('[data-testid="reader-locate"]').trigger('click');expect(w.emitted('locate')?.[0]).toEqual(['pages/2.png']);w.unmount();expect(mocks.unlisten).toHaveBeenCalled();expect(URL.revokeObjectURL).toHaveBeenCalled()
})
it('reader1557 component skips unsupported images and offers exact original extraction',async()=>{
 const entries=['a.svg','c.png'].map(picture),w=mount(ArchiveImageReader,{props:{...props,entries,initialPath:'a.svg',directory:''},global:{stubs:{Teleport:true}}});await flushPromises();expect(w.get('[data-testid="reader-error"]').text()).toContain('此图片格式暂不支持');expect(mocks.invoke).not.toHaveBeenCalled();await w.get('[data-testid="reader-extract"]').trigger('click');expect(w.emitted('extract')?.[0]).toEqual(['a.svg']);await w.get('[data-testid="reader-skip"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('c.png');expect(w.find('[data-testid="reader-image"]').exists()).toBe(true);w.unmount()
})
it('reader1557 component shows native progress and stopping cancels only its registered image request',async()=>{
 let complete!:(value:any)=>void;mocks.invoke.mockImplementation(async(command)=>command==='read_archive_image'?new Promise(resolve=>complete=resolve):undefined);const w=mount(ArchiveImageReader,{props,global:{stubs:{Teleport:true}}});await flushPromises();const call=mocks.invoke.mock.calls.find(([c])=>c==='read_archive_image')!;mocks.callback({payload:{requestId:call[1].requestId,progress:{phase:'reading',processedBytes:1024,totalBytes:2048}}});await flushPromises();expect(w.get('[data-testid="reader-progress"]').text()).toContain('1.0 KiB');await w.get('[data-testid="reader-stop"]').trigger('click');expect(mocks.invoke).toHaveBeenCalledWith('cancel_archive_browse',{browseId:call[1].requestId});expect(w.get('[data-testid="reader-error"]').text()).toContain('读取已停止');complete(preview('pages/2.png'));await flushPromises();expect(w.find('[data-testid="reader-image"]').exists()).toBe(false);await w.get('[role="dialog"]').trigger('keydown',{key:'Escape'});expect(w.emitted('close')).toHaveLength(1);w.unmount()
})

it('egg1615 solid reader loads only demanded pages and validates cached pages without speculative shared reads',async()=>{
 const w=mount(ArchiveImageReader,{props:{...props,format:'EGG'},global:{stubs:{Teleport:true}}});await flushPromises()
 const reads=()=>mocks.invoke.mock.calls.filter(([cmd])=>cmd==='read_archive_image')
 expect(reads()).toHaveLength(1);expect(reads()[0][1].entryPath).toBe('pages/2.png')
 await w.get('[data-testid="reader-next"]').trigger('click');await flushPromises();expect(reads()).toHaveLength(2);expect(reads()[1][1].entryPath).toBe('pages/10.png')
 await w.get('[data-testid="reader-previous"]').trigger('click');await flushPromises();expect(reads()).toHaveLength(2)
 expect(mocks.invoke).toHaveBeenCalledWith('check_image_reader_source',expect.objectContaining({revision:'version'}));expect(w.get('[data-testid="reader-current-path"]').text()).toBe('pages/2.png');w.unmount()
})
it('egg1615 shared read has an explicit wait message and stopping discards late pixels',async()=>{
 let finish!:(value:any)=>void
 mocks.invoke.mockImplementation(async cmd=>cmd==='read_archive_image'?new Promise(resolve=>finish=resolve):undefined)
 const w=mount(ArchiveImageReader,{props:{...props,format:'EGG'},global:{stubs:{Teleport:true}}});await flushPromises()
 expect(w.get('[data-testid="reader-progress"]').text()).toContain('正在读取固实包共享内容')
 const request=mocks.invoke.mock.calls.find(([cmd])=>cmd==='read_archive_image')![1]
 await w.get('[data-testid="reader-stop"]').trigger('click');expect(mocks.invoke).toHaveBeenCalledWith('cancel_archive_browse',{browseId:request.requestId})
 finish(preview('pages/2.png'));await flushPromises();expect(w.find('[data-testid="reader-image"]').exists()).toBe(false);w.unmount()
})
