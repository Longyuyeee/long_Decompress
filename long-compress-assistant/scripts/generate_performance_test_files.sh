#!/bin/bash

# 胧压缩·方便助手 - 性能测试文件生成脚本
# 专门生成用于性能测试的大文件和压缩包

set -e

# 配置
TEST_DIR="./test_data/performance"
LOG_FILE="./test_data/performance_generation.log"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1" | tee -a "$LOG_FILE"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1" | tee -a "$LOG_FILE"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1" | tee -a "$LOG_FILE"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1" | tee -a "$LOG_FILE"
}

# 检查依赖
check_dependencies() {
    log_info "检查依赖..."

    local missing_deps=()

    # 检查基本命令
    for cmd in dd zip tar; do
        if ! command -v $cmd &> /dev/null; then
            missing_deps+=($cmd)
        fi
    done

    if [ ${#missing_deps[@]} -gt 0 ]; then
        log_error "缺少依赖: ${missing_deps[*]}"
        log_error "请安装缺少的命令后重试"
        exit 1
    fi

    log_success "所有依赖已安装"
}

# 创建测试目录
create_test_dir() {
    log_info "创建测试目录: $TEST_DIR"
    mkdir -p "$TEST_DIR"

    # 创建子目录
    mkdir -p "$TEST_DIR/large_files"
    mkdir -p "$TEST_DIR/concurrent_files"
    mkdir -p "$TEST_DIR/memory_test"
    mkdir -p "$TEST_DIR/compressed"

    log_success "测试目录结构已创建"
}

# 生成大文件（1GB）
generate_large_file() {
    local size_gb=$1
    local filename=$2
    local filepath="$TEST_DIR/large_files/$filename"

    log_info "生成 ${size_gb}GB 文件: $filename"

    local size_mb=$((size_gb * 1024))

    # 使用dd生成文件，显示进度
    dd if=/dev/urandom of="$filepath" bs=1M count=$size_mb status=progress 2>&1 | \
        while read line; do
            echo -ne "\r${BLUE}[生成中]${NC} $line"
        done
    echo ""

    # 验证文件大小
    local actual_size=$(du -h "$filepath" | cut -f1)
    log_success "文件生成完成: $filename ($actual_size)"
}

# 生成混合内容的大文件
generate_mixed_content_file() {
    local size_mb=$1
    local filename=$2
    local filepath="$TEST_DIR/large_files/$filename"

    log_info "生成混合内容文件: $filename (${size_mb}MB)"

    # 创建临时目录
    local temp_dir=$(mktemp -d)

    # 生成不同类型的内容
    local text_size=$((size_mb * 50 / 100))  # 50%文本
    local binary_size=$((size_mb * 30 / 100)) # 30%二进制
    local compressed_size=$((size_mb * 20 / 100)) # 20%已压缩

    # 1. 生成文本内容（可压缩）
    log_info "  生成文本内容 (${text_size}MB)..."
    for i in $(seq 1 10); do
        # 生成重复的文本模式，便于压缩
        for j in $(seq 1 100); do
            echo "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua." >> "$temp_dir/text_$i.txt"
        done
    done

    # 2. 生成二进制内容（中等压缩）
    log_info "  生成二进制内容 (${binary_size}MB)..."
    dd if=/dev/urandom of="$temp_dir/binary.dat" bs=1M count=$binary_size status=none

    # 3. 生成已压缩内容（低压缩）
    log_info "  生成已压缩内容 (${compressed_size}MB)..."
    dd if=/dev/urandom of="$temp_dir/compressed.dat" bs=1M count=$compressed_size status=none
    gzip -c "$temp_dir/compressed.dat" > "$temp_dir/compressed.dat.gz"

    # 合并所有内容
    log_info "  合并文件内容..."
    cat "$temp_dir"/* > "$filepath"

    # 清理临时目录
    rm -rf "$temp_dir"

    local actual_size=$(du -h "$filepath" | cut -f1)
    log_success "混合内容文件生成完成: $filename ($actual_size)"
}

# 生成ZIP压缩包
generate_zip_archive() {
    local source_dir=$1
    local zip_name=$2
    local zip_path="$TEST_DIR/compressed/$zip_name"

    log_info "生成ZIP压缩包: $zip_name"

    # 进入源目录创建ZIP
    (cd "$source_dir" && zip -r "$zip_path" . -q)

    local zip_size=$(du -h "$zip_path" | cut -f1)
    log_success "ZIP压缩包生成完成: $zip_name ($zip_size)"
}

# 生成大ZIP文件（1GB）
generate_large_zip() {
    log_info "生成1GB ZIP文件..."

    # 创建源目录
    local source_dir="$TEST_DIR/large_zip_source"
    mkdir -p "$source_dir"

    # 生成不同类型的内容
    log_info "  准备ZIP文件内容..."

    # 1. 文本文件（50%）
    mkdir -p "$source_dir/text"
    for i in $(seq 1 100); do
        cat << EOF > "$source_dir/text/file_$i.txt"
这是测试文本文件 #$i
Lorem ipsum dolor sit amet, consectetur adipiscing elit.
Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua.
Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris.
重复模式便于压缩测试。
EOF
        # 复制多次以增加大小
        for j in $(seq 1 100); do
            cat "$source_dir/text/file_$i.txt" >> "$source_dir/text/file_$i.txt"
        done
    done

    # 2. 二进制文件（30%）
    mkdir -p "$source_dir/binary"
    dd if=/dev/urandom of="$source_dir/binary/data.bin" bs=10M count=30 status=none

    # 3. 代码文件（10%）
    mkdir -p "$source_dir/code"
    for i in $(seq 1 20); do
        cat << EOF > "$source_dir/code/program_$i.rs"
// 测试Rust程序 #$i
use std::fs;
use std::io::{self, Write};

fn process_data(data: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(data.len());
    for &byte in data {
        result.push(byte.wrapping_add(1));
    }
    result
}

fn main() -> io::Result<()> {
    let test_data = vec![0u8; 1024];
    let processed = process_data(&test_data);

    let mut file = fs::File::create("output.bin")?;
    file.write_all(&processed)?;

    println!("处理完成: {} 字节", processed.len());
    Ok(())
}
EOF
    done

    # 4. JSON文件（10%）
    mkdir -p "$source_dir/json"
    cat << EOF > "$source_dir/json/large_data.json"
{
  "metadata": {
    "generated": "$(date -Iseconds)",
    "purpose": "性能测试",
    "version": "1.0.0"
  },
  "items": [
EOF

    for i in $(seq 1 5000); do
        if [ $i -eq 5000 ]; then
            cat << EOF >> "$source_dir/json/large_data.json"
    {
      "id": $i,
      "name": "item_$i",
      "value": $((RANDOM % 1000)),
      "timestamp": "$(date -Iseconds)",
      "tags": ["test", "performance", "data"],
      "description": "测试数据项 #$i，用于性能测试验证"
    }
EOF
        else
            cat << EOF >> "$source_dir/json/large_data.json"
    {
      "id": $i,
      "name": "item_$i",
      "value": $((RANDOM % 1000)),
      "timestamp": "$(date -Iseconds)",
      "tags": ["test", "performance", "data"],
      "description": "测试数据项 #$i，用于性能测试验证"
    },
EOF
        fi
    done

    cat << EOF >> "$source_dir/json/large_data.json"
  ]
}
EOF

    # 生成ZIP文件
    generate_zip_archive "$source_dir" "large_test_1gb.zip"

    # 清理源目录（可选）
    # rm -rf "$source_dir"
}

# 生成并发测试文件
generate_concurrent_test_files() {
    local count=$1
    local size_mb=$2

    log_info "生成并发测试文件 ($count 个 ${size_mb}MB 文件)..."

    mkdir -p "$TEST_DIR/concurrent_files/source"

    for i in $(seq 1 $count); do
        log_info "  生成文件 $i/$count..."
        local filename="concurrent_${i}.dat"
        local filepath="$TEST_DIR/concurrent_files/source/$filename"

        # 生成不同模式的文件
        if [ $((i % 3)) -eq 0 ]; then
            # 文本模式
            for j in $(seq 1 $((size_mb * 10))); do
                echo "这是并发测试文件 #$i，行号: $j" >> "$filepath"
                echo "重复文本内容用于压缩测试，模式识别验证。" >> "$filepath"
                echo "性能测试数据生成，验证并发处理能力。" >> "$filepath"
            done
        elif [ $((i % 3)) -eq 1 ]; then
            # 二进制模式
            dd if=/dev/urandom of="$filepath" bs=1M count=$size_mb status=none
        else
            # 混合模式
            dd if=/dev/urandom of="$filepath" bs=1M count=$((size_mb / 2)) status=none
            for j in $(seq 1 $((size_mb * 5))); do
                echo "混合文件 #$i，部分文本，部分二进制" >> "$filepath"
            done
        fi

        # 创建对应的ZIP文件
        (cd "$TEST_DIR/concurrent_files/source" && zip "../concurrent_${i}.zip" "$filename" -q)
    done

    log_success "并发测试文件生成完成: $count 个文件"
}

# 生成内存测试文件
generate_memory_test_files() {
    local count=$1
    local size_kb=$2

    log_info "生成内存测试文件 ($count 个 ${size_kb}KB 文件)..."

    mkdir -p "$TEST_DIR/memory_test"

    for i in $(seq 1 $count); do
        if [ $((i % 100)) -eq 0 ]; then
            echo -ne "\r${BLUE}[生成中]${NC} $i/$count"
        fi

        local filename="memory_test_$(printf "%04d" $i).txt"
        local filepath="$TEST_DIR/memory_test/$filename"

        # 生成有规律的文本，便于压缩
        for j in $(seq 1 $((size_kb / 10))); do
            echo "内存测试文件 #$i，块号: $j" >> "$filepath"
            echo "测试数据: $(date +%s).$RANDOM" >> "$filepath"
            echo "重复模式: ABCDEFGHIJKLMNOPQRSTUVWXYZ" >> "$filepath"
            echo "数字序列: 0123456789" >> "$filepath"
            echo "分隔线: $(printf '=%.0s' {1..50})" >> "$filepath"
        done
    done
    echo ""

    log_success "内存测试文件生成完成: $count 个文件"
}

# 生成测试报告
generate_test_report() {
    log_info "生成测试报告..."

    local report_file="$TEST_DIR/test_report.md"

    cat << EOF > "$report_file"
# 性能测试数据报告

生成时间: $(date)
生成脚本: $(basename "$0")
测试目录: $TEST_DIR

## 文件统计

### 大文件测试
EOF

    # 统计大文件
    echo "| 文件名 | 大小 | 类型 |" >> "$report_file"
    echo "|--------|------|------|" >> "$report_file"
    find "$TEST_DIR/large_files" -type f -exec du -h {} \; | while read size file; do
        echo "| $(basename "$file") | $size | 大文件 |" >> "$report_file"
    done

    cat << EOF >> "$report_file"

### 并发测试文件
EOF

    echo "| 文件名 | 大小 | 类型 |" >> "$report_file"
    echo "|--------|------|------|" >> "$report_file"
    find "$TEST_DIR/concurrent_files" -name "*.zip" -type f -exec du -h {} \; | while read size file; do
        echo "| $(basename "$file") | $size | ZIP文件 |" >> "$report_file"
    done

    cat << EOF >> "$report_file"

### 内存测试文件
EOF

    local memory_file_count=$(find "$TEST_DIR/memory_test" -type f | wc -l)
    local memory_total_size=$(du -sh "$TEST_DIR/memory_test" | cut -f1)
    echo "- 文件数量: $memory_file_count" >> "$report_file"
    echo "- 总大小: $memory_total_size" >> "$report_file"

    cat << EOF >> "$report_file"

### 压缩文件
EOF

    echo "| 文件名 | 大小 | 类型 |" >> "$report_file"
    echo "|--------|------|------|" >> "$report_file"
    find "$TEST_DIR/compressed" -type f -exec du -h {} \; | while read size file; do
        echo "| $(basename "$file") | $size | 压缩包 |" >> "$report_file"
    done

    # 总体统计
    cat << EOF >> "$report_file"

## 总体统计

\`\`\`
$(find "$TEST_DIR" -type f | wc -l) 个文件
$(du -sh "$TEST_DIR" | cut -f1) 总大小
\`\`\`

## 使用说明

这些测试文件用于以下性能测试场景：

1. **大文件解压测试**: 使用 \`large_test_1gb.zip\`
2. **并发任务测试**: 使用 \`concurrent_files/*.zip\`
3. **内存使用测试**: 使用 \`memory_test/*.txt\`
4. **压缩性能测试**: 使用各种大小的源文件

## 清理命令

如需清理测试数据，运行:
\`\`\`bash
rm -rf "$TEST_DIR"
\`\`\`
EOF

    log_success "测试报告已生成: $report_file"
}

# 显示使用说明
show_usage() {
    cat << EOF
性能测试文件生成脚本

用法: $0 [选项]

选项:
  --all              生成所有测试文件（默认）
  --large            只生成大文件测试数据
  --concurrent       只生成并发测试数据
  --memory           只生成内存测试数据
  --clean            清理测试数据目录
  --help             显示此帮助信息

示例:
  $0 --all           生成所有测试文件
  $0 --large         生成大文件测试数据
  $0 --concurrent --count=5 --size=100  生成5个100MB并发测试文件

配置:
  并发测试文件数量: 通过环境变量 CONCURRENT_COUNT 设置（默认: 5）
  并发测试文件大小: 通过环境变量 CONCURRENT_SIZE_MB 设置（默认: 100）
  内存测试文件数量: 通过环境变量 MEMORY_COUNT 设置（默认: 1000）
  内存测试文件大小: 通过环境变量 MEMORY_SIZE_KB 设置（默认: 1024）
EOF
}

# 主函数
main() {
    # 清空日志文件
    > "$LOG_FILE"

    log_info "开始生成性能测试文件"
    log_info "日志文件: $LOG_FILE"

    # 解析参数
    local generate_all=true
    local generate_large=false
    local generate_concurrent=false
    local generate_memory=false
    local clean_mode=false

    # 配置参数
    local concurrent_count=${CONCURRENT_COUNT:-5}
    local concurrent_size_mb=${CONCURRENT_SIZE_MB:-100}
    local memory_count=${MEMORY_COUNT:-1000}
    local memory_size_kb=${MEMORY_SIZE_KB:-1024}

    # 解析命令行参数
    for arg in "$@"; do
        case $arg in
            --all)
                generate_all=true
                ;;
            --large)
                generate_large=true
                generate_all=false
                ;;
            --concurrent)
                generate_concurrent=true
                generate_all=false
                ;;
            --memory)
                generate_memory=true
                generate_all=false
                ;;
            --clean)
                clean_mode=true
                ;;
            --help)
                show_usage
                exit 0
                ;;
            --count=*)
                concurrent_count="${arg#*=}"
                ;;
            --size=*)
                concurrent_size_mb="${arg#*=}"
                ;;
            *)
                log_warning "未知参数: $arg"
                ;;
        esac
    done

    # 清理模式
    if [ "$clean_mode" = true ]; then
        log_info "清理测试数据目录..."
        if [ -d "$TEST_DIR" ]; then
            rm -rf "$TEST_DIR"
            log_success "测试数据目录已清理"
        else
            log_warning "测试数据目录不存在: $TEST_DIR"
        fi
        exit 0
    fi

    # 检查依赖
    check_dependencies

    # 创建测试目录
    create_test_dir

    # 生成测试文件
    if [ "$generate_all" = true ] || [ "$generate_large" = true ]; then
        log_info "=== 生成大文件测试数据 ==="
        generate_large_file 1 "large_1gb.dat"
        generate_mixed_content_file 500 "mixed_500mb.dat"
        generate_large_zip
    fi

    if [ "$generate_all" = true ] || [ "$generate_concurrent" = true ]; then
        log_info "=== 生成并发测试数据 ==="
        generate_concurrent_test_files "$concurrent_count" "$concurrent_size_mb"
    fi

    if [ "$generate_all" = true ] || [ "$generate_memory" = true ]; then
        log_info "=== 生成内存测试数据 ==="
        generate_memory_test_files "$memory_count" "$memory_size_kb"
    fi

    # 生成测试报告
    generate_test_report

    log_success "性能测试文件生成完成！"
    log_info "测试数据目录: $TEST_DIR"
    log_info "测试报告: $TEST_DIR/test_report.md"
    log_info "日志文件: $LOG_FILE"

    # 显示磁盘使用情况
    echo ""
    echo "磁盘使用情况:"
    echo "-------------"
    df -h . | tail -1
    du -sh "$TEST_DIR"
}

# 运行主函数
main "$@"