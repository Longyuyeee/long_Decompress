<template>
  <aside class="w-64 h-full border-r border-white/10 bg-white/5 backdrop-blur-xl flex flex-col">
    <div class="p-6">
      <h2 class="text-xl font-bold bg-gradient-to-r from-primary to-accent-500 bg-clip-text text-transparent">
        胧压缩助手
      </h2>
    </div>

    <nav class="flex-1 px-4 py-2 space-y-2">
      <router-link
        v-for="item in menuItems"
        :key="item.path"
        :to="item.path"
        class="flex items-center px-4 py-3 rounded-xl transition-all duration-200 group"
        active-class="bg-primary/20 text-primary"
      >
        <i :class="[item.icon, 'mr-3 text-lg group-hover:scale-110 transition-transform']"></i>
        <span class="font-medium">{{ item.name }}</span>
      </router-link>
    </nav>

    <div class="p-4 mt-auto">
      <div class="p-4 rounded-2xl bg-gradient-to-br from-primary/10 to-accent-500/10 border border-white/10">
        <p class="text-xs text-gray-500 mb-2">当前版本</p>
        <p class="text-sm font-semibold">v1.0.0</p>
      </div>
    </div>
  </aside>
</template>

<script setup lang="ts">
const menuItems = [
  { name: '首页', path: '/', icon: 'pi pi-home' },
  { name: '解压文件', path: '/decompress', icon: 'pi pi-play' },
  { name: '压缩文件', path: '/compress', icon: 'pi pi-compress' },
  { name: '密码本', path: '/passwords', icon: 'pi pi-lock' },
  { name: '任务管理', path: '/tasks', icon: 'pi pi-list' },
  { name: '设置', path: '/settings', icon: 'pi pi-cog' },
  { name: '关于', path: '/about', icon: 'pi pi-info-circle' },
]
</script>

<style scoped>
.router-link-active {
  @apply shadow-lg shadow-primary/10;
}
</style>
