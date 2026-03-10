<template>
  <Transition
    :name="transitionName"
    :mode="mode"
    @before-enter="beforeEnter"
    @enter="enter"
    @after-enter="afterEnter"
    @enter-cancelled="enterCancelled"
    @before-leave="beforeLeave"
    @leave="leave"
    @after-leave="afterLeave"
    @leave-cancelled="leaveCancelled"
  >
    <slot />
  </Transition>
</template>

<script setup lang="ts">
import { ref, watch, onMounted, onUnmounted } from 'vue'
import { useRoute } from 'vue-router'

interface Props {
  mode?: 'default' | 'out-in' | 'in-out'
  duration?: number
  disable?: boolean
}

const props = withDefaults(defineProps<Props>(), {
  mode: 'out-in',
  duration: 300,
  disable: false
})

const route = useRoute()
const transitionName = ref('fade')
const isReducedMotion = ref(false)

// 检测减少运动偏好
const checkReducedMotion = () => {
  isReducedMotion.value = window.matchMedia('(prefers-reduced-motion: reduce)').matches
}

// 根据路由变化决定过渡动画
watch(
  () => route.path,
  (to, from) => {
    if (props.disable || isReducedMotion.value) {
      transitionName.value = 'none'
      return
    }

    // 增加空值检查，防止初次加载时 from 为 undefined 导致的 split 报错
    if (!to || !from) {
      transitionName.value = 'fade'
      return
    }

    // 简单的路由深度检测
    const toDepth = to.split('/').length
    const fromDepth = from.split('/').length

    if (toDepth > fromDepth) {
      // 进入更深层页面
      transitionName.value = 'slide-left'
    } else if (toDepth < fromDepth) {
      // 返回上层页面
      transitionName.value = 'slide-right'
    } else {
      // 同级页面切换
      transitionName.value = 'fade'
    }
  },
  { immediate: true }
)

// 过渡生命周期钩子
const beforeEnter = (el: Element) => {
  ;(el as HTMLElement).style.animationDuration = `${props.duration}ms`
}

const enter = (el: Element, done: () => void) => {
  if (props.disable || isReducedMotion.value) {
    done()
    return
  }
  // 动画完成后调用done
  setTimeout(done, props.duration)
}

const afterEnter = (el: Element) => {
  // 清理样式
  ;(el as HTMLElement).style.animationDuration = ''
}

const enterCancelled = (el: Element) => {
  // 清理样式
  ;(el as HTMLElement).style.animationDuration = ''
}

const beforeLeave = (el: Element) => {
  ;(el as HTMLElement).style.animationDuration = `${props.duration}ms`
}

const leave = (el: Element, done: () => void) => {
  if (props.disable || isReducedMotion.value) {
    done()
    return
  }
  // 动画完成后调用done
  setTimeout(done, props.duration)
}

const afterLeave = (el: Element) => {
  // 清理样式
  ;(el as HTMLElement).style.animationDuration = ''
}

const leaveCancelled = (el: Element) => {
  // 清理样式
  ;(el as HTMLElement).style.animationDuration = ''
}

// 监听减少运动偏好变化
let mediaQuery: MediaQueryList
const setupReducedMotionListener = () => {
  mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)')
  mediaQuery.addEventListener('change', checkReducedMotion)
}

// 初始�?
onMounted(() => {
  checkReducedMotion()
  setupReducedMotionListener()
})

// 清理
onUnmounted(() => {
  if (mediaQuery) {
    mediaQuery.removeEventListener('change', checkReducedMotion)
  }
})
</script>

<style scoped>
/* 淡入淡出过渡 */
.fade-enter-active,
.fade-leave-active {
  transition: opacity v-bind(duration + 'ms') ease;
}

.fade-enter-from,
.fade-leave-to {
  opacity: 0;
}

/* 向左滑动过渡 */
.slide-left-enter-active,
.slide-left-leave-active {
  transition: all v-bind(duration + 'ms') ease;
}

.slide-left-enter-from {
  opacity: 0;
  transform: translateX(30px);
}

.slide-left-leave-to {
  opacity: 0;
  transform: translateX(-30px);
}

/* 向右滑动过渡 */
.slide-right-enter-active,
.slide-right-leave-active {
  transition: all v-bind(duration + 'ms') ease;
}

.slide-right-enter-from {
  opacity: 0;
  transform: translateX(-30px);
}

.slide-right-leave-to {
  opacity: 0;
  transform: translateX(30px);
}

/* 缩放过渡 */
.scale-enter-active,
.scale-leave-active {
  transition: all v-bind(duration + 'ms') ease;
}

.scale-enter-from,
.scale-leave-to {
  opacity: 0;
  transform: scale(0.95);
}

/* 无过渡（用于减少运动偏好�?*/
.none-enter-active,
.none-leave-active {
  transition: none;
}

.none-enter-from,
.none-leave-to {
  opacity: 1;
  transform: none;
}
</style>
