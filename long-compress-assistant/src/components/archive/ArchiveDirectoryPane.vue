<script setup lang="ts">
export interface ArchiveDirectoryNode {
  path: string
  name: string
  depth: number
  hasChildren: boolean
}

defineProps<{
  activeDirectory: string
  directories: ArchiveDirectoryNode[]
  expandedDirectories: Set<string>
}>()

defineEmits<{
  select: [path: string]
  toggle: [path: string]
}>()
</script>

<template>
  <aside class="directory-pane min-h-0 min-w-0 overflow-y-auto overflow-x-hidden custom-scrollbar border-r border-subtle/70 p-3">
    <p class="directory-heading">目录树</p>
    <button class="directory-entry" :class="{ active: activeDirectory === '' }" type="button" @click="$emit('select', '')">
      <i class="pi pi-home"></i><span>全部文件</span>
    </button>
    <div v-for="directory in directories" :key="directory.path" class="directory-tree-row" :style="{ paddingLeft: `${directory.depth * 0.9}rem` }">
      <button v-if="directory.hasChildren" class="directory-toggle" type="button" :aria-label="expandedDirectories.has(directory.path) ? '折叠目录' : '展开目录'" @click="$emit('toggle', directory.path)">
        <i :class="expandedDirectories.has(directory.path) ? 'pi pi-chevron-down' : 'pi pi-chevron-right'"></i>
      </button>
      <span v-else class="directory-toggle-spacer"></span>
      <button class="directory-entry" :class="{ active: activeDirectory === directory.path }" type="button" :title="directory.path" @click="$emit('select', directory.path)">
        <i :class="expandedDirectories.has(directory.path) ? 'pi pi-folder-open' : 'pi pi-folder'"></i><span class="truncate">{{ directory.name }}</span>
      </button>
    </div>
  </aside>
</template>

<style scoped>
.directory-heading { padding: .2rem .75rem .5rem; color: var(--text-muted); font-size: .64rem; font-weight: 900; letter-spacing: .12em; }
.directory-tree-row { display: flex; min-width: 0; align-items: center; }
.directory-toggle, .directory-toggle-spacer { flex: 0 0 1.35rem; width: 1.35rem; height: 2.15rem; display: grid; place-items: center; color: var(--text-muted); font-size: .58rem; }
.directory-toggle:hover { color: var(--dynamic-accent); }
.directory-entry { flex: 1; width: 100%; min-width: 0; display: flex; align-items: center; gap: .55rem; border-radius: .7rem; padding: .56rem .65rem; color: var(--text-muted); font-size: .74rem; font-weight: 800; text-align: left; }
.directory-entry:hover, .directory-entry.active { background: color-mix(in srgb, var(--dynamic-accent) 13%, transparent); color: var(--dynamic-accent); }
@media (max-width: 760px) { .directory-pane { border-right: 0; border-bottom: 1px solid var(--border-subtle); } }
</style>
