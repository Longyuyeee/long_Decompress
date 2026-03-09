<template>
  <div class="p-8">
    <h1 class="text-2xl font-bold mb-4">动画测试</h1>
    <div class="glass-card p-6 animate-fade-in">
      <p>这是一个测试动画的页面。</p>
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
