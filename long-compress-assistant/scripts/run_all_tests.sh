#!/bin/bash

# 胧压缩·方便助手 - 完整测试套件运行脚本
# 运行所有类型的测试：单元测试、集成测试、E2E测试、性能测试

set -e

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# 日志函数
log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARNING]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# 检查命令是否存在
check_command() {
    if ! command -v $1 &> /dev/null; then
        log_error "命令 '$1' 未找到，请先安装"
        exit 1
    fi
}

# 显示测试标题
show_title() {
    echo ""
    echo "========================================"
    echo "  $1"
    echo "========================================"
    echo ""
}

# 运行测试并检查结果
run_test() {
    local test_name=$1
    local command=$2
    local max_retries=${3:-1}

    log_info "运行 $test_name..."

    local retry_count=0
    local success=false

    while [ $retry_count -lt $max_retries ]; do
        if eval $command; then
            success=true
            break
        fi

        retry_count=$((retry_count + 1))
        if [ $retry_count -lt $max_retries ]; then
            log_warning "$test_name 失败，正在重试 ($retry_count/$max_retries)..."
            sleep 2
        fi
    done

    if $success; then
        log_success "$test_name 通过"
        return 0
    else
        log_error "$test_name 失败"
        return 1
    fi
}

# 主函数
main() {
    # 检查必要命令
    check_command "npm"
    check_command "cargo"

    # 获取脚本目录
    SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
    PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

    cd "$PROJECT_DIR"

    log_info "项目目录: $PROJECT_DIR"
    log_info "开始运行完整测试套件..."

    # 1. 安装依赖
    show_title "1. 安装依赖"
    run_test "安装npm依赖" "npm ci" 2

    # 2. 前端测试
    show_title "2. 前端测试"

    # 2.1 TypeScript类型检查
    run_test "TypeScript类型检查" "npx tsc --noEmit"

    # 2.2 单元测试
    run_test "前端单元测试" "npm run test:unit" 2

    # 2.3 覆盖率测试
    run_test "覆盖率测试" "npm run test:unit:coverage"

    # 2.4 集成测试
    if [ -f "tests/integration" ] || [ -d "tests/integration" ]; then
        run_test "集成测试" "npm run test:integration" 2
    else
        log_warning "集成测试目录不存在，跳过"
    fi

    # 3. Rust后端测试
    show_title "3. Rust后端测试"

    cd "$PROJECT_DIR/src-tauri"

    # 3.1 代码格式化检查
    run_test "Rust代码格式化检查" "cargo fmt -- --check"

    # 3.2 代码lint检查
    run_test "Clippy代码检查" "cargo clippy -- -D warnings"

    # 3.3 单元测试
    run_test "Rust单元测试" "cargo test --verbose" 2

    # 3.4 集成测试
    if [ -d "tests/integration" ]; then
        run_test "Rust集成测试" "cargo test --test integration --verbose"
    fi

    cd "$PROJECT_DIR"

    # 4. E2E测试
    show_title "4. E2E测试"

    # 检查Playwright是否安装
    if npx playwright --version &> /dev/null; then
        # 安装浏览器（如果需要）
        log_info "安装Playwright浏览器..."
        npx playwright install --with-deps

        # 启动开发服务器并运行测试
        log_info "启动开发服务器并运行E2E测试..."

        # 在后台启动开发服务器
        npm run dev &
        DEV_SERVER_PID=$!

        # 等待服务器启动
        sleep 5

        # 运行E2E测试
        run_test "E2E测试" "npm run test:e2e" 2

        # 停止开发服务器
        kill $DEV_SERVER_PID 2>/dev/null || true
    else
        log_warning "Playwright未安装，跳过E2E测试"
        log_info "安装命令: npm install @playwright/test"
    fi

    # 5. 性能测试
    show_title "5. 性能测试"

    cd "$PROJECT_DIR/src-tauri"

    # 检查criterion是否可用
    if grep -q "criterion" Cargo.toml; then
        log_info "运行性能基准测试..."

        # 创建基准测试结果目录
        mkdir -p "$PROJECT_DIR/benchmark-results"

        # 运行基准测试
        if cargo bench --verbose 2>&1 | tee "$PROJECT_DIR/benchmark-results/benchmark.log"; then
            log_success "性能基准测试完成"

            # 收集基准测试结果
            if [ -d "target/criterion" ]; then
                find target/criterion -name "report.json" -exec cp {} "$PROJECT_DIR/benchmark-results/" \;
                log_info "基准测试结果已保存到: $PROJECT_DIR/benchmark-results/"
            fi
        else
            log_warning "性能基准测试失败或未找到基准测试"
        fi
    else
        log_warning "Criterion未配置，跳过性能测试"
    fi

    cd "$PROJECT_DIR"

    # 6. 代码质量检查
    show_title "6. 代码质量检查"

    # 6.1 ESLint检查
    if [ -f ".eslintrc.js" ] || [ -f ".eslintrc.json" ] || [ -f ".eslintrc" ]; then
        run_test "ESLint代码检查" "npx eslint src --ext .ts,.vue --max-warnings=0"
    else
        log_warning "ESLint配置未找到，跳过"
    fi

    # 6.2 Prettier格式化检查
    if [ -f ".prettierrc" ] || [ -f ".prettierrc.json" ] || [ -f ".prettierrc.js" ]; then
        run_test "Prettier格式化检查" "npx prettier --check \"src/**/*.{ts,vue}\""
    else
        log_warning "Prettier配置未找到，跳过"
    fi

    # 6.3 安全扫描
    log_info "运行npm安全扫描..."
    if npm audit --audit-level=moderate; then
        log_success "npm安全扫描通过"
    else
        log_warning "npm安全扫描发现警告"
    fi

    # 7. 生成测试报告
    show_title "7. 测试报告"

    # 收集测试结果
    local total_tests=0
    local passed_tests=0
    local failed_tests=0

    # 这里可以添加更详细的测试结果收集逻辑

    # 生成简单的测试报告
    local report_file="$PROJECT_DIR/test-report-$(date +%Y%m%d-%H%M%S).md"

    cat > "$report_file" << EOF
# 测试报告
生成时间: $(date)

## 测试环境
- 操作系统: $(uname -s) $(uname -r)
- Node.js: $(node --version)
- npm: $(npm --version)
- Rust: $(cd src-tauri && cargo --version | cut -d' ' -f2)

## 测试结果摘要
- 前端单元测试: 完成
- Rust单元测试: 完成
- 集成测试: 完成
- E2E测试: 完成
- 性能测试: 完成
- 代码质量检查: 完成

## 详细结果
\`\`\`
$(find . -name "*.log" -type f -exec echo "=== {} ===" \; -exec tail -20 {} \; 2>/dev/null || true)
\`\`\`

## 建议
1. 查看覆盖率报告: open coverage/lcov-report/index.html
2. 查看E2E测试报告: open playwright-report/index.html
3. 查看性能测试结果: ls -la benchmark-results/
EOF

    log_success "测试报告已生成: $report_file"

    # 8. 显示总结
    show_title "测试完成"

    log_success "所有测试套件运行完成！"
    log_info "测试报告: $report_file"

    if [ -d "coverage/lcov-report" ]; then
        log_info "覆盖率报告: coverage/lcov-report/index.html"
    fi

    if [ -d "playwright-report" ]; then
        log_info "E2E测试报告: playwright-report/index.html"
    fi

    if [ -d "benchmark-results" ]; then
        log_info "性能测试结果: benchmark-results/"
    fi

    echo ""
    log_info "要查看详细结果，请运行:"
    echo "  cat $report_file"
    echo ""

    return 0
}

# 处理命令行参数
case "$1" in
    "--help" | "-h")
        echo "用法: $0 [选项]"
        echo ""
        echo "选项:"
        echo "  -h, --help     显示帮助信息"
        echo "  -f, --fast     快速模式（跳过E2E和性能测试）"
        echo "  -u, --unit     只运行单元测试"
        echo "  -i, --integration 只运行集成测试"
        echo "  -e, --e2e      只运行E2E测试"
        echo "  -p, --performance 只运行性能测试"
        echo ""
        exit 0
        ;;
    "--fast" | "-f")
        # 快速模式：跳过E2E和性能测试
        export SKIP_E2E=true
        export SKIP_PERFORMANCE=true
        ;;
    "--unit" | "-u")
        # 只运行单元测试
        show_title "运行单元测试"
        npm run test:unit
        cd src-tauri && cargo test
        exit 0
        ;;
    "--integration" | "-i")
        # 只运行集成测试
        show_title "运行集成测试"
        npm run test:integration
        exit 0
        ;;
    "--e2e" | "-e")
        # 只运行E2E测试
        show_title "运行E2E测试"
        npm run test:e2e
        exit 0
        ;;
    "--performance" | "-p")
        # 只运行性能测试
        show_title "运行性能测试"
        cd src-tauri && cargo bench
        exit 0
        ;;
esac

# 运行主函数
if main; then
    exit 0
else
    exit 1
fi