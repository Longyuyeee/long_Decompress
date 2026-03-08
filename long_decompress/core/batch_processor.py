"""
批量处理器模块

提供批量解压文件的功能，支持多线程并行处理。
"""

import os
import time
import threading
import concurrent.futures
from typing import List, Dict, Optional, Callable
from dataclasses import dataclass, field
from queue import Queue, Empty
import logging

from .extractor import Extractor, ExtractResult

logger = logging.getLogger(__name__)


@dataclass
class BatchTask:
    """批量任务"""
    source_path: str
    dest_dir: str
    password: Optional[str] = None
    create_subfolder: bool = True
    retry_count: int = 0


@dataclass
class BatchResult:
    """批量处理结果"""
    task: BatchTask
    result: Optional[ExtractResult] = None
    error: Optional[str] = None
    start_time: float = field(default_factory=time.time)
    end_time: float = 0.0

    def __post_init__(self):
        if self.end_time == 0:
            self.end_time = time.time()

    @property
    def elapsed_time(self) -> float:
        """耗时"""
        return self.end_time - self.start_time

    @property
    def success(self) -> bool:
        """是否成功"""
        return self.result is not None and self.result.success


@dataclass
class BatchSummary:
    """批量处理摘要"""
    total_tasks: int = 0
    completed_tasks: int = 0
    successful_tasks: int = 0
    failed_tasks: int = 0
    skipped_tasks: int = 0
    total_size: int = 0
    total_time: float = 0.0
    start_time: float = field(default_factory=time.time)
    end_time: float = 0.0

    def update(self, result: BatchResult):
        """更新统计"""
        self.completed_tasks += 1

        if result.success:
            self.successful_tasks += 1
            if result.result:
                self.total_size += result.result.total_size
        elif result.error and "跳过" in result.error:
            self.skipped_tasks += 1
        else:
            self.failed_tasks += 1

    def finalize(self):
        """完成统计"""
        self.end_time = time.time()
        self.total_time = self.end_time - self.start_time

    @property
    def progress_percentage(self) -> float:
        """进度百分比"""
        if self.total_tasks == 0:
            return 0.0
        return (self.completed_tasks / self.total_tasks) * 100

    @property
    def average_speed(self) -> float:
        """平均速度（字节/秒）"""
        if self.total_time == 0:
            return 0.0
        return self.total_size / self.total_time


