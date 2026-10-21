import type { WorkflowPlan,WorkflowRunRequest } from './workflow'
export interface WorkflowDirectoryRule {
 id:string;name:string;planId:string;root:string;outputDirectory:string;extensions:string;recursive:boolean;mode:'files'|'folders';settleSeconds:number;enabled:boolean;includeExisting:boolean;note:string
}
export type WorkflowQueueStatus='settling'|'waiting_parts'|'queued'|'needs_input'|'missing'|'running'|'completed'|'failed'|'stopped'|'cancelled'|'interrupted'|'missed'|'skipped'
export interface WorkflowQueueItem {id:string;ruleId:string;ruleName:string;sourceKey:string;paths:string[];revision:string;plan:WorkflowPlan;outputDirectory:string;state:WorkflowQueueStatus;reason:string;createdAt:string;runId?:string|null;originalRunId?:string;trigger?:'schedule'|'';scheduledAt?:string|null;explicit?:boolean}
export interface WorkflowTriggerState {rules:WorkflowDirectoryRule[];schedules?:WorkflowScheduleRule[];items:WorkflowQueueItem[];paused:boolean}
export type WorkflowSessionCredentials=Pick<WorkflowRunRequest,'sourcePassword'|'sourceEncoding'|'passwords'>
export const workflowQueueStatuses:Record<WorkflowQueueStatus,string>={settling:'等待稳定',waiting_parts:'等待分卷',queued:'已排队',needs_input:'待补密码',missing:'来源不可用',running:'正在运行',completed:'已完成',failed:'未完成',stopped:'已停止',cancelled:'已取消',interrupted:'上次中断',missed:'错过 · 未执行',skipped:'已跳过'}
export const workflowQueueTerminal=(state:string)=>['completed','failed','stopped','cancelled','interrupted','skipped'].includes(state)

export interface WorkflowScheduleRule {
 id:string;name:string;planId:string;paths:string[];outputDirectory:string;kind:'once'|'daily'|'weekly';time:string;date:string;weekdays:number[];busyPolicy:'queue'|'skip';enabled:boolean;nextAt?:string|null;checkedAt:number;note:string
}
export function workflowScheduleLabel(rule:WorkflowScheduleRule){return (rule.kind==='once'?rule.date:rule.kind==='daily'?'每天':'每周 '+rule.weekdays.map(day=>['','一','二','三','四','五','六','日'][day]).join('、'))+' '+rule.time}
