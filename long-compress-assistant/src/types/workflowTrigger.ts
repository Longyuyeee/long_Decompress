import type { WorkflowPlan,WorkflowRunRequest } from './workflow'
export interface WorkflowDirectoryRule {
 id:string;name:string;planId:string;root:string;outputDirectory:string;extensions:string;recursive:boolean;mode:'files'|'folders';settleSeconds:number;enabled:boolean;includeExisting:boolean;note:string
}
export type WorkflowQueueStatus='settling'|'waiting_parts'|'queued'|'needs_input'|'missing'|'running'|'completed'|'failed'|'stopped'|'cancelled'|'interrupted'
export interface WorkflowQueueItem {id:string;ruleId:string;ruleName:string;sourceKey:string;paths:string[];revision:string;plan:WorkflowPlan;outputDirectory:string;state:WorkflowQueueStatus;reason:string;createdAt:string;runId?:string|null;originalRunId?:string}
export interface WorkflowTriggerState {rules:WorkflowDirectoryRule[];items:WorkflowQueueItem[];paused:boolean}
export type WorkflowSessionCredentials=Pick<WorkflowRunRequest,'sourcePassword'|'sourceEncoding'|'passwords'>
export const workflowQueueStatuses:Record<WorkflowQueueStatus,string>={settling:'等待稳定',waiting_parts:'等待分卷',queued:'已排队',needs_input:'待补密码',missing:'来源不可用',running:'正在运行',completed:'已完成',failed:'未完成',stopped:'已停止',cancelled:'已取消',interrupted:'上次中断'}
export const workflowQueueTerminal=(state:string)=>['completed','failed','stopped','cancelled','interrupted'].includes(state)