class BatchProcessor:
    """批量处理器"""

    def __init__(self, max_workers: Optional[int] = None):
        self.extractor = Extractor()
        self.max_workers = max_workers or min(4, os.cpu_count() or 1)
        self.is_running = False
        self._progress_callbacks: List[Callable] = []
        self._cancel_requested = False

    def process_batch(self, tasks: List[BatchTask],
                     output_dir: Optional[str] = None) -> Dict:
        """处理批量任务"""
        self.is_running = True
        self._cancel_requested = False

        # 准备输出目录
        if output_dir:
            os.makedirs(output_dir, exist_ok=True)

        # 创建任务队列
        task_queue = Queue()
        for task in tasks:
            if output_dir and not task.dest_dir:
                task.dest_dir = output_dir
            task_queue.put(task)

        # 创建结果列表
        results: List[BatchResult] = []
        summary = BatchSummary(total_tasks=len(tasks))

        # 进度跟踪
        self._notify_progress(summary)

        try:
            # 使用线程池处理任务
            with concurrent.futures.ThreadPoolExecutor(
                max_workers=self.max_workers) as executor:

                # 提交所有任务
                future_to_task = {}
                while not task_queue.empty() and not self._cancel_requested:
                    try:
                        task = task_queue.get_nowait()
                        future = executor.submit(
                            self._process_single_task, task)
                        future_to_task[future] = task
                    except Empty:
                        break

                # 收集结果
                for future in concurrent.futures.as_completed(future_to_task):
                    if self._cancel_requested:
                        break

                    task = future_to_task[future]
                    try:
                        batch_result = future.result()
                        results.append(batch_result)
                        summary.update(batch_result)
                        self._notify_progress(summary)
                    except Exception as e:
                        logger.error(f"任务处理异常: {e}")
                        error_result = BatchResult(
                            task=task,
                            error=f"处理异常: {str(e)}"
                        )
                        results.append(error_result)
                        summary.update(error_result)
                        self._notify_progress(summary)

        except Exception as e:
            logger.error(f"批量处理失败: {e}")
            raise

        finally:
            self.is_running = False
            summary.finalize()
            self._notify_progress(summary, finished=True)

        return {
            'summary': summary,
            'results': results,
            'successful': summary.successful_tasks,
            'failed': summary.failed_tasks,
            'skipped': summary.skipped_tasks
        }

    def _process_single_task(self, task: BatchTask) -> BatchResult:
        """处理单个任务"""
        logger.info(f"处理任务: {task.source_path}")

        # 检查文件是否存在
        if not os.path.exists(task.source_path):
            return BatchResult(
                task=task,
                error=f"文件不存在: {task.source_path}"
            )

        # 检查是否支持该格式
        try:
            format_name = self.extractor.detect_format(task.source_path)
            if not self.extractor.is_format_supported(format_name):
                return BatchResult(
                    task=task,
                    error=f"不支持的文件格式: {format_name}"
                )
        except Exception as e:
            return BatchResult(
                task=task,
                error=f"格式检测失败: {str(e)}"
            )

        # 执行解压
        try:
            result = self.extractor.extract(
                source_path=task.source_path,
                dest_path=task.dest_dir,
                password=task.password,
                create_subfolder=task.create_subfolder
            )

            return BatchResult(task=task, result=result)

        except Exception as e:
            logger.error(f"解压失败: {task.source_path} - {e}")

            # 重试逻辑
            if task.retry_count < 3:
                task.retry_count += 1
                logger.info(f"重试 {task.retry_count}/3: {task.source_path}")
                time.sleep(1)  # 等待1秒后重试
                return self._process_single_task(task)
            else:
                return BatchResult(
                    task=task,
                    error=f"解压失败（已重试3次）: {str(e)}"
                )

    def add_progress_callback(self, callback: Callable):
        """添加进度回调"""
        self._progress_callbacks.append(callback)

    def remove_progress_callback(self, callback: Callable):
        """移除进度回调"""
        if callback in self._progress_callbacks:
            self._progress_callbacks.remove(callback)

    def _notify_progress(self, summary: BatchSummary, finished: bool = False):
        """通知进度更新"""
        for callback in self._progress_callbacks:
            try:
                callback(summary, finished)
            except Exception as e:
                logger.error(f"进度回调失败: {e}")

    def cancel(self):
        """取消批量处理"""
        self._cancel_requested = True
        logger.info("批量处理已取消")

    def find_compressed_files(self, directory: str,
                             recursive: bool = True) -> List[str]:
        """查找目录中的压缩文件"""
        compressed_files = []
        supported_formats = self.extractor.get_supported_formats()

        def should_include_file(filepath: str) -> bool:
            """检查文件是否应该包含"""
            ext = os.path.splitext(filepath)[1].lower().lstrip('.')
            return ext in supported_formats

        if recursive:
            for root, dirs, files in os.walk(directory):
                for file in files:
                    filepath = os.path.join(root, file)
                    if should_include_file(filepath):
                        compressed_files.append(filepath)
        else:
            for item in os.listdir(directory):
                itempath = os.path.join(directory, item)
                if os.path.isfile(itempath) and should_include_file(itempath):
                    compressed_files.append(itempath)

        return compressed_files

    def create_batch_from_directory(self, directory: str,
                                   output_dir: Optional[str] = None,
                                   recursive: bool = True) -> List[BatchTask]:
        """从目录创建批量任务"""
        files = self.find_compressed_files(directory, recursive)

        tasks = []
        for filepath in files:
            task = BatchTask(
                source_path=filepath,
                dest_dir=output_dir or os.path.dirname(filepath)
            )
            tasks.append(task)

        return tasks