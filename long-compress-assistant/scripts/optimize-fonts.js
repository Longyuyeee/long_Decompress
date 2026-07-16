#!/usr/bin/env node
/**
 * 批量优化字体大小脚本
 * 自动将所有小字体提升到可读大小
 */

import fs from 'fs';
import path from 'path';
import { glob } from 'glob';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const replacements = [
  // 字体大小优化 - 第一批
  { from: /text-\[0\.4375rem\]/g, to: 'text-[0.625rem]', desc: '7px → 10px' },
  { from: /text-\[0\.5rem\]/g, to: 'text-xs', desc: '8px → 12px' },
  { from: /text-\[0\.5625rem\]/g, to: 'text-xs', desc: '9px → 12px' },
  { from: /text-\[0\.625rem\]/g, to: 'text-sm', desc: '10px → 14px' },
  { from: /text-\[0\.6875rem\]/g, to: 'text-sm', desc: '11px → 14px' },

  // 不透明度优化
  { from: /\bopacity-30\b/g, to: 'opacity-75', desc: '30% → 75%' },
  { from: /\bopacity-40\b/g, to: 'opacity-80', desc: '40% → 80%' },
  { from: /\bopacity-50\b/g, to: 'opacity-85', desc: '50% → 85%' },
  { from: /\bopacity-60\b/g, to: 'opacity-90', desc: '60% → 90%' },
];

async function optimizeFile(filePath) {
  let content = fs.readFileSync(filePath, 'utf-8');
  let changed = false;
  const changes = [];

  for (const { from, to, desc } of replacements) {
    const matches = content.match(from);
    if (matches) {
      content = content.replace(from, to);
      changed = true;
      changes.push(`  ${desc}: ${matches.length} 处`);
    }
  }

  if (changed) {
    fs.writeFileSync(filePath, content, 'utf-8');
    console.log(`\n✅ ${path.relative(process.cwd(), filePath)}`);
    changes.forEach(c => console.log(c));
    return true;
  }
  return false;
}

async function main() {
  console.log('🔍 正在扫描 Vue 文件...\n');

  const patterns = [
    'src/views/**/*.vue',
    'src/components/**/*.vue',
  ];

  let totalFiles = 0;
  let changedFiles = 0;

  for (const pattern of patterns) {
    const files = await glob(pattern, { cwd: process.cwd() });

    for (const file of files) {
      totalFiles++;
      const fullPath = path.join(process.cwd(), file);
      if (await optimizeFile(fullPath)) {
        changedFiles++;
      }
    }
  }

  console.log(`\n✨ 优化完成！`);
  console.log(`📊 扫描文件: ${totalFiles}`);
  console.log(`🎨 修改文件: ${changedFiles}`);
  console.log(`⏭️  未修改: ${totalFiles - changedFiles}\n`);
}

main().catch(console.error);
