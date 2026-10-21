import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { invoke } from '@tauri-apps/api/tauri';
import { useTaskStore } from './task';
import { useTauriCommands } from '@/composables/useTauriCommands';
import { cleanWorkflowPlan, workflowClone, workflowFilename, workflowCompressionOptions, validateWorkflow, workflowTemplate, workflowSettings } from '@/utils/workflow';
import { detectArchiveGroup } from '@/utils/archiveSource';
import { zipPasswordEncodingForSource } from '@/utils/zipPasswordSource';
import { workflowKinds, type WorkflowPlan, type WorkflowRun, type WorkflowOutput, type WorkflowRunRequest } from '@/types/workflow';
import type { PublishedArchiveOutput } from '@/types/archiveOutput';
export const useWorkflowStore = defineStore('workflows', () => {
    const tasks = useTaskStore(), commands = useTauriCommands();
    const plans = ref<WorkflowPlan[]>([]), runs = ref<WorkflowRun[]>([]), draft = ref<WorkflowPlan>(workflowTemplate('repack')), selectedRunId = ref('');
    const loading = ref(false), saving = ref(false), busy = ref(false), stopRequested = ref(false), error = ref(''), notice = ref(''), saveError = ref(''), currentTaskId = ref(''), collectorId = ref('');
    let initialized = false, initializing: Promise<void> | undefined;
    const selectedRun = computed(() => runs.value.find(run => run.id === selectedRunId.value)), currentTask = computed(() => tasks.tasks.find(task => task.id === currentTaskId.value));
    async function saveRun(run: WorkflowRun) { try {
        await invoke('workflow_save_run', { run: workflowClone(run) });
        saveError.value = '';
    }
    catch (cause) {
        saveError.value = '运行记录暂未保存，已完成结果仍可在任务历史查看：' + String(cause);
    } }
    async function initialize() { if (initialized)
        return; if (initializing)
        return initializing; initializing = (async () => { loading.value = true; try {
        const state = await invoke<{
            plans: WorkflowPlan[];
            runs: WorkflowRun[];
        }>('workflow_state');
        plans.value = state.plans.map(plan => ({ ...plan, steps: plan.steps.map(step => ({ ...step, settings: { ...workflowSettings(), ...step.settings, password: '' } })) }));
        runs.value = state.runs;
        for (const run of runs.value)
            if (run.status === 'running') {
                run.status = 'interrupted';
                for (const step of run.steps)
                    if (step.status === 'running')
                        step.status = 'interrupted';
                await saveRun(run);
            }
        ;
        if (plans.value[0])
            draft.value = workflowClone(plans.value[0]);
        if (runs.value[0])
            selectedRunId.value = runs.value[0].id;
        initialized = true;
    }
    catch (cause) {
        error.value = String(cause);
    }
    finally {
        loading.value = false;
        initializing = undefined;
    } })(); return initializing; }
    function newPlan(template: 'repack' | 'dated' | 'empty') { if (busy.value)
        return; draft.value = workflowTemplate(template); error.value = ''; }
    function choosePlan(id: string) { if (busy.value)
        return; const plan = plans.value.find(item => item.id === id); if (plan) {
        draft.value = workflowClone(plan);
        error.value = '';
    } }
    function copyPlan() { if (busy.value)
        return; const plan = workflowClone(draft.value); plan.id = crypto.randomUUID(); plan.name = plan.name.slice(0, 56) + ' 副本'; plan.steps.forEach(step => step.id = crypto.randomUUID()); draft.value = plan; notice.value = '已复制为新方案，可调整后保存'; }
    async function savePlan() { if (saving.value || busy.value)
        return false; saving.value = true; error.value = ''; try {
        const saved = await invoke<WorkflowPlan>('workflow_save_plan', { plan: cleanWorkflowPlan(draft.value) });
        plans.value = [saved, ...plans.value.filter(item => item.id !== saved.id)];
        notice.value = '方案已保存；密码仅在当前会话使用';
        return true;
    }
    catch (cause) {
        error.value = String(cause);
        return false;
    }
    finally {
        saving.value = false;
    } }
    async function deletePlan(id: string) { if (busy.value)
        return; try {
        await invoke('workflow_delete_plan', { id });
        plans.value = plans.value.filter(item => item.id !== id);
        if (draft.value.id === id)
            draft.value = workflowClone(plans.value[0] || workflowTemplate('repack'));
        notice.value = '方案已删除，运行记录与产物保留';
    }
    catch (cause) {
        error.value = String(cause);
    } }
    async function stop() { if (!busy.value || stopRequested.value)
        return; stopRequested.value = true; if (collectorId.value)
        await invoke('workflow_cancel_inputs', { jobId: collectorId.value }).catch(cause => error.value = String(cause)); if (currentTaskId.value)
        await tasks.cancelTask(currentTaskId.value); }
    const item = (path: string, directory = false, output?: PublishedArchiveOutput): WorkflowOutput => ({ path, directory, files: output?.files || [], totalBytes: output?.totalBytes || 0, split: output?.split || false });
    async function archiveSources(inputs: WorkflowOutput[]) { const seen = new Set<string>(), result: Array<{
        path: string;
        format?: string;
    }> = []; for (const input of inputs) {
        if (stopRequested.value)
            break;
        if (input.directory)
            throw Error('此步骤需要归档文件，上一步是目录；请先打包，或改为压缩步骤');
        const group = await detectArchiveGroup(input.path);
        if (group?.is_split && group.is_complete === false)
            throw Error('缺少分卷：' + (group.missing_parts || []).join('、'));
        const path = group?.first_part || input.path, key = path.replace(/\\/g, '/').toLowerCase();
        if (!seen.has(key)) {
            seen.add(key);
            result.push({ path, format: group?.archive_format || undefined });
        }
    } if (result.length > 1000)
        throw Error('此步骤超过 1000 份归档，请缩小来源范围'); return result; }
    async function start(request: WorkflowRunRequest) {
        if (busy.value)
            return false;
        error.value = '';
        notice.value = '';
        const plan = workflowClone(draft.value), now = new Date();
        try {
            validateWorkflow(plan, request.paths, request.outputDirectory, request.passwords, now);
        }
        catch (cause) {
            error.value = String(cause);
            return false;
        }
        busy.value = true;
        stopRequested.value = false;
        let run: WorkflowRun | undefined;
        try {
            const id = crypto.randomUUID(), label = plan.name.replace(/[<>:"/\\|?*{}\x00-\x1f]/g, '_').slice(0, 40), directory = await invoke<string>('workflow_begin_directory', { parent: request.outputDirectory, name: `${label}-${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}-${String(now.getHours()).padStart(2, '0')}${String(now.getMinutes()).padStart(2, '0')}${String(now.getSeconds()).padStart(2, '0')}-${id.slice(0, 6)}` });
            run = { id, planId: plan.id, name: plan.name, status: 'running', startedAt: now.toISOString(), directory, inputs: [...request.paths], steps: plan.steps.map(step => ({ id: step.id, name: step.name, kind: step.kind, status: 'pending', inputs: [], outputs: [], taskIds: [], message: '' })) };
            runs.value.unshift(run);
            run = runs.value[0];
            selectedRunId.value = id;
            await invoke('workflow_save_run', { run: workflowClone(run) });
            await tasks.initListeners();
            let inputs: WorkflowOutput[] = [], inputPassword = request.sourcePassword, inputEncoding = request.sourceEncoding;
            for (const path of request.paths) {
                if (stopRequested.value)
                    break;
                const info = await commands.getFileInfo(path);
                if (!info)
                    throw Error('来源无法访问：' + path);
                inputs.push(item(path, info.isDir));
            }
            for (const [index, definition] of plan.steps.entries()) {
                if (stopRequested.value)
                    break;
                const step = run.steps[index];
                step.status = 'running';
                step.inputs = inputs.map(row => row.path);
                await saveRun(run);
                try {
                    if (definition.kind === 'filter') {
                        collectorId.value = 'workflow-filter-' + id;
                        const result = await invoke<{
                            paths: string[];
                            visited: number;
                            bytes: number;
                        }>('workflow_collect', { paths: inputs.map(row => row.path), extensions: definition.extensions, exclude: directory, outputBase: request.outputDirectory, jobId: collectorId.value });
                        collectorId.value = '';
                        if (stopRequested.value)
                            break;
                        step.outputs = result.paths.map(path => item(path));
                        step.message = `已筛选 ${result.paths.length} 个文件 · 扫描 ${result.visited} 个位置`;
                    }
                    else {
                        const stepDirectory = await invoke<string>('workflow_begin_directory', { parent: directory, name: `${String(index + 1).padStart(2, '0')}-${workflowKinds[definition.kind].label}` }), password = request.passwords[definition.id] || definition.settings.password;
                        if (definition.kind === 'compress') {
                            const paths = inputs.length === 1 && inputs[0].directory ? await invoke<string[]>('workflow_children', { path: inputs[0].path }) : inputs.map(row => row.path);
                            if (stopRequested.value)
                                break;
                            const options = workflowCompressionOptions(definition.settings, password), target = stepDirectory + '/' + workflowFilename(definition.settings.filename, run.inputs[0], index + 1, now) + '.' + options.format;
                            const taskId = crypto.randomUUID();
                            step.taskIds.push(taskId);
                            currentTaskId.value = taskId;
                            tasks.addTask({ id: taskId, name: plan.name + ' · ' + step.name, type: 'compression', sourceFiles: paths, outputPath: target, format: options.format, executionConfig: { schemaVersion: 1, kind: 'archive-compression', outputDirectory: stepDirectory, passwordRequired: !!password, settings: { ...definition.settings, format: options.format as typeof definition.settings.format, password: undefined, deleteAfter: undefined } as any } });
                            tasks.updateTaskStatus(taskId, 'compressing');
                            await saveRun(run);
                            if (stopRequested.value) {
                                tasks.updateTaskStatus(taskId, 'cancelled');
                                break;
                            }
                            try {
                                const output = await commands.compressFiles(taskId, paths, target, options);
                                const task = tasks.tasks.find(t => t.id === taskId)!;
                                task.archiveOutput = output;
                                task.outputPath = output.primaryPath;
                                task.outputBytes = output.totalBytes;
                                tasks.updateTaskStatus(taskId, 'completed');
                                step.outputs = [item(output.primaryPath, false, output)];
                            }
                            catch (cause) {
                                if (!stopRequested.value)
                                    tasks.failTask(taskId, cause);
                                throw cause;
                            }
                        }
                        else {
                            const sources = await archiveSources(inputs);
                            for (const [sourceIndex, source] of sources.entries()) {
                                if (stopRequested.value)
                                    break;
                                const taskId = crypto.randomUUID(), name = workflowFilename('{name}', source.path, index + 1, now) + (sources.length > 1 ? '_' + (sourceIndex + 1) : ''), sourcePassword = inputPassword, sourceEncoding = zipPasswordEncodingForSource(source.path, inputEncoding, source.format);
                                currentTaskId.value = taskId;
                                step.taskIds.push(taskId);
                                if (definition.kind === 'extract') {
                                    const target = stepDirectory + '/' + name;
                                    tasks.addTask({ id: taskId, name: plan.name + ' · ' + step.name + ' · ' + name, type: 'decompression', sourceFiles: [source.path], outputPath: target, format: source.format });
                                    await saveRun(run);
                                    if (stopRequested.value) {
                                        tasks.updateTaskStatus(taskId, 'cancelled');
                                        break;
                                    }
                                    try {
                                        const output = await commands.decompressFile(source.path, { outputPath: target, password: sourcePassword || undefined, sourceFormat: source.format, zipPasswordEncoding: sourceEncoding, keepStructure: true, overwrite: false, deleteAfter: false, conflictPolicy: 'rename' }, taskId);
                                        step.outputs.push(item(output, true));
                                    }
                                    catch (cause) {
                                        throw cause;
                                    }
                                }
                                else {
                                    const options = workflowCompressionOptions(definition.settings, password), target = stepDirectory + '/' + workflowFilename(definition.settings.filename, source.path, index + 1, now) + (sources.length > 1 ? '_' + (sourceIndex + 1) : '') + '.' + options.format;
                                    tasks.addTask({ id: taskId, name: plan.name + ' · ' + step.name + ' · ' + name, type: 'compression', sourceFiles: [source.path], outputPath: target, format: options.format, executionConfig: { schemaVersion: 1, kind: 'archive-conversion', outputPath: target, format: options.format!, level: options.level, splitSize: options.split_size || 0, encryptFilenames: options.encrypt_filenames || false, passwordRequired: !!password, sourcePasswordRequired: !!sourcePassword, sourceZipPasswordEncoding: sourceEncoding, zipPasswordEncoding: options.zip_password_encoding, zipMethod: options.zip_method, zipEncryption: options.zip_encryption, zipSplitMode: options.zip_split_mode, wimMethod: options.wim_method } });
                                    tasks.updateTaskStatus(taskId, 'preparing');
                                    await saveRun(run);
                                    if (stopRequested.value) {
                                        tasks.updateTaskStatus(taskId, 'cancelled');
                                        break;
                                    }
                                    try {
                                        const output = await invoke<PublishedArchiveOutput>('convert_archive', { taskId, sourcePath: source.path, sourcePassword: sourcePassword || null, ...(sourceEncoding ? { sourceZipPasswordEncoding: sourceEncoding } : {}), outputPath: target, options });
                                        const task = tasks.tasks.find(t => t.id === taskId)!;
                                        task.archiveOutput = output;
                                        task.outputPath = output.primaryPath;
                                        task.outputBytes = output.totalBytes;
                                        tasks.updateTaskStatus(taskId, 'completed');
                                        step.outputs.push(item(output.primaryPath, false, output));
                                    }
                                    catch (cause) {
                                        if (stopRequested.value)
                                            tasks.updateTaskStatus(taskId, 'cancelled');
                                        else
                                            tasks.failTask(taskId, cause);
                                        throw cause;
                                    }
                                }
                                await tasks.waitForHistoryPersistence(taskId);
                                currentTaskId.value = '';
                                await saveRun(run);
                            }
                        }
                    }
                    if (stopRequested.value) {
                        step.status = 'stopped';
                        step.message = '已停止，已完成产物保留';
                        break;
                    }
                    if (!step.outputs.length)
                        throw Error('此步骤没有产物，后续步骤未开始');
                    step.status = 'completed';
                    if (definition.kind === 'extract') {
                        inputPassword = '';
                        inputEncoding = undefined;
                    }
                    else if (['compress', 'convert'].includes(definition.kind)) {
                        inputPassword = request.passwords[definition.id] || definition.settings.password;
                        inputEncoding = definition.settings.zipPasswordEncoding;
                    }
                    inputs = step.outputs;
                    currentTaskId.value = '';
                    await saveRun(run);
                }
                catch (cause) {
                    step.status = stopRequested.value ? 'stopped' : 'failed';
                    step.message = String(cause);
                    throw cause;
                }
            }
            run.status = stopRequested.value ? 'stopped' : 'completed';
            notice.value = stopRequested.value ? '已停止工作流，已完成结果保留' : '工作流已完成，可以逐步查看产物';
        }
        catch (cause) {
            if (run) {
                run.status = stopRequested.value ? 'stopped' : 'failed';
                const active = run.steps.find(step => step.status === 'running') || (!run.steps.some(step => ['failed', 'stopped'].includes(step.status)) ? run.steps.find(step => step.status === 'pending') : undefined);
                if (active) {
                    active.status = run.status;
                    active.message = String(cause);
                }
            }
            error.value = String(cause);
        }
        finally {
            if (run) {
                for (const step of run.steps)
                    if (step.status === 'running' && stopRequested.value) {
                        step.status = 'stopped';
                        step.message = '已停止，后续步骤未开始';
                    }
                run.finishedAt = new Date().toISOString();
                await saveRun(run);
            }
            currentTaskId.value = '';
            collectorId.value = '';
            busy.value = false;
        }
        return run?.status === 'completed';
    }
    return { plans, runs, draft, selectedRunId, selectedRun, currentTask, loading, saving, busy, stopRequested, error, notice, saveError, initialize, newPlan, choosePlan, copyPlan, savePlan, deletePlan, start, stop };
});
