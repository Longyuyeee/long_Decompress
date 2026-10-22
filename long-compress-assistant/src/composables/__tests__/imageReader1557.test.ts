import {beforeEach,expect,it,vi} from 'vitest'
import {readerError,readerImages,useArchiveImageReader} from '../useArchiveImageReader'
import type {ArchiveImagePreview,ArchiveEntryInfo} from '../useTauriCommands'
const preview=(path:string,bytes=10,pixels=100):ArchiveImagePreview=>({entryPath:path,dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:bytes,width:pixels,height:1})
let client:any
beforeEach(()=>{client={read:vi.fn(async(path:string)=>preview(path)),checkSource:vi.fn(async()=>{}),cancel:vi.fn(async()=>{}),createUrl:vi.fn(async(p:ArchiveImagePreview)=>'blob:'+p.entryPath),revokeUrl:vi.fn()}})
it('reader1557 uses natural path ordering, excludes unrelated files and marks duplicate references',()=>{
 const entry=(path:string,isDir=false)=>({path,name:path.split('/').pop()!,isDir,size:1,compressedSize:null,modified:null,crc:null,encrypted:false} as ArchiveEntryInfo)
 const result=readerImages(['10.png','2.png','1.png','note.txt','a/3.png','2.png'].map(p=>entry(p)),name=>name.endsWith('.png'))
 expect(result.map(e=>e.path)).toEqual(['1.png','2.png','10.png','a/3.png']);expect(result[1].ambiguous).toBe(true)
})
it('reader1557 displays a page, preloads only the next page and validates cached pages without decoding twice',async()=>{
 const r=useArchiveImageReader(client);await r.show('1','2');await vi.waitFor(()=>expect(r.cache.has('2')).toBe(true));await r.show('2','3');expect(client.read.mock.calls.map((c:any)=>c[0])).toEqual(['1','2','3']);expect(client.checkSource).toHaveBeenCalledOnce();expect(r.current.value?.path).toBe('2');r.dispose()
})
it('reader1557 quick switching cancels the old page and ignores its late success',async()=>{
 let complete!:(p:ArchiveImagePreview)=>void;client.read.mockImplementation((path:string)=>path==='slow'?new Promise(resolve=>complete=resolve):Promise.resolve(preview(path)))
 const r=useArchiveImageReader(client),first=r.show('slow');await r.show('new');complete(preview('slow'));await first;expect(client.cancel).toHaveBeenCalledOnce();expect(r.current.value?.path).toBe('new');expect(r.cache.has('slow')).toBe(false);r.dispose()
})
it('reader1557 byte and pixel budgets evict other images while keeping the current page',async()=>{
 client.read.mockImplementation(async(path:string)=>({...preview(path,8*1024*1024,4000),height:4000}))
 const r=useArchiveImageReader(client);await r.show('a');await r.show('b');expect(r.cache.size).toBe(1);expect(r.current.value?.path).toBe('b');expect(client.revokeUrl).toHaveBeenCalledWith('blob:a');r.dispose()
})
it('reader1557 keeps at most three small decoded image references',async()=>{
 const r=useArchiveImageReader(client);for(const p of ['a','b','c','d'])await r.show(p);expect([...r.cache.keys()]).toEqual(['b','c','d']);expect(client.revokeUrl).toHaveBeenCalledWith('blob:a');r.dispose()
})
it('reader1557 a changed source during a cache hit releases the whole session and blocks reuse',async()=>{
 const r=useArchiveImageReader(client);await r.show('a');client.checkSource.mockRejectedValue(Error('IMAGE_READER_CHANGED|来源已变化'));await r.show('a');expect(r.stale.value).toBe(true);expect(r.current.value).toBeNull();expect(r.cache.size).toBe(0);const calls=client.read.mock.calls.length;await r.show('b');expect(client.read).toHaveBeenCalledTimes(calls);r.dispose()
})
it('reader1557 closing during URL creation releases the late object URL',async()=>{
 let complete!:(p:string)=>void;client.createUrl.mockImplementation(()=>new Promise(resolve=>complete=resolve));const r=useArchiveImageReader(client),p=r.show('a');await vi.waitFor(()=>expect(client.createUrl).toHaveBeenCalled());r.dispose();complete('blob:late');await p;expect(client.revokeUrl).toHaveBeenCalledWith('blob:late');expect(r.cache.size).toBe(0)
})
it('reader1557 stopping preserves finished images and retry can continue',async()=>{
 let complete!:(p:ArchiveImagePreview)=>void;const r=useArchiveImageReader(client);await r.show('a');client.read.mockImplementationOnce(()=>new Promise(resolve=>complete=resolve));const p=r.show('slow');r.cancel();complete(preview('slow'));await p;expect(r.cache.has('a')).toBe(true);expect(r.error.value).toContain('停止');client.read.mockResolvedValue(preview('slow'));await r.show('slow');expect(r.current.value?.path).toBe('slow');r.dispose()
})
it('reader1557 unsupported and browser decode failures remain actionable without showing fake success',async()=>{
 const r=useArchiveImageReader(client);await r.show('svg',undefined,'此格式请提取');expect(client.read).not.toHaveBeenCalled();await r.show('broken');r.displayError();expect(r.current.value).toBeNull();expect(r.error.value).toContain('无法完整显示');expect(r.cache.has('broken')).toBe(false);r.dispose()
})
it('reader1557 accepts progress only for the visible active request and rejects obsolete events',async()=>{
 let complete!:(p:ArchiveImagePreview)=>void;client.read.mockImplementation(()=>new Promise(resolve=>complete=resolve));const r=useArchiveImageReader(client),p=r.show('a'),id=client.read.mock.calls[0][1];r.update('old',{phase:'reading',processedBytes:100,totalBytes:100});expect(r.progress.value.processedBytes).toBe(0);r.update(id,{phase:'reading',processedBytes:3,totalBytes:10});expect(r.progress.value.processedBytes).toBe(3);complete(preview('a'));await p;r.dispose()
})

it('egg1613 reader failures hide internal tags while preserving the cause and extraction hint',()=>{expect(readerError('[archive-inspection:damaged] [egg-io] EGG 图片校验失败')).toBe('EGG 图片校验失败');expect(readerError('EGG 固实条目暂不支持图片预览，请先提取后打开')).toContain('提取');expect(readerError('Image dimensions exceed the preview limit')).toContain('尺寸超过')})
