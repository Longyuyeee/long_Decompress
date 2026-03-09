<template>
  <div class="design-system-showcase space-y-12">
    <!-- 标题 -->
    <div class="text-center mb-8">
      <h1 class="text-4xl font-bold text-gray-900 dark:text-white mb-4">设计系统展示</h1>
      <p class="text-lg text-gray-600 dark:text-gray-400">基于DESIGN_SYSTEM.md规范的设计系统实现</p>
    </div>

    <!-- 颜色系统 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">颜色系统</h2>

      <!-- 主色系 -->
      <div class="space-y-4">
        <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">主色系</h3>
        <div class="grid grid-cols-2 md:grid-cols-5 gap-4">
          <div v-for="(color, key) in primaryColors" :key="key" class="space-y-2">
            <div
              class="h-20 rounded-lg shadow-md"
              :style="{ backgroundColor: color }"
            ></div>
            <div class="text-center">
              <p class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ key }}</p>
              <p class="text-xs text-gray-500 dark:text-gray-400 font-mono">{{ color }}</p>
            </div>
          </div>
        </div>
      </div>

      <!-- 功能色 -->
      <div class="space-y-4">
        <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">功能色</h3>
        <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
          <div v-for="color in functionalColors" :key="color.name" class="space-y-2">
            <div
              class="h-20 rounded-lg shadow-md"
              :style="{ backgroundColor: color.value }"
            ></div>
            <div class="text-center">
              <p class="text-sm font-medium" :class="`text-${color.name}`">{{ color.label }}</p>
              <p class="text-xs text-gray-500 dark:text-gray-400 font-mono">{{ color.value }}</p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 排版系统 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">排版系统</h2>

      <div class="space-y-4">
        <div v-for="item in typographyItems" :key="item.name" class="space-y-2">
          <div :class="item.class">
            {{ item.name }} - {{ item.description }}
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 font-mono">
            类名: {{ item.class }}
          </p>
        </div>
      </div>
    </section>

    <!-- 间距系统 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">间距系统</h2>

      <div class="space-y-4">
        <div v-for="spacing in spacingItems" :key="spacing.name" class="flex items-center space-x-4">
          <div class="w-32">
            <p class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ spacing.name }}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400 font-mono">{{ spacing.value }}</p>
          </div>
          <div class="flex-1">
            <div class="h-4 rounded" :style="{
              width: spacing.width,
              backgroundColor: 'var(--color-primary-500)'
            }"></div>
          </div>
        </div>
      </div>
    </section>

    <!-- 圆角系统 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">圆角系统</h2>

      <div class="grid grid-cols-2 md:grid-cols-4 gap-6">
        <div v-for="radius in borderRadiusItems" :key="radius.name" class="space-y-2">
          <div
            class="h-20 w-20 mx-auto border-2 border-primary-500"
            :class="radius.class"
          ></div>
          <div class="text-center">
            <p class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ radius.name }}</p>
            <p class="text-xs text-gray-500 dark:text-gray-400 font-mono">{{ radius.value }}</p>
          </div>
        </div>
      </div>
    </section>

    <!-- 阴影系统 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">阴影系统</h2>

      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        <div v-for="shadow in shadowItems" :key="shadow.name" class="space-y-2">
          <div
            class="h-32 rounded-lg bg-white dark:bg-gray-800 flex items-center justify-center"
            :class="shadow.class"
          >
            <p class="text-gray-700 dark:text-gray-300">{{ shadow.name }}</p>
          </div>
          <p class="text-xs text-gray-500 dark:text-gray-400 font-mono text-center">
            类名: {{ shadow.class }}
          </p>
        </div>
      </div>
    </section>

    <!-- 组件展示 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">组件展示</h2>

      <div class="space-y-8">
        <!-- 玻璃卡片 -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">玻璃卡片</h3>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <GlassCard class="p-6">
              <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">默认玻璃卡片</h4>
              <p class="text-gray-600 dark:text-gray-400">这是一个标准的玻璃卡片，具有毛玻璃效果和适当的阴影。</p>
            </GlassCard>

            <GlassCard hoverable class="p-6">
              <h4 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">可悬停玻璃卡片</h4>
              <p class="text-gray-600 dark:text-gray-400">悬停时会有背景和阴影变化。</p>
            </GlassCard>
          </div>
        </div>

        <!-- 玻璃按钮 -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">玻璃按钮</h3>
          <div class="flex flex-wrap gap-4">
            <GlassButton variant="default" text="默认按钮" />
            <GlassButton variant="primary" text="主要按钮" />
            <GlassButton variant="secondary" text="次要按钮" />
            <GlassButton variant="danger" text="危险按钮" />
          </div>

          <div class="flex flex-wrap gap-4 mt-4">
            <GlassButton size="sm" variant="primary" text="小按钮" />
            <GlassButton size="md" variant="primary" text="中按钮" />
            <GlassButton size="lg" variant="primary" text="大按钮" />
            <GlassButton size="xl" variant="primary" text="特大按钮" />
          </div>

          <div class="flex flex-wrap gap-4 mt-4">
            <GlassButton variant="primary" text="加载中" loading />
            <GlassButton variant="primary" text="禁用" disabled />
            <GlassButton variant="primary" icon="pi-check" text="带图标" />
            <GlassButton variant="primary" fullWidth text="全宽按钮" />
          </div>
        </div>

        <!-- 动画展示 -->
        <div class="space-y-4">
          <h3 class="text-xl font-medium text-gray-800 dark:text-gray-200">动画效果</h3>
          <div class="grid grid-cols-2 md:grid-cols-4 gap-4">
            <div
              v-for="animation in animationItems"
              :key="animation.name"
              class="p-4 rounded-lg bg-gray-100 dark:bg-gray-800 text-center"
              :class="animation.class"
            >
              <p class="text-sm font-medium text-gray-700 dark:text-gray-300">{{ animation.name }}</p>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- 设计令牌使用示例 -->
    <section class="space-y-6">
      <h2 class="text-2xl font-semibold text-gray-900 dark:text-white border-b pb-2">设计令牌使用</h2>

      <div class="space-y-4">
        <div class="p-6 rounded-lg glass-effect">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">CSS变量使用示例</h3>
          <p class="text-gray-600 dark:text-gray-400 mb-4">
            设计系统通过CSS变量提供设计令牌，可以在任何地方使用。
          </p>
          <div class="space-y-2 font-mono text-sm">
            <code class="block p-2 bg-gray-900 text-gray-100 rounded">
              .custom-element &#123;<br>
              &nbsp;&nbsp;color: var(--color-primary);<br>
              &nbsp;&nbsp;padding: var(--spacing-4);<br>
              &nbsp;&nbsp;border-radius: var(--radius-lg);<br>
              &nbsp;&nbsp;box-shadow: var(--shadow-glass);<br>
              &#125;
            </code>
          </div>
        </div>

        <div class="p-6 rounded-lg bg-gray-50 dark:bg-gray-900">
          <h3 class="text-lg font-semibold text-gray-900 dark:text-white mb-2">TypeScript类型使用</h3>
          <p class="text-gray-600 dark:text-gray-400 mb-4">
            设计令牌也通过TypeScript类型导出，提供类型安全：
          </p>
          <div class="space-y-2 font-mono text-sm">
            <code class="block p-2 bg-gray-900 text-gray-100 rounded">
              import &#123; designTokens &#125; from '@/tokens'<br><br>
              // 使用颜色<br>
              const primaryColor = designTokens.colors.primary.DEFAULT<br><br>
              // 使用间距<br>
              const spacing = designTokens.spacing[4] // "1rem"<br><br>
              // 使用圆角<br>
              const borderRadius = designTokens.borderRadius.lg // "0.5rem"
            </code>
          </div>
        </div>
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue'
import GlassCard from './GlassCard.vue'
import GlassButton from './GlassButton.vue'
import { designTokens } from '@/tokens'

