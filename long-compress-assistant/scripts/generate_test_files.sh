#!/bin/bash

# 胧压缩·方便助手 - 测试文件生成脚本
# 生成用于性能测试的各种类型和大小的文件

set -e

TEST_DIR="./test_data"
mkdir -p "$TEST_DIR"

echo "生成测试文件..."

# 生成文本文件
generate_text_files() {
    echo "生成文本文件..."
    for size in 1 10 100 1024; do
        if [ $size -lt 1024 ]; then
            filename="${TEST_DIR}/text_${size}kb.txt"
            dd if=/dev/urandom of="$filename" bs=1024 count=$size 2>/dev/null
        else
            filename="${TEST_DIR}/text_1mb.txt"
            dd if=/dev/urandom of="$filename" bs=1048576 count=1 2>/dev/null
        fi
        echo "生成: $filename"
    done
}

# 生成代码文件
generate_code_files() {
    echo "生成代码文件..."

    # Rust文件
    cat > "${TEST_DIR}/test.rs" << 'EOF'
// 测试Rust文件
fn main() {
    println!("Hello, World!");

    let numbers = vec![1, 2, 3, 4, 5];
    let sum: i32 = numbers.iter().sum();
    println!("Sum: {}", sum);

    // 一些测试数据
    let data = "Lorem ipsum dolor sit amet, consectetur adipiscing elit.";
    println!("Data: {}", data);
}
EOF

    # JavaScript文件
    cat > "${TEST_DIR}/test.js" << 'EOF'
// 测试JavaScript文件
function calculateSum(numbers) {
    return numbers.reduce((a, b) => a + b, 0);
}

const data = {
    name: "Test File",
    size: 1024,
    type: "text/plain",
    content: "Lorem ipsum dolor sit amet, consectetur adipiscing elit."
};

console.log("File data:", data);
EOF

    # TypeScript文件
    cat > "${TEST_DIR}/test.ts" << 'EOF'
// 测试TypeScript文件
interface FileInfo {
    name: string;
    size: number;
    type: string;
    content: string;
}

const fileInfo: FileInfo = {
    name: "test.ts",
    size: 2048,
    type: "text/typescript",
    content: "TypeScript test content"
};

function processFile(info: FileInfo): string {
    return `Processing ${info.name} (${info.size} bytes)`;
}

console.log(processFile(fileInfo));
EOF
}

# 生成JSON文件
generate_json_files() {
    echo "生成JSON文件..."

    cat > "${TEST_DIR}/data_small.json" << 'EOF'
{
  "name": "Small Test Data",
  "items": [
    {"id": 1, "value": "test1"},
    {"id": 2, "value": "test2"},
    {"id": 3, "value": "test3"}
  ],
  "metadata": {
    "created": "2024-01-01",
    "version": "1.0.0"
  }
}
EOF

    # 生成大JSON文件
    echo "{" > "${TEST_DIR}/data_large.json"
    echo "  \"items\": [" >> "${TEST_DIR}/data_large.json"
    for i in {1..1000}; do
        if [ $i -eq 1000 ]; then
            echo "    {\"id\": $i, \"name\": \"item$i\", \"value\": $i}" >> "${TEST_DIR}/data_large.json"
        else
            echo "    {\"id\": $i, \"name\": \"item$i\", \"value\": $i}," >> "${TEST_DIR}/data_large.json"
        fi
    done
    echo "  ]" >> "${TEST_DIR}/data_large.json"
    echo "}" >> "${TEST_DIR}/data_large.json"
}

# 生成二进制文件
generate_binary_files() {
    echo "生成二进制文件..."

    # 小二进制文件
    dd if=/dev/urandom of="${TEST_DIR}/binary_1k.bin" bs=1024 count=1 2>/dev/null
    dd if=/dev/urandom of="${TEST_DIR}/binary_10k.bin" bs=1024 count=10 2>/dev/null
    dd if=/dev/urandom of="${TEST_DIR}/binary_100k.bin" bs=1024 count=100 2>/dev/null
    dd if=/dev/urandom of="${TEST_DIR}/binary_1m.bin" bs=1048576 count=1 2>/dev/null
}

