#!/usr/bin/env python3
"""
胧解压·方便助手 - 使用示例

展示核心功能的使用方法。
"""

import os
import sys
import tempfile
import zipfile
from pathlib import Path

# 添加项目路径
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

from long_decompress.core.extractor import Extractor
from long_decompress.core.batch_processor import BatchProcessor, BatchTask
from long_decompress.core.password_manager import PasswordManager
from long_decompress.core.error_handler import ErrorHandler


def create_test_zip(filename: str, content: str, password: str = None):
    """创建测试ZIP文件"""
    with zipfile.ZipFile(filename, 'w') as zipf:
        if password:
            zipf.writestr('test.txt', content, pwd=password.encode('utf-8'))
        else:
            zipf.writestr('test.txt', content)
    print(f"创建测试文件: {filename}")


def demo_extractor():
    """演示解压器功能"""
    print("\n" + "="*50)
    print("演示：解压器功能")
    print("="*50)

    # 创建临时目录
    with tempfile.TemporaryDirectory() as tmpdir:
        # 创建测试文件
        test_zip = os.path.join(tmpdir, 'test.zip')
        create_test_zip(test_zip, "Hello, 胧解压!")

        # 创建解压器
        extractor = Extractor()

        # 检测格式
        format_name = extractor.detect_format(test_zip)
        print(f"检测到格式: {format_name}")

        # 检查是否支持
        if extractor.is_format_supported(format_name):
            print("✓ 格式支持")

        # 解压文件
        output_dir = os.path.join(tmpdir, 'output')
        result = extractor.extract(test_zip, output_dir)

        if result.success:
            print(f"✓ 解压成功")
            print(f"  解压文件数: {result.files_extracted}")
            print(f"  总大小: {result.total_size} 字节")
            print(f"  耗时: {result.elapsed_time:.2f} 秒")

            # 验证解压结果
            extracted_file = os.path.join(output_dir, 'test.txt')
            if os.path.exists(extracted_file):
                with open(extracted_file, 'r') as f:
                    content = f.read()
                print(f"  内容: {content}")
        else:
            print(f"✗ 解压失败: {result.error_message}")

        # 显示支持的格式
        formats = extractor.get_supported_formats()
        print(f"\n支持的格式: {', '.join(formats)}")


def demo_batch_processor():
    """演示批量处理器功能"""
    print("\n" + "="*50)
    print("演示：批量处理器功能")
    print("="*50)

    with tempfile.TemporaryDirectory() as tmpdir:
        # 创建多个测试文件
        test_files = []
        for i in range(3):
            filename = os.path.join(tmpdir, f'test_{i}.zip')
            create_test_zip(filename, f"文件 {i} 的内容")
            test_files.append(filename)

        # 创建批量处理器
        processor = BatchProcessor(max_workers=2)

        # 创建批量任务
        tasks = []
        for filepath in test_files:
            task = BatchTask(
                source_path=filepath,
                dest_dir=tmpdir,
                create_subfolder=True
            )
            tasks.append(task)

        # 添加进度回调
        def progress_callback(summary, finished):
            if finished:
                print(f"批量处理完成！")
                print(f"  成功: {summary.successful_tasks}")
                print(f"  失败: {summary.failed_tasks}")
                print(f"  跳过: {summary.skipped_tasks}")
                print(f"  总耗时: {summary.total_time:.2f} 秒")
            else:
                print(f"进度: {summary.progress_percentage:.1f}% "
                      f"({summary.completed_tasks}/{summary.total_tasks})")

        processor.add_progress_callback(progress_callback)

        # 执行批量处理
        print(f"开始批量处理 {len(tasks)} 个文件...")
        result = processor.process_batch(tasks)

        print(f"\n批量处理结果:")
        print(f"  成功: {result['successful']}")
        print(f"  失败: {result['failed']}")
        print(f"  跳过: {result['skipped']}")


def demo_password_manager():
    """演示密码管理器功能"""
    print("\n" + "="*50)
    print("演示：密码管理器功能")
    print("="*50)

    with tempfile.TemporaryDirectory() as tmpdir:
        # 创建密码管理器
        password_manager = PasswordManager(data_dir=tmpdir)

        # 添加一些密码
        password_manager.add_password("password123", source="demo")
        password_manager.add_password("hello2024", source="demo", tags=["测试"])
        password_manager.add_password("", source="common")  # 空密码

        # 创建测试文件
        test_file = os.path.join(tmpdir, "project_backup.zip")

        # 获取密码建议
        print(f"为文件获取密码建议: {os.path.basename(test_file)}")
        suggestions = password_manager.get_passwords_for_file(test_file, max_suggestions=5)

        for i, suggestion in enumerate(suggestions, 1):
            print(f"  {i}. {suggestion.password} "
                  f"(置信度: {suggestion.confidence:.1%}, 来源: {suggestion.source})")

        # 记录成功解压
        password_manager.record_success(test_file, "password123")

        # 获取统计信息
        stats = password_manager.get_statistics()
        print(f"\n密码管理器统计:")
        print(f"  总密码数: {stats['total_passwords']}")
        print(f"  总使用次数: {stats['total_usage']}")
        print(f"  总成功次数: {stats['total_success']}")
        print(f"  成功率: {stats['success_rate']:.1%}")
        print(f"  缓存大小: {stats['cache_size']}")


