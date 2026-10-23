import {createPinia,setActivePinia} from 'pinia'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { DOMWrapper, flushPromises, mount } from '@vue/test-utils'
import ArchiveBrowserView from '../ArchiveBrowserView.vue'

const mocks = vi.hoisted(() => ({
  invoke: vi.fn(),
  capabilityRefresh: vi.fn(),
  pendingPath: '',
  readOptions: {} as Record<string, unknown>,
  selectFiles: vi.fn(),
  selectDirectory: vi.fn(),
  browseArchive: vi.fn(),
  cancelArchiveBrowse: vi.fn(),
  previewArchiveImage: vi.fn(),
  previewArchiveText: vi.fn(),
  materializeNestedArchive: vi.fn(),
  openArchiveEntry: vi.fn(),
  decompressFile: vi.fn(),
  testArchiveIntegrity: vi.fn(),
  convertSnapshot: vi.fn(),
  clipboardWrite: vi.fn(),
  setError: vi.fn(),
  clearError: vi.fn(),
  currentError: null as string | null,
  setSuccess: vi.fn(),
}))

vi.mock('@tauri-apps/api/tauri', () => ({ invoke: mocks.invoke }))
vi.mock('@/stores/app', () => ({
  useAppStore: () => ({ get pendingArchiveBrowserPath() { return mocks.pendingPath }, takeArchiveBrowserOptions: () => mocks.readOptions, takeArchiveBrowserPath: () => { const path=mocks.pendingPath; mocks.pendingPath=''; return path }, setError: mocks.setError, clearError: mocks.clearError, get error() { return mocks.currentError }, setSuccess: mocks.setSuccess })
}))
vi.mock('@/components/archive/ZpaqSnapshotDiff.vue', () => ({ default: { template: '<div />' } }))
vi.mock('@/stores/zipEdit', () => ({useZipEditStore: () => ({ sessionFor: () => undefined, taskFor: () => undefined, opening: false, preparing: false, openingError: null, begin: vi.fn() })}))
vi.mock('@/stores/archiveConversion', () => ({ useArchiveConversionStore: () => ({ openSnapshot: mocks.convertSnapshot }) }))

vi.mock('@/composables/useArchiveEngine', () => ({
  useArchiveEngine: () => ({
    capabilities: { value: {
      available: true,
      fullEngine: true,
      formats: [],
      browseExtensions: ['zip', '7z', 'rar', 'tar', 'zst'],
      nestedExtensions: ['zip', '7z', 'rar', 'tar', 'zst'],
      boundedPreviewFormats: ['ZIP', 'TAR', 'ZPAQ', 'ALZ', 'EGG', '7Z'],
      boundedTextPreviewFormats: ['ZIP', 'TAR', 'ZPAQ', 'ALZ', 'EGG', '7Z'],
      imagePreviewExtensions: ['png', 'jpg', 'jpeg', 'gif', 'webp', 'bmp'],
      textPreviewExtensions: ['txt', 'md', 'log'],
      message: 'ready',
    } },
    loading: { value: false },
    refresh: mocks.capabilityRefresh,
  }),
}))

vi.mock('@/composables/useTauriCommands', () => ({
  useTauriCommands: () => ({
    selectFiles: mocks.selectFiles,
    selectDirectory: mocks.selectDirectory,
    browseArchive: mocks.browseArchive,
    cancelArchiveBrowse: mocks.cancelArchiveBrowse,
    previewArchiveImage: mocks.previewArchiveImage,
    previewArchiveText: mocks.previewArchiveText,
    materializeNestedArchive: mocks.materializeNestedArchive,
    openArchiveEntry: mocks.openArchiveEntry,
    decompressFile: mocks.decompressFile,
    testArchiveIntegrity: mocks.testArchiveIntegrity,
  })
}))

