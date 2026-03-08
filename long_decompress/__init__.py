"""
胧解压·方便助手 - 压缩文件解压工具

一个功能强大、易于使用的压缩文件解压工具，
专注于提供便捷、高效、安全的解压体验。
"""

__version__ = "1.0.0"
__author__ = "胧解压团队"
__email__ = "support@example.com"

import logging

# 配置日志
logging.basicConfig(
    level=logging.INFO,
    format='%(asctime)s - %(name)s - %(levelname)s - %(message)s'
)

logger = logging.getLogger(__name__)

# 导出主要类
from .core.extractor import Extractor
from .core.batch_processor import BatchProcessor
from .core.password_manager import PasswordManager
from .core.error_handler import ErrorHandler

__all__ = [
    'Extractor',
    'BatchProcessor',
    'PasswordManager',
    'ErrorHandler',
    'logger'
]