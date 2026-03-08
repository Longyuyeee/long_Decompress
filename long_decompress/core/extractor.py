"""
核心解压器模块

提供统一的解压接口，支持多种压缩格式。
"""

import os
import logging
from abc import ABC, abstractmethod
from typing import Optional, List, Dict, Any
from dataclasses import dataclass

logger = logging.getLogger(__name__)


class ExtractError(Exception):
    """解压错误异常"""
    pass


@dataclass
class ExtractResult:
    """解压结果"""
    source: str
    destination: str
    success: bool
    files_extracted: int
    total_size: int
    elapsed_time: float
    error_message: Optional[str] = None


class ExtractEngine(ABC):
    """解压引擎抽象基类"""

    @abstractmethod
    def extract(self, source_path: str, dest_path: str,
                password: Optional[str] = None) -> bool:
        """解压文件"""
        pass

    @abstractmethod
    def list_contents(self, source_path: str,
                     password: Optional[str] = None) -> List[str]:
        """列出压缩包内容"""
        pass

    @abstractmethod
    def test_integrity(self, source_path: str,
                      password: Optional[str] = None) -> bool:
        """测试完整性"""
        pass

    @abstractmethod
    def get_info(self, source_path: str) -> Dict[str, Any]:
        """获取压缩包信息"""
        pass

    @property
    @abstractmethod
    def supported_formats(self) -> List[str]:
        """支持的格式列表"""
        pass


class Extractor:
    """主解压器类"""

    def __init__(self):
        self.engines: Dict[str, ExtractEngine] = {}
        self._register_engines()

    def _register_engines(self):
        """注册解压引擎"""
        # 延迟导入，避免循环依赖
        try:
            from ..engines.zip_engine import ZipEngine
            self.engines['zip'] = ZipEngine()
        except ImportError as e:
            logger.warning(f"无法加载ZIP引擎: {e}")

        try:
            from ..engines.rar_engine import RarEngine
            self.engines['rar'] = RarEngine()
        except ImportError as e:
            logger.warning(f"无法加载RAR引擎: {e}")

        # 可以添加更多引擎...

    def detect_format(self, filepath: str) -> str:
        """检测文件格式"""
        import magic

        # 检查扩展名
        ext = os.path.splitext(filepath)[1].lower().lstrip('.')

        # 使用magic检测文件类型
        mime = magic.from_file(filepath, mime=True)

        # 映射到格式
        format_map = {
            'application/zip': 'zip',
            'application/x-rar': 'rar',
            'application/x-7z-compressed': '7z',
            'application/gzip': 'gz',
            'application/x-bzip2': 'bz2',
            'application/x-xz': 'xz',
            'application/x-tar': 'tar',
        }

        format_name = format_map.get(mime, ext)

        # 验证是否支持
        if not self.is_format_supported(format_name):
            raise ExtractError(f"不支持的文件格式: {format_name}")

        return format_name

    def is_format_supported(self, format_name: str) -> bool:
        """检查格式是否支持"""
        # 检查是否有对应的引擎
        for engine in self.engines.values():
            if format_name in engine.supported_formats:
                return True
        return False

    def extract(self, source_path: str, dest_path: str,
                password: Optional[str] = None,
                create_subfolder: bool = True) -> ExtractResult:
        """解压文件"""
        import time

        start_time = time.time()

        try:
            # 验证输入
            self._validate_input(source_path, dest_path)

            # 检测格式
            format_name = self.detect_format(source_path)

            # 选择引擎
            engine = self._select_engine(format_name)

            # 准备输出目录
            final_dest = self._prepare_destination(
                source_path, dest_path, create_subfolder)

            # 执行解压
            logger.info(f"开始解压: {source_path} -> {final_dest}")
            success = engine.extract(source_path, final_dest, password)

            if success:
                # 统计结果
                files_extracted = len(engine.list_contents(source_path))
                total_size = self._calculate_total_size(final_dest)

                result = ExtractResult(
                    source=source_path,
                    destination=final_dest,
                    success=True,
                    files_extracted=files_extracted,
                    total_size=total_size,
                    elapsed_time=time.time() - start_time
                )

                logger.info(f"解压成功: {result}")
                return result
            else:
                raise ExtractError("解压失败")

        except Exception as e:
            logger.error(f"解压失败: {e}")
            return ExtractResult(
                source=source_path,
                destination=dest_path,
                success=False,
                files_extracted=0,
                total_size=0,
                elapsed_time=time.time() - start_time,
                error_message=str(e)
            )

    def _validate_input(self, source_path: str, dest_path: str):
        """验证输入参数"""
        if not os.path.exists(source_path):
            raise ExtractError(f"源文件不存在: {source_path}")

        if not os.path.isfile(source_path):
            raise ExtractError(f"不是文件: {source_path}")

        # 检查目标目录是否可写
        dest_dir = os.path.dirname(dest_path) or '.'
        if not os.access(dest_dir, os.W_OK):
            raise ExtractError(f"目标目录不可写: {dest_dir}")

    def _select_engine(self, format_name: str) -> ExtractEngine:
        """选择解压引擎"""
        for engine_name, engine in self.engines.items():
            if format_name in engine.supported_formats:
                return engine

        raise ExtractError(f"没有找到支持 {format_name} 格式的引擎")

    def _prepare_destination(self, source_path: str, dest_path: str,
                            create_subfolder: bool) -> str:
        """准备输出目录"""
        if os.path.isdir(dest_path):
            # 目标是一个目录
            if create_subfolder:
                # 创建基于文件名的子目录
                basename = os.path.splitext(os.path.basename(source_path))[0]
                final_dest = os.path.join(dest_path, basename)
            else:
                final_dest = dest_path
        else:
            # 目标是一个文件路径
            final_dest = dest_path

        # 创建目录（如果不存在）
        os.makedirs(os.path.dirname(final_dest), exist_ok=True)

        return final_dest

    def _calculate_total_size(self, directory: str) -> int:
        """计算目录总大小"""
        total_size = 0
        for dirpath, dirnames, filenames in os.walk(directory):
            for filename in filenames:
                filepath = os.path.join(dirpath, filename)
                total_size += os.path.getsize(filepath)
        return total_size

    def get_supported_formats(self) -> List[str]:
        """获取所有支持的格式"""
        formats = []
        for engine in self.engines.values():
            formats.extend(engine.supported_formats)
        return list(set(formats))  # 去重