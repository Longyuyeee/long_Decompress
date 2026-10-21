import type { ZipPasswordEncoding } from './zipPassword'
export type ZipEditEncryption = 'none' | 'zipcrypto' | 'aes128' | 'aes192' | 'aes256'
export type ZipEditOutputMode = 'keep' | 'none' | 'aes256' | 'zipcrypto'
export interface ZipEditCredentials { sourcePassword: string; sourceEncoding?: ZipPasswordEncoding; outputMode: ZipEditOutputMode; outputPassword: string; outputEncoding: ZipPasswordEncoding }
export interface ZipEditEntry { path: string; isDir: boolean; size: number }
export interface ZipEditCatalog { revision: string; entries: ZipEditEntry[]; methods: string[]; encrypted?: boolean; defaultEncryption?: ZipEditEncryption; encryptions?: Record<string,ZipEditEncryption> }
export interface ZipEditInput { sourcePath: string; targetPath: string; isDir: boolean; size: number; sourceRevision: string }
export type ZipEditChoice = 'choose' | 'replace' | 'skip' | 'keep-both'
export interface ZipEditDraft extends ZipEditInput { id: string; batch: string; choice: ZipEditChoice; order?: number }
export interface ZipEditChange extends ZipEditInput { action: 'add' | 'replace'; replacesPath?: string }
export interface ZipEditRow { draft: ZipEditDraft; target: string; status: 'add' | 'replace' | 'merge' | 'skip' | 'pending' | 'superseded'; collision?: 'file' | 'directory'; issue?: string }

export interface ZipOriginalEdit { sourcePath: string; targetPath: string | null }
export interface ZipWorkspaceNode extends ZipEditEntry { id: string; originalPath?: string; originalAlias?: string; input?: ZipEditInput }
export interface ZipStructureOperation { id: string; order: number; kind: 'delete' | 'rename'; targets: { id: string; path: string | null }[] }
export interface ZipStructureRow { operation: ZipStructureOperation; affected: { before: string; after: string | null; isDir: boolean }[]; issue?: string }
