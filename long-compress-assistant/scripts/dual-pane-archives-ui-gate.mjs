import assert from 'node:assert/strict'
import { createHash } from 'node:crypto'
import { execFileSync } from 'node:child_process'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import path from 'node:path'
import { By, Key } from 'selenium-webdriver'

export async function runDualPaneArchivesUiGate({ driver, bridge, application, root }) {
  const base = path.join(root, 'test-results/dual-pane-archives-ui'), work = path.join(base, 'fixtures')
  mkdirSync(work, { recursive: true })
  const layoutOnly = process.argv.includes('--dual-pane-archives-layout-only')
  const receipt = path.join(base, layoutOnly ? 'layout-result.json' : 'result.json'), sha = createHash('sha256').update(readFileSync(application)).digest('hex')
  const state = existsSync(receipt) ? JSON.parse(readFileSync(receipt, 'utf8')) : { binarySha256: sha, checks: [], layouts: [], screenshots: [] }
  assert.equal(state.binarySha256, sha, 'Retain a separate receipt for each native build')
  const save = () => writeFileSync(receipt, JSON.stringify(state, null, 2) + '\n')
  const passed = name => state.checks.includes(name)
  const done = name => { state.checks.push(name); save(); console.log('[dual-pane-ui] ' + name) }
  const find = selector => driver.wait(async () => {
    try { for (const e of await driver.findElements(By.css(selector))) if (await e.isDisplayed()) return e; return false }
    catch (error) { if (error.name === 'StaleElementReferenceError') return false; throw error }
  }, 15000, 'Missing visible ' + selector)
  async function click(selector, twice = false) {
    const e = await find(selector)
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), e)
    await driver.wait(async () => await e.isEnabled() && await driver.executeScript(e => { const r = e.getBoundingClientRect(), hit = document.elementFromPoint(r.x + r.width / 2, r.y + r.height / 2); return e === hit || e.contains(hit) }, e), 10000, 'Control obscured or disabled: ' + selector)
    if (twice) await driver.actions().doubleClick(e).perform(); else await e.click()
  }
  async function fill(selector, value, enter = false) {
    const e = await find(selector)
    await driver.executeScript(e => e.scrollIntoView({ block: 'center', behavior: 'instant' }), e)
    await e.sendKeys(Key.chord(Key.CONTROL, 'a'), Key.BACK_SPACE, value)
    if (enter) await e.sendKeys(Key.ENTER)
  }
  const text = async selector => (await find(selector)).getAttribute('textContent')
  const pane = side => `[data-file-pane="${side}"]`
  const local = (side, name) => `${pane(side)} .file-row[data-path$="${name}"]`
  const entry = (side, name) => `${pane(side)} [data-entry-path="${name}"]`
  async function shot(name, sizes = [[1100, 700], [760, 520]]) {
    for (const [width, height] of sizes) {
      if (state.layouts.some(r => r.name === name && r.width === width)) continue
      await driver.manage().window().setRect({ width, height })
      await driver.wait(() => driver.executeScript(() => document.getAnimations().filter(a => a.effect?.getTiming().iterations !== Infinity).every(a => a.playState !== 'running')), 10000)
      const layout = await driver.executeScript(layoutOnly => {
        const nav = document.querySelector('.app-sidebar > nav'), panes = [...document.querySelectorAll('[data-file-pane]')]
        return { width: innerWidth, height: innerHeight, overflow: document.documentElement.scrollWidth - innerWidth,
          navScrollbar: getComputedStyle(nav).scrollbarWidth, webkitScrollbar: getComputedStyle(nav, '::-webkit-scrollbar').display,
          panes: panes.map(e => ({ width: e.getBoundingClientRect().width, overflow: e.scrollWidth - e.clientWidth,
            archive: !!e.querySelector('[data-testid="archive-pane-browser"]'), listHeight: (layoutOnly ? e.querySelector('.browser-row')?.parentElement?.getBoundingClientRect().height : e.querySelector('.browser-workspace')?.getBoundingClientRect().height) || e.querySelector('.file-list')?.getBoundingClientRect().height })) }
      }, layoutOnly)
      assert.equal(layout.panes.length, 2); assert.ok(Math.abs(layout.panes[0].width - layout.panes[1].width) <= 1)
      assert.ok(layout.overflow <= 1 && layout.panes.every(p => p.overflow <= 1 && p.listHeight > (layoutOnly ? 100 : 70)), JSON.stringify(layout))
      assert.equal(layout.navScrollbar, 'none'); assert.equal(layout.webkitScrollbar, 'none')
      const file = `${layoutOnly ? 'final-' : ''}${name}-${width}x${height}.png`; writeFileSync(path.join(base, file), Buffer.from(await driver.takeScreenshot(), 'base64'))
      state.layouts.push({ name, ...layout }); state.screenshots.push(file); save()
    }
    await driver.manage().window().setRect({ width: 1100, height: 700 })
  }
  if (!existsSync(path.join(work, 'left', '项目资料.zip'))) {
    execFileSync('python', ['-X', 'utf8', '-c', `import pathlib,zipfile,zlib,struct,io,sys
p=pathlib.Path(sys.argv[1]);left=p/'left';right=p/'right';left.mkdir(exist_ok=True);right.mkdir(exist_ok=True)
def chunk(k,v):return struct.pack('>I',len(v))+k+v+struct.pack('>I',zlib.crc32(k+v)&0xffffffff)
rows=b''.join(b'\\0'+b''.join(bytes((20+x,160+y//2,150,255)) for x in range(160)) for y in range(96))
image=b'\\x89PNG\\r\\n\\x1a\\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',160,96,8,6,0,0,0))+chunk(b'IDAT',zlib.compress(rows))+chunk(b'IEND',b'')
(left/'预览.png').write_bytes(image);(left/'说明.txt').write_text('本地文本预览\\n中文说明与常用 TXT 文件。\\n只读查看后仍保留双栏。',encoding='utf8');(left/'旧编码.log').write_bytes('中文日志内容'.encode('gbk'))
inner=io.BytesIO()
with zipfile.ZipFile(inner,'w') as z:z.writestr('内层说明.txt','nested-pane-content')
with zipfile.ZipFile(left/'项目资料.zip','w',zipfile.ZIP_DEFLATED) as z:
 z.writestr('文档/说明.txt','包内文本预览：只取回这一份说明。');z.writestr('图片/预览.png',image);z.writestr('内层.zip',inner.getvalue());z.writestr('数据.csv','name,value\\nexample,1')
(right/'保留选择.txt').write_text('opposite-pane-selection',encoding='utf8');(right/'密码来源').mkdir(exist_ok=True);(right/'密码来源'/'私密说明.txt').write_text('right-pane-password-content',encoding='utf8')
`, work], { windowsHide: true })
    execFileSync(path.join(root, 'src-tauri/resources/archive-engine/7z.exe'), ['a', '-tzip', '-pdual-pane-key', '-mem=AES256', path.join(work, 'right/密码资料.zip'), path.join(work, 'right/密码来源/私密说明.txt')], { windowsHide: true, stdio: 'pipe' })
  }
  await driver.manage().setTimeouts({ script: 120000 }); await driver.manage().window().setRect({ width: 1100, height: 700 })
  await click('[data-testid="nav-Settings"]'); await click('.settings-theme-options button:nth-child(2)'); await click('.settings-accent-options button[title="emerald"]')
  await click('[data-testid="nav-ArchiveBrowser"]')
  await fill('[data-testid="file-manager-path-input-left"]', path.join(work, 'left'), true)
  await find(local('left', '项目资料.zip'))
  await fill('[data-testid="file-manager-path-input-right"]', path.join(work, 'right'), true)
  await find(local('right', '保留选择.txt'))
  if (layoutOnly) {
    await click(local('right', '保留选择.txt') + ' .file-name .truncate')
    await click(local('left', '项目资料.zip'), true); await find(entry('left', '文档/'))
    await shot('archive-and-folder')
    await click(entry('left', '文档/'), true)
    await click(local('right', '密码资料.zip'), true); await find(entry('right', '私密说明.txt'))
    await click(pane('right') + ' [data-testid="archive-pane-tools"]')
    await fill(pane('right') + ' .browser-toolbar input[type="password"]', 'dual-pane-key', true)
    await driver.wait(async () => await (await find(pane('right') + ' .browser-toolbar input[type="password"]')).isEnabled(), 15000)
    await find(entry('right', '私密说明.txt'))
    await shot('archive-options-popover', [[760, 520]])
    await click(pane('right') + ' [data-testid="archive-pane-tools"]')
    assert.match(await text(pane('right') + ' footer .browser-primary'), /解压所选文件/)
    assert.match(await text(pane('right') + ' footer .output-target'), /选择输出目录/)
    await shot('two-independent-archives')
    done('compact-archive-lists-and-floating-options')
    await click(entry('right', '私密说明.txt'), true)
    assert.match(await text('[data-testid="archive-text-preview"]'), /right-pane-password-content/)
    await shot('password-text-preview', [[760, 520]]); await click('.preview-close')
    done('password-preview-after-options-layout-change')
    await click(pane('left') + ' [data-testid="archive-pane-leave"]'); await find(local('left', '项目资料.zip'))
    await click(pane('right') + ' [data-testid="archive-pane-leave"]'); await find(local('right', '密码资料.zip'))
    await shot('returned-folders-no-nav-scrollbar', [[900, 600]])
    await click(local('left', '说明.txt') + ' [data-testid="file-manager-preview"]')
    assert.match(await text('[data-testid="local-text-preview"]'), /本地文本预览/)
    await shot('local-text-preview', [[760, 520]]); await click('[data-testid="local-preview-close"]')
    done('compact-local-text-and-hidden-nav-scrollbar')
    state.completedAt = new Date().toISOString(); save()
    console.log(JSON.stringify({ checks: state.checks.length, layouts: state.layouts.length, binarySha256: sha }))
    return
  }
  if (!passed('local-text-gbk-and-image-previews')) {
    await click(local('left', '说明.txt'), true)
    assert.match(await text('[data-testid="local-text-preview"]'), /本地文本预览/)
    await shot('local-text-preview'); await click('[data-testid="local-preview-close"]')
    await click(local('left', '旧编码.log') + ' [data-testid="file-manager-preview"]')
    assert.match(await text('[data-testid="local-text-preview"]'), /中文日志内容/); await click('[data-testid="local-preview-close"]')
    await click(local('left', '预览.png') + ' [data-testid="file-manager-preview"]')
    await driver.wait(() => driver.executeScript(() => document.querySelector('[data-testid="local-image-preview"]')?.naturalWidth === 160), 10000)
    await shot('local-image-preview', [[760, 520]]); await click('[data-testid="local-preview-close"]')
    done('local-text-gbk-and-image-previews')
  }
  await click(local('right', '保留选择.txt') + ' .file-name .truncate')
  await click(local('left', '项目资料.zip'), true)
  await find(entry('left', '文档/'))
  assert.ok((await driver.findElements(By.css(local('right', '保留选择.txt') + '.selected'))).length)
  assert.equal((await driver.findElements(By.css('.browser-title'))).length, 0)
  assert.equal(await driver.executeScript(() => getComputedStyle(document.querySelector('[data-file-pane="left"] .archive-options')).display), 'none')
  await shot('archive-and-folder')
  await click(entry('left', '文档/'), true)
  if (!passed('archive-text-and-exact-selected-extraction-to-peer')) {
    await click(entry('left', '文档/说明.txt'), true)
    assert.match(await text('[data-testid="archive-text-preview"]'), /包内文本预览/)
    await shot('archive-text-preview', [[760, 520]]); await click('.preview-close')
    await click(entry('left', '文档/说明.txt') + ' .browser-checkbox')
    await click(pane('left') + ' footer .browser-primary')
    const output = path.join(work, 'right/文档/说明.txt')
    await driver.wait(() => existsSync(output), 15000)
    assert.equal(readFileSync(output, 'utf8'), '包内文本预览：只取回这一份说明。')
    assert.equal(await (await find('[data-testid="file-manager-path-input-right"]')).getAttribute('value'), path.join(work, 'right'))
    await find(entry('left', '文档/说明.txt')); done('archive-text-and-exact-selected-extraction-to-peer')
  }
  if (!passed('archive-image-preview-and-continuous-reader')) {
    await click(pane('left') + ' [data-testid="archive-nav-up"]')
    await click(entry('left', '图片/'), true)
    await click(entry('left', '图片/预览.png') + ' .preview-trigger')
    await driver.wait(() => driver.executeScript(() => document.querySelector('[data-testid="archive-image-preview"] img')?.naturalWidth === 160), 10000)
    await click('[data-testid="archive-preview-read-images"]')
    await find('[data-testid="reader-close"]')
    await driver.wait(() => driver.executeScript(() => document.querySelector('[data-testid="reader-image"]')?.naturalWidth === 160), 15000)
    await shot('archive-image-reader', [[760, 520]])
    await click('[data-testid="reader-close"]')
    await click(pane('left') + ' [data-testid="archive-nav-up"]'); await click(entry('left', '文档/'), true)
    done('archive-image-preview-and-continuous-reader')
  }
  await click(local('right', '密码资料.zip'), true)
  await find(entry('right', '私密说明.txt'))
  await click(pane('right') + ' [data-testid="archive-pane-tools"]')
  await fill(pane('right') + ' .browser-toolbar input[type="password"]', 'dual-pane-key', true)
  await driver.wait(async () => await (await find(pane('right') + ' .browser-toolbar input[type="password"]')).isEnabled(), 15000)
  await find(entry('right', '私密说明.txt'))
  assert.equal(await driver.executeScript(() => document.querySelector('[data-file-pane="left"] input[type="password"]').value), '')
  await click(pane('right') + ' [data-testid="archive-pane-tools"]')
  if (!passed('two-archives-independent-password-and-text-preview')) {
    await click(entry('right', '私密说明.txt'), true)
    assert.match(await text('[data-testid="archive-text-preview"]'), /right-pane-password-content/)
    await click('.preview-close'); await shot('two-independent-archives')
    assert.ok((await driver.findElements(By.css(entry('left', '文档/说明.txt')))).length)
    done('two-archives-independent-password-and-text-preview')
  }
  if (!passed('nested-browsing-and-return-preserve-peer')) {
    await click(pane('left') + ' [data-testid="archive-nav-up"]')
    await click(entry('left', '内层.zip'), true)
    await find(entry('left', '内层说明.txt')); assert.match(await text(pane('left') + ' [data-testid="archive-chain"]'), /2 \/ 3/)
    assert.ok((await driver.findElements(By.css(entry('right', '私密说明.txt')))).length)
    await shot('nested-archive-in-pane', [[760, 520]])
    await click(pane('left') + ' [data-testid="archive-pane-leave"]')
    await find(local('left', '项目资料.zip')); assert.ok((await driver.findElements(By.css(entry('right', '私密说明.txt')))).length)
    assert.ok((await driver.findElements(By.css('.transfer-bar button:disabled'))).length === 3)
    await click(pane('right') + ' [data-testid="archive-pane-leave"]'); await find(local('right', '密码资料.zip'))
    await shot('returned-folders-no-nav-scrollbar', [[900, 600]])
    done('nested-browsing-and-return-preserve-peer')
  }
  state.completedAt = new Date().toISOString(); save()
  console.log(JSON.stringify({ checks: state.checks.length, layouts: state.layouts.length, binarySha256: sha }))
}
