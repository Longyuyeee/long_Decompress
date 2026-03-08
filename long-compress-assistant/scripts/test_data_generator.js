#!/usr/bin/env node

/**
 * 胧压缩·方便助手 - 测试数据生成器
 * 生成用于测试的各种类型和大小的文件
 */

const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');

// 配置
const CONFIG = {
  outputDir: path.join(__dirname, '..', 'test_data'),
  fileTypes: {
    text: ['.txt', '.md', '.json', '.yaml', '.yml'],
    code: ['.js', '.ts', '.rs', '.py', '.java', '.cpp'],
    image: ['.jpg', '.png', '.gif', '.bmp'],
    document: ['.pdf', '.docx', '.xlsx', '.pptx'],
    archive: ['.zip', '.tar.gz', '.7z', '.rar'],
    binary: ['.bin', '.dat', '.iso']
  },
  sizes: {
    small: 1024,           // 1KB
    medium: 1024 * 1024,   // 1MB
    large: 10 * 1024 * 1024, // 10MB
    xlarge: 100 * 1024 * 1024 // 100MB
  }
};

// 工具函数
class TestDataGenerator {
  constructor() {
    this.stats = {
      totalFiles: 0,
      totalSize: 0,
      byType: {},
      bySize: {}
    };
  }

  // 确保目录存在
  ensureDir(dirPath) {
    if (!fs.existsSync(dirPath)) {
      fs.mkdirSync(dirPath, { recursive: true });
    }
  }

  // 生成随机文本
  generateRandomText(size) {
    const words = [
      'Lorem', 'ipsum', 'dolor', 'sit', 'amet', 'consectetur',
      'adipiscing', 'elit', 'sed', 'do', 'eiusmod', 'tempor',
      'incididunt', 'ut', 'labore', 'et', 'dolore', 'magna',
      'aliqua', 'Ut', 'enim', 'ad', 'minim', 'veniam', 'quis',
      'nostrud', 'exercitation', 'ullamco', 'laboris', 'nisi',
      'ut', 'aliquip', 'ex', 'ea', 'commodo', 'consequat'
    ];

    let result = '';
    while (result.length < size) {
      const word = words[Math.floor(Math.random() * words.length)];
      result += word + ' ';
      if (result.length > size) {
        result = result.substring(0, size);
        break;
      }
    }
    return result;
  }

  // 生成随机二进制数据
  generateRandomBinary(size) {
    const buffer = Buffer.alloc(size);
    for (let i = 0; i < size; i++) {
      buffer[i] = Math.floor(Math.random() * 256);
    }
    return buffer;
  }

  // 生成JSON数据
  generateJsonData(size) {
    const baseData = {
      metadata: {
        generated: new Date().toISOString(),
        tool: 'TestDataGenerator',
        version: '1.0.0'
      },
      items: []
    };

    // 添加足够的数据以达到指定大小
    let currentSize = JSON.stringify(baseData).length;
    const itemSize = 100; // 每个item大约100字节

    while (currentSize < size) {
      baseData.items.push({
        id: `item_${baseData.items.length}`,
        name: `Test Item ${baseData.items.length}`,
        value: Math.random() * 1000,
        timestamp: new Date().toISOString(),
        tags: ['test', 'data', 'generated'],
        description: this.generateRandomText(50)
      });
      currentSize = JSON.stringify(baseData).length;
    }

    return JSON.stringify(baseData, null, 2);
  }

  // 生成代码文件
  generateCodeFile(extension, size) {
    const templates = {
      '.js': `// Test JavaScript file
function calculateSum(numbers) {
  return numbers.reduce((a, b) => a + b, 0);
}

const data = {
  name: "test_file",
  size: ${size},
  type: "application/javascript",
  content: "${this.generateRandomText(100)}"
};

module.exports = { calculateSum, data };
`,
      '.ts': `// Test TypeScript file
interface TestData {
  id: string;
  name: string;
  size: number;
  type: string;
}

const testData: TestData = {
  id: "test_${Date.now()}",
  name: "test_file",
  size: ${size},
  type: "text/typescript"
};

export function processData(data: TestData): string {
  return \`Processing \${data.name} (\${data.size} bytes)\`;
}
`,
      '.rs': `// Test Rust file
use std::fs;
use std::io::{self, Write};

pub struct TestFile {
    pub name: String,
    pub size: u64,
    pub content: Vec<u8>,
}

impl TestFile {
    pub fn new(name: &str, size: u64) -> Self {
        Self {
            name: name.to_string(),
            size,
            content: vec![0; size as usize],
        }
    }

    pub fn save(&self, path: &str) -> io::Result<()> {
        let mut file = fs::File::create(path)?;
        file.write_all(&self.content)?;
        Ok(())
    }
}

fn main() {
    println!("Test file generator");
    let file = TestFile::new("test.bin", ${size});
    println!("Created file: {} ({} bytes)", file.name, file.size);
}
`
    };

    return templates[extension] || `// Test ${extension} file\n${this.generateRandomText(size)}`;
  }