// 颜色数据
const primaryColors = {
  '50': designTokens.colors.primary[50],
  '100': designTokens.colors.primary[100],
  '500': designTokens.colors.primary[500],
  '700': designTokens.colors.primary[700],
  '900': designTokens.colors.primary[900],
}

const functionalColors = [
  { name: 'success', label: '成功', value: designTokens.colors.success.DEFAULT },
  { name: 'warning', label: '警告', value: designTokens.colors.warning.DEFAULT },
  { name: 'error', label: '错误', value: designTokens.colors.error.DEFAULT },
  { name: 'info', label: '信息', value: designTokens.colors.info.DEFAULT },
]

// 排版数据
const typographyItems = [
  { name: 'H1 - 页面主标题', class: 'text-4xl font-bold', description: '用于页面主标题' },
  { name: 'H2 - 区块标题', class: 'text-3xl font-semibold', description: '用于主要区块标题' },
  { name: 'H3 - 卡片标题', class: 'text-2xl font-medium', description: '用于卡片和组件标题' },
  { name: 'Body - 常规正文', class: 'text-base', description: '用于常规正文内容' },
  { name: 'Caption - 说明文字', class: 'text-sm text-gray-500', description: '用于辅助说明文字' },
]

// 间距数据
const spacingItems = [
  { name: 'xs (4px)', value: designTokens.spacing[1], width: '1rem' },
  { name: 'sm (8px)', value: designTokens.spacing[2], width: '2rem' },
  { name: 'md (16px)', value: designTokens.spacing[4], width: '4rem' },
  { name: 'lg (24px)', value: designTokens.spacing[6], width: '6rem' },
  { name: 'xl (32px)', value: designTokens.spacing[8], width: '8rem' },
]