def demo_error_handler():
    """演示错误处理器功能"""
    print("\n" + "="*50)
    print("演示：错误处理器功能")
    print("="*50)

    with tempfile.TemporaryDirectory() as tmpdir:
        # 创建错误处理器
        error_handler = ErrorHandler(log_dir=tmpdir)

        # 模拟各种错误
        test_errors = [
            FileNotFoundError("文件不存在: /path/to/nonexistent/file.zip"),
            ValueError("密码错误"),
            PermissionError("没有写入权限"),
            OSError("磁盘空间不足"),
            RuntimeError("未知错误")
        ]

        for i, error in enumerate(test_errors, 1):
            context = {
                'operation': 'extract',
                'filepath': f'/path/to/file_{i}.zip',
                'timestamp': '2024-01-01 12:00:00'
            }

            print(f"\n处理错误 {i}: {type(error).__name__}")
            error_info = error_handler.handle_error(error, context)

            print(f"  错误类型: {error_info.error_type}")
            print(f"  用户消息: {error_info.error_message[:50]}...")
            print(f"  可以重试: {'是' if error_info.can_retry else '否'}")
            print(f"  需要用户操作: {'是' if error_info.requires_user_action else '否'}")

            if error_info.recovery_suggestions:
                print(f"  恢复建议:")
                for suggestion in error_info.recovery_suggestions[:2]:
                    print(f"    - {suggestion}")

        # 获取错误统计
        stats = error_handler.get_error_statistics()
        print(f"\n错误统计:")
        print(f"  总错误数: {stats['total_errors']}")
        print(f"  错误类型分布:")
        for error_type, count in stats['error_types'].items():
            print(f"    {error_type}: {count}")

        # 获取最近的错误
        recent_errors = error_handler.get_recent_errors(limit=3)
        if recent_errors:
            print(f"\n最近 {len(recent_errors)} 个错误:")
            for error in recent_errors:
                print(f"  - {error.error_type}: {error.error_message[:30]}...")


def demo_integration():
    """演示集成功能"""
    print("\n" + "="*50)
    print("演示：集成功能")
    print("="*50)

    with tempfile.TemporaryDirectory() as tmpdir:
        # 创建各个组件
        extractor = Extractor()
        batch_processor = BatchProcessor()
        password_manager = PasswordManager(data_dir=tmpdir)
        error_handler = ErrorHandler(log_dir=tmpdir)

        # 创建加密的测试文件
        test_zip = os.path.join(tmpdir, 'encrypted.zip')
        with zipfile.ZipFile(test_zip, 'w') as zipf:
            zipf.writestr('secret.txt', '机密内容',
                         pwd='mypassword'.encode('utf-8'))

        print(f"创建加密测试文件: {test_zip}")

        # 添加密码到密码管理器
        password_manager.add_password("mypassword", source="test")
        password_manager.add_password("wrongpass", source="test")

        # 尝试解压
        try:
            # 第一次尝试（使用错误密码）
            print("\n第一次尝试（错误密码）...")
            result = extractor.extract(test_zip, tmpdir, password="wrongpass")
            if not result.success:
                print(f"✗ 解压失败: {result.error_message}")
                password_manager.record_failure("wrongpass")

            # 从密码管理器获取建议
            print("\n从密码管理器获取建议...")
            suggestions = password_manager.get_passwords_for_file(test_zip)

            # 尝试建议的密码
            for suggestion in suggestions[:3]:
                print(f"尝试密码: {suggestion.password} "
                      f"(置信度: {suggestion.confidence:.1%})")

                result = extractor.extract(test_zip, tmpdir,
                                          password=suggestion.password)

                if result.success:
                    print(f"✓ 解压成功！使用密码: {suggestion.password}")
                    password_manager.record_success(test_zip, suggestion.password)
                    break
                else:
                    print(f"✗ 密码错误")
                    password_manager.record_failure(suggestion.password)

        except Exception as e:
            # 使用错误处理器处理异常
            context = {
                'filepath': test_zip,
                'operation': 'demo_integration'
            }
            error_info = error_handler.handle_error(e, context)

            print(f"\n错误处理结果:")
            print(error_handler.format_user_message(error_info))


def main():
    """主函数"""
    print("胧解压·方便助手 - 功能演示")
    print("="*50)

    # 演示各个功能模块
    demo_extractor()
    demo_batch_processor()
    demo_password_manager()
    demo_error_handler()
    demo_integration()

    print("\n" + "="*50)
    print("演示完成！")
    print("="*50)


if __name__ == "__main__":
    main()