"""
错误处理器模块

提供统一的错误处理和恢复机制。
"""

import os
import logging
import traceback
from typing import Dict, Any, Optional, List
from dataclasses import dataclass
from datetime import datetime
import json

logger = logging.getLogger(__name__)


@dataclass
class ErrorInfo:
    """错误信息"""
    error_type: str
    error_message: str
    error_details: str
    context: Dict[str, Any]
    timestamp: datetime
    recovery_suggestions: List[str]
    can_retry: bool
    requires_user_action: bool

    def to_dict(self) -> Dict[str, Any]:
        """转换为字典"""
        return {
            'error_type': self.error_type,
            'error_message': self.error_message,
            'error_details': self.error_details,
            'context': self.context,
            'timestamp': self.timestamp.isoformat(),
            'recovery_suggestions': self.recovery_suggestions,
            'can_retry': self.can_retry,
            'requires_user_action': self.requires_user_action
        }


class ErrorHandler:
    """错误处理器"""

    # 错误分类映射
    ERROR_CATEGORIES = {
        'file_not_found': {
            'user_message': "文件不存在",
            'suggestions': [
                "检查文件路径是否正确",
                "确认文件是否被移动或删除",
                "检查文件权限"
            ],
            'can_retry': True,
            'requires_user_action': True
        },
        'file_corrupted': {
            'user_message': "文件可能已损坏",
            'suggestions': [
                "尝试使用文件修复工具",
                "重新下载文件",
                "检查文件完整性"
            ],
            'can_retry': False,
            'requires_user_action': True
        },
        'wrong_password': {
            'user_message': "密码不正确",
            'suggestions': [
                "尝试空密码",
                "尝试文件名作为密码",
                "使用密码本功能自动尝试",
                "联系文件提供者获取正确密码"
            ],
            'can_retry': True,
            'requires_user_action': True
        },
        'disk_full': {
            'user_message': "磁盘空间不足",
            'suggestions': [
                "清理磁盘空间",
                "选择其他磁盘",
                "删除临时文件"
            ],
            'can_retry': True,
            'requires_user_action': True
        },
        'permission_denied': {
            'user_message': "没有写入权限",
            'suggestions': [
                "以管理员身份运行程序",
                "选择用户目录",
                "修改目录权限"
            ],
            'can_retry': True,
            'requires_user_action': True
        },
        'format_not_supported': {
            'user_message': "不支持的文件格式",
            'suggestions': [
                "转换为支持的格式（如ZIP）",
                "使用其他解压工具",
                "检查文件扩展名是否正确"
            ],
            'can_retry': False,
            'requires_user_action': True
        },
        'network_error': {
            'user_message': "网络错误",
            'suggestions': [
                "检查网络连接",
                "重试操作",
                "使用本地文件"
            ],
            'can_retry': True,
            'requires_user_action': False
        },
        'unknown_error': {
            'user_message': "发生未知错误",
            'suggestions': [
                "重启程序",
                "检查系统日志",
                "联系技术支持"
            ],
            'can_retry': True,
            'requires_user_action': True
        }
    }

    def __init__(self, log_dir: Optional[str] = None):
        self.log_dir = log_dir or self._get_default_log_dir()
        self.error_log_file = os.path.join(self.log_dir, 'errors.json')
        self._setup_logging()

    def _get_default_log_dir(self) -> str:
        """获取默认日志目录"""
        if os.name == 'nt':  # Windows
            base_dir = os.getenv('APPDATA', '')
            return os.path.join(base_dir, '胧解压', 'logs')
        else:  # Linux/macOS
            base_dir = os.path.expanduser('~')
            return os.path.join(base_dir, '.config', '胧解压', 'logs')

    def _setup_logging(self):
        """设置日志"""
        os.makedirs(self.log_dir, exist_ok=True)

    def handle_error(self, error: Exception, context: Dict[str, Any]) -> ErrorInfo:
        """处理错误"""
        # 分类错误
        error_type = self._classify_error(error)

        # 获取错误分类信息
        category = self.ERROR_CATEGORIES.get(
            error_type, self.ERROR_CATEGORIES['unknown_error'])

        # 创建错误信息
        error_info = ErrorInfo(
            error_type=error_type,
            error_message=str(error),
            error_details=self._get_error_details(error),
            context=context,
            timestamp=datetime.now(),
            recovery_suggestions=category['suggestions'],
            can_retry=category['can_retry'],
            requires_user_action=category['requires_user_action']
        )

        # 记录错误
        self._log_error(error_info)

        # 根据错误类型采取不同行动
        self._take_action(error_info)

        return error_info

    def _classify_error(self, error: Exception) -> str:
        """错误分类"""
        error_str = str(error).lower()

        # 文件相关错误
        if any(word in error_str for word in ['no such file', 'not found']):
            return 'file_not_found'
        elif any(word in error_str for word in ['corrupt', 'damage', 'bad']):
            return 'file_corrupted'

        # 密码相关错误
        elif any(word in error_str for word in ['password', 'encrypt', 'decrypt']):
            return 'wrong_password'

        # 磁盘相关错误
        elif any(word in error_str for word in ['space', 'full', 'disk']):
            return 'disk_full'
        elif any(word in error_str for word in ['permission', 'access', 'denied']):
            return 'permission_denied'

        # 格式相关错误
        elif any(word in error_str for word in ['format', '不支持', 'unsupported']):
            return 'format_not_supported'

        # 网络相关错误
        elif any(word in error_str for word in ['network', 'connection', 'timeout']):
            return 'network_error'

        # 未知错误
        else:
            return 'unknown_error'

    def _get_error_details(self, error: Exception) -> str:
        """获取错误详情"""
        try:
            return traceback.format_exc()
        except:
            return str(error)

    def _log_error(self, error_info: ErrorInfo):
        """记录错误到日志文件"""
        try:
            # 读取现有错误日志
            errors = []
            if os.path.exists(self.error_log_file):
                with open(self.error_log_file, 'r', encoding='utf-8') as f:
                    try:
                        errors = json.load(f)
                    except json.JSONDecodeError:
                        errors = []

            # 添加新错误
            errors.append(error_info.to_dict())

            # 限制日志大小（最多保留1000个错误）
            if len(errors) > 1000:
                errors = errors[-1000:]

            # 写入文件
            with open(self.error_log_file, 'w', encoding='utf-8') as f:
                json.dump(errors, f, ensure_ascii=False, indent=2)

            # 同时记录到应用日志
            logger.error(f"错误类型: {error_info.error_type}, "
                        f"消息: {error_info.error_message}")

        except Exception as e:
            logger.error(f"记录错误失败: {e}")

    def _take_action(self, error_info: ErrorInfo):
        """根据错误类型采取行动"""
        if error_info.error_type == 'disk_full':
            self._handle_disk_full(error_info)
        elif error_info.error_type == 'permission_denied':
            self._handle_permission_denied(error_info)
        elif error_info.error_type == 'file_corrupted':
            self._handle_file_corrupted(error_info)

    def _handle_disk_full(self, error_info: ErrorInfo):
        """处理磁盘空间不足"""
        # 尝试清理临时文件
        temp_dir = self._get_temp_dir()
        if os.path.exists(temp_dir):
            try:
                import shutil
                shutil.rmtree(temp_dir)
                logger.info(f"已清理临时目录: {temp_dir}")
            except Exception as e:
                logger.warning(f"清理临时目录失败: {e}")

    def _handle_permission_denied(self, error_info: ErrorInfo):
        """处理权限不足"""
        # 记录权限问题，供后续分析
        context = error_info.context
        if 'filepath' in context:
            filepath = context['filepath']
            try:
                # 检查文件权限
                stat_info = os.stat(filepath)
                logger.info(f"文件权限: {oct(stat_info.st_mode)}")
            except Exception as e:
                logger.warning(f"检查文件权限失败: {e}")

    def _handle_file_corrupted(self, error_info: ErrorInfo):
        """处理文件损坏"""
        # 记录损坏文件信息
        context = error_info.context
        if 'filepath' in context:
            filepath = context['filepath']
            try:
                file_size = os.path.getsize(filepath)
                logger.info(f"损坏文件: {filepath}, 大小: {file_size} 字节")
            except Exception as e:
                logger.warning(f"获取文件大小失败: {e}")

    def _get_temp_dir(self) -> str:
        """获取临时目录"""
        import tempfile
        return os.path.join(tempfile.gettempdir(), 'long_decompress')

    def get_recent_errors(self, limit: int = 50) -> List[ErrorInfo]:
        """获取最近的错误"""
        try:
            if not os.path.exists(self.error_log_file):
                return []

            with open(self.error_log_file, 'r', encoding='utf-8') as f:
                errors_data = json.load(f)

            errors = []
            for error_data in errors_data[-limit:]:
                error_info = ErrorInfo(
                    error_type=error_data['error_type'],
                    error_message=error_data['error_message'],
                    error_details=error_data['error_details'],
                    context=error_data['context'],
                    timestamp=datetime.fromisoformat(error_data['timestamp']),
                    recovery_suggestions=error_data['recovery_suggestions'],
                    can_retry=error_data['can_retry'],
                    requires_user_action=error_data['requires_user_action']
                )
                errors.append(error_info)

            return errors

        except Exception as e:
            logger.error(f"获取错误日志失败: {e}")
            return []

    def clear_error_log(self):
        """清空错误日志"""
        try:
            if os.path.exists(self.error_log_file):
                os.remove(self.error_log_file)
                logger.info("已清空错误日志")
        except Exception as e:
            logger.error(f"清空错误日志失败: {e}")

    def get_error_statistics(self) -> Dict[str, Any]:
        """获取错误统计"""
        try:
            errors = self.get_recent_errors(limit=1000)

            stats = {
                'total_errors': len(errors),
                'error_types': {},
                'recent_errors': [],
                'most_common_error': None,
                'error_trend': []
            }

            # 统计错误类型
            for error in errors:
                error_type = error.error_type
                stats['error_types'][error_type] = \
                    stats['error_types'].get(error_type, 0) + 1

            # 找出最常见的错误
            if stats['error_types']:
                stats['most_common_error'] = max(
                    stats['error_types'].items(),
                    key=lambda x: x[1]
                )[0]

            # 最近5个错误
            stats['recent_errors'] = [
                {
                    'type': e.error_type,
                    'message': e.error_message[:100],
                    'time': e.timestamp.isoformat()
                }
                for e in errors[-5:]
            ]

            # 错误趋势（按天统计）
            from collections import defaultdict
            daily_counts = defaultdict(int)
            for error in errors:
                date_str = error.timestamp.strftime('%Y-%m-%d')
                daily_counts[date_str] += 1

            stats['error_trend'] = [
                {'date': date, 'count': count}
                for date, count in sorted(daily_counts.items())
            ]

            return stats

        except Exception as e:
            logger.error(f"获取错误统计失败: {e}")
            return {
                'total_errors': 0,
                'error_types': {},
                'recent_errors': [],
                'most_common_error': None,
                'error_trend': []
            }

    def format_user_message(self, error_info: ErrorInfo) -> str:
        """格式化用户友好的错误消息"""
        category = self.ERROR_CATEGORIES.get(
            error_info.error_type,
            self.ERROR_CATEGORIES['unknown_error']
        )

        message = f"{category['user_message']}\n\n"

        if error_info.recovery_suggestions:
            message += "建议：\n"
            for i, suggestion in enumerate(error_info.recovery_suggestions, 1):
                message += f"{i}. {suggestion}\n"

        # 添加技术细节（可展开）
        message += f"\n错误详情：{error_info.error_message}"

        return message