import type { WorkflowKind, WorkflowPlan, WorkflowStep } from '@/types/workflow';
import type { CompressionOptions } from '@/stores/compression';
import type { CompressOptions } from '@/composables/useTauriCommands';
import { effectiveFormatForPassword, supportsArchiveSplit, supportsFilenameEncryption } from './compressionFormat';
export const workflowSettings = (): CompressionOptions => ({ format: 'zip', level: 6, password: '', filename: '资料-{date}', splitArchive: false, splitSize: '1024', keepStructure: true, deleteAfter: false, verifyAfter: true, createSolidArchive: false });
export const workflowStep = (kind: WorkflowKind): WorkflowStep => ({ id: crypto.randomUUID(), name: ({ filter: '筛选需要的资料', extract: '解压到本次目录', compress: '生成交付压缩包', convert: '转换归档格式' })[kind], kind, extensions: kind === 'filter' ? 'txt,md,pdf,docx,xlsx' : '', passwordRequired: false, settings: workflowSettings() });
export function workflowTemplate(template: 'repack' | 'dated' | 'empty'): WorkflowPlan { const steps = template === 'repack' ? [workflowStep('extract'), workflowStep('compress')] : template === 'dated' ? [workflowStep('filter'), workflowStep('compress')] : [workflowStep('compress')]; if (template === 'repack')
    steps[1].settings.format = '7z'; return { id: crypto.randomUUID(), name: template === 'repack' ? '解压后重新打包' : template === 'dated' ? '按日期归档资料' : '新工作流', steps, updatedAt: '' }; }
export const cleanWorkflowPlan = (plan: WorkflowPlan): WorkflowPlan => ({ ...plan, steps: plan.steps.map(step => ({ ...step, passwordRequired: step.passwordRequired || !!step.settings.password, settings: { ...step.settings, password: '', deleteAfter: false, fastPack: false } })) });
export const workflowClone = <T>(value: T): T => JSON.parse(JSON.stringify(value));
export function workflowFilename(pattern: string, source: string, step: number, now: Date) {
    const name = (source.split(/[\\/]/).pop() || '资料').replace(/\.[^.]+$/, '');
    const date = [now.getFullYear(), now.getMonth() + 1, now.getDate()].map((n, i) => String(n).padStart(i ? 2 : 4, '0')).join('-'), time = [now.getHours(), now.getMinutes(), now.getSeconds()].map(n => String(n).padStart(2, '0')).join('');
    const value = (pattern.trim() || '{name}-{date}').replaceAll('{name}', name).replaceAll('{date}', date).replaceAll('{time}', time).replaceAll('{step}', String(step));
    if (!value || value.length > 100 || /[<>:"/\\|?*{}\x00-\x1f]/.test(value) || /[. ]$/.test(value))
        throw Error('归档名称请使用普通文件名，可用 {name}、{date}、{time}、{step} 占位符，最多 100 字');
    return value;
}
export function workflowCompressionOptions(settings: CompressionOptions, password: string): CompressOptions {
    const format = effectiveFormatForPassword(settings.format, password);
    if (!Number.isFinite(settings.level) || settings.level < 0 || settings.level > 9)
        throw Error('压缩强度需要在 0～9 之间');
    const size = settings.splitArchive ? Number(settings.splitSize) * 1048576 : 0;
    if (settings.splitArchive && (!supportsArchiveSplit(format) || !Number.isFinite(size) || size <= 0))
        throw Error('请确认此格式支持分卷，并填写有效的分卷大小');
    return { format, level: settings.level, password: password || undefined, split_size: size || null, preserve_paths: settings.keepStructure, delete_after: false, verify_after: settings.verifyAfter, fast_pack: false, create_solid_archive: settings.createSolidArchive, encrypt_filenames: !!password && supportsFilenameEncryption(format) && (settings.encryptFilenames ?? true), zip_method: settings.zipMethod ?? 'deflate', zip_encryption: settings.zipEncryption ?? 'aes256', zip_split_mode: settings.zipSplitMode ?? 'numeric', zip_password_encoding: settings.zipPasswordEncoding ?? 'utf8', wim_method: settings.wimMethod ?? (format === 'esd' ? 'solid' : 'store'), rar_recovery_percent: settings.rarRecoveryPercent ?? 0, rar_recovery_volumes: settings.rarRecoveryVolumes ?? 0, allow_insecure_password_cli: false };
}
export function validateWorkflow(plan: WorkflowPlan, paths: string[], output: string, passwords: Record<string, string>, now = new Date()) {
    if (!plan.name.trim() || plan.name.length > 60)
        throw Error('请填写 1～60 字的方案名称');
    if (!plan.steps.length || plan.steps.length > 8)
        throw Error('方案需要 1～8 个步骤');
    if (!paths.length || paths.length > 1000)
        throw Error('请选择 1～1000 个文件或目录');
    if (!output.trim())
        throw Error('请选择本次保存位置');
    for (const [index, step] of plan.steps.entries())
        if (['compress', 'convert'].includes(step.kind)) {
            const password = passwords[step.id] || step.settings.password;
            if (step.passwordRequired && !password)
                throw Error(`「${step.name}」需要本次输出密码，请填写或从保险箱选择`);
            workflowFilename(step.settings.filename, paths[0], index + 1, now);
            workflowCompressionOptions(step.settings, password);
        }
}
