import { describe, expect, it } from 'vitest'
import { inferExtractionSubfolder } from '../smartExtraction'

describe('smart extraction roots', () => {
  it.each([
    [['报告.txt'], false],
    [['资料/', '资料/说明.txt', '资料/照片/a.jpg'], false],
    [['资料/说明.txt', '资料/照片/a.jpg'], false],
    [['甲/说明.txt', '乙/说明.txt'], true],
    [['甲\\说明.txt', '乙\\说明.txt'], true],
    [['照片.jpg', '说明.txt'], true],
    [[], undefined],
    [['../说明.txt'], undefined],
    [['/绝对/路径'], undefined],
    [['C:\\路径.txt'], undefined],
  ] as const)('plans %j without relying on explicit directory records', (entries, expected) => {
    expect(inferExtractionSubfolder(entries)).toBe(expected)
  })
})
