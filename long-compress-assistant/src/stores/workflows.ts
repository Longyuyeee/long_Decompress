import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/tauri';
import { useTaskStore } from './task';
import { useTauriCommands } from '@/composables/useTauriCommands';
import { cleanWorkflowPlan, workflowClone, workflowFilename, workflowCompressionOptions, validateWorkflow, workflowTemplate, workflowSettings, normalizeWorkflowPlan, workflowResumeFrom, workflowOutputPaths, workflowInputCredential } from '@/utils/workflow';
import { detectArchiveGroup } from '@/utils/archiveSource';
import { zipPasswordEncodingForSource } from '@/utils/zipPasswordSource';
import { workflowKinds, type WorkflowPlan, type WorkflowRun, type WorkflowOutput, type WorkflowRunRequest, type WorkflowProof, type WorkflowProofPolicy, type WorkflowProofProgress, type WorkflowReuseIssue } from '@/types/workflow';
import type { PublishedArchiveOutput } from '@/types/archiveOutput';
export const useWorkflowStore = defineStore('workflows', () => {
    const tasks = useTaskStore(), commands = useTauriCommands();
    const plans = ref<WorkflowPlan[]>([]), runs = ref<WorkflowRun[]>([]), draft = ref<WorkflowPlan>(workflowTemplate('repack')), selectedRunId = ref('');
    const loading = ref(false), saving = ref(false), busy = ref(false), stopRequested = ref(false), error = ref(''), notice = ref(''), saveError = ref(''), currentTaskId = ref(''), collectorId = ref('');
    const workspace=ref<'edit'|'runs'|'directory'>('edit');
    const phase=ref(''), proofProgress=ref<WorkflowProofProgress>(), resumeIssue=ref<WorkflowReuseIssue>();
    let initialized = false, initializing: Promise<void> | undefined;
    const selectedRun = computed(() => runs.value.find(run => run.id === selectedRunId.value)), currentTask = computed(() => tasks.tasks.find(task => task.id === currentTaskId.value));
    let persistence:Promise<boolean>=Promise.resolve(true);
    function saveRun(run:WorkflowRun):Promise<boolean>{
      const snapshot=workflowClone(run);
      persistence=persistence.then(async()=>{try{await invoke('workflow_save_run',{run:snapshot});saveError.value='';return true}catch(cause){saveError.value='运行记录暂未保存，已完成结果仍保留：'+String(cause);return false}});
      return persistence;
    }
    async function flush(){
      const active=runs.value.find(run=>run.status==='running');if(active)void saveRun(active);
      let pending:Promise<boolean>,ok:boolean;do{pending=persistence;ok=await pending}while(pending!==persistence);return ok;
    }
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
        if(runs.value[0]&&runs.value[0].status!=='completed')workspace.value='runs';
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
    async function openRun(id:string){try{if(!runs.value.some(run=>run.id===id)){const run=await invoke<WorkflowRun>('workflow_run',{id});if(run.status==='running'){run.status='interrupted';for(const step of run.steps)if(step.status==='running')step.status='interrupted';await saveRun(run)}runs.value.push(run)}selectedRunId.value=id;workspace.value='runs'}catch(cause){error.value='暂时无法读取原运行记录：'+String(cause)}}
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
    async function start(request: WorkflowRunRequest) {return execute(draft.value,request)}
    async function resume(runId:string,plan:WorkflowPlan,request:WorkflowRunRequest,requested:number){
      const parent=runs.value.find(run=>run.id===runId)
      if(!parent?.plan){error.value='旧记录缺少执行快照，请带入当前方案重新运行';return false}
      return execute(plan,request,parent,requested)
    }
    async function startQueued(prepare:()=>Promise<{plan:WorkflowPlan;request:WorkflowRunRequest;id:string}>){
      if(busy.value)return false;
      busy.value=true;stopRequested.value=false;
      try{const entry=await prepare();if(stopRequested.value)return false;return await execute(entry.plan,entry.request,undefined,0,entry.id,true)}finally{busy.value=false}
    }
    async function execute(selectedPlan:WorkflowPlan,request:WorkflowRunRequest,parent?:WorkflowRun,requested=0,queuedId?:string,reserved=false) {
        if (busy.value&&!reserved)
            return false;
        error.value = '';
        notice.value = '';
        const plan = normalizeWorkflowPlan(selectedPlan), now = new Date();
        let from=0;resumeIssue.value=undefined;
        try {
            from=parent?workflowResumeFrom(parent,plan,request.paths,requested):0;
            if(parent&&workflowInputCredential(parent,plan,from).required&&!request.sourcePassword)throw Error('请提供已有密码产物的读取密码');
            validateWorkflow(plan, request.paths, request.outputDirectory, request.passwords, now,from);
        }
        catch (cause) {
            error.value = String(cause);
            return false;
        }
        busy.value = true;
        stopRequested.value = false;
        let run: WorkflowRun | undefined;
        let unlisten:UnlistenFn|undefined;
        async function witness(paths:string[],policy:WorkflowProofPolicy,metadataOnly=false){
          if(stopRequested.value)throw Error('已停止本次处理');
          collectorId.value='workflow-proof-'+crypto.randomUUID();proofProgress.value=undefined;
          try{return await invoke<WorkflowProof>('workflow_witness',{paths,policy,jobId:collectorId.value,metadataOnly})}finally{collectorId.value=''}
        }
        try {
            unlisten=await listen<WorkflowProofProgress>('workflow-proof-progress',event=>{if(event.payload.jobId===collectorId.value)proofProgress.value=event.payload})
            if(parent && from>0){
              phase.value='正在核对可复用步骤';collectorId.value='workflow-resume-'+crypto.randomUUID();
              const report=await invoke<{reusable:number;issue?:WorkflowReuseIssue}>('workflow_check_reuse',{runId:parent.id,count:from,jobId:collectorId.value});collectorId.value='';
              if(report.issue){resumeIssue.value=report.issue;throw Error(report.issue.reason)}
              if(stopRequested.value)throw Error('已停止核对，尚未创建新的任务')
            }
            phase.value='';if(stopRequested.value)throw Error('已停止核对，尚未创建新的任务');
            const id = queuedId || crypto.randomUUID(), label = plan.name.replace(/[<>:"/\\|?*{}\x00-\x1f]/g, '_').slice(0, 40), directory = await invoke<string>('workflow_begin_directory', { parent: request.outputDirectory, name: `${label}-${now.getFullYear()}${String(now.getMonth() + 1).padStart(2, '0')}${String(now.getDate()).padStart(2, '0')}-${String(now.getHours()).padStart(2, '0')}${String(now.getMinutes()).padStart(2, '0')}${String(now.getSeconds()).padStart(2, '0')}-${id.slice(0, 6)}` });
            run = { id, planId: plan.id, name: plan.name, status: 'running', startedAt: now.toISOString(), directory, inputs: [...request.paths], steps: plan.steps.map(step => ({ id: step.id, name: step.name, kind: step.kind, status: 'pending', inputs: [], outputs: [], taskIds: [], message: '' })) };
            run.plan=cleanWorkflowPlan({...plan,steps:plan.steps.map(step=>({...step,passwordRequired:step.passwordRequired||!!request.passwords[step.id]}))});run.outputDirectory=request.outputDirectory;run.sourceEncoding=request.sourceEncoding;
            if(parent){run.parentRunId=parent.id;run.resumeFrom=from;for(let index=0;index<from;index++)run.steps[index]={...workflowClone(parent.steps[index]),name:plan.steps[index].name,reusedFrom:parent.id,message:'已核对内容，沿用原产物；本次未重新执行'};}

            runs.value.unshift(run);
            runs.value=runs.value.filter((record,index)=>index<100||record.id===selectedRunId.value);
            run = runs.value[0];
            if(!queuedId||!selectedRunId.value)selectedRunId.value = id;
            await invoke('workflow_save_run', { run: workflowClone(run) });
            await tasks.initListeners();
            let inputs: WorkflowOutput[] = [], inputPassword = request.sourcePassword, inputEncoding = request.sourceEncoding;
            if(from>0)inputs=workflowClone(run.steps[from-1].outputs);
            else for (const path of request.paths) {
                if (stopRequested.value)
                    break;
                const info = await commands.getFileInfo(path);
                if (!info)
                    throw Error('来源无法访问：' + path);
                inputs.push(item(path, info.isDir));
            }
            for (const [index, definition] of plan.steps.entries()) {
                if(index<from)continue;
                if (stopRequested.value)
                    break;
                const step = run.steps[index];
                step.status = 'running';
                step.inputs = workflowOutputPaths(inputs);
                await saveRun(run);
                try {
                    phase.value=`正在记录第 ${index+1} 步的输入`;
                    const policy:WorkflowProofPolicy={excludes:[directory,...(definition.kind==='filter'?[request.outputDirectory]:[])],filesOnly:definition.kind==='filter',archiveMembers:['extract','convert'].includes(definition.kind),...(definition.kind==='filter'?{extensions:definition.extensions}:{})};
                    try{step.inputProof=await witness(step.inputs,policy)}catch(cause){if(stopRequested.value)throw cause;step.reuseIssue='本步未能记录输入内容，接续时需从此步重做：'+String(cause)}
                    phase.value='';if(stopRequested.value)break;
                    if (definition.kind === 'filter') {
                        collectorId.value = 'workflow-filter-' + id;
                        const result = await invoke<{
                            paths: string[];
                            visited: number;
                            bytes: number;
                        }>('workflow_collect', { paths: workflowOutputPaths(inputs), extensions: definition.extensions, exclude: directory, outputBase: request.outputDirectory, jobId: collectorId.value });
                        collectorId.value = '';
                        if (stopRequested.value)
                            break;
                        step.outputs = result.paths.map(path => item(path));
                        step.message = `已筛选 ${result.paths.length} 个文件 · 扫描 ${result.visited} 个位置`;
                    }
                    else {
                        const stepDirectory = await invoke<string>('workflow_begin_directory', { parent: directory, name: `${String(index + 1).padStart(2, '0')}-${workflowKinds[definition.kind].label}` }), password = request.passwords[definition.id] || definition.settings.password;
                        if (definition.kind === 'compress') {
                            const paths = inputs.length === 1 && inputs[0].directory ? await invoke<string[]>('workflow_children', { path: inputs[0].path }) : workflowOutputPaths(inputs);
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
                                await saveRun(run);
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
                    phase.value=`正在记录第 ${index+1} 步的产物`;
                    if(step.inputProof){
                      try{
                        const stable=await witness(step.inputs,step.inputProof.policy,true);
                        if(stable.metadata!==step.inputProof.metadata)throw Error('处理期间输入发生变化');
                        step.outputProof=await witness(workflowOutputPaths(step.outputs),{excludes:[],filesOnly:false,archiveMembers:false});
                      }catch(cause){if(stopRequested.value)throw cause;step.reuseIssue='本步结果保留，但缺少完整接续依据：'+String(cause)}
                    }
                    phase.value='';
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
            phase.value='';proofProgress.value=undefined;unlisten?.();
            busy.value = false;
        }
        return run?.status === 'completed';
    }
    return { plans, runs, draft, selectedRunId, selectedRun, currentTask, loading, saving, busy, stopRequested, error, notice, saveError, flush,workspace,openRun, phase,proofProgress,resumeIssue, initialize, newPlan, choosePlan, copyPlan, savePlan, deletePlan, start, startQueued, resume, stop };
});
