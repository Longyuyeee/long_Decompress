import { describe, it, expect, vi } from 'vitest'
import { mount } from '@vue/test-utils'
import TaskDetailPanel from '../TaskDetailPanel.vue'
import type { DecompressTask } from '@/stores'

// 模拟任务数据
const mockTask: DecompressTask = {
  id: 'task-123',
  fileId: 'file-456',
  filePath: '/path/to/archive.zip',
  fileName: 'archive.zip',
  outputPath: '/path/to/output',
  password: 'secret123',
  options: {
    keepStructure: true,
    overwrite: false,
    deleteAfter: false
  },
  status: 'processing',
  progress: 50,
  startTime: new Date('2026-03-09T10:00:00'),
  endTime: undefined,
  error: undefined,
  createdAt: new Date('2026-03-09T09:55:00')
}

const mockCompletedTask: DecompressTask = {
  ...mockTask,
  id: 'task-124',
  status: 'completed',
  progress: 100,
  endTime: new Date('2026-03-09T10:05:00')
}

const mockErrorTask: DecompressTask = {
  ...mockTask,
  id: 'task-125',
  status: 'error',
  progress: 0,
  error: '密码错误，解压失?'
}

describe('TaskDetailPanel组件', () => {
  it('渲染正确', () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.task-detail-panel').exists()).toBe(true)
    expect(wrapper.text()).toContain('archive.zip')
    expect(wrapper.text()).toContain('进行?)'
  })

  it('显示处理中任务的状?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    // 检查状态显?
    expect(wrapper.text()).toContain('进行?)'
    expect(wrapper.find('.pi-spin.pi-spinner').exists()).toBe(true)

    // 检查进度条
    const progressBar = wrapper.find('.bg-primary')
    expect(progressBar.exists()).toBe(true)
    expect(progressBar.attributes('style')).toContain('width: 50%')

    // 检查进度百分比
    expect(wrapper.text()).toContain('50%')
  })

  it('显示已完成任务的状?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockCompletedTask
      }
    })

    expect(wrapper.text()).toContain('已完?)'
    expect(wrapper.find('.pi-check').exists()).toBe(true)
    expect(wrapper.text()).toContain('解压成功')
    expect(wrapper.text()).toContain('解压时长')
  })

  it('显示失败任务的状态和错误信息', () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockErrorTask
      }
    })

    expect(wrapper.text()).toContain('失败')
    expect(wrapper.find('.pi-times').exists()).toBe(true)
    expect(wrapper.text()).toContain('密码错误，解压失?)'
    expect(wrapper.text()).toContain('建议解决方案')
  })

  it('显示文件信息', () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    expect(wrapper.text()).toContain('archive.zip')
    expect(wrapper.text()).toContain('/path/to/archive.zip')
    expect(wrapper.text()).toContain('/path/to/output')
    expect(wrapper.text()).toContain('已设置密?)'
  })

  it('显示解压选项', () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    expect(wrapper.text()).toContain('保持原结?)'
    expect(wrapper.text()).toContain('询问用户')
    expect(wrapper.text()).toContain('保留原文?)'
  })

  it('显示时间线信?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    expect(wrapper.text()).toContain('任务创建')
    expect(wrapper.text()).toContain('开始解?)'
    expect(wrapper.text()).toContain('2026???)'
  })

  it('显示技术信?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    expect(wrapper.text()).toContain('任务ID')
    expect(wrapper.text()).toContain('task-123')
    expect(wrapper.text()).toContain('文件ID')
    expect(wrapper.text()).toContain('file-456')
    expect(wrapper.text()).toContain('API版本')
  })

  it('处理关闭事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    const closeButton = wrapper.find('button:contains("关闭")')
    await closeButton.trigger('click')

    expect(wrapper.emitted('close')).toBeTruthy()
  })

  it('处理重试事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockErrorTask
      }
    })

    const retryButton = wrapper.find('button:contains("重试")')
    await retryButton.trigger('click')

    expect(wrapper.emitted('retry')).toBeTruthy()
    expect(wrapper.emitted('retry')?.[0]?.[0]).toBe('task-125')
  })

  it('处理取消事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    const cancelButton = wrapper.find('button:contains("取消")')
    await cancelButton.trigger('click')

    expect(wrapper.emitted('cancel')).toBeTruthy()
    expect(wrapper.emitted('cancel')?.[0]?.[0]).toBe('task-123')
  })

  it('处理打开输出目录事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockCompletedTask
      }
    })

    const openButton = wrapper.find('button:contains("打开输出目录")')
    await openButton.trigger('click')

    expect(wrapper.emitted('open-output')).toBeTruthy()
    expect(wrapper.emitted('open-output')?.[0]?.[0]).toBe('/path/to/output')
  })

  it('处理复制路径事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    const copyButton = wrapper.find('button:contains("复制输出路径")')
    await copyButton.trigger('click')

    expect(wrapper.emitted('copy-path')).toBeTruthy()
    expect(wrapper.emitted('copy-path')?.[0]?.[0]).toBe('/path/to/output')
  })

  it('处理在资源管理器中显示事?, async () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    const showButton = wrapper.find('button:contains("在资源管理器中显?)')\"
    await showButton.trigger('click')

    expect(wrapper.emitted('show-in-explorer')).toBeTruthy()
    expect(wrapper.emitted('show-in-explorer')?.[0]?.[0]).toBe('/path/to/archive.zip')
  })

  it('处理导出日志事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    const exportButton = wrapper.find('button:contains("导出日志")')
    await exportButton.trigger('click')

    expect(wrapper.emitted('export-log')).toBeTruthy()
    expect(wrapper.emitted('export-log')?.[0]?.[0]).toBe('task-123')
  })

  it('处理删除事件', async () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    const deleteButton = wrapper.find('button:contains("删除任务记录")')
    await deleteButton.trigger('click')

    expect(wrapper.emitted('delete')).toBeTruthy()
    expect(wrapper.emitted('delete')?.[0]?.[0]).toBe('task-123')
  })

  it('禁用状态下的按?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask,
        isProcessing: true
      }
    })

    const buttons = wrapper.findAll('button')
    buttons.forEach(button => {
      if (!button.text().includes('关闭')) {
        expect(button.attributes('disabled')).toBeDefined()
      }
    })
  })

  it('提供错误解决方案建议', () => {
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockErrorTask
      }
    })

    expect(wrapper.text()).toContain('建议解决方案')
    expect(wrapper.text()).toContain('请检查输入的密码是否正确')
  })

  it('计算预计剩余时间', () => {
    const taskWithProgress: DecompressTask = {
      ...mockTask,
      startTime: new Date(Date.now() - 30000), // 30秒前开?
      progress: 50
    }

    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: taskWithProgress
      }
    })

    expect(wrapper.text()).toContain('预计剩余')
  })

  it('格式化时间显?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockTask
      }
    })

    expect(wrapper.text()).toContain('2026???)'
    expect(wrapper.text()).toContain('10:00:00')
  })

  it('格式化路径显?, () => {'
    const longPathTask: DecompressTask = {
      ...mockTask,
      filePath: '/very/long/path/to/the/archive/file/that/is/very/long/archive.zip',
      outputPath: '/another/very/long/output/path/that/is/also/very/long/output'
    }

    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: longPathTask
      }
    })

    // 应该显示缩短的路?
    expect(wrapper.text()).toContain('...')
  })

  it('格式化时长显?, () => {'
    const wrapper = mount(TaskDetailPanel, {
      props: {
        task: mockCompletedTask
      }
    })

    expect(wrapper.text()).toContain('5分钟')
  })

  describe('响应式设?, () => {'
    it('网格布局适应不同屏幕尺寸', () => {
      const wrapper = mount(TaskDetailPanel, {
        props: {
          task: mockTask
        }
      })

      const mainGrid = wrapper.find('.grid.grid-cols-1.lg\\:grid-cols-3')
      expect(mainGrid.exists()).toBe(true)
    })

    it('文件信息区域适应不同屏幕尺寸', () => {
      const wrapper = mount(TaskDetailPanel, {
        props: {
          task: mockTask
        }
      })

      const fileInfoGrid = wrapper.find('.grid.grid-cols-1.sm\\:grid-cols-2')
      expect(fileInfoGrid.exists()).toBe(true)
    })
  })

  describe('暴露的方?, () => {'
    it('暴露getTaskInfo方法', () => {
      const wrapper = mount(TaskDetailPanel, {
        props: {
          task: mockTask
        }
      })

      expect(wrapper.vm.getTaskInfo).toBeDefined()
      expect(typeof wrapper.vm.getTaskInfo).toBe('function')

      const taskInfo = wrapper.vm.getTaskInfo()
      expect(taskInfo).toEqual(mockTask)
    })
  })
})
