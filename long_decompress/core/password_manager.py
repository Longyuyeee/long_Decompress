"""
密码管理器模块

提供密码本管理和智能密码尝试功能。
"""

import os
import json
import logging
import hashlib
from typing import List, Dict, Optional, Set
from dataclasses import dataclass, field
from datetime import datetime, timedelta
import threading

logger = logging.getLogger(__name__)


@dataclass
class PasswordEntry:
    """密码条目"""
    password: str
    source: str  # 来源：manual, imported, auto_generated, cached
    tags: List[str] = field(default_factory=list)
    usage_count: int = 0
    success_count: int = 0
    last_used: Optional[datetime] = None
    created_at: datetime = field(default_factory=datetime.now)
    notes: str = ""

    @property
    def success_rate(self) -> float:
        """成功率"""
        if self.usage_count == 0:
            return 0.0
        return self.success_count / self.usage_count

    def record_usage(self, success: bool):
        """记录使用情况"""
        self.usage_count += 1
        if success:
            self.success_count += 1
        self.last_used = datetime.now()


@dataclass
class PasswordMatch:
    """密码匹配结果"""
    password: str
    confidence: float  # 置信度 0.0-1.0
    source: str
    reason: str  # 匹配原因


class PasswordManager:
    """密码管理器"""

    def __init__(self, data_dir: Optional[str] = None):
        self.data_dir = data_dir or self._get_default_data_dir()
        self.passwords_file = os.path.join(self.data_dir, 'passwords.json')
        self.cache_file = os.path.join(self.data_dir, 'cache.json')

        self.passwords: Dict[str, PasswordEntry] = {}  # key: password_hash
        self.password_cache: Dict[str, str] = {}  # key: file_hash, value: password
        self._lock = threading.RLock()

        self._load_data()

    def _get_default_data_dir(self) -> str:
        """获取默认数据目录"""
        if os.name == 'nt':  # Windows
            base_dir = os.getenv('APPDATA', '')
            return os.path.join(base_dir, '胧解压', 'passwords')
        else:  # Linux/macOS
            base_dir = os.path.expanduser('~')
            return os.path.join(base_dir, '.config', '胧解压', 'passwords')

    def _load_data(self):
        """加载数据"""
        os.makedirs(self.data_dir, exist_ok=True)

        # 加载密码本
        if os.path.exists(self.passwords_file):
            try:
                with open(self.passwords_file, 'r', encoding='utf-8') as f:
                    data = json.load(f)

                for pwd_data in data.get('passwords', []):
                    entry = PasswordEntry(
                        password=pwd_data['password'],
                        source=pwd_data.get('source', 'manual'),
                        tags=pwd_data.get('tags', []),
                        usage_count=pwd_data.get('usage_count', 0),
                        success_count=pwd_data.get('success_count', 0),
                        last_used=datetime.fromisoformat(pwd_data['last_used'])
                        if pwd_data.get('last_used') else None,
                        created_at=datetime.fromisoformat(pwd_data['created_at']),
                        notes=pwd_data.get('notes', '')
                    )
                    pwd_hash = self._hash_password(entry.password)
                    self.passwords[pwd_hash] = entry

                logger.info(f"已加载 {len(self.passwords)} 个密码")
            except Exception as e:
                logger.error(f"加载密码本失败: {e}")

        # 加载缓存
        if os.path.exists(self.cache_file):
            try:
                with open(self.cache_file, 'r', encoding='utf-8') as f:
                    self.password_cache = json.load(f)
                logger.info(f"已加载 {len(self.password_cache)} 个缓存密码")
            except Exception as e:
                logger.error(f"加载缓存失败: {e}")

    def _save_data(self):
        """保存数据"""
        with self._lock:
            # 保存密码本
            passwords_data = []
            for entry in self.passwords.values():
                passwords_data.append({
                    'password': entry.password,
                    'source': entry.source,
                    'tags': entry.tags,
                    'usage_count': entry.usage_count,
                    'success_count': entry.success_count,
                    'last_used': entry.last_used.isoformat()
                    if entry.last_used else None,
                    'created_at': entry.created_at.isoformat(),
                    'notes': entry.notes
                })

            try:
                with open(self.passwords_file, 'w', encoding='utf-8') as f:
                    json.dump({'passwords': passwords_data}, f,
                             ensure_ascii=False, indent=2)
            except Exception as e:
                logger.error(f"保存密码本失败: {e}")

            # 保存缓存
            try:
                with open(self.cache_file, 'w', encoding='utf-8') as f:
                    json.dump(self.password_cache, f,
                             ensure_ascii=False, indent=2)
            except Exception as e:
                logger.error(f"保存缓存失败: {e}")

    def _hash_password(self, password: str) -> str:
        """计算密码哈希"""
        return hashlib.sha256(password.encode('utf-8')).hexdigest()

    def _hash_file(self, filepath: str) -> str:
        """计算文件哈希（用于缓存键）"""
        # 使用文件名和大小作为简单哈希
        stat = os.stat(filepath)
        key = f"{os.path.basename(filepath)}:{stat.st_size}"
        return hashlib.md5(key.encode('utf-8')).hexdigest()

    def add_password(self, password: str, source: str = 'manual',
                    tags: Optional[List[str]] = None,
                    notes: str = ""):
        """添加密码"""
        with self._lock:
            pwd_hash = self._hash_password(password)

            if pwd_hash in self.passwords:
                # 更新现有条目
                entry = self.passwords[pwd_hash]
                if tags:
                    # 合并标签
                    for tag in tags:
                        if tag not in entry.tags:
                            entry.tags.append(tag)
                if notes:
                    entry.notes = notes
            else:
                # 创建新条目
                entry = PasswordEntry(
                    password=password,
                    source=source,
                    tags=tags or [],
                    notes=notes
                )
                self.passwords[pwd_hash] = entry

            self._save_data()
            logger.info(f"添加密码: {password[:10]}... ({source})")

    def import_from_file(self, filepath: str, source: str = 'imported'):
        """从文件导入密码"""
        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                for line in f:
                    password = line.strip()
                    if password and not password.startswith('#'):
                        self.add_password(password, source)

            logger.info(f"从文件导入密码: {filepath}")
            return True
        except Exception as e:
            logger.error(f"导入密码文件失败: {e}")
            return False

    def get_passwords_for_file(self, filepath: str,
                              max_suggestions: int = 20) -> List[PasswordMatch]:
        """获取文件相关的密码建议"""
        suggestions = []

        # 1. 检查缓存
        file_hash = self._hash_file(filepath)
        if file_hash in self.password_cache:
            cached_password = self.password_cache[file_hash]
            suggestions.append(PasswordMatch(
                password=cached_password,
                confidence=1.0,
                source='cache',
                reason='之前成功解压过此文件'
            ))

        # 2. 基于文件名的密码
        filename = os.path.splitext(os.path.basename(filepath))[0]
        filename_variants = self._generate_filename_variants(filename)

        for variant in filename_variants:
            suggestions.append(PasswordMatch(
                password=variant,
                confidence=0.7,
                source='filename',
                reason=f'基于文件名: {filename}'
            ))

        # 3. 常用密码
        common_passwords = self._get_common_passwords()
        for pwd in common_passwords[:10]:
            suggestions.append(PasswordMatch(
                password=pwd,
                confidence=0.5,
                source='common',
                reason='常用密码'
            ))

        # 4. 从密码本中获取
        password_entries = list(self.passwords.values())

        # 按成功率和使用频率排序
        password_entries.sort(
            key=lambda x: (x.success_rate, x.usage_count),
            reverse=True
        )

        for entry in password_entries[:max_suggestions]:
            confidence = 0.3 + (entry.success_rate * 0.5)

            # 如果有相关标签，提高置信度
            if self._has_relevant_tags(entry.tags, filename):
                confidence += 0.2

            suggestions.append(PasswordMatch(
                password=entry.password,
                confidence=min(confidence, 1.0),
                source=entry.source,
                reason=f'来自密码本（成功率: {entry.success_rate:.1%}）'
            ))

        # 按置信度排序
        suggestions.sort(key=lambda x: x.confidence, reverse=True)

        # 去重
        seen = set()
        unique_suggestions = []
        for suggestion in suggestions:
            if suggestion.password not in seen:
                seen.add(suggestion.password)
                unique_suggestions.append(suggestion)

        return unique_suggestions[:max_suggestions]

    def _generate_filename_variants(self, filename: str) -> List[str]:
        """生成基于文件名的密码变体"""
        variants = []

        # 原始文件名
        variants.append(filename)

        # 小写
        variants.append(filename.lower())

        # 大写
        variants.append(filename.upper())

        # 首字母大写
        variants.append(filename.capitalize())

        # 常见后缀
        for year in ['2023', '2024', '2025', '2026']:
            variants.append(f"{filename}{year}")
            variants.append(f"{filename}_{year}")
            variants.append(f"{filename}.{year}")

        # 简单数字后缀
        for i in range(1, 10):
            variants.append(f"{filename}{i}")
            variants.append(f"{filename}_{i}")

        return variants

    def _get_common_passwords(self) -> List[str]:
        """获取常用密码列表"""
        return [
            '',  # 空密码
            '123456',
            'password',
            '12345678',
            'qwerty',
            '123456789',
            '12345',
            '1234',
            '111111',
            '1234567',
            'dragon',
            '123123',
            'baseball',
            'abc123',
            'football',
            'monkey',
            'letmein',
            'shadow',
            'master',
            '666666',
            'qwertyuiop',
            '123321',
            'mustang',
            '1234567890',
            'michael',
            '654321',
            'superman',
            '1qaz2wsx',
            '7777777',
            '121212',
            '000000',
            'qazwsx',
            '123qwe',
            'killer',
            'trustno1',
            'jordan',
            'jennifer',
            'zxcvbnm',
            'asdfgh',
            'hunter',
            'buster',
            'soccer',
            'harley',
            'batman',
            'andrew',
            'tigger',
            'sunshine',
            'iloveyou',
            '2000',
            'charlie',
            'robert',
            'thomas',
            'hockey',
            'ranger',
            'daniel',
            'starwars',
            'klaster',
            '112233',
            'george',
            'computer',
            'michelle',
            'jessica',
            'pepper',
            '1111',
            'zxcvbn',
            '555555',
            '11111111',
            '131313',
            'freedom',
            '777777',
            'pass',
            'maggie',
            '159753',
            'aaaaaa',
            'ginger',
            'princess',
            'joshua',
            'cheese',
            'amanda',
            'summer',
            'love',
            'ashley',
            'nicole',
            'chelsea',
            'biteme',
            'matthew',
            'access',
            'yankees',
            '987654321',
            'dallas',
            'austin',
            'thunder',
            'taylor',
            'matrix',
            'mobilemail',
            'mom',
            'monitor',
            'monitoring',
            'montana',
            'moon',
            'moscow'
        ]

    def _has_relevant_tags(self, tags: List[str], filename: str) -> bool:
        """检查是否有相关标签"""
        filename_lower = filename.lower()

        for tag in tags:
            tag_lower = tag.lower()
            if tag_lower in filename_lower or filename_lower in tag_lower:
                return True

        return False

    def record_success(self, filepath: str, password: str):
        """记录成功解压"""
        with self._lock:
            # 更新密码条目
            pwd_hash = self._hash_password(password)
            if pwd_hash in self.passwords:
                entry = self.passwords[pwd_hash]
                entry.record_usage(success=True)
            else:
                # 创建新条目
                entry = PasswordEntry(
                    password=password,
                    source='cached',
                    tags=[]
                )
                entry.record_usage(success=True)
                self.passwords[pwd_hash] = entry

            # 更新缓存
            file_hash = self._hash_file(filepath)
            self.password_cache[file_hash] = password

            self._save_data()
            logger.info(f"记录成功解压: {filepath}")

    def record_failure(self, password: str):
        """记录失败尝试"""
        with self._lock:
            pwd_hash = self._hash_password(password)
            if pwd_hash in self.passwords:
                entry = self.passwords[pwd_hash]
                entry.record_usage(success=False)
                self._save_data()

    def clear_old_cache(self, days: int = 30):
        """清理旧的缓存"""
        cutoff_date = datetime.now() - timedelta(days=days)

        with self._lock:
            # 清理密码条目
            to_remove = []
            for pwd_hash, entry in self.passwords.items():
                if (entry.last_used and entry.last_used < cutoff_date and
                    entry.source == 'cached' and entry.usage_count == 1):
                    to_remove.append(pwd_hash)

            for pwd_hash in to_remove:
                del self.passwords[pwd_hash]

            # 清理文件缓存（需要更复杂的逻辑，这里简化）
            if len(self.password_cache) > 1000:
                # 只保留最近使用的
                pass  # 实际实现需要记录使用时间

            if to_remove:
                self._save_data()
                logger.info(f"清理了 {len(to_remove)} 个旧缓存")

    def get_statistics(self) -> Dict:
        """获取统计信息"""
        total_passwords = len(self.passwords)
        total_usage = sum(e.usage_count for e in self.passwords.values())
        total_success = sum(e.success_count for e in self.passwords.values())

        return {
            'total_passwords': total_passwords,
            'total_usage': total_usage,
            'total_success': total_success,
            'cache_size': len(self.password_cache),
            'success_rate': total_success / total_usage if total_usage > 0 else 0
        }