  // 创建测试文件
  createTestFile(filename, content, size) {
    const filePath = path.join(CONFIG.outputDir, filename);

    if (typeof content === 'string') {
      fs.writeFileSync(filePath, content);
    } else {
      fs.writeFileSync(filePath, content);
    }

    // 更新统计
    this.stats.totalFiles++;
    this.stats.totalSize += size;

    const ext = path.extname(filename);
    const sizeCategory = this.getSizeCategory(size);

    this.stats.byType[ext] = (this.stats.byType[ext] || 0) + 1;
    this.stats.bySize[sizeCategory] = (this.stats.bySize[sizeCategory] || 0) + 1;

    return filePath;
  }

  // 获取文件大小分类
  getSizeCategory(size) {
    if (size <= CONFIG.sizes.small) return 'small';
    if (size <= CONFIG.sizes.medium) return 'medium';
    if (size <= CONFIG.sizes.large) return 'large';
    return 'xlarge';
  }

  // 生成压缩文件
  async generateArchiveFiles() {
    console.log('生成压缩文件...');

    // 创建一些源文件用于压缩
    const sourceDir = path.join(CONFIG.outputDir, 'archive_source');
    this.ensureDir(sourceDir);

    // 创建一些文本文件
    for (let i = 1; i <= 5; i++) {
      const content = this.generateRandomText(CONFIG.sizes.small);
      this.createTestFile(`archive_source/file${i}.txt`, content, CONFIG.sizes.small);
    }

    // 尝试使用系统命令创建压缩文件
    try {
      // ZIP文件
      const zipPath = path.join(CONFIG.outputDir, 'test.zip');
      execSync(`cd "${sourceDir}" && zip -r "${zipPath}" .`, { stdio: 'pipe' });
      console.log(`生成: ${zipPath}`);

      // TAR.GZ文件
      const tarPath = path.join(CONFIG.outputDir, 'test.tar.gz');
      execSync(`cd "${sourceDir}" && tar -czf "${tarPath}" .`, { stdio: 'pipe' });
      console.log(`生成: ${tarPath}`);

    } catch (error) {
      console.warn('无法创建压缩文件，跳过:', error.message);
    }
  }

  // 生成嵌套目录结构
  generateNestedDirectories() {
    console.log('生成嵌套目录结构...');

    const nestedDir = path.join(CONFIG.outputDir, 'nested');
    this.ensureDir(nestedDir);

    // 创建多级目录
    const dirs = [
      'level1/level2/level3',
      'a/b/c/d/e',
      'deep/nested/structure',
      'flat/structure'
    ];

    dirs.forEach(dirPath => {
      const fullPath = path.join(nestedDir, dirPath);
      this.ensureDir(fullPath);

      // 在每个目录中创建文件
      const fileName = `file_in_${dirPath.replace(/\//g, '_')}.txt`;
      const content = this.generateRandomText(CONFIG.sizes.small);
      this.createTestFile(`nested/${dirPath}/${fileName}`, content, CONFIG.sizes.small);
    });

    // 在根目录创建文件
    const rootFiles = ['root_file1.txt', 'root_file2.txt', 'hidden_file.txt'];
    rootFiles.forEach(filename => {
      const content = this.generateRandomText(CONFIG.sizes.small);
      this.createTestFile(`nested/${filename}`, content, CONFIG.sizes.small);
    });
  }

  // 生成特殊文件名
  generateSpecialFilenames() {
    console.log('生成特殊文件名...');

    const specialFiles = [
      'file with spaces.txt',
      'file_with_underscores.txt',
      'file-with-dashes.txt',
      'file.multiple.dots.txt',
      '中文文件名.txt',
      'ファイル名日本語.txt',
      'file!@#$%^&().txt',
      'UPPERCASE.TXT',
      'MixedCaseFile.Txt'
    ];

    specialFiles.forEach(filename => {
      const content = this.generateRandomText(CONFIG.sizes.small);
      this.createTestFile(filename, content, CONFIG.sizes.small);
    });
  }

  // 显示统计信息
  displayStats() {
    console.log('\n' + '='.repeat(50));
    console.log('测试数据生成完成！');
    console.log('='.repeat(50));

    console.log(`\n输出目录: ${CONFIG.outputDir}`);
    console.log(`总文件数: ${this.stats.totalFiles}`);
    console.log(`总大小: ${(this.stats.totalSize / 1024 / 1024).toFixed(2)} MB`);

    console.log('\n按类型分布:');
    Object.entries(this.stats.byType).forEach(([type, count]) => {
      console.log(`  ${type}: ${count} 个文件`);
    });

    console.log('\n按大小分布:');
    Object.entries(this.stats.bySize).forEach(([sizeCat, count]) => {
      console.log(`  ${sizeCat}: ${count} 个文件`);
    });

    console.log('\n' + '='.repeat(50));
  }

