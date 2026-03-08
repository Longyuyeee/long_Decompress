#!/usr/bin/env python3
"""
密码保险库原型实现
演示密码本管理系统的核心功能
"""

import os
import json
import sqlite3
import hashlib
import base64
from datetime import datetime
from typing import List, Dict, Optional, Tuple
from dataclasses import dataclass, asdict, field
import uuid

@dataclass
class PasswordRecord:
    """密码记录"""
    id: str = field(default_factory=lambda: str(uuid.uuid4()))
    filename: str = ""
    encrypted_password: str = ""
    file_hash: Optional[str] = None
    tags: List[str] = field(default_factory=list)
    category: Optional[str] = None
    strength: int = 3  # 1-5
    usage_count: int = 0
    success_count: int = 0
    last_used: Optional[datetime] = None
    created_at: datetime = field(default_factory=datetime.now)
    source: str = "manual"
    notes: Optional[str] = None

class SimpleCrypto:
    """简单加密实现（演示用，生产环境应使用更安全的实现）"""

    @staticmethod
    def encrypt(plaintext: str, key: str) -> str:
        """简单加密"""
        # 注意：这只是演示，生产环境应使用AES等加密算法
        import hashlib
        key_hash = hashlib.sha256(key.encode()).digest()

        # 简单XOR加密（仅用于演示）
        encrypted = []
        for i, char in enumerate(plaintext):
            key_char = key_hash[i % len(key_hash)]
            encrypted_char = chr(ord(char) ^ key_char)
            encrypted.append(encrypted_char)

        # Base64编码
        encrypted_bytes = ''.join(encrypted).encode('latin-1')
        return base64.b64encode(encrypted_bytes).decode('utf-8')

    @staticmethod
    def decrypt(encrypted_text: str, key: str) -> str:
        """解密"""
        # Base64解码
        encrypted_bytes = base64.b64decode(encrypted_text.encode('utf-8'))
        encrypted = encrypted_bytes.decode('latin-1')

        key_hash = hashlib.sha256(key.encode()).digest()

        # 解密
        decrypted = []
        for i, char in enumerate(encrypted):
            key_char = key_hash[i % len(key_hash)]
            decrypted_char = chr(ord(char) ^ key_char)
            decrypted.append(decrypted_char)

        return ''.join(decrypted)

