import type { CompressionOptions } from '@/stores/compression';
import type { PublishedArchiveOutput } from './archiveOutput';
export type WorkflowKind = 'filter' | 'extract' | 'compress' | 'convert';
export interface WorkflowStep {
    id: string;
    name: string;
    kind: WorkflowKind;
    extensions: string;
    passwordRequired: boolean;
    settings: CompressionOptions;
}
export interface WorkflowPlan {
    id: string;
    name: string;
    steps: WorkflowStep[];
    updatedAt: string;
}
export interface WorkflowOutput {
    path: string;
    directory: boolean;
    files: PublishedArchiveOutput['files'];
    totalBytes: number;
    split: boolean;
}
export type WorkflowStatus = 'pending' | 'running' | 'completed' | 'failed' | 'stopped' | 'interrupted';
export interface WorkflowStepRun {
    id: string;
    name: string;
    kind: WorkflowKind;
    status: WorkflowStatus;
    inputs: string[];
    outputs: WorkflowOutput[];
    taskIds: string[];
    message: string;
    inputProof?: WorkflowProof;
    outputProof?: WorkflowProof;
    reuseIssue?: string;
    reusedFrom?: string;
}
export interface WorkflowRun {
    id: string;
    planId: string;
    name: string;
    status: WorkflowStatus;
    startedAt: string;
    finishedAt?: string;
    directory: string;
    inputs: string[];
    steps: WorkflowStepRun[];
    plan?: WorkflowPlan;
    outputDirectory?: string;
    sourceEncoding?: import('./zipPassword').ZipPasswordEncoding;
    parentRunId?: string;
    resumeFrom?: number;
}
export interface WorkflowRunRequest {
    paths: string[];
    outputDirectory: string;
    sourcePassword: string;
    sourceEncoding?: import('./zipPassword').ZipPasswordEncoding;
    passwords: Record<string, string>;
}
export const workflowKinds: Record<WorkflowKind, {
    label: string;
    icon: string;
    description: string;
}> = { filter: { label: '筛选文件', icon: 'pi-filter', description: '读取本地文件，按扩展名筛选；包内文件请先解压' }, extract: { label: '解压', icon: 'pi-folder-open', description: '逐个解压归档，后续步骤接收实际输出目录' }, compress: { label: '打包压缩', icon: 'pi-box', description: '将上一步的文件或目录组成一个新包' }, convert: { label: '转换格式', icon: 'pi-sync', description: '逐个转换归档，后续步骤接收新包' } };
export const workflowStatuses: Record<WorkflowStatus, string> = { pending: '尚未开始', running: '正在处理', completed: '已完成', failed: '未完成', stopped: '已停止', interrupted: '上次已中断' };

export interface WorkflowProofPolicy {excludes:string[];extensions?:string;filesOnly:boolean;archiveMembers:boolean}
export interface WorkflowProof {digest:string;metadata:string;files:number;bytes:number;policy:WorkflowProofPolicy}
export interface WorkflowReuseIssue {index:number;reason:string}
export interface WorkflowProofProgress {jobId:string;files:number;bytes:number;path:string}