  // 主生成函数
  async generateAll() {
    console.log('开始生成测试数据...');
    console.log(`输出目录: ${CONFIG.outputDir}`);

    // 确保输出目录存在
    this.ensureDir(CONFIG.outputDir);

    // 1. 生成文本文件
    console.log('\n生成文本文件...');
    Object.values(CONFIG.fileTypes.text).forEach(ext => {
      Object.entries(CONFIG.sizes).forEach(([sizeName, size]) => {
        const filename = `text_${sizeName}${ext}`;
        const content = this.generateRandomText(size);
        this.createTestFile(filename, content, size);
        console.log(`  生成: ${filename} (${size} bytes)`);
      });
    });

    // 2. 生成代码文件
    console.log('\n生成代码文件...');
    Object.values(CONFIG.fileTypes.code).forEach(ext => {
      const filename = `code_sample${ext}`;
      const content = this.generateCodeFile(ext, CONFIG.sizes.small);
      this.createTestFile(filename, content, CONFIG.sizes.small);
      console.log(`  生成: ${filename}`);
    });

    // 3. 生成JSON文件
    console.log('\n生成JSON文件...');
    const jsonSizes = {
      'small': CONFIG.sizes.small,
      'large': CONFIG.sizes.medium * 5
    };

    Object.entries(jsonSizes).forEach(([sizeName, size]) => {
      const filename = `data_${sizeName}.json`;
      const content = this.generateJsonData(size);
      this.createTestFile(filename, content, size);
      console.log(`  生成: ${filename} (${size} bytes)`);
    });

    // 4. 生成二进制文件
    console.log('\n生成二进制文件...');
    Object.values(CONFIG.fileTypes.binary).forEach(ext => {
      Object.entries(CONFIG.sizes).forEach(([sizeName, size]) => {
        const filename = `binary_${sizeName}${ext}`;
        const content = this.generateRandomBinary(size);
        this.createTestFile(filename, content, size);
        console.log(`  生成: ${filename} (${size} bytes)`);
      });
    });

    // 5. 生成大文件
    console.log('\n生成大文件...');
    const largeFiles = [
      { name: 'large_10mb.dat', size: CONFIG.sizes.large },
      { name: 'large_50mb.dat', size: CONFIG.sizes.large * 5 },
      { name: 'large_100mb.dat', size: CONFIG.sizes.xlarge }
    ];

    largeFiles.forEach(file => {
      const content = this.generateRandomBinary(file.size);
      this.createTestFile(file.name, content, file.size);
      console.log(`  生成: ${file.name} (${(file.size / 1024 / 1024).toFixed(1)} MB)`);
    });

    // 6. 生成压缩文件
    await this.generateArchiveFiles();

    // 7. 生成嵌套目录
    this.generateNestedDirectories();

    // 8. 生成特殊文件名
    this.generateSpecialFilenames();

    // 显示统计信息
    this.displayStats();

    console.log('\n测试数据已准备就绪！');
    console.log('这些文件可用于压缩、解压、文件操作等测试。');
  }
}

// 命令行接口
async function main() {
  const generator = new TestDataGenerator();

  const args = process.argv.slice(2);

  if (args.includes('--help') || args.includes('-h')) {
    console.log(`
测试数据生成器

用法:
  node test_data_generator.js [选项]

选项:
  --help, -h     显示帮助信息
  --clean, -c    清理测试数据目录
  --quick, -q    快速模式（只生成小文件）
  --dir <path>   指定输出目录

示例:
  node test_data_generator.js
  node test_data_generator.js --clean
  node test_data_generator.js --quick
    `);
    return;
  }

  if (args.includes('--clean') || args.includes('-c')) {
    console.log('清理测试数据目录...');
    if (fs.existsSync(CONFIG.outputDir)) {
      fs.rmSync(CONFIG.outputDir, { recursive: true, force: true });
      console.log('清理完成');
    }
    return;
  }

  if (args.includes('--dir')) {
    const dirIndex = args.indexOf('--dir');
    if (dirIndex + 1 < args.length) {
      CONFIG.outputDir = path.resolve(args[dirIndex + 1]);
    }
  }

  if (args.includes('--quick') || args.includes('-q')) {
    console.log('快速模式：只生成小文件');
    CONFIG.sizes = {
      small: 1024,
      medium: 1024 * 10,  // 10KB
      large: 1024 * 100,  // 100KB
      xlarge: 1024 * 1024 // 1MB
    };
  }

  try {
    await generator.generateAll();
  } catch (error) {
    console.error('生成测试数据时出错:', error);
    process.exit(1);
  }
}

// 运行主函数
if (require.main === module) {
  main();
}

module.exports = TestDataGenerator;