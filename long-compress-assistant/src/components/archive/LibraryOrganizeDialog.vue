<script setup lang="ts">
import HelpHint from '@/components/ui/HelpHint.vue'
import { ref } from 'vue'
import Modal from '@/components/ui/Modal.vue'
import { useArchiveLibraryStore } from '@/stores/archiveLibrary'
import type { LibrarySource } from '@/types/archiveLibrary'
const props=defineProps<{source:LibrarySource}>(),emit=defineEmits<{close:[]}>()
const library=useArchiveLibraryStore(),initial=library.labels(props.source.id),favorite=ref(initial.favorite),tags=ref(initial.tags.join('，')),busy=ref(false)
async function save(){busy.value=true;try{if(await library.organize(props.source.id,{favorite:favorite.value,tags:tags.value.split(/[,，;；\n]/)}))emit('close')}finally{busy.value=false}}
</script>
<template><Modal :visible="true" title="整理这份资料" icon="pi pi-tags" size="md" :show-footer="true" :close-on-backdrop="!busy" :close-on-escape="!busy" @update:visible="!busy && emit('close')"><div class="space-y-4" data-testid="library-organize"><strong class="break-all">{{ source.name }}</strong><label class="flex items-center gap-3 text-sm"><input v-model="favorite" type="checkbox" data-testid="library-organize-favorite" :disabled="busy">收藏这份来源，方便下次查找</label><label class="block text-xs font-bold">标签<input v-model="tags" class="organize-control w-full mt-2" data-testid="library-organize-tags" placeholder="例如：项目资料，合同，常用素材" :disabled="busy"></label><span class="context-help"><HelpHint label="标签说明">用逗号分隔，可添加 12 个标签，每个最多 30 字。只整理资料库记录，原包不变。</HelpHint></span><p v-if="library.error" role="alert" class="text-xs text-red-500 break-all">{{ library.error }}</p></div><template #footer><button class="organize-control" :disabled="busy" @click="emit('close')">返回</button><button class="organize-control bg-primary text-white" data-testid="library-organize-save" :disabled="busy" @click="save">保存整理</button></template></Modal></template>
<style scoped>.organize-control{border:1px solid var(--border-subtle);border-radius:10px;padding:10px 12px;background:var(--bg-input);font-size:12px}.organize-control.bg-primary{background:var(--dynamic-accent)}</style>
