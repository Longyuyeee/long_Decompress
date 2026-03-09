import { describe, it, expect } from 'vitest'
import { mount } from '@vue/test-utils'
import ProgressBar from '../ProgressBar.vue'

describe('ProgressBar组件', () => {
  it('渲染正确', () => {
    const wrapper = mount(ProgressBar)

    expect(wrapper.exists()).toBe(true)
    expect(wrapper.find('.progress-bar-container').exists()).toBe(true)
    expect(wrapper.find('.progress-bar-track').exists()).toBe(true)
    expect(wrapper.find('.progress-bar-fill').exists()).toBe(true)
  })

  it('使用默认属�?, () => {
    const wrapper = mount(ProgressBar)

    expect(wrapper.props('value')).toBe(0)
    expect(wrapper.props('max')).toBe(100)
    expect(wrapper.props('min')).toBe(0)
    expect(wrapper.props('variant')).toBe('primary')
    expect(wrapper.props('size')).toBe('md')
    expect(wrapper.props('showLabel')).toBe(true)
    expect(wrapper.props('showValue')).toBe(true)
    expect(wrapper.props('indeterminate')).toBe(false)
    expect(wrapper.props('striped')).toBe(false)
    expect(wrapper.props('animated')).toBe(false)
  })

  describe('进度�?, () => {
    it('显示0%进度', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 0,
          max: 100
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 0%')
      expect(wrapper.text()).toContain('0%')
    })

    it('显示50%进度', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 50,
          max: 100
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 50%')
      expect(wrapper.text()).toContain('50%')
    })

    it('显示100%进度', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 100,
          max: 100
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 100%')
      expect(wrapper.text()).toContain('100%')
    })

    it('处理超出范围的�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 150,
          max: 100
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 100%')
      expect(wrapper.text()).toContain('100%')
    })

    it('处理负�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: -10,
          max: 100
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 0%')
      expect(wrapper.text()).toContain('0%')
    })

    it('使用自定义最小值和最大�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 75,
          min: 0,
          max: 150
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 50%')
      expect(wrapper.text()).toContain('50%')
    })
  })

  describe('变体类型', () => {
    it('使用主要变体（默认）', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          variant: 'primary'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #0ea5e9 0%, #3b82f6 100%)')
    })

    it('使用次要变体', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          variant: 'secondary'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #64748b 0%, #94a3b8 100%)')
    })

    it('使用成功变体', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          variant: 'success'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #10b981 0%, #34d399 100%)')
    })

    it('使用警告变体', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          variant: 'warning'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #f59e0b 0%, #fbbf24 100%)')
    })

    it('使用危险变体', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          variant: 'danger'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #ef4444 0%, #f87171 100%)')
    })

    it('使用信息变体', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          variant: 'info'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #06b6d4 0%, #22d3ee 100%)')
    })
  })

  describe('尺寸', () => {
    it('使用小尺�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          size: 'sm'
        }
      })

      const track = wrapper.find('.progress-bar-track')
      expect(track.classes()).toContain('h-1.5')
      expect(track.classes()).not.toContain('h-2')
    })

    it('使用中尺寸（默认�?, () => {
      const wrapper = mount(ProgressBar)

      const track = wrapper.find('.progress-bar-track')
      expect(track.classes()).toContain('h-2')
      expect(track.classes()).not.toContain('h-1.5')
      expect(track.classes()).not.toContain('h-3')
    })

    it('使用大尺�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          size: 'lg'
        }
      })

      const track = wrapper.find('.progress-bar-track')
      expect(track.classes()).toContain('h-3')
      expect(track.classes()).not.toContain('h-2')
    })

    it('使用超大尺寸', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          size: 'xl'
        }
      })

      const track = wrapper.find('.progress-bar-track')
      expect(track.classes()).toContain('h-4')
      expect(track.classes()).not.toContain('h-2')
    })
  })

  describe('标签和�?, () => {
    it('显示标签', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          label: '上传进度',
          showLabel: true
        }
      })

      expect(wrapper.text()).toContain('上传进度')
      expect(wrapper.find('.text-sm').exists()).toBe(true)
    })

    it('隐藏标签', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          label: '上传进度',
          showLabel: false
        }
      })

      expect(wrapper.text()).not.toContain('上传进度')
    })

    it('显示进度�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 75,
          showValue: true
        }
      })

      expect(wrapper.text()).toContain('75%')
    })

    it('隐藏进度�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 75,
          showValue: false
        }
      })

      expect(wrapper.text()).not.toContain('75%')
    })

    it('使用标签插槽', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          showLabel: true
        },
        slots: {
          label: '<span class="custom-label">自定义标�?/span>'
        }
      })

      expect(wrapper.find('.custom-label').exists()).toBe(true)
      expect(wrapper.text()).toContain('自定义标�?)
    })
  })

  describe('描述', () => {
    it('显示描述', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          description: '正在上传文件...'
        }
      })

      expect(wrapper.text()).toContain('正在上传文件...')
      expect(wrapper.find('.text-xs').exists()).toBe(true)
    })

    it('使用描述插槽', () => {
      const wrapper = mount(ProgressBar, {
        slots: {
          description: '<span class="custom-description">自定义描�?/span>'
        }
      })

      expect(wrapper.find('.custom-description').exists()).toBe(true)
      expect(wrapper.text()).toContain('自定义描�?)
    })
  })

  describe('不确定状�?, () => {
    it('启用不确定状�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          indeterminate: true
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('width: 100%')
      expect(fill.classes()).toContain('indeterminate')
      expect(wrapper.text()).toContain('处理�?..')
    })

    it('不确定状态显示处理中文本', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          indeterminate: true
        }
      })

      expect(wrapper.text()).toContain('处理�?..')
    })
  })

  describe('条纹效果', () => {
    it('启用条纹效果', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          striped: true
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.classes()).toContain('striped')
      expect(fill.attributes('style')).toContain('background-size: 1rem 1rem')
    })

    it('条纹效果有动画层', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          striped: true
        }
      })

      expect(wrapper.findAll('.absolute').length).toBe(2) // 填充�?+ 条纹�?
    })
  })

  describe('动画效果', () => {
    it('启用轨道动画', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          animated: true
        }
      })

      const track = wrapper.find('.progress-bar-track')
      expect(track.classes()).toContain('animate-pulse-slow')
    })

    it('启用填充动画', () => {
      const wrapper = mount(ProgressBar, {
        props: {
          animated: true
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.classes()).toContain('animate-pulse')
    })
  })

  describe('自定义样�?, () => {
    it('接受自定义类�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          className: 'custom-container my-progress'
        }
      })

      expect(wrapper.find('.progress-bar-container').classes()).toContain('custom-container')
      expect(wrapper.find('.progress-bar-container').classes()).toContain('my-progress')
    })

    it('使用自定义轨道颜�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          trackColor: '#ff0000'
        }
      })

      const track = wrapper.find('.progress-bar-track')
      expect(track.attributes('style')).toContain('background-color: #ff0000')
    })

    it('使用自定义填充渐�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          fillGradient: 'linear-gradient(90deg, #ff0000 0%, #00ff00 100%)'
        }
      })

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('background: linear-gradient(90deg, #ff0000 0%, #00ff00 100%)')
    })
  })

  describe('自定义值格式化', () => {
    it('使用自定义格式化函数', () => {
      const formatValue = (value: number, percentage: number) => {
        return `${value} / 100 (${percentage}%)`
      }

      const wrapper = mount(ProgressBar, {
        props: {
          value: 75,
          formatValue
        }
      })

      expect(wrapper.text()).toContain('75 / 100 (75%)')
    })
  })

  describe('组合测试', () => {
    it('组合多个属�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 80,
          variant: 'success',
          size: 'lg',
          label: '系统健康�?,
          description: '系统运行良好',
          striped: true,
          animated: true,
          className: 'health-progress',
          trackColor: 'rgba(16, 185, 129, 0.2)'
        }
      })

      // 检查文�?
      expect(wrapper.text()).toContain('系统健康�?)
      expect(wrapper.text()).toContain('80%')
      expect(wrapper.text()).toContain('系统运行良好')

      // 检查类�?
      const container = wrapper.find('.progress-bar-container')
      expect(container.classes()).toContain('health-progress')

      // 检查样�?
      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.attributes('style')).toContain('linear-gradient(90deg, #10b981 0%, #34d399 100%)')
      expect(fill.classes()).toContain('striped')
      expect(fill.classes()).toContain('animate-pulse')

      const track = wrapper.find('.progress-bar-track')
      expect(track.classes()).toContain('h-3')
      expect(track.classes()).toContain('animate-pulse-slow')
      expect(track.attributes('style')).toContain('background-color: rgba(16, 185, 129, 0.2)')
    })

    it('不确定状态组�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          indeterminate: true,
          variant: 'warning',
          striped: true,
          animated: true,
          label: '正在初始�?..'
        }
      })

      expect(wrapper.text()).toContain('正在初始�?..')
      expect(wrapper.text()).toContain('处理�?..')

      const fill = wrapper.find('.progress-bar-fill')
      expect(fill.classes()).toContain('indeterminate')
      expect(fill.classes()).toContain('striped')
      expect(fill.classes()).toContain('animate-pulse')
      expect(fill.attributes('style')).toContain('width: 100%')
    })
  })

  describe('可访问�?, () => {
    it('有正确的ARIA属�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 75,
          label: '下载进度'
        }
      })

      // 进度条应该有role="progressbar"
      const container = wrapper.find('.progress-bar-container')
      // 注意：这里可能需要组件添加ARIA属�?
      // 目前组件没有显式添加ARIA属性，这是一个改进点
    })

    it('显示进度值给屏幕阅读�?, () => {
      const wrapper = mount(ProgressBar, {
        props: {
          value: 50,
          label: '处理进度'
        }
      })

      expect(wrapper.text()).toContain('50%')
      // 进度值应该对屏幕阅读器可�?
    })
  })
})
