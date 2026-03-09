<template>
  <div class="p-8">
    <h1 class="text-2xl font-bold mb-4">响应式测试</h1>
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
      <div class="glass-card p-6">卡片 1</div>
      <div class="glass-card p-6">卡片 2</div>
      <div class="glass-card p-6">卡片 3</div>
    </div>
  </div>
</template>

<script setup lang="ts">
</script>

<style scoped>
.glass-card {
  @apply rounded-2xl border border-white/20 bg-white/10 backdrop-blur-md shadow-lg dark:border-white/10 dark:bg-black/20;
}
</style>
