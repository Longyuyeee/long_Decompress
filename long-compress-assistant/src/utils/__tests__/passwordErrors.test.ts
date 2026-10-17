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
    '[archive-output:Publishing:rollback-incomplete] wrong password',
    '[archive-output:Publishing:publication-failed] wrong password',
    '[archive-output:Publishing:output-conflict] encrypted archive',
    '[archive-output:Verifying:verification-failed] wrong password',
    '[archive-source:Pre-checking:source-missing] wrong password',
    '[archive-source:Extracting:source-unavailable] 密码错误',
    '[archive-source:Pre-checking:source-invalid] encrypted archive',
    '[archive-source:Pre-checking:source-changed] Wrong password',
    '[archive-source:Extracting:source-changed] 密码错误',
    '目标磁盘空间不足',
    '归档结构损坏',
    '归档检测失败：[archive-inspection:damaged] Unexpected end of archive; Wrong password',
    '[archive-inspection:ambiguous-data] Data error in encrypted file. Wrong password?',
    '[archive-inspection:password-encoding] 此密码无法按所选 ZIP 编码无损表示，请更换编码',
    '[archive-inspection:password-encoding] ZIP 密码编码转换失败：InvalidPassword is not a password mismatch here',
    '归档检测失败：encrypted archive inspection timed out',
  ])('does not misclassify an unrelated failure: %s', message => {
    expect(isPasswordRelatedError(message)).toBe(false)
  })
})