# 生成压缩测试文件
generate_compressed_files() {
    echo "生成压缩测试文件..."

    # 创建一些内容用于压缩
    mkdir -p "${TEST_DIR}/compress_source"
    for i in {1..10}; do
        echo "File $i content" > "${TEST_DIR}/compress_source/file$i.txt"
    done

    # 创建ZIP文件（需要zip命令）
    if command -v zip &> /dev/null; then
        cd "${TEST_DIR}/compress_source" && zip -r "../test.zip" ./* && cd - > /dev/null
        echo "生成: ${TEST_DIR}/test.zip"
    fi

    # 创建TAR文件
    tar -czf "${TEST_DIR}/test.tar.gz" -C "${TEST_DIR}/compress_source" .
    echo "生成: ${TEST_DIR}/test.tar.gz"
}

# 生成大文件（用于性能测试）
generate_large_files() {
    echo "生成大文件..."

    # 10MB文件
    dd if=/dev/urandom of="${TEST_DIR}/large_10mb.dat" bs=1048576 count=10 2>/dev/null
    echo "生成: ${TEST_DIR}/large_10mb.dat"

    # 50MB文件
    dd if=/dev/urandom of="${TEST_DIR}/large_50mb.dat" bs=1048576 count=50 2>/dev/null
    echo "生成: ${TEST_DIR}/large_50mb.dat"

    # 100MB文件
    dd if=/dev/urandom of="${TEST_DIR}/large_100mb.dat" bs=1048576 count=100 2>/dev/null
    echo "生成: ${TEST_DIR}/large_100mb.dat"
}

# 生成嵌套目录结构
generate_nested_directories() {
    echo "生成嵌套目录结构..."

    mkdir -p "${TEST_DIR}/nested/dir1/dir2/dir3"
    mkdir -p "${TEST_DIR}/nested/another/deep/path"

    # 在各个目录中创建文件
    echo "File in root" > "${TEST_DIR}/nested/root_file.txt"
    echo "File in dir1" > "${TEST_DIR}/nested/dir1/file1.txt"
    echo "File in dir2" > "${TEST_DIR}/nested/dir1/dir2/file2.txt"
    echo "File in dir3" > "${TEST_DIR}/nested/dir1/dir2/dir3/file3.txt"
    echo "Deep file" > "${TEST_DIR}/nested/another/deep/path/deep_file.txt"

    # 创建一些二进制文件
    dd if=/dev/urandom of="${TEST_DIR}/nested/binary.bin" bs=1024 count=10 2>/dev/null
}

# 生成特殊字符文件名
generate_special_filenames() {
    echo "生成特殊字符文件名..."

    # 注意：这些文件名在某些系统上可能有问题
    echo "content" > "${TEST_DIR}/file with spaces.txt"
    echo "content" > "${TEST_DIR}/file_with_underscores.txt"
    echo "content" > "${TEST_DIR}/file-with-dashes.txt"
    echo "content" > "${TEST_DIR}/file.multiple.dots.txt"
    echo "content" > "${TEST_DIR}/中文文件名.txt"
    echo "content" > "${TEST_DIR}/ファイル名日本語.txt"
}

# 生成文件统计
generate_stats() {
    echo ""
    echo "测试文件生成完成！"
    echo "目录: $TEST_DIR"
    echo ""

    # 显示文件统计
    echo "文件统计:"
    echo "---------"
    find "$TEST_DIR" -type f | wc -l | xargs echo "文件总数:"
    du -sh "$TEST_DIR" | cut -f1 | xargs echo "总大小:"

    echo ""
    echo "文件类型分布:"
    echo "------------"
    find "$TEST_DIR" -type f -name "*.txt" | wc -l | xargs echo "文本文件:"
    find "$TEST_DIR" -type f -name "*.json" | wc -l | xargs echo "JSON文件:"
    find "$TEST_DIR" -type f -name "*.rs" -o -name "*.js" -o -name "*.ts" | wc -l | xargs echo "代码文件:"
    find "$TEST_DIR" -type f -name "*.bin" -o -name "*.dat" | wc -l | xargs echo "二进制文件:"
    find "$TEST_DIR" -type f -name "*.zip" -o -name "*.tar.gz" | wc -l | xargs echo "压缩文件:"
}

# 主函数
main() {
    echo "开始生成测试文件..."

    generate_text_files
    generate_code_files
    generate_json_files
    generate_binary_files
    generate_compressed_files
    generate_large_files
    generate_nested_directories
    generate_special_filenames
    generate_stats

    echo ""
    echo "测试文件已生成到: $TEST_DIR"
    echo "这些文件可用于压缩、解压、文件操作等测试。"
}

# 执行主函数
main "$@"