// 圆角数据
const borderRadiusItems = [
  { name: '无', class: 'rounded-none', value: designTokens.borderRadius.none },
  { name: '小', class: 'rounded-sm', value: designTokens.borderRadius.sm },
  { name: '中', class: 'rounded-md', value: designTokens.borderRadius.md },
  { name: '大', class: 'rounded-lg', value: designTokens.borderRadius.lg },
  { name: '特大', class: 'rounded-xl', value: designTokens.borderRadius.xl },
  { name: '全圆', class: 'rounded-full', value: designTokens.borderRadius.full },
]

// 阴影数据
const shadowItems = [
  { name: '小阴影', class: 'shadow-sm' },
  { name: '默认阴影', class: 'shadow' },
  { name: '中阴影', class: 'shadow-md' },
  { name: '大阴影', class: 'shadow-lg' },
  { name: '特大阴影', class: 'shadow-xl' },
  { name: '玻璃阴影', class: 'shadow-glass' },
]

// 动画数据
const animationItems = [
  { name: '淡入', class: 'animate-fade-in' },
  { name: '上滑', class: 'animate-slide-up' },
  { name: '缩放', class: 'animate-scale-up' },
  { name: '旋转', class: 'animate-spin' },
  { name: '脉动', class: 'animate-pulse' },
  { name: '弹跳', class: 'animate-bounce' },
]
</script>

<style scoped>
.design-system-showcase {
  max-width: 1200px;
  margin: 0 auto;
  padding: 2rem;
}

/* 自定义动画演示 */
@keyframes demoFadeIn {
  from { opacity: 0; }
  to { opacity: 1; }
}

@keyframes demoSlideUp {
  from {
    transform: translateY(20px);
    opacity: 0;
  }
  to {
    transform: translateY(0);
    opacity: 1;
  }
}

@keyframes demoScaleUp {
  from {
    transform: scale(0.9);
    opacity: 0;
  }
  to {
    transform: scale(1);
    opacity: 1;
  }
}

.animate-fade-in {
  animation: demoFadeIn 0.5s ease-in-out;
}

.animate-slide-up {
  animation: demoSlideUp 0.5s ease-out;
}

.animate-scale-up {
  animation: demoScaleUp 0.5s ease-out;
}

.animate-spin {
  animation: spin 2s linear infinite;
}

.animate-pulse {
  animation: pulse 2s ease-in-out infinite;
}

.animate-bounce {
  animation: bounce 1s ease-in-out infinite;
}
</style>