describe('ArchiveBrowserView', () => {
  it('sevenz1624 encrypted prefix uses existing password and selected revision with partial-only label',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'4'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:2097152,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:2097152,isDir:false,compressedSize:1200000,encrypted:true}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'readme.txt',content:'INFO 密码日志',encoding:'UTF-8',byteSize:1048576,totalSize:2097152,truncated:true,lineCount:1})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1624,path:'C:/password.7z'}},global:{stubs:{Teleport:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-meta').text()).toContain('部分内容 · 未校验整文件');expect(w.find('[data-testid="archive-prefix-open-full"]').exists()).toBe(true);expect(mocks.previewArchiveText.mock.calls.at(-1)![5]).toEqual({previewId:expect.stringMatching(/^7z-text-/),sourceRevision:'4'.repeat(64)});w.unmount()
  })
  it('sevenz1623 full text action keeps the exact selected original open command',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'3'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:2097152,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:2097152,isDir:false,compressedSize:1200000}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'readme.txt',content:'INFO 开头',encoding:'UTF-8',byteSize:1048575,totalSize:2097152,truncated:true,lineCount:1});mocks.openArchiveEntry.mockResolvedValueOnce({status:'opened',path:'D:/temp/readme.txt'})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1623,path:'C:/logs.7z'}},global:{stubs:{Teleport:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();await w.get('[data-testid="archive-prefix-open-full"]').trigger('click');await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenCalledWith('C:/logs.7z','readme.txt','',false);expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1623 prefix stays in the original pane with explicit partial bytes and contextual scope',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'3'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:2097152,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:2097152,isDir:false,compressedSize:1200000}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'readme.txt',content:'INFO 开头',encoding:'UTF-8',byteSize:1048575,totalSize:2097152,truncated:true,lineCount:1})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1623,path:'C:/logs.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-meta').text()).toContain('部分内容 · 未校验整文件');expect(w.get('.preview-meta').text()).not.toContain('完整显示');expect(w.text()).not.toContain('压缩包浏览中心')
    expect(mocks.previewArchiveText.mock.calls.at(-1)![5]).toEqual({previewId:expect.stringMatching(/^7z-text-/),sourceRevision:'3'.repeat(64)})
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('非固实大文本（含 AES 密码包）仅显示前 1 MiB');w.unmount()
  })
  it('sevenz1623 closing a prefix cancels its exact request and discards late partial content',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'3'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:2097152,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:2097152,isDir:false,compressedSize:1200000}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1623,path:'C:/logs.7z'}},global:{stubs:{Teleport:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();const request=mocks.previewArchiveText.mock.calls.at(-1)![5]
    await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(request.previewId);finish({entryPath:'readme.txt',content:'late prefix',encoding:'UTF-8',byteSize:11,totalSize:2097152,truncated:true,lineCount:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it.each(['固实大文本'])('sevenz1623 %s keeps extraction guidance without publishing content',async kind=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'3'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:2097152,encrypted:kind==='密码大文本',entries:[{path:'readme.txt',name:'readme.txt',size:2097152,isDir:false,compressedSize:null}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error(`7Z ${kind}暂不支持开头预览，请先提取后打开`))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1623,path:'C:/logs.7z'}},global:{stubs:{Teleport:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-stage').text()).toContain('先提取后打开');expect(w.find('[data-testid="archive-text-preview"]').exists()).toBe(false);w.unmount()
  })
  it('library1547 waits for the initial engine capabilities before deciding preview support',async()=>{
    let ready!:()=>void;mocks.capabilityRefresh.mockReturnValue(new Promise<void>(resolve=>{ready=resolve}))
    mocks.pendingPath='C:/archives/demo.zip';mocks.readOptions={librarySourceId:'source-a',focusEntryPath:'docs/readme.txt',entryAction:'preview'}
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();expect(mocks.previewArchiveText).not.toHaveBeenCalled();ready();await flushPromises();expect(mocks.previewArchiveText).toHaveBeenCalled();wrapper.unmount()
  })
  it('library1547 focuses just the matching internal file and opens the original preview', async () => {
    mocks.pendingPath='C:/archives/demo.zip';mocks.readOptions={librarySourceId:'source-a',focusEntryPath:'docs/readme.txt',entryAction:'preview'}
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    expect(mocks.previewArchiveText).toHaveBeenCalledWith('C:/archives/demo.zip','docs/readme.txt','')
    expect(wrapper.get('footer').text()).toContain('已选择 1 / 2')
    expect(wrapper.get('[data-testid="archive-library-origin"]').text()).toContain('docs/readme.txt')
    expect(mocks.decompressFile).not.toHaveBeenCalled();wrapper.unmount()
  })
  it.each([null,'C:/recovered'])('library1547 asks for output and extracts only the hit; destination=%s',async destination=>{
    mocks.pendingPath='C:/archives/demo.zip';mocks.readOptions={librarySourceId:'source-a',focusEntryPath:'docs/readme.txt',entryAction:'extract'};mocks.selectDirectory.mockResolvedValue(destination)
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{RouterLink:true}}});await flushPromises()
    expect(mocks.selectDirectory).toHaveBeenCalled()
    if(destination)expect(mocks.decompressFile).toHaveBeenCalledWith('C:/archives/demo.zip',expect.objectContaining({outputPath:destination,selectedEntries:['docs/readme.txt'],overwrite:false,deleteAfter:false}))
    else expect(mocks.decompressFile).not.toHaveBeenCalled()
    wrapper.unmount()
  })
  it('library1547 rejects stale source before preview or extraction and missing hits never select everything',async()=>{
    mocks.pendingPath='C:/archives/demo.zip';mocks.readOptions={librarySourceId:'source-a',focusEntryPath:'gone.txt',entryAction:'extract'}
    const wrapper=mount(ArchiveBrowserView);await flushPromises();expect(wrapper.get('footer').text()).toContain('已选择 0 / 2');expect(mocks.decompressFile).not.toHaveBeenCalled();wrapper.unmount()
    mocks.pendingPath='C:/archives/demo.zip';mocks.invoke.mockRejectedValue(new Error('原包发生变化'))
    const stale=mount(ArchiveBrowserView);await flushPromises();expect(mocks.setError).toHaveBeenLastCalledWith(expect.stringContaining('原包发生变化'));expect(mocks.previewArchiveText).not.toHaveBeenCalled();expect(mocks.decompressFile).not.toHaveBeenCalled();stale.unmount()
  })

  it('zpaq1530 switches snapshots with empty selection and keeps preview, refresh, extraction and conversion bound', async () => {
    const one={version:1,date:20261020130000},latest={version:3,date:20261020150000}
    const entry=(path:string)=>({path,name:path,size:8,modified:null,compressedSize:null,crc:null,encrypted:false,isDir:false})
    const view=(selected:typeof one)=>({format:'ZPAQ',totalFiles:2,totalDirectories:0,totalUncompressedSize:16,totalCompressedSize:12,encrypted:false,
      entries:[entry('说明.txt'),entry(selected.version===1?'后来删除.txt':'新增.txt')],zpaqHistory:{selected,snapshots:[{...one,updates:2,deletes:0},{...latest,updates:1,deletes:1}]}})
    mocks.browseArchive.mockResolvedValueOnce(view(latest)).mockResolvedValueOnce(view(one)).mockResolvedValueOnce(view(one))
    const wrapper=mount(ArchiveBrowserView,{attachTo:document.body})
    await wrapper.get('header .browser-primary').trigger('click');await flushPromises()
    await wrapper.get('[data-testid="zpaq-snapshot-select"]').setValue('1');await flushPromises()
    expect(wrapper.get('footer').text()).toContain('已选择 0 / 2')
    expect(wrapper.find('[data-entry-path="新增.txt"]').exists()).toBe(false)
    mocks.previewArchiveText.mockResolvedValue({entryPath:'说明.txt',content:'old snapshot',encoding:'UTF-8',byteSize:8,totalSize:8,truncated:false,lineCount:1})
    await wrapper.get('[data-entry-path="说明.txt"] .preview-trigger').trigger('click');await flushPromises()
    expect(mocks.previewArchiveText).toHaveBeenLastCalledWith('C:/archives/demo.zip','说明.txt','',undefined,one)
    ;(document.querySelector('.preview-close') as HTMLButtonElement).click();await flushPromises()
    await wrapper.get('[data-testid="archive-nav-refresh"]').trigger('click');await flushPromises()
    expect(mocks.browseArchive).toHaveBeenLastCalledWith('C:/archives/demo.zip','',expect.any(String),undefined,one)
    await wrapper.get('[data-entry-path="后来删除.txt"] .browser-checkbox').trigger('click')
    await wrapper.get('footer .browser-primary').trigger('click');await flushPromises()
    expect(mocks.decompressFile).toHaveBeenLastCalledWith('C:/archives/demo.zip',expect.objectContaining({zpaqSnapshot:one,selectedEntries:['后来删除.txt']}))
    await wrapper.get('[data-testid="zpaq-snapshot-convert"]').trigger('click');expect(mocks.convertSnapshot).toHaveBeenCalledWith('C:/archives/demo.zip',one,'')
    wrapper.unmount()
  })
  it.each([false,true])('clears only its own failed-browse message after successful password correction; unrelated=%s', async unrelated => {
    const wrapper=mount(ArchiveBrowserView)
    mocks.browseArchive.mockRejectedValueOnce(new Error('ARCHIVE_BROWSE_PASSWORD|密码不正确，请修改密码'))
    await wrapper.get('header .browser-primary').trigger('click');await flushPromises()
    expect(mocks.setError).toHaveBeenLastCalledWith('密码不正确，请修改密码')
    if(unrelated)mocks.currentError='另一项任务输出失败'
    await wrapper.get('input[type="password"]').setValue('corrected password')
    await wrapper.get('button[title="重新读取"]').trigger('click');await flushPromises()
    expect(wrapper.find('[data-entry-path="image.png"]').exists()).toBe(true)
    expect(mocks.clearError).toHaveBeenCalledTimes(unrelated?0:1)
    if(unrelated)expect(mocks.currentError).toBe('另一项任务输出失败')
    wrapper.unmount()
  })
  it.each(['cp932','cp936','cp950'] as const)('uses %s for preview, refresh, open and selected extraction without changing the source password', async encoding => {
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{Teleport:true}}})
    await wrapper.get('header .browser-primary').trigger('click'); await flushPromises()
    wrapper.findComponent({name:'ZipPasswordEncodingField'}).vm.$emit('update:modelValue',encoding)
    await wrapper.get('input[type="password"]').setValue('source-only-password')
    await wrapper.get('[data-entry-path="image.png"] .preview-trigger').trigger('click'); await flushPromises()
    expect(mocks.previewArchiveImage).toHaveBeenLastCalledWith('C:/archives/demo.zip','image.png','source-only-password',encoding)
    await wrapper.get('.preview-close').trigger('click')
    await wrapper.get('[data-entry-path="image.png"]').trigger('dblclick'); await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenLastCalledWith('C:/archives/demo.zip','image.png','source-only-password',false,encoding)
    await wrapper.get('.browser-search input').setValue('readme')
    await wrapper.get('[data-entry-path="docs/readme.txt"] .preview-trigger').trigger('click'); await flushPromises()
    expect(mocks.previewArchiveText).toHaveBeenLastCalledWith('C:/archives/demo.zip','docs/readme.txt','source-only-password',encoding)
    await wrapper.get('.preview-close').trigger('click')
    await wrapper.get('footer .browser-primary').trigger('click'); await flushPromises()
    expect(mocks.decompressFile).toHaveBeenLastCalledWith('C:/archives/demo.zip',expect.objectContaining({zipPasswordEncoding:encoding,password:'source-only-password'}))
    await wrapper.get('button[title="重新读取"]').trigger('click'); await flushPromises()
    expect(mocks.browseArchive).toHaveBeenLastCalledWith('C:/archives/demo.zip','source-only-password',expect.any(String),encoding)
    wrapper.unmount()
  })
  it('gives a nested ZIP its own automatic input choice and restores the parent choice on return', async () => {
    const listing={format:'ZIP',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:true,
      entries:[{path:'inner.zip',name:'inner.zip',size:10,encrypted:true,isDir:false}]}
    mocks.browseArchive.mockResolvedValue(listing)
    mocks.materializeNestedArchive.mockResolvedValue({entryPath:'inner.zip',cachePath:'C:/cache/inner.zip',parentSha256:'outer-hash',contentSha256:'inner-hash',depth:2})
    const wrapper=mount(ArchiveBrowserView)
    await wrapper.get('header .browser-primary').trigger('click'); await flushPromises()
    wrapper.findComponent({name:'ZipPasswordEncodingField'}).vm.$emit('update:modelValue','cp932')
    await wrapper.get('input[type="password"]').setValue('outer-password')
    await wrapper.get('[data-entry-path="inner.zip"]').trigger('dblclick'); await flushPromises()
    expect(mocks.materializeNestedArchive).toHaveBeenLastCalledWith('C:/archives/demo.zip','inner.zip','outer-password',2,[],'cp932')
    expect(mocks.browseArchive).toHaveBeenLastCalledWith('C:/cache/inner.zip','',expect.any(String))
    expect(wrapper.get('[data-testid="browser-zip-password-encoding"]').text()).toContain('自动兼容')
    wrapper.findComponent({name:'ZipPasswordEncodingField'}).vm.$emit('update:modelValue','cp950')
    await wrapper.get('[data-testid="archive-chain"] button').trigger('click'); await flushPromises()
    expect(wrapper.get('[data-testid="browser-zip-password-encoding"]').text()).toContain('CP932')
    expect((wrapper.get('input[type="password"]').element as HTMLInputElement).value).toBe('outer-password')
    await wrapper.get('header .browser-primary').trigger('click'); await flushPromises()
    expect(wrapper.get('[data-testid="browser-zip-password-encoding"]').text()).toContain('自动兼容')
    expect((wrapper.get('input[type="password"]').element as HTMLInputElement).value).toBe('')
    wrapper.unmount()
  })
  it('does not show a late preview after changing its input encoding', async () => {
    const wrapper=mount(ArchiveBrowserView)
    await wrapper.get('header .browser-primary').trigger('click'); await flushPromises()
    let finish!:(value:unknown)=>void
    mocks.previewArchiveImage.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
    await wrapper.get('[data-entry-path="image.png"] .preview-trigger').trigger('click')
    wrapper.findComponent({name:'ZipPasswordEncodingField'}).vm.$emit('update:modelValue','cp932'); await flushPromises()
    finish({dataUrl:'data:image/png;base64,late'}); await flushPromises()
    expect(document.querySelector('.preview-close')).toBeNull()
    expect(document.querySelector('img[src="data:image/png;base64,late"]')).toBeNull()
    wrapper.unmount()
  })
  beforeEach(() => {
    localStorage.clear();setActivePinia(createPinia())

    vi.clearAllMocks()
    mocks.invoke.mockReset(); mocks.invoke.mockResolvedValue(undefined)
    mocks.capabilityRefresh.mockReset(); mocks.capabilityRefresh.mockResolvedValue(undefined)
    mocks.pendingPath=''; mocks.readOptions={}
    mocks.currentError=null
    mocks.setError.mockImplementation(message=>{mocks.currentError=message})
    mocks.clearError.mockImplementation(()=>{mocks.currentError=null})
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText: mocks.clipboardWrite },
    })
    mocks.clipboardWrite.mockResolvedValue(undefined)
    mocks.cancelArchiveBrowse.mockResolvedValue(undefined)
    mocks.selectDirectory.mockResolvedValue(null)
    mocks.selectFiles.mockResolvedValue([{ path: 'C:/archives/demo.zip', name: 'demo.zip', size: 12, isDir: false, modified: 0 }])
    mocks.browseArchive.mockResolvedValue({
      format: 'ZIP', totalFiles: 2, totalDirectories: 1,
      totalUncompressedSize: 30, totalCompressedSize: 20, encrypted: false,
      entries: [
        { path: 'docs/', name: 'docs', size: 0, compressedSize: 0, modified: null, crc: null, encrypted: false, isDir: true },
        { path: 'docs/readme.txt', name: 'readme.txt', size: 10, compressedSize: 8, modified: null, crc: '11111111', encrypted: false, isDir: false },
        { path: 'image.png', name: 'image.png', size: 20, compressedSize: 12, modified: null, crc: '22222222', encrypted: false, isDir: false },
      ]
    })
    mocks.decompressFile.mockResolvedValue('C:/archives/demo')
    mocks.previewArchiveImage.mockResolvedValue({
      entryPath: 'image.png', mimeType: 'image/png',
      dataUrl: 'data:image/png;base64,c2FmZQ==', byteSize: 4, width: 1, height: 1,
    })
    mocks.previewArchiveText.mockResolvedValue({
      entryPath: 'docs/readme.txt', content: '你好，归档文本\n第二行', encoding: 'UTF-8',
      byteSize: 28, totalSize: 28, truncated: false, lineCount: 2,
    })
    mocks.openArchiveEntry.mockResolvedValue({
      status: 'opened', entryPath: 'docs/readme.txt', cachePath: 'C:/cache/readme.txt', dangerous: false,
    })
    mocks.materializeNestedArchive.mockResolvedValue({
      entryPath: 'nested.7z', cachePath: 'C:/cache/nested.7z',
      parentSha256: 'parent-hash', contentSha256: 'child-hash', depth: 2,
    })
  })

  it('sevenz1622 encrypted solid image keeps the original modal, shared wait text and contextual limits',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'e'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:20,totalCompressedSize:8,encrypted:true,entries:[{path:'cover.png',name:'cover.png',size:20,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveImage.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1622,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-stage').text()).toContain('正在读取固实包共享内容')
    finish({entryPath:'cover.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',width:320,height:240,byteSize:20});await flushPromises()
    expect(w.find('[data-testid="archive-image-preview"]').exists()).toBe(true)
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('检查整个共享块后显示');expect(w.text()).toContain('64 MiB');expect(w.text()).toContain('AES 内容加密、文件名加密');expect(w.text()).not.toContain('加密固实或超限请先提取');w.unmount()
  })

  it('sevenz1622 closing an image cancels its exact request and discards late bytes',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'8'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:72,encrypted:true,entries:[{path:'cover.png',name:'cover.png',size:72,isDir:false,compressedSize:null}]})
    let finish!:(value:unknown)=>void;mocks.previewArchiveImage.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1622,path:'C:/相册.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises()
    const request=mocks.previewArchiveImage.mock.calls.at(-1)![5];await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(request.previewId)
    finish({entryPath:'cover.png',mimeType:'image/png',dataUrl:'data:image/png;base64,AA==',byteSize:1,width:1,height:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1622 encrypted reader loads only the current page and keeps the original close behavior',async()=>{
    vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:sevenz-reader'),revokeObjectURL:vi.fn()})
    mocks.browseArchive.mockResolvedValueOnce({revision:'8'.repeat(64),format:'7Z',totalFiles:2,totalDirectories:0,totalUncompressedSize:20,encrypted:true,entries:['1.png','2.png'].map(path=>({path,name:path,isDir:false,size:10,compressedSize:null}))})
    mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?{entryPath:args.entryPath,dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1}:undefined)
    const w=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{
      await w.get('header .browser-primary').trigger('click');await flushPromises();await w.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises()
      expect(w.get('[data-testid="reader-current-path"]').text()).toBe('1.png');expect(mocks.invoke.mock.calls.filter(c=>c[0]==='read_archive_image')).toHaveLength(1)
      await w.get('[data-testid="reader-next"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('2.png');expect(mocks.invoke.mock.calls.filter(c=>c[0]==='read_archive_image')).toHaveLength(2)
      await w.get('[data-testid="reader-close"]').trigger('click');expect(w.find('[data-testid="archive-image-reader"]').exists()).toBe(false)
    }finally{w.unmount();vi.unstubAllGlobals()}
  })
  it('sevenz1622 solid reader explains shared work and stop discards the late page',async()=>{
    let finish!:(v:unknown)=>void
    mocks.browseArchive.mockResolvedValueOnce({revision:'0'.repeat(64),format:'7Z',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,encrypted:true,entries:[{path:'1.png',name:'1.png',isDir:false,size:10,compressedSize:null}]})
    mocks.invoke.mockImplementation((cmd)=>cmd==='read_archive_image'?new Promise(resolve=>finish=resolve):Promise.resolve())
    const w=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{await w.get('header .browser-primary').trigger('click');await flushPromises();await w.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-progress"]').text()).toContain('正在读取固实包共享内容');await w.get('[data-testid="reader-stop"]').trigger('click');await flushPromises();expect(mocks.invoke.mock.calls.some(c=>c[0]==='cancel_archive_browse')).toBe(true);finish({entryPath:'1.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1});await flushPromises();expect(w.find('[data-testid="reader-image"]').exists()).toBe(false)}finally{w.unmount()}
  })

  it('sevenz1620 solid image keeps the original modal, shared wait text and contextual limits',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'e'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:20,totalCompressedSize:8,encrypted:false,entries:[{path:'cover.png',name:'cover.png',size:20,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveImage.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1620,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-stage').text()).toContain('正在读取固实包共享内容')
    finish({entryPath:'cover.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',width:320,height:240,byteSize:20});await flushPromises()
    expect(w.find('[data-testid="archive-image-preview"]').exists()).toBe(true)
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('检查整个共享块后显示');expect(w.text()).toContain('64 MiB');w.unmount()
  })

  it('sevenz1620 closing an image cancels its exact request and discards late bytes',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'8'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:72,encrypted:false,entries:[{path:'cover.png',name:'cover.png',size:72,isDir:false,compressedSize:null}]})
    let finish!:(value:unknown)=>void;mocks.previewArchiveImage.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1620,path:'C:/相册.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises()
    const request=mocks.previewArchiveImage.mock.calls.at(-1)![5];await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(request.previewId)
    finish({entryPath:'cover.png',mimeType:'image/png',dataUrl:'data:image/png;base64,AA==',byteSize:1,width:1,height:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1620 actual reader loads only the current page and keeps the original close behavior',async()=>{
    vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:sevenz-reader'),revokeObjectURL:vi.fn()})
    mocks.browseArchive.mockResolvedValueOnce({revision:'8'.repeat(64),format:'7Z',totalFiles:2,totalDirectories:0,totalUncompressedSize:20,encrypted:false,entries:['1.png','2.png'].map(path=>({path,name:path,isDir:false,size:10,compressedSize:null}))})
    mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?{entryPath:args.entryPath,dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1}:undefined)
    const w=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{
      await w.get('header .browser-primary').trigger('click');await flushPromises();await w.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises()
      expect(w.get('[data-testid="reader-current-path"]').text()).toBe('1.png');expect(mocks.invoke.mock.calls.filter(c=>c[0]==='read_archive_image')).toHaveLength(1)
      await w.get('[data-testid="reader-next"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('2.png');expect(mocks.invoke.mock.calls.filter(c=>c[0]==='read_archive_image')).toHaveLength(2)
      await w.get('[data-testid="reader-close"]').trigger('click');expect(w.find('[data-testid="archive-image-reader"]').exists()).toBe(false)
    }finally{w.unmount();vi.unstubAllGlobals()}
  })
  it('sevenz1620 solid reader explains shared work and stop discards the late page',async()=>{
    let finish!:(v:unknown)=>void
    mocks.browseArchive.mockResolvedValueOnce({revision:'0'.repeat(64),format:'7Z',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,encrypted:false,entries:[{path:'1.png',name:'1.png',isDir:false,size:10,compressedSize:null}]})
    mocks.invoke.mockImplementation((cmd)=>cmd==='read_archive_image'?new Promise(resolve=>finish=resolve):Promise.resolve())
    const w=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{await w.get('header .browser-primary').trigger('click');await flushPromises();await w.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-progress"]').text()).toContain('正在读取固实包共享内容');await w.get('[data-testid="reader-stop"]').trigger('click');await flushPromises();expect(mocks.invoke.mock.calls.some(c=>c[0]==='cancel_archive_browse')).toBe(true);finish({entryPath:'1.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1});await flushPromises();expect(w.find('[data-testid="reader-image"]').exists()).toBe(false)}finally{w.unmount()}
  })

  it('sevenz1618 closing an image cancels its exact request and discards late bytes',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'8'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:72,encrypted:false,entries:[{path:'cover.png',name:'cover.png',size:72,isDir:false,compressedSize:50}]})
    let finish!:(value:unknown)=>void;mocks.previewArchiveImage.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1618,path:'C:/相册.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises()
    const request=mocks.previewArchiveImage.mock.calls.at(-1)![5];await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(request.previewId)
    finish({entryPath:'cover.png',mimeType:'image/png',dataUrl:'data:image/png;base64,AA==',byteSize:1,width:1,height:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1618 actual reader loads only the current page and keeps the original close behavior',async()=>{
    vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:sevenz-reader'),revokeObjectURL:vi.fn()})
    mocks.browseArchive.mockResolvedValueOnce({revision:'8'.repeat(64),format:'7Z',totalFiles:2,totalDirectories:0,totalUncompressedSize:20,encrypted:false,entries:['1.png','2.png'].map(path=>({path,name:path,isDir:false,size:10,compressedSize:8}))})
    mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?{entryPath:args.entryPath,dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1}:undefined)
    const w=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{
      await w.get('header .browser-primary').trigger('click');await flushPromises();await w.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises()
      expect(w.get('[data-testid="reader-current-path"]').text()).toBe('1.png');expect(mocks.invoke.mock.calls.filter(c=>c[0]==='read_archive_image')).toHaveLength(1)
      await w.get('[data-testid="reader-next"]').trigger('click');await flushPromises();expect(w.get('[data-testid="reader-current-path"]').text()).toBe('2.png');expect(mocks.invoke.mock.calls.filter(c=>c[0]==='read_archive_image')).toHaveLength(2)
      await w.get('[data-testid="reader-close"]').trigger('click');expect(w.find('[data-testid="archive-image-reader"]').exists()).toBe(false)
    }finally{w.unmount();vi.unstubAllGlobals()}
  })
  it('sevenz1618 image reader failures keep useful next actions without adapter tags',async()=>{
    const {readerError}=await import('@/composables/useArchiveImageReader')
    expect(readerError('[sevenz-preview:password] 7Z 加密内容未通过读取或校验，请核对密码与分卷来源')).toBe('7Z 加密内容未通过读取或校验，请核对密码与分卷来源')
    expect(readerError('[archive-inspection:incomplete] 7Z 文件或卷组不完整，请补齐分卷')).toBe('7Z 文件或卷组不完整，请补齐分卷')
  })
  it('sevenz1618 complete text remains available and images now open in the original modal',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'7'.repeat(64),totalFiles:2,totalDirectories:0,totalUncompressedSize:100,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:20},{path:'cover.png',name:'cover.png',size:72,isDir:false,compressedSize:50}]})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1617,path:'C:/资料.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-meta').text()).toContain('完整显示')
    expect(mocks.previewArchiveText.mock.calls.at(-1)![5]).toEqual({previewId:expect.stringMatching(/^7z-text-/),sourceRevision:'7'.repeat(64)})
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('7Z 支持 LZMA / LZMA2、1 MiB')
    await w.get('.preview-close').trigger('click');await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises();expect(mocks.previewArchiveImage).toHaveBeenCalledWith('C:/资料.7z','cover.png','',undefined,undefined,expect.objectContaining({previewId:expect.stringMatching(/^7z-image-/),sourceRevision:'7'.repeat(64)}));w.unmount()
  })

  it('sevenz1621 shared wait becomes complete text in the same pane with contextual help',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'9'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1621,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('正在读取固实包共享内容');expect(w.text()).not.toContain('压缩包浏览中心')
    finish({entryPath:'readme.txt',content:'共享块内说明',encoding:'UTF-8',byteSize:28,totalSize:28,truncated:false,lineCount:1});await flushPromises();expect(w.get('.preview-meta').text()).toContain('完整显示')
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('检查整个共享块后显示');expect(w.text()).toContain('AES 内容加密、文件名加密');w.unmount()
  })
  it('sevenz1621 closing a shared read cancels the exact request and discards late success',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'9'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1621,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();const r=mocks.previewArchiveText.mock.calls.at(-1)![5]
    await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(r.previewId);finish({entryPath:'readme.txt',content:'late',encoding:'UTF-8',byteSize:4,totalSize:4,truncated:false,lineCount:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1621 password or damage keeps the useful source hint without internal markers',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'1'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:null,encrypted:true}]})
    mocks.previewArchiveText.mockRejectedValueOnce('[sevenz-preview:password] 7Z 加密内容未通过读取或校验，请核对密码与分卷来源；必要时重新下载')
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1621,path:'C:/encrypted.7z'}},global:{stubs:{Teleport:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-stage').text()).toContain('核对密码与分卷来源');expect(w.get('.preview-stage').text()).not.toContain('sevenz-preview');expect(w.find('[data-testid="archive-text-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1621 image help keeps extraction for unsupported ranges',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'1'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:72,encrypted:true,entries:[{path:'cover.png',name:'cover.png',size:72,isDir:false,compressedSize:null,encrypted:true}]})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1621,path:'C:/encrypted.7z'}},global:{stubs:{Teleport:true}}});await flushPromises();await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises();await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('超限请先提取');w.unmount()
  })

  it('sevenz1619 shared wait becomes complete text in the same pane with contextual help',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'9'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1619,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('正在读取固实包共享内容');expect(w.text()).not.toContain('压缩包浏览中心')
    finish({entryPath:'readme.txt',content:'共享块内说明',encoding:'UTF-8',byteSize:28,totalSize:28,truncated:false,lineCount:1});await flushPromises();expect(w.get('.preview-meta').text()).toContain('完整显示')
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('检查整个共享块后显示');expect(w.text()).toContain('超限请先提取');w.unmount()
  })
  it('sevenz1619 closing a shared read cancels the exact request and discards late success',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'9'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1619,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();const r=mocks.previewArchiveText.mock.calls.at(-1)![5]
    await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(r.previewId);finish({entryPath:'readme.txt',content:'late',encoding:'UTF-8',byteSize:4,totalSize:4,truncated:false,lineCount:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1619 encrypted solid errors keep extraction available without displaying partial content',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'9'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:null,encrypted:true}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error('7Z 加密固实文本暂不支持预览，请先提取后打开'))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1619,path:'C:/solid.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-stage').text()).toContain('先提取后打开');expect(w.find('[data-testid="archive-text-preview"]').exists()).toBe(false);w.unmount()
  })

  it('sevenz1617 close cancels the exact request and ignores late text',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'7'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:20}]})
    let finish!:(value:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1617,path:'C:/资料.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    const request=mocks.previewArchiveText.mock.calls.at(-1)![5];await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(request.previewId)
    finish({entryPath:'readme.txt',content:'late',encoding:'UTF-8',byteSize:4,totalSize:4,truncated:false,lineCount:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('sevenz1617 encrypted failures explain the next action without internal tags',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'7Z',revision:'7'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:28,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:28,isDir:false,compressedSize:20,encrypted:true}]})
    mocks.previewArchiveText.mockRejectedValueOnce('[sevenz-preview:password] 7Z 加密文本未通过读取或校验，请核对密码与分卷来源；必要时重新下载')
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1617,path:'C:/资料.7z'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('核对密码与分卷来源');expect(w.get('.preview-stage').text()).not.toContain('sevenz-preview');w.unmount()
  })

  it('egg1616 large text labels partial content and explains unverified file integrity in the original modal',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'f'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:3000000,totalCompressedSize:8000,encrypted:false,entries:[{path:'session.txt',name:'session.txt',size:3000000,isDir:false,compressedSize:8000}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'session.txt',content:'INFO 包内日志开头',encoding:'UTF-8',byteSize:1048575,totalSize:3000000,truncated:true,lineCount:1})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1616,path:'C:/log.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="session.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-meta').text()).toContain('部分内容 · 未校验整文件');expect(w.get('.preview-meta').text()).not.toContain('完整显示')
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('非固实大文本仅显示前 1 MiB');w.unmount()
  })
  it('egg1616 a closed prefix request cannot display a late result',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'f'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:3000000,totalCompressedSize:8000,encrypted:false,entries:[{path:'session.txt',name:'session.txt',size:3000000,isDir:false,compressedSize:8000}]})
    let finish!:(value:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1616,path:'C:/log.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="session.txt"]').trigger('dblclick');await flushPromises()
    const request=mocks.previewArchiveText.mock.calls.at(-1)![5];await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(request.previewId)
    finish({entryPath:'session.txt',content:'late prefix',encoding:'UTF-8',byteSize:1048576,totalSize:3000000,truncated:true,lineCount:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })
  it('egg1616 input budget errors offer extraction without internal tags',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'f'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:3000000,totalCompressedSize:70000000,encrypted:false,entries:[{path:'session.txt',name:'session.txt',size:3000000,isDir:false,compressedSize:70000000}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error('[archive-inspection:io] EGG 文本前缀读取超过 64 MiB 范围，请先提取后打开'))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1616,path:'C:/log.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="session.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('需要扫描超过 64 MiB');expect(w.get('.preview-stage').text()).not.toContain('archive-inspection');w.unmount()
  })

  it('egg1615 solid image keeps the original modal, shared wait text and contextual limits',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'e'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:20,totalCompressedSize:8,encrypted:false,entries:[{path:'cover.png',name:'cover.png',size:20,isDir:false,compressedSize:null}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveImage.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1615,path:'C:/solid.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises();expect(w.get('.preview-stage').text()).toContain('正在读取固实包共享内容')
    finish({entryPath:'cover.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',width:320,height:240,byteSize:20});await flushPromises()
    expect(w.find('[data-testid="archive-image-preview"]').exists()).toBe(true)
    await w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('完整校验后显示');expect(w.text()).toContain('64 MiB');w.unmount()
  })

  it('egg1614 solid text keeps the original complete preview and its bounded shared-stream hint',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'d'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false,compressedSize:null}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'readme.txt',content:'固实包内说明',encoding:'UTF-8',byteSize:10,totalSize:10,truncated:false,lineCount:1})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1614,path:'C:/solid.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('[data-testid="archive-text-preview"]').text()).toContain('固实包内说明');expect(w.get('.preview-meta').text()).toContain('完整显示')
    const help=w.findAll('button').find(b=>b.attributes('aria-label')==='预览说明')!;await help.trigger('mouseenter');await flushPromises();expect(w.text()).toContain('共享压缩数据与解码总量');expect(w.text()).toContain('64 MiB');w.unmount()
  })

  it('egg1614 unsupported shared text gives an extraction hint without showing partial content',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'d'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:true,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false,compressedSize:null}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error('EGG 加密固实文本暂不支持预览，请先提取后打开'))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1614,path:'C:/solid.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('先提取后打开');expect(w.find('[data-testid="archive-text-preview"]').exists()).toBe(false);w.unmount()
  })

  it('EGG text and images use the existing modal with their own source context',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'c'.repeat(64),totalFiles:2,totalDirectories:0,totalUncompressedSize:30,totalCompressedSize:20,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false},{path:'cover.png',name:'cover.png',size:20,isDir:false}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'readme.txt',content:'EGG 包内说明',encoding:'UTF-8',byteSize:10,totalSize:10,truncated:false,lineCount:1})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1612,path:'C:/read.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    expect(w.get('[data-entry-path="cover.png"] .preview-trigger').attributes('disabled')).toBeUndefined()
    await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises();expect(w.get('[data-testid="archive-text-preview"]').text()).toContain('EGG 包内说明');expect(w.get('.preview-meta').text()).toContain('完整显示')
    expect(mocks.previewArchiveText).toHaveBeenCalledWith('C:/read.egg','readme.txt','',undefined,undefined,expect.objectContaining({sourceRevision:'c'.repeat(64),previewId:expect.stringMatching(/^egg-text-/)}));await w.get('.preview-close').trigger('click');await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises();expect(mocks.previewArchiveImage).toHaveBeenCalledWith('C:/read.egg','cover.png','',undefined,undefined,expect.objectContaining({sourceRevision:'c'.repeat(64),previewId:expect.stringMatching(/^egg-image-/)}));w.unmount()
  })

  it('EGG closing a pending preview cancels it and ignores late content',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'c'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false}]})
    let finish!:(v:unknown)=>void;mocks.previewArchiveText.mockReturnValueOnce(new Promise(resolve=>finish=resolve))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1612,path:'C:/read.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    const context=mocks.previewArchiveText.mock.calls.at(-1)![5];await w.get('.preview-close').trigger('click');expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(context.previewId)
    finish({entryPath:'readme.txt',content:'late EGG result',encoding:'UTF-8',byteSize:1,totalSize:1,truncated:false,lineCount:1});await flushPromises();expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })

  it('EGG preview errors give the cause without internal classification tags',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'c'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error('[archive-inspection:damaged] EGG 内容、大小或 CRC 校验失败'))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1612,path:'C:/read.egg'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises();await w.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('EGG 内容、大小或 CRC 校验失败');expect(w.get('.preview-stage').text()).not.toContain('archive-inspection');w.unmount()
  })

  it('ALZ large text explicitly shows partial content without whole-file verification',async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'ALZ',revision:'b'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:3000000,totalCompressedSize:8000,encrypted:false,entries:[{path:'session.txt',name:'session.txt',size:3000000,isDir:false}]})
    mocks.previewArchiveText.mockResolvedValueOnce({entryPath:'session.txt',content:'INFO 日志开头',encoding:'UTF-8',byteSize:1048574,totalSize:3000000,truncated:true,lineCount:1})
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1611,path:'C:/logs.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="session.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('[data-testid="archive-text-preview"]').text()).toContain('日志开头')
    expect(w.get('.preview-meta').text()).toContain('部分内容 · 未校验整文件');expect(w.get('.preview-meta').text()).not.toContain('完整显示')
    expect(mocks.previewArchiveText).toHaveBeenCalledWith('C:/logs.alz','session.txt','',undefined,undefined,expect.objectContaining({sourceRevision:'b'.repeat(64)}));w.unmount()
  })

  it('ALZ prefix scan limit gives an extraction hint without internal I/O markers', async () => {
    mocks.browseArchive.mockResolvedValueOnce({format:'ALZ',revision:'b'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:3000000,totalCompressedSize:70000000,encrypted:false,entries:[{path:'session.txt',name:'session.txt',size:3000000,isDir:false}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error('[archive-inspection:io] [alz-io] ALZ 文件读写失败: ALZ 文本前缀读取超过 64 MiB 范围，请先提取后打开'))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1611,path:'C:/logs.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="session.txt"]').trigger('dblclick');await flushPromises()
    expect(w.get('.preview-stage').text()).toContain('读取这段文本需要扫描超过 64 MiB，已停止预览。请先提取后打开。')
    expect(w.get('.preview-stage').text()).not.toMatch(/archive-inspection|alz-io|文件读写失败/)
    expect(w.find('[data-testid="archive-text-preview"]').exists()).toBe(false);w.unmount()
  })

  it('ALZ stale preview asks to refresh without exposing an internal marker', async () => {
    mocks.browseArchive.mockResolvedValueOnce({format:'ALZ',revision:'a'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false}]})
    mocks.previewArchiveText.mockRejectedValueOnce(new Error('IMAGE_READER_CHANGED|来源无法确认'))
    const wrapper=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:169,path:'C:/sample.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await wrapper.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(wrapper.get('.preview-stage').text()).toContain('刷新此栏后重试')
    expect(wrapper.get('.preview-stage').text()).not.toContain('IMAGE_READER_CHANGED')
    expect(wrapper.find('[data-testid="archive-text-preview"]').exists()).toBe(false);wrapper.unmount()
  })

  it('ALZ text and image open inside the existing preview with a bound source', async () => {
    mocks.browseArchive.mockResolvedValueOnce({format:'ALZ',revision:'a'.repeat(64),totalFiles:2,totalDirectories:0,totalUncompressedSize:30,totalCompressedSize:20,encrypted:false,entries:[
      {path:'readme.txt',name:'readme.txt',size:10,isDir:false}, {path:'image.png',name:'image.png',size:20,isDir:false},
    ]})
    const wrapper=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:169,path:'C:/sample.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}})
    await flushPromises()
    expect(wrapper.get('[data-entry-path="image.png"] .preview-trigger').attributes('disabled')).toBeUndefined()
    await wrapper.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    expect(mocks.previewArchiveText).toHaveBeenCalledWith('C:/sample.alz','readme.txt','',undefined,undefined,expect.objectContaining({previewId:expect.stringContaining('alz-text-'),sourceRevision:'a'.repeat(64)}))
    expect(wrapper.get('[data-testid="archive-text-preview"]').text()).toContain('你好，归档文本')
    expect(mocks.previewArchiveImage).not.toHaveBeenCalled()
    await wrapper.get('.preview-close').trigger('click')
    await wrapper.get('[data-entry-path="image.png"]').trigger('dblclick');await flushPromises()
    expect(mocks.previewArchiveImage).toHaveBeenCalledWith('C:/sample.alz','image.png','',undefined,undefined,expect.objectContaining({previewId:expect.stringContaining('alz-image-'),sourceRevision:'a'.repeat(64)}))
    expect(wrapper.find('[data-testid="archive-entry-preview"]').exists()).toBe(true);wrapper.unmount()
  })

  it('ALZ closes its in-flight native preview and ignores a late result', async () => {
    mocks.browseArchive.mockResolvedValueOnce({format:'ALZ',revision:'a'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'readme.txt',name:'readme.txt',size:10,isDir:false}]})
    let finish!:(value:any)=>void
    mocks.previewArchiveText.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
    const wrapper=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:169,path:'C:/sample.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await wrapper.get('[data-entry-path="readme.txt"]').trigger('dblclick');await flushPromises()
    const id=mocks.previewArchiveText.mock.calls.at(-1)![5].previewId
    await wrapper.get('.preview-close').trigger('click');await flushPromises()
    expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(id)
    finish({content:'late response',encoding:'UTF-8',byteSize:13,totalSize:13,truncated:false,lineCount:1});await flushPromises()
    expect(wrapper.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);wrapper.unmount()
  })

  it('ALZ image close cancels its own request and never shows a late image', async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'ALZ',revision:'a'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:20,totalCompressedSize:8,encrypted:false,entries:[{path:'cover.png',name:'cover.png',size:20,isDir:false}]})
    let finish!:(value:any)=>void
    mocks.previewArchiveImage.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1610,path:'C:/pictures.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises()
    const id=mocks.previewArchiveImage.mock.calls.at(-1)![5].previewId
    await w.get('.preview-close').trigger('click');await flushPromises();expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(id)
    finish({entryPath:'cover.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:20,width:1,height:1});await flushPromises()
    expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })

  it('EGG image close cancels its own request and never shows a late image', async()=>{
    mocks.browseArchive.mockResolvedValueOnce({format:'EGG',revision:'a'.repeat(64),totalFiles:1,totalDirectories:0,totalUncompressedSize:20,totalCompressedSize:8,encrypted:false,entries:[{path:'cover.png',name:'cover.png',size:20,isDir:false}]})
    let finish!:(value:any)=>void
    mocks.previewArchiveImage.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
    const w=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1610,path:'C:/pictures.alz'}},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await w.get('[data-entry-path="cover.png"]').trigger('dblclick');await flushPromises()
    const id=mocks.previewArchiveImage.mock.calls.at(-1)![5].previewId
    await w.get('.preview-close').trigger('click');await flushPromises();expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(id)
    finish({entryPath:'cover.png',dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:20,width:1,height:1});await flushPromises()
    expect(w.find('[data-testid="archive-entry-preview"]').exists()).toBe(false);w.unmount()
  })

  it('keeps parent navigation actions inside the wrapping header', () => {
    const wrapper = mount(ArchiveBrowserView, { slots: { 'header-actions': '<button data-testid="parent-navigation">返回双栏</button>' } })
    expect(wrapper.get('header .browser-header-actions [data-testid="parent-navigation"]').text()).toBe('返回双栏')
    expect(wrapper.get('header .browser-primary').text()).toContain('打开压缩包')
  })

  it('loads structured entries and extracts only the checked files', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(mocks.browseArchive).toHaveBeenCalledWith('C:/archives/demo.zip', '', expect.any(String))
    expect(wrapper.text()).toContain('docs')
    expect(wrapper.text()).toContain('image.png')

    await wrapper.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
    await wrapper.find('footer .browser-primary').trigger('click')
    await flushPromises()

    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/archives/demo.zip', expect.objectContaining({
      outputPath: 'C:/archives',
      keepStructure: true,
      selectedEntries: ['docs/readme.txt'],
      conflictPolicy: 'rename',
    }))
    expect(mocks.setSuccess).toHaveBeenCalledWith('已解压 1 个所选文件')
  })

  it('shows unknown Z expanded size without treating the single file as empty', async () => {
    mocks.browseArchive.mockResolvedValueOnce({format:'Z',totalFiles:1,totalDirectories:0,
      totalUncompressedSize:0,totalCompressedSize:6,encrypted:false,entries:[{
        path:'说明.txt',name:'说明.txt',size:0,sizeKnown:false,compressedSize:6,
        modified:null,crc:null,encrypted:false,isDir:false,
      }]})
    const wrapper=mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click'); await flushPromises()
    expect(wrapper.get('[data-entry-path="说明.txt"]').text()).toContain('大小未知')
    expect(wrapper.get('.browser-summary').text()).toContain('大小未知')
    expect(wrapper.get('.browser-summary').text()).not.toContain('展开后 0 B')
  })

  it('cancels an active archive metadata read and shows a clear idle result', async () => {
    mocks.browseArchive.mockReturnValueOnce(new Promise(() => undefined))
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.get('[data-testid="archive-browse-cancel"]').text()).toContain('取消读取')
    await wrapper.get('[data-testid="archive-browse-cancel"]').trigger('click')
    await flushPromises()

    expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(expect.any(String))
    expect(wrapper.get('[data-testid="archive-browse-notice"]').text()).toContain('已取消读取压缩包内容')
  })

  it('shows object-specific context actions without changing selection', async () => {
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')
    await wrapper.get('[data-entry-path="image.png"]').trigger('contextmenu', { clientX: 80, clientY: 90 })

    const menu = document.querySelector('[data-testid="archive-context-menu"]') as HTMLElement
    expect(menu).not.toBeNull()
    expect(menu.textContent).toContain('image.png')
    expect(menu.querySelector('[data-testid="archive-context-preview"]')).not.toBeNull()
    expect(menu.querySelector('[data-testid="archive-context-default-open"]')).not.toBeNull()
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')

    ;(menu.querySelector('[data-testid="archive-context-copy-path"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(mocks.clipboardWrite).toHaveBeenCalledWith('image.png')
    expect(mocks.setSuccess).toHaveBeenCalledWith('已复制归档内路径')
    wrapper.unmount()
  })

  it('opens ordinary entries with the Windows default application route', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="image.png"]').trigger('dblclick')
    await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenCalledWith('C:/archives/demo.zip', 'image.png', '', false)
    expect(mocks.setSuccess).toHaveBeenCalledWith('已使用默认应用打开：image.png')
  })

  it('keeps active content unopened until the explicit warning is confirmed', async () => {
    mocks.browseArchive.mockResolvedValueOnce({
      format: 'ZIP', totalFiles: 1, totalDirectories: 0,
      totalUncompressedSize: 8, totalCompressedSize: 8, encrypted: false,
      entries: [{ path: 'scripts/run.cmd', name: 'run.cmd', size: 8, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
    })
    mocks.openArchiveEntry
      .mockResolvedValueOnce({ status: 'confirmationRequired', entryPath: 'scripts/run.cmd', cachePath: null, dangerous: true })
      .mockResolvedValueOnce({ status: 'opened', entryPath: 'scripts/run.cmd', cachePath: 'C:/cache/run.cmd', dangerous: true })
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="scripts/"]').trigger('dblclick')
    await wrapper.get('[data-entry-path="scripts/run.cmd"]').trigger('dblclick')
    await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenCalledTimes(1)
    expect(mocks.openArchiveEntry).toHaveBeenLastCalledWith('C:/archives/demo.zip', 'scripts/run.cmd', '', false)
    expect(document.querySelector('[data-testid="archive-dangerous-open-dialog"]')).not.toBeNull()
    expect((document.querySelector('[data-testid="archive-dangerous-cancel"]') as HTMLButtonElement).autofocus).toBe(true)

    ;(document.querySelector('[data-testid="archive-dangerous-confirm"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(mocks.openArchiveEntry).toHaveBeenLastCalledWith('C:/archives/demo.zip', 'scripts/run.cmd', '', true)
    wrapper.unmount()
  })

  it('routes context extraction and details through existing safe entry semantics', async () => {
    mocks.selectDirectory.mockResolvedValue('C:/archives/custom-output')
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
    await wrapper.find('input[placeholder="搜索文件名或路径"]').setValue('readme')
    const readmeRow = wrapper.get('[data-entry-path="docs/readme.txt"]')
    await readmeRow.trigger('contextmenu', { clientX: 100, clientY: 110 })
    ;(document.querySelector('[data-testid="archive-context-details"]') as HTMLButtonElement).click()
    await flushPromises()
    expect((document.querySelector('[data-testid="archive-entry-details"]') as HTMLElement).textContent).toContain('docs/readme.txt')
    ;(document.querySelector('[aria-label="关闭条目详情"]') as HTMLButtonElement).click()

    await readmeRow.trigger('contextmenu', { clientX: 100, clientY: 110 })
    ;(document.querySelector('[data-testid="archive-context-extract-choose"]') as HTMLButtonElement).click()
    await flushPromises()
    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/archives/demo.zip', expect.objectContaining({
      outputPath: 'C:/archives/custom-output',
      selectedEntries: ['docs/readme.txt'],
      conflictPolicy: 'rename',
    }))
    wrapper.unmount()
  })

  it('provides keyboard equivalents for context menu actions', async () => {
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    const imageRow = wrapper.get('[data-entry-path="image.png"]')
    await imageRow.get('.browser-checkbox').trigger('click')
    await imageRow.trigger('click')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'F10', shiftKey: true })
    expect(document.querySelector('[data-testid="archive-context-menu"]')).not.toBeNull()
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Escape' })
    expect(document.querySelector('[data-testid="archive-context-menu"]')).toBeNull()

    await wrapper.get('.browser-page').trigger('keydown', { key: 'c', ctrlKey: true, shiftKey: true })
    await flushPromises()
    expect(mocks.clipboardWrite).toHaveBeenCalledWith('image.png')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Enter', altKey: true })
    expect(document.querySelector('[data-testid="archive-entry-details"]')).not.toBeNull()
    ;(document.querySelector('[aria-label="关闭条目详情"]') as HTMLButtonElement).click()

    await wrapper.get('[data-entry-path="docs/"]').trigger('click')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'c', ctrlKey: true, shiftKey: true })
    await flushPromises()
    expect(mocks.clipboardWrite).toHaveBeenLastCalledWith('docs')
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Enter', altKey: true })
    expect((document.querySelector('[data-testid="archive-entry-details"]') as HTMLElement).textContent).toContain('docs')
    expect((document.querySelector('[data-testid="archive-entry-details"]') as HTMLElement).textContent).not.toContain('docs/readme.txt')
    wrapper.unmount()
  })

  it('uses file-manager focus, multiselect and directory navigation semantics', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    const imageRow = wrapper.get('[data-entry-path="image.png"]')
    await imageRow.trigger('click')
    expect(imageRow.classes()).toContain('focused')
    expect(wrapper.find('footer').text()).toContain('已选择 2 / 2 个文件')

    await imageRow.trigger('click', { ctrlKey: true })
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')

    await wrapper.get('[data-entry-path="docs/"]').trigger('dblclick')
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).toContain('docs')
    expect(wrapper.get('[data-entry-path="docs/readme.txt"]').text()).toContain('readme.txt')
    expect(wrapper.get('[data-testid="archive-nav-back"]').attributes('disabled')).toBeUndefined()

    await wrapper.get('[data-testid="archive-nav-refresh"]').trigger('click')
    await flushPromises()
    expect(mocks.browseArchive).toHaveBeenCalledTimes(2)
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).toContain('docs')
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')

    await wrapper.get('.browser-page').trigger('keydown', { key: 'Backspace' })
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).not.toContain('docs')
    expect(wrapper.get('[data-entry-path="docs/"]').exists()).toBe(true)

    await wrapper.get('[data-testid="archive-nav-back"]').trigger('click')
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).toContain('docs')
    await wrapper.get('[data-testid="archive-nav-forward"]').trigger('click')
    expect(wrapper.get('[data-testid="archive-breadcrumbs"]').text()).not.toContain('docs')
  })

  it('filters entries by directory, search text and type without horizontal overflow containers', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.find('input[placeholder="搜索文件名或路径"]').setValue('readme')
    expect(wrapper.findAll('.browser-row')).toHaveLength(1)
    expect(wrapper.find('.browser-row').text()).toContain('readme.txt')
    expect(wrapper.find('.browser-workspace').classes()).toContain('overflow-hidden')
    expect(wrapper.find('.browser-workspace [class*="overflow-y-auto"]').exists()).toBe(true)
  })

  it('renders archive folders as a collapsible hierarchy instead of full-path rows', async () => {
    mocks.browseArchive.mockResolvedValueOnce({
      format: 'ZIP', totalFiles: 1, totalDirectories: 2,
      totalUncompressedSize: 10, totalCompressedSize: 8, encrypted: false,
      entries: [
        { path: 'docs/guides/readme.txt', name: 'readme.txt', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false },
      ],
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.find('.directory-pane').text()).toContain('docs')
    expect(wrapper.find('.directory-pane').text()).not.toContain('guides')
    await wrapper.get('.directory-toggle').trigger('click')
    expect(wrapper.find('.directory-pane').text()).toContain('guides')
    expect(wrapper.find('.directory-pane').text()).not.toContain('docs/guides')
  })

  it('previews a bounded raster entry without changing its selection', async () => {
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.find('footer').text()).toContain('已选择 2 / 2 个文件')
    await wrapper.get('.preview-trigger').trigger('click')
    await flushPromises()

    expect(mocks.previewArchiveImage).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'image.png', '',
    )
    const preview = new DOMWrapper(document.body)
    expect(preview.get('[data-testid="archive-image-preview"] img').attributes('src'))
      .toBe('data:image/png;base64,c2FmZQ==')
    expect(preview.get('[data-testid="archive-entry-preview"]').text()).toContain('只读')
    expect(wrapper.find('footer').text()).toContain('已选择 2 / 2 个文件')

    await preview.get('[aria-label="关闭预览"]').trigger('click')
    expect(preview.find('[data-testid="archive-image-preview"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it.each([
    ['provided password is incorrect', '密码不正确，请修改密码后重试。'],
    ['CRC32 checksum failed', 'CRC32 checksum failed'],
    ['unsupported compression method', 'unsupported compression method'],
  ])('explains the exact native preview error without relabeling damage: %s', async (error, expected) => {
    mocks.previewArchiveImage.mockRejectedValueOnce(new Error(error))
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('.preview-trigger').trigger('click')
    await flushPromises()
    const preview = new DOMWrapper(document.body)
    expect(preview.get('.preview-stage').text()).toContain(expected)
    expect(preview.find('[data-testid="archive-image-preview"]').exists()).toBe(false)
    if (error !== 'provided password is incorrect') expect(preview.get('.preview-stage').text()).not.toContain('密码')
    await preview.get('[aria-label="关闭预览"]').trigger('click')
    wrapper.unmount()
  })

  it('previews bounded decoded text and explains internal versus default opening', async () => {
    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    await wrapper.get('[data-entry-path="docs/"]').trigger('dblclick')
    await wrapper.get('[data-entry-path="docs/readme.txt"] .preview-trigger').trigger('click')
    await flushPromises()

    expect(mocks.previewArchiveText).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'docs/readme.txt', '',
    )
    const preview = new DOMWrapper(document.body)
    expect(preview.get('[data-testid="archive-text-preview"]').text()).toContain('你好，归档文本')
    expect(preview.get('[data-testid="archive-entry-preview"]').text()).toContain('UTF-8')
    expect(preview.get('[data-testid="archive-entry-preview"]').text()).toContain('完整显示')
    expect(mocks.openArchiveEntry).not.toHaveBeenCalled()
    await wrapper.get('.browser-page').trigger('keydown', { key: 'Escape' })
    expect(preview.find('[data-testid="archive-entry-preview"]').exists()).toBe(false)
    wrapper.unmount()
  })

  it('keeps preview disabled when the archive route cannot prove bounded reading', async () => {
    mocks.browseArchive.mockResolvedValueOnce({
      format: '7Z', totalFiles: 1, totalDirectories: 0,
      totalUncompressedSize: 20, totalCompressedSize: 12, encrypted: false,
      entries: [{ path: 'image.png', name: 'image.png', size: 20, compressedSize: 12, modified: null, crc: null, encrypted: false, isDir: false }],
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()

    expect(wrapper.get('.preview-trigger').attributes('disabled')).toBeDefined()
    expect(wrapper.get('.preview-trigger').attributes('title')).toContain('暂不支持此类预览')
    expect(mocks.previewArchiveImage).not.toHaveBeenCalled()
  })

  it('enters nested archives, preserves outer state, and enforces the three-layer UI boundary', async () => {
    mocks.browseArchive
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 2, totalDirectories: 0,
        totalUncompressedSize: 30, totalCompressedSize: 20, encrypted: false,
        entries: [
          { path: 'middle.7z', name: 'middle.7z', size: 20, compressedSize: 12, modified: null, crc: null, encrypted: false, isDir: false },
          { path: 'outer-note.txt', name: 'outer-note.txt', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false },
        ],
      })
      .mockResolvedValueOnce({
        format: '7Z', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 12, totalCompressedSize: 8, encrypted: false,
        entries: [{ path: 'inner.zip', name: 'inner.zip', size: 12, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
      })
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 6, totalCompressedSize: 4, encrypted: false,
        entries: [{ path: 'fourth.zip', name: 'fourth.zip', size: 6, compressedSize: 4, modified: null, crc: null, encrypted: false, isDir: false }],
      })
    mocks.materializeNestedArchive
      .mockResolvedValueOnce({ entryPath: 'middle.7z', cachePath: 'C:/cache/middle.7z', parentSha256: 'outer-hash', contentSha256: 'middle-hash', depth: 2 })
      .mockResolvedValueOnce({ entryPath: 'inner.zip', cachePath: 'C:/cache/inner.zip', parentSha256: 'middle-hash', contentSha256: 'inner-hash', depth: 3 })

    const wrapper = mount(ArchiveBrowserView, { attachTo: document.body })
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('[data-entry-path="outer-note.txt"] .browser-checkbox').trigger('click')
    await wrapper.get('[data-entry-path="middle.7z"]').trigger('dblclick')
    await flushPromises()

    expect(mocks.materializeNestedArchive).toHaveBeenLastCalledWith(
      'C:/archives/demo.zip', 'middle.7z', '', 2, [],
    )
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('demo.zip')
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('middle.7z')
    expect(wrapper.get('input[type="password"]').element.value).toBe('')

    await wrapper.get('input[type="password"]').setValue('middle-only-password')
    await wrapper.get('[data-entry-path="inner.zip"]').trigger('dblclick')
    await flushPromises()
    expect(mocks.materializeNestedArchive).toHaveBeenLastCalledWith(
      'C:/cache/middle.7z', 'inner.zip', 'middle-only-password', 3, ['outer-hash', 'middle-hash'],
    )
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('3 / 3 层')

    await wrapper.get('[data-entry-path="fourth.zip"]').trigger('contextmenu', { clientX: 100, clientY: 100 })
    const nestedButton = document.querySelector('[data-testid="archive-context-enter-nested"]') as HTMLButtonElement
    expect(nestedButton.disabled).toBe(true)
    expect(nestedButton.textContent).toContain('已达到 3 层上限')
    ;(document.querySelector('[data-testid="archive-chain"] button') as HTMLButtonElement).click()
    await flushPromises()
    expect(wrapper.get('[data-entry-path="middle.7z"]').exists()).toBe(true)
    expect(wrapper.find('footer').text()).toContain('已选择 1 / 2 个文件')
    wrapper.unmount()
  })

  it('does not inherit an outer password when an inner archive requires its own password', async () => {
    mocks.browseArchive
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 10, totalCompressedSize: 8, encrypted: true,
        entries: [{ path: 'locked.7z', name: 'locked.7z', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
      })
      .mockRejectedValueOnce(new Error('Unable to read 7Z metadata: PasswordRequired'))
      .mockResolvedValueOnce({
        format: '7Z', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 4, totalCompressedSize: 4, encrypted: true,
        entries: [{ path: 'secret.txt', name: 'secret.txt', size: 4, compressedSize: 4, modified: null, crc: null, encrypted: true, isDir: false }],
      })
    mocks.materializeNestedArchive.mockResolvedValueOnce({
      entryPath: 'locked.7z', cachePath: 'C:/cache/locked.7z', parentSha256: 'outer-hash', contentSha256: 'locked-hash', depth: 2,
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('input[type="password"]').setValue('outer-password')
    await wrapper.get('[data-entry-path="locked.7z"]').trigger('dblclick')
    await flushPromises()

    expect(mocks.materializeNestedArchive).toHaveBeenCalledWith(
      'C:/archives/demo.zip', 'locked.7z', 'outer-password', 2, [],
    )
    expect(wrapper.get('input[type="password"]').element.value).toBe('')
    expect(wrapper.text()).toContain('内层归档尚未打开')
    expect(mocks.setError).not.toHaveBeenCalled()
    await wrapper.get('input[type="password"]').setValue('inner-password')
    await wrapper.get('[data-testid="archive-nested-retry"]').trigger('click')
    await flushPromises()
    expect(mocks.browseArchive).toHaveBeenLastCalledWith('C:/cache/locked.7z', 'inner-password', expect.any(String))
  })

  it('ignores a late inner browse result after the user returns to the outer archive', async () => {
    let resolveNested!: (value: any) => void
    const pendingNested = new Promise(resolve => { resolveNested = resolve })
    mocks.browseArchive
      .mockResolvedValueOnce({
        format: 'ZIP', totalFiles: 1, totalDirectories: 0,
        totalUncompressedSize: 10, totalCompressedSize: 8, encrypted: false,
        entries: [{ path: 'slow.7z', name: 'slow.7z', size: 10, compressedSize: 8, modified: null, crc: null, encrypted: false, isDir: false }],
      })
      .mockReturnValueOnce(pendingNested)
    mocks.materializeNestedArchive.mockResolvedValueOnce({
      entryPath: 'slow.7z', cachePath: 'C:/cache/slow.7z', parentSha256: 'outer-hash', contentSha256: 'slow-hash', depth: 2,
    })
    const wrapper = mount(ArchiveBrowserView)
    await wrapper.find('header .browser-primary').trigger('click')
    await flushPromises()
    await wrapper.get('[data-entry-path="slow.7z"]').trigger('dblclick')
    await flushPromises()
    await wrapper.get('[data-testid="archive-chain"] button').trigger('click')
    expect(mocks.cancelArchiveBrowse).toHaveBeenCalledWith(expect.any(String))
    resolveNested({
      format: '7Z', totalFiles: 1, totalDirectories: 0,
      totalUncompressedSize: 4, totalCompressedSize: 4, encrypted: false,
      entries: [{ path: 'late.txt', name: 'late.txt', size: 4, compressedSize: 4, modified: null, crc: null, encrypted: false, isDir: false }],
    })
    await flushPromises()
    expect(wrapper.get('[data-entry-path="slow.7z"]').exists()).toBe(true)
    expect(wrapper.find('[data-entry-path="late.txt"]').exists()).toBe(false)
    expect(wrapper.get('[data-testid="archive-chain"]').text()).toContain('1 / 3 层')
  })
  it('reader1557 passes the current source revision to reading and locates without changing selection',async()=>{
    const entry=(path:string)=>({path,name:path.split('/').pop(),isDir:false,size:10,compressedSize:8,modified:null,crc:null,encrypted:false})
    mocks.browseArchive.mockResolvedValueOnce({revision:'reader-source-version',format:'ZIP',totalFiles:2,totalDirectories:1,totalUncompressedSize:20,totalCompressedSize:16,encrypted:false,entries:[entry('1.png'),entry('docs/2.png')]})
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{ArchiveImageReader:{props:['revision','filePath','entries'],emits:['locate'],template:`<div data-testid="reader-stub" :data-revision="revision"><button @click="$emit('locate','docs/2.png')">locate</button></div>`}}}})
    await wrapper.get('header .browser-primary').trigger('click');await flushPromises();const before=wrapper.get('footer').text();await wrapper.get('[data-testid="archive-reader-open"]').trigger('click');expect(wrapper.get('[data-testid="reader-stub"]').attributes('data-revision')).toBe('reader-source-version');await wrapper.get('[data-testid="reader-stub"] button').trigger('click');await flushPromises();expect(wrapper.get('[data-entry-path="docs/2.png"]').classes()).toContain('focused');expect(wrapper.get('footer').text()).toBe(before);expect(wrapper.find('[data-testid="reader-stub"]').exists()).toBe(false);wrapper.unmount()
  })

  it('reader1557 actual reader mounts from the browser and closed readers do not steal password focus',async()=>{
    vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:actual-reader'),revokeObjectURL:vi.fn()})
    mocks.browseArchive.mockResolvedValueOnce({revision:'reader-source-version',format:'ZIP',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'1.png',name:'1.png',isDir:false,size:10,compressedSize:8,modified:null,crc:null,encrypted:false}]})
    mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?{entryPath:args.entryPath,dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1}:undefined)
    const wrapper=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{await wrapper.get('header .browser-primary').trigger('click');await flushPromises();(wrapper.get('[data-testid="archive-reader-open"]').element as HTMLButtonElement).focus();await wrapper.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises();expect(wrapper.get('[data-testid="reader-current-path"]').text()).toBe('1.png');expect(wrapper.get('[data-testid="reader-image"]').attributes('src')).toBe('blob:actual-reader');await wrapper.get('[data-testid="reader-close"]').trigger('click');await flushPromises();const input=wrapper.get('.browser-toolbar input[type="password"]');(input.element as HTMLInputElement).focus();await input.setValue('new-source-password');await flushPromises();expect(document.activeElement).toBe(input.element);expect(wrapper.find('[data-testid="archive-image-reader"]').exists()).toBe(false)}finally{wrapper.unmount();vi.unstubAllGlobals()}
  })

  it('egg1613 actual reader mounts with the EGG source and keeps the original close behavior',async()=>{
    vi.stubGlobal('URL',{createObjectURL:vi.fn(()=> 'blob:actual-reader'),revokeObjectURL:vi.fn()})
    mocks.browseArchive.mockResolvedValueOnce({revision:'reader-source-version',format:'EGG',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'1.png',name:'1.png',isDir:false,size:10,compressedSize:8,modified:null,crc:null,encrypted:false}]})
    mocks.invoke.mockImplementation(async(command,args)=>command==='read_archive_image'?{entryPath:args.entryPath,dataUrl:'data:image/png;base64,AA==',mimeType:'image/png',byteSize:1,width:1,height:1}:undefined)
    const wrapper=mount(ArchiveBrowserView,{attachTo:document.body,global:{stubs:{Teleport:true}}})
    try{await wrapper.get('header .browser-primary').trigger('click');await flushPromises();(wrapper.get('[data-testid="archive-reader-open"]').element as HTMLButtonElement).focus();await wrapper.get('[data-testid="archive-reader-open"]').trigger('click');await flushPromises();expect(wrapper.get('[data-testid="reader-current-path"]').text()).toBe('1.png');expect(wrapper.get('[data-testid="reader-image"]').attributes('src')).toBe('blob:actual-reader');await wrapper.get('[data-testid="reader-close"]').trigger('click');await flushPromises();const input=wrapper.get('.browser-toolbar input[type="password"]');(input.element as HTMLInputElement).focus();await input.setValue('new-source-password');await flushPromises();expect(document.activeElement).toBe(input.element);expect(wrapper.find('[data-testid="archive-image-reader"]').exists()).toBe(false)}finally{wrapper.unmount();vi.unstubAllGlobals()}
  })

  it.each([false,true])('reader1558 recent reading reloads the canonical source and passes path-based resume even if original page is missing=%s',async missing=>{
    const {useArchiveReadingStore}=await import('@/stores/archiveReading')
    const revision='a'.repeat(64),canonical='C:/archives/pictures.zip.001',path=missing?'gone.png':'image.png'
    useArchiveReadingStore().save({filePath:canonical,format:'ZIP',revision,path,directory:'',scope:'archive',fit:false,zoom:1.25,selected:[path],updatedAt:new Date().toISOString()})
    mocks.browseArchive.mockResolvedValueOnce({revision,sourcePath:canonical,format:'ZIP',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'image.png',name:'image.png',size:10,compressedSize:8,modified:null,crc:null,encrypted:false,isDir:false}]})
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{ArchiveImageReader:{props:['filePath','resume'],template:'<div data-testid="resume-reader" :data-source="filePath" :data-path="resume.path"/>'}}}})
    expect(mocks.browseArchive).not.toHaveBeenCalled();await wrapper.get('[data-testid="reading-recent-open"]').trigger('click');await flushPromises();expect(mocks.browseArchive).toHaveBeenCalledWith(canonical,'',expect.any(String));expect(wrapper.get('[data-testid="resume-reader"]').attributes('data-source')).toBe(canonical);expect(wrapper.get('[data-testid="resume-reader"]').attributes('data-path')).toBe(path);expect(mocks.decompressFile).not.toHaveBeenCalled();wrapper.unmount()
  })

  it('reader1558 current single image keeps its exact reference in the original immediate extraction flow',async()=>{
    const revision='a'.repeat(64);mocks.browseArchive.mockResolvedValueOnce({revision,format:'ZIP',totalFiles:1,totalDirectories:0,totalUncompressedSize:10,totalCompressedSize:8,encrypted:false,entries:[{path:'image.png',name:'image.png',size:10,compressedSize:8,modified:null,crc:null,encrypted:false,isDir:false}]});mocks.selectDirectory.mockResolvedValue('D:/out')
    const wrapper=mount(ArchiveBrowserView,{global:{stubs:{ArchiveImageReader:{emits:['extract'],template:`<button data-testid="single-image-take" @click="$emit('extract','image.png')"/>`}}}});await wrapper.get('header .browser-primary').trigger('click');await flushPromises();await wrapper.get('[data-testid="archive-reader-open"]').trigger('click');await wrapper.get('[data-testid="single-image-take"]').trigger('click');await flushPromises();expect(mocks.decompressFile).toHaveBeenCalledWith('C:/archives/demo.zip',expect.objectContaining({selectedEntries:['image.png'],readerSelection:{revision,paths:['image.png'],keepStructure:true,conflictPolicy:'rename'}}));wrapper.unmount()
  })

  it('dual panes use independent sources, passwords and selections and extract to the opposite directory', async () => {
    mocks.pendingPath='C:/unrelated.zip'
    const left=mount(ArchiveBrowserView,{props:{embedded:true,active:true,source:{id:1,path:'C:/left.zip',options:{password:'left-key'}},destination:'D:/target'},global:{stubs:{Teleport:true,RouterLink:true}}})
    const right=mount(ArchiveBrowserView,{props:{embedded:true,active:false,source:{id:2,path:'C:/right.zip',options:{password:'right-key'}},destination:'C:/target'},global:{stubs:{Teleport:true,RouterLink:true}}})
    await flushPromises()
    expect(mocks.pendingPath).toBe('C:/unrelated.zip')
    expect(mocks.browseArchive).toHaveBeenCalledWith('C:/left.zip','left-key',expect.any(String))
    expect(mocks.browseArchive).toHaveBeenCalledWith('C:/right.zip','right-key',expect.any(String))
    expect(left.text()).not.toContain('压缩包浏览中心')
    expect(left.get('.archive-options').isVisible()).toBe(false)
    expect(left.get('footer').text()).toContain('已选择 0')
    await left.get('[data-entry-path="image.png"] .browser-checkbox').trigger('click')
    expect(right.get('footer').text()).toContain('已选择 0')
    await left.get('footer .browser-primary').trigger('click');await flushPromises()
    expect(mocks.decompressFile).toHaveBeenCalledWith('C:/left.zip',expect.objectContaining({outputPath:'D:/target',password:'left-key',selectedEntries:['image.png']}))
    expect(left.emitted('extracted')?.[0]).toEqual(['D:/target'])
    left.unmount();right.unmount()
  })
  it('dual-pane text opens internally and keyboard refresh affects only the active pane', async () => {
    const left=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1,path:'C:/left.zip'},active:true},global:{stubs:{Teleport:true,RouterLink:true}}})
    const right=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:2,path:'C:/right.zip'},active:false},global:{stubs:{Teleport:true,RouterLink:true}}});await flushPromises()
    await left.get('[data-entry-path="docs/"]').trigger('dblclick')
    await left.get('[data-entry-path="docs/readme.txt"]').trigger('dblclick');await flushPromises()
    expect(left.get('[data-testid="archive-text-preview"]').text()).toContain('你好，归档文本')
    expect(mocks.openArchiveEntry).not.toHaveBeenCalled()
    await left.get('.preview-close').trigger('click');await flushPromises()
    mocks.browseArchive.mockClear()
    await right.get('[data-testid="archive-pane-browser"]').trigger('keydown',{key:'F5'});await flushPromises()
    expect(mocks.browseArchive).not.toHaveBeenCalled()
    await left.get('[data-testid="archive-pane-browser"]').trigger('keydown',{key:'F5'});await flushPromises()
    expect(mocks.browseArchive).toHaveBeenCalledTimes(1)
    expect(mocks.browseArchive.mock.calls[0][0]).toBe('C:/left.zip')
    left.unmount();right.unmount()
  })
  it('closing a loading archive pane ignores the late result and never opens the requested preview',async()=>{
    let finish!:(value:any)=>void
    mocks.browseArchive.mockImplementationOnce(()=>new Promise(resolve=>{finish=resolve}))
    const view=mount(ArchiveBrowserView,{props:{embedded:true,source:{id:1,path:'C:/slow.zip',options:{focusEntryPath:'a.txt',entryAction:'preview'}}}})
    await flushPromises();view.unmount()
    finish({format:'ZIP',totalFiles:1,totalDirectories:0,totalUncompressedSize:1,totalCompressedSize:1,encrypted:false,entries:[{path:'a.txt',name:'a.txt',size:1,isDir:false,encrypted:false}]});await flushPromises()
    expect(mocks.cancelArchiveBrowse).toHaveBeenCalled()
    expect(mocks.previewArchiveText).not.toHaveBeenCalled()
  })

})
