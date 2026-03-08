<template>
  <div class="max-w-4xl mx-auto">
    <!-- 页面标题 -->
    <div class="mb-8">
      <h1 class="text-2xl font-bold text-gray-900 dark:text-white">关于</h1>
      <p class="text-gray-600 dark:text-gray-400">胧压缩·方便助手</p>
    </div>

      <main class="space-y-8">
        <!-- 应用信息 -->
        <div class="glass-card">
          <div class="flex items-center space-x-6 mb-6">
            <div class="w-20 h-20 rounded-2xl bg-gradient-to-br from-primary-500 to-accent-500 flex items-center justify-center">
              <i class="pi pi-box text-white text-3xl"></i>
            </div>
            <div>
              <h2 class="text-2xl font-bold text-gray-900 dark:text-white">胧压缩·方便助手</h2>
              <p class="text-gray-600 dark:text-gray-400">版本 1.0.0</p>
              <p class="text-gray-500 dark:text-gray-400 text-sm mt-1">构建时间: 2024-03-08</p>
            </div>
          </div>

          <p class="text-gray-700 dark:text-gray-300 mb-4">
            胧压缩·方便助手是一个现代化的文件解压工具，专注于提供安全、高效、美观的文件解压体验。
            支持多种压缩格式，具备智能文件管理功能，采用苹果毛玻璃设计风格，带来优雅的用户界面。
          </p>

          <div class="grid grid-cols-2 md:grid-cols-4 gap-4 mt-6">
            <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="text-2xl font-bold text-primary mb-1">100+</div>
              <div class="text-sm text-gray-600 dark:text-gray-400">支持格式</div>
            </div>
            <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="text-2xl font-bold text-accent mb-1">∞</div>
              <div class="text-sm text-gray-600 dark:text-gray-400">文件大小</div>
            </div>
            <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="text-2xl font-bold text-green-500 mb-1">256位</div>
              <div class="text-sm text-gray-600 dark:text-gray-400">加密强度</div>
            </div>
            <div class="text-center p-4 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="text-2xl font-bold text-purple-500 mb-1">8x</div>
              <div class="text-sm text-gray-600 dark:text-gray-400">解压速度</div>
            </div>
          </div>
        </div>

        <!-- 功能特性 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">功能特性</h2>

          <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
            <div class="flex items-start space-x-4">
              <div class="w-10 h-10 rounded-lg bg-primary/10 flex items-center justify-center flex-shrink-0">
                <i class="pi pi-shield text-primary"></i>
              </div>
              <div>
                <h3 class="font-medium text-gray-900 dark:text-white mb-1">安全可靠</h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  内置病毒扫描，可疑文件检测，密码加密存储，确保文件安全。
                </p>
              </div>
            </div>

            <div class="flex items-start space-x-4">
              <div class="w-10 h-10 rounded-lg bg-accent/10 flex items-center justify-center flex-shrink-0">
                <i class="pi pi-bolt text-accent"></i>
              </div>
              <div>
                <h3 class="font-medium text-gray-900 dark:text-white mb-1">高效快速</h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  多线程解压，智能缓存，GPU加速，大幅提升大文件处理速度。
                </p>
              </div>
            </div>

            <div class="flex items-start space-x-4">
              <div class="w-10 h-10 rounded-lg bg-green-500/10 flex items-center justify-center flex-shrink-0">
                <i class="pi pi-eye text-green-500"></i>
              </div>
              <div>
                <h3 class="font-medium text-gray-900 dark:text-white mb-1">预览功能</h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  解压前预览文件内容，支持图片、文本、PDF、Office文档等多种格式。
                </p>
              </div>
            </div>

            <div class="flex items-start space-x-4">
              <div class="w-10 h-10 rounded-lg bg-purple-500/10 flex items-center justify-center flex-shrink-0">
                <i class="pi pi-palette text-purple-500"></i>
              </div>
              <div>
                <h3 class="font-medium text-gray-900 dark:text-white mb-1">美观界面</h3>
                <p class="text-gray-600 dark:text-gray-400 text-sm">
                  苹果毛玻璃设计风格，暗色/亮色主题，流畅动画，优雅的用户体验。
                </p>
              </div>
            </div>
          </div>
        </div>

        <!-- 技术支持 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-6">技术支持</h2>

          <div class="space-y-4">
            <div class="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center">
                <i class="pi pi-question-circle text-gray-500 mr-3"></i>
                <div>
                  <p class="font-medium text-gray-900 dark:text-white">帮助文档</p>
                  <p class="text-sm text-gray-500 dark:text-gray-400">查看完整的使用指南和常见问题</p>
                </div>
              </div>
              <button class="glass-button px-4 py-2">
                查看
              </button>
            </div>

            <div class="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center">
                <i class="pi pi-comments text-gray-500 mr-3"></i>
                <div>
                  <p class="font-medium text-gray-900 dark:text-white">用户社区</p>
                  <p class="text-sm text-gray-500 dark:text-gray-400">加入用户社区交流使用经验</p>
                </div>
              </div>
              <button class="glass-button px-4 py-2">
                加入
              </button>
            </div>

            <div class="flex items-center justify-between p-3 rounded-lg bg-gray-50 dark:bg-gray-800">
              <div class="flex items-center">
                <i class="pi pi-bug text-gray-500 mr-3"></i>
                <div>
                  <p class="font-medium text-gray-900 dark:text-white">报告问题</p>
                  <p class="text-sm text-gray-500 dark:text-gray-400">提交bug报告或功能建议</p>
                </div>
              </div>
              <button class="glass-button px-4 py-2">
                报告
              </button>
            </div>
          </div>
        </div>

        <!-- 开源许可 -->
        <div class="glass-card">
          <h2 class="text-xl font-semibold text-gray-900 dark:text-white mb-4">开源许可</h2>

          <div class="space-y-4">
            <p class="text-gray-700 dark:text-gray-300">
              胧压缩·方便助手基于MIT开源协议发布。您可以自由使用、修改和分发本软件，但需保留原作者的版权声明。
            </p>

            <div class="bg-gray-50 dark:bg-gray-800 rounded-lg p-4">
              <pre class="text-sm text-gray-600 dark:text-gray-400 overflow-x-auto">MIT License

Copyright (c) 2024 胧压缩·方便助手

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.</pre>
            </div>

            <div class="flex items-center justify-between pt-4 border-t border-gray-200 dark:border-gray-700">
              <div class="text-sm text-gray-600 dark:text-gray-400">
                © 2024 胧压缩·方便助手. 保留所有权利。
              </div>
              <div class="flex items-center space-x-4">
                <a href="#" class="text-primary hover:underline text-sm">隐私政策</a>
                <a href="#" class="text-primary hover:underline text-sm">服务条款</a>
              </div>
            </div>
          </div>
        </div>
      </main>
    </div>
</template>

<script setup lang="ts">
import { useRouter } from 'vue-router'

const router = useRouter()
</script>

<style scoped>
pre {
  font-family: 'JetBrains Mono', monospace;
  line-height: 1.5;
}
</style>