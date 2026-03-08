"""
ZIP解压引擎

使用Python标准库的zipfile模块实现ZIP格式支持。
"""

import zipfile
import os
import logging
from typing import Optional, List, Dict, Any
from ..core.extractor import ExtractEngine, ExtractError

logger = logging.getLogger(__name__)


class ZipEngine(ExtractEngine):
    """ZIP解压引擎"""

    def __init__(self):
        self.supported = ['zip', 'zipx']

    def extract(self, source_path: str, dest_path: str,
                password: Optional[str] = None) -> bool:
        """解压ZIP文件"""
        try:
            with zipfile.ZipFile(source_path, 'r') as zip_ref:
                # 设置密码（如果有）
                if password:
                    zip_ref.setpassword(password.encode('utf-8'))

                # 验证文件完整性
                if zip_ref.testzip() is not None:
                    logger.warning(f"ZIP文件可能损坏: {source_path}")

                # 解压所有文件
                zip_ref.extractall(dest_path)

                logger.info(f"成功解压ZIP文件: {source_path}")
                return True

        except zipfile.BadZipFile as e:
            raise ExtractError(f"ZIP文件损坏: {str(e)}")
        except RuntimeError as e:
            if 'password' in str(e).lower():
                raise ExtractError("密码错误或需要密码")
            else:
                raise ExtractError(f"解压失败: {str(e)}")
        except Exception as e:
            raise ExtractError(f"解压失败: {str(e)}")

    def list_contents(self, source_path: str,
                     password: Optional[str] = None) -> List[str]:
        """列出ZIP文件内容"""
        try:
            with zipfile.ZipFile(source_path, 'r') as zip_ref:
                if password:
                    zip_ref.setpassword(password.encode('utf-8'))

                return zip_ref.namelist()

        except Exception as e:
            logger.error(f"无法列出ZIP内容: {e}")
            return []

    def test_integrity(self, source_path: str,
                      password: Optional[str] = None) -> bool:
        """测试ZIP文件完整性"""
        try:
            with zipfile.ZipFile(source_path, 'r') as zip_ref:
                if password:
                    zip_ref.setpassword(password.encode('utf-8'))

                # 测试所有文件的CRC
                bad_file = zip_ref.testzip()
                if bad_file is not None:
                    logger.warning(f"文件损坏: {bad_file}")
                    return False

                return True

        except Exception as e:
            logger.error(f"完整性测试失败: {e}")
            return False

    def get_info(self, source_path: str) -> Dict[str, Any]:
        """获取ZIP文件信息"""
        try:
            with zipfile.ZipFile(source_path, 'r') as zip_ref:
                info = {
                    'format': 'ZIP',
                    'file_count': len(zip_ref.namelist()),
                    'compressed_size': os.path.getsize(source_path),
                    'uncompressed_size': sum(
                        info.file_size for info in zip_ref.infolist()
                    ),
                    'compression_ratio': 0,
                    'encrypted': any(info.flag_bits & 0x1
                                   for info in zip_ref.infolist()),
                    'comment': zip_ref.comment.decode('utf-8', errors='ignore')
                    if zip_ref.comment else '',
                    'files': []
                }

                # 计算压缩率
                if info['uncompressed_size'] > 0:
                    info['compression_ratio'] = (
                        info['compressed_size'] / info['uncompressed_size']
                    )

                # 添加文件详细信息
                for file_info in zip_ref.infolist():
                    info['files'].append({
                        'filename': file_info.filename,
                        'compressed_size': file_info.compress_size,
                        'uncompressed_size': file_info.file_size,
                        'compress_type': self._get_compress_type(
                            file_info.compress_type),
                        'encrypted': bool(file_info.flag_bits & 0x1),
                        'modified': self._parse_dos_time(file_info.date_time),
                        'crc': hex(file_info.CRC)[2:].upper()
                    })

                return info

        except Exception as e:
            logger.error(f"获取ZIP信息失败: {e}")
            return {
                'format': 'ZIP',
                'error': str(e)
            }

    def _get_compress_type(self, compress_type: int) -> str:
        """获取压缩类型名称"""
        types = {
            zipfile.ZIP_STORED: 'STORED',
            zipfile.ZIP_DEFLATED: 'DEFLATED',
            zipfile.ZIP_BZIP2: 'BZIP2',
            zipfile.ZIP_LZMA: 'LZMA',
        }
        return types.get(compress_type, f'UNKNOWN({compress_type})')

    def _parse_dos_time(self, dos_time: tuple) -> str:
        """解析DOS时间格式"""
        if not dos_time or len(dos_time) != 6:
            return ''

        year, month, day, hour, minute, second = dos_time
        # DOS时间：年从1980开始
        year += 1980

        return f"{year:04d}-{month:02d}-{day:02d} {hour:02d}:{minute:02d}:{second:02d}"

    @property
    def supported_formats(self) -> List[str]:
        """支持的格式列表"""
        return self.supported