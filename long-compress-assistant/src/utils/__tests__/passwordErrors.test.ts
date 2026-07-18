import { describe, expect, it } from 'vitest'
import { isPasswordRelatedError } from '@/utils'

describe('isPasswordRelatedError', () => {
  it.each([
    'PasswordRequired',
    'wrong password',
    'InvalidPassword',
    '需要输入密码才能解压',
    '提供的密码不正确',
    'RAR 解压失败: 密码错误',
    'AES 解密失败: 密码错误或文件已损坏',
  ])('recognizes password failure: %s', message => {
    expect(isPasswordRelatedError(message)).toBe(true)
  })

  it.each([
    '文件不存在',
    '目标磁盘空间不足',
    '归档结构损坏',
  ])('does not misclassify an unrelated failure: %s', message => {
    expect(isPasswordRelatedError(message)).toBe(false)
  })
})