class PasswordVault:
    """密码保险库"""

    def __init__(self, db_path: str = "passwords.db", master_password: str = None):
        self.db_path = db_path
        self.master_password = master_password or "default_password"
        self.connection = None
        self.crypto = SimpleCrypto()

    def connect(self):
        """连接数据库"""
        self.connection = sqlite3.connect(self.db_path)
        self._create_tables()

    def _create_tables(self):
        """创建数据库表"""
        cursor = self.connection.cursor()

        # 密码表
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS passwords (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                encrypted_password TEXT NOT NULL,
                file_hash TEXT,
                tags_json TEXT DEFAULT '[]',
                category TEXT,
                strength INTEGER DEFAULT 3,
                usage_count INTEGER DEFAULT 0,
                success_count INTEGER DEFAULT 0,
                last_used TIMESTAMP,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                source TEXT DEFAULT 'manual',
                notes TEXT
            )
        """)

        # 使用历史表
        cursor.execute("""
            CREATE TABLE IF NOT EXISTS usage_history (
                id TEXT PRIMARY KEY,
                password_id TEXT,
                filename TEXT NOT NULL,
                success BOOLEAN NOT NULL,
                timestamp TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
                FOREIGN KEY (password_id) REFERENCES passwords(id) ON DELETE CASCADE
            )
        """)

        # 创建索引
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_filename ON passwords(filename)")
        cursor.execute("CREATE INDEX IF NOT EXISTS idx_filehash ON passwords(file_hash)")

        self.connection.commit()

    def add_password(self, filename: str, plain_password: str, **kwargs) -> str:
        """添加密码"""
        # 加密密码
        encrypted = self.crypto.encrypt(plain_password, self.master_password)

        # 计算文件哈希（可选）
        file_hash = self._calculate_file_hash(filename) if kwargs.get('calculate_hash', True) else None

        # 创建记录
        record = PasswordRecord(
            filename=filename,
            encrypted_password=encrypted,
            file_hash=file_hash,
            tags=kwargs.get('tags', []),
            category=kwargs.get('category'),
            strength=kwargs.get('strength', 3),
            notes=kwargs.get('notes')
        )

        # 保存到数据库
        cursor = self.connection.cursor()
        cursor.execute("""
            INSERT INTO passwords (
                id, filename, encrypted_password, file_hash, tags_json,
                category, strength, notes
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        """, (
            record.id,
            record.filename,
            record.encrypted_password,
            record.file_hash,
            json.dumps(record.tags),
            record.category,
            record.strength,
            record.notes
        ))

        self.connection.commit()
        return record.id

    def find_passwords(self, filename: str, limit: int = 10) -> List[Dict]:
        """查找密码"""
        cursor = self.connection.cursor()

        # 精确匹配
        cursor.execute("""
            SELECT * FROM passwords
            WHERE filename = ?
            ORDER BY usage_count DESC, last_used DESC
            LIMIT ?
        """, (filename, limit))

        exact_matches = self._rows_to_dicts(cursor)

        # 模糊匹配（如果精确匹配不够）
        if len(exact_matches) < limit:
            remaining = limit - len(exact_matches)
            cursor.execute("""
                SELECT * FROM passwords
                WHERE filename LIKE ?
                AND filename != ?
                ORDER BY usage_count DESC, last_used DESC
                LIMIT ?
            """, (f"%{filename}%", filename, remaining))

            fuzzy_matches = self._rows_to_dicts(cursor)
            exact_matches.extend(fuzzy_matches)

        # 解密密码并计算相似度
        for match in exact_matches:
            try:
                match['password'] = self.crypto.decrypt(match['encrypted_password'], self.master_password)
            except:
                match['password'] = "[解密失败]"

            # 计算相似度
            match['similarity'] = self._calculate_similarity(filename, match['filename'])

            # 计算优先级
            match['priority'] = self._calculate_priority(match)

        # 按优先级排序
        exact_matches.sort(key=lambda x: x['priority'], reverse=True)
        return exact_matches[:limit]

    def record_usage(self, password_id: str, filename: str, success: bool):
        """记录使用历史"""
        cursor = self.connection.cursor()

        # 记录使用历史
        cursor.execute("""
            INSERT INTO usage_history (id, password_id, filename, success)
            VALUES (?, ?, ?, ?)
        """, (str(uuid.uuid4()), password_id, filename, success))

        # 更新密码统计
        cursor.execute("""
            UPDATE passwords
            SET usage_count = usage_count + 1,
                success_count = success_count + ?,
                last_used = ?
            WHERE id = ?
        """, (1 if success else 0, datetime.now().isoformat(), password_id))

        self.connection.commit()

    def generate_password_suggestions(self, filename: str, count: int = 5) -> List[str]:
        """生成密码建议"""
        suggestions = []

        # 提取文件名中的关键词
        keywords = self._extract_keywords(filename)

        # 常见密码模式
        patterns = [
            "{keyword}123",
            "{keyword}@2024",
            "{keyword}#123",
            "{keyword}_2024",
            "pass_{keyword}",
            "{keyword}!"
        ]

        # 生成建议
        for keyword in keywords[:3]:  # 最多使用3个关键词
            for pattern in patterns:
                suggestion = pattern.format(keyword=keyword.lower())
                suggestions.append(suggestion)

                # 添加变体
                suggestions.append(suggestion.upper())
                suggestions.append(keyword.capitalize() + "123")

                if len(suggestions) >= count * 2:  # 生成多一些供筛选
                    break
            if len(suggestions) >= count * 2:
                break

        # 去重并返回
        unique_suggestions = list(set(suggestions))
        return unique_suggestions[:count]

    def import_from_text(self, filepath: str, delimiter: str = ":") -> Dict:
        """从文本文件导入密码"""
        results = {
            'success': 0,
            'failed': 0,
            'duplicates': 0
        }

        try:
            with open(filepath, 'r', encoding='utf-8') as f:
                for line_num, line in enumerate(f, 1):
                    line = line.strip()

                    # 跳过空行和注释
                    if not line or line.startswith('#'):
                        continue

                    # 解析行
                    if delimiter in line:
                        parts = line.split(delimiter, 1)
                        if len(parts) == 2:
                            filename = parts[0].strip()
                            password = parts[1].strip()

                            # 检查是否已存在
                            cursor = self.connection.cursor()
                            cursor.execute("SELECT COUNT(*) FROM passwords WHERE filename = ?", (filename,))
                            exists = cursor.fetchone()[0] > 0

                            if not exists:
                                try:
                                    self.add_password(filename, password)
                                    results['success'] += 1
                                except Exception as e:
                                    print(f"第{line_num}行导入失败: {e}")
                                    results['failed'] += 1
                            else:
                                results['duplicates'] += 1
                        else:
                            results['failed'] += 1
                    else:
                        results['failed'] += 1

        except Exception as e:
            print(f"导入文件失败: {e}")

        return results

    def export_to_text(self, filepath: str):
        """导出密码到文本文件"""
        cursor = self.connection.cursor()
        cursor.execute("SELECT filename, encrypted_password FROM passwords")
        rows = cursor.fetchall()

        with open(filepath, 'w', encoding='utf-8') as f:
            f.write("# 密码本导出\n")
            f.write(f"# 导出时间: {datetime.now().isoformat()}\n")
            f.write(f"# 记录数量: {len(rows)}\n\n")

            for filename, encrypted in rows:
                try:
                    password = self.crypto.decrypt(encrypted, self.master_password)
                    f.write(f"{filename}: {password}\n")
                except:
                    f.write(f"{filename}: [解密失败]\n")

    def get_statistics(self) -> Dict:
        """获取统计信息"""
        cursor = self.connection.cursor()

        # 基本统计
        cursor.execute("SELECT COUNT(*) FROM passwords")
        total_passwords = cursor.fetchone()[0]

        cursor.execute("SELECT COUNT(DISTINCT category) FROM passwords WHERE category IS NOT NULL")
        categories_count = cursor.fetchone()[0]

        cursor.execute("SELECT SUM(usage_count), SUM(success_count) FROM passwords")
        usage_stats = cursor.fetchone()
        total_usage = usage_stats[0] or 0
        total_success = usage_stats[1] or 0

        success_rate = total_success / total_usage if total_usage > 0 else 0

        # 按强度统计
        cursor.execute("""
            SELECT strength, COUNT(*)
            FROM passwords
            GROUP BY strength
            ORDER BY strength
        """)
        strength_stats = dict(cursor.fetchall())

        # 按类别统计
        cursor.execute("""
            SELECT category, COUNT(*)
            FROM passwords
            WHERE category IS NOT NULL
            GROUP BY category
            ORDER BY COUNT(*) DESC
            LIMIT 10
        """)
        category_stats = dict(cursor.fetchall())

        return {
            'total_passwords': total_passwords,
            'categories_count': categories_count,
            'total_usage': total_usage,
            'total_success': total_success,
            'success_rate': success_rate,
            'strength_distribution': strength_stats,
            'top_categories': category_stats
        }

    def _calculate_file_hash(self, filename: str) -> str:
        """计算文件名的哈希值（简化版）"""
        return hashlib.md5(filename.encode()).hexdigest()

    def _calculate_similarity(self, str1: str, str2: str) -> float:
        """计算字符串相似度（简化版）"""
        from difflib import SequenceMatcher
        return SequenceMatcher(None, str1.lower(), str2.lower()).ratio()

    def _calculate_priority(self, record: Dict) -> float:
        """计算优先级"""
        priority = 0.0

        # 使用频率权重
        usage_score = min(record.get('usage_count', 0) / 100, 1.0) * 0.4
        priority += usage_score

        # 成功率权重
        success_rate = record.get('success_count', 0) / max(record.get('usage_count', 1), 1)
        success_score = success_rate * 0.3
        priority += success_score

        # 相似度权重
        similarity = record.get('similarity', 0)
        priority += similarity * 0.3

        return priority

    def _extract_keywords(self, filename: str) -> List[str]:
        """提取关键词"""
        import re

        # 移除扩展名
        name_without_ext = filename.rsplit('.', 1)[0] if '.' in filename else filename

        # 分割单词
        words = re.split(r'[_\-\s\.]+', name_without_ext)

        # 过滤短单词
        keywords = [word.lower() for word in words if len(word) > 2]

        return keywords

    def _rows_to_dicts(self, cursor) -> List[Dict]:
        """将查询结果转换为字典列表"""
        columns = [column[0] for column in cursor.description]
        result = []

        for row in cursor.fetchall():
            row_dict = dict(zip(columns, row))

            # 解析JSON字段
            if 'tags_json' in row_dict and row_dict['tags_json']:
                row_dict['tags'] = json.loads(row_dict.pop('tags_json'))
            else:
                row_dict['tags'] = []

            result.append(row_dict)

        return result

    def close(self):
        """关闭数据库连接"""
        if self.connection:
            self.connection.close()

def demo():
    """演示功能"""
    print("=== 密码保险库演示 ===\n")

    # 创建保险库
    vault = PasswordVault("demo_passwords.db", "demo_master_password")
    vault.connect()

    print("1. 添加示例密码...")
    vault.add_password("game_patch_2024.zip", "Game@2024!",
                      tags=["游戏", "补丁"], category="游戏")
    vault.add_password("work_document_backup.rar", "Company123#",
                      tags=["工作", "文档"], category="工作")
    vault.add_password("photos_vacation_2023.7z", "Family2023!",
                      tags=["个人", "照片"], category="个人")
    vault.add_password("ebook_programming_python.zip", "LearnPython2024",
                      tags=["学习", "电子书"], category="学习")

    print("2. 查找密码...")
    matches = vault.find_passwords("game_patch", limit=5)
    print(f"找到 {len(matches)} 个匹配:")
    for i, match in enumerate(matches, 1):
        print(f"  {i}. {match['filename']} -> {match['password']} "
              f"(相似度: {match['similarity']:.1%}, 优先级: {match['priority']:.2f})")

    print("\n3. 生成密码建议...")
    suggestions = vault.generate_password_suggestions("document_backup_20240308.zip", count=5)
    print("密码建议:")
    for i, suggestion in enumerate(suggestions, 1):
        print(f"  {i}. {suggestion}")

    print("\n4. 记录使用历史...")
    if matches:
        vault.record_usage(matches[0]['id'], "game_patch_2024.zip", True)
        print("使用记录已保存")

    print("\n5. 获取统计信息...")
    stats = vault.get_statistics()
    print(f"总密码数: {stats['total_passwords']}")
    print(f"总使用次数: {stats['total_usage']}")
    print(f"成功率: {stats['success_rate']:.1%}")
    print(f"强度分布: {stats['strength_distribution']}")

    print("\n6. 导出密码...")
    vault.export_to_text("exported_passwords.txt")
    print("密码已导出到 exported_passwords.txt")

    # 清理
    vault.close()

    # 删除演示文件
    import os
    if os.path.exists("demo_passwords.db"):
        os.remove("demo_passwords.db")

    print("\n=== 演示完成 ===")

if __name__ == "__main__":
    demo()