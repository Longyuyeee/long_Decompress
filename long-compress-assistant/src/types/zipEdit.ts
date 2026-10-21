export interface ZipEditEntry { path: string; isDir: boolean; size: number }
export interface ZipEditCatalog { revision: string; entries: ZipEditEntry[]; methods: string[] }
export interface ZipEditInput { sourcePath: string; targetPath: string; isDir: boolean; size: number; sourceRevision: string }
export type ZipEditChoice = 'choose' | 'replace' | 'skip' | 'keep-both'
export interface ZipEditDraft extends ZipEditInput { id: string; batch: string; choice: ZipEditChoice }
export interface ZipEditChange extends ZipEditInput { action: 'add' | 'replace' }
export interface ZipEditRow { draft: ZipEditDraft; target: string; status: 'add' | 'replace' | 'merge' | 'skip' | 'pending' | 'superseded'; collision?: 'file' | 'directory'; issue?: string }
