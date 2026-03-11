# 全局数据字典 (Global Data Dictionary)

## 1. 全局枚举定义 (Enums)
| 枚举名称 (Enum Name) | 类型 (Type) | 枚举值 (Value) | 说明 (Definition) |
| :--- | :--- | :--- | :--- |
| `ItemType` | `uint8` | `1: Weapon, 2: Consumable, 3: Material` | 物品分类 |
| `CombatState` | `uint8` | `0: Idle, 1: Attack, 2: Defend` | 战斗状态 |

## 2. 单位规范 (Unit Standards)
- **数值单位**: [如: 毫米 (mm), 毫秒 (ms), 弧度 (rad)]
- **计算精度**: [如: 小数点后 4 位, 浮点数/定点数]

## 3. 错误码定义 (Error Codes)
| 错误码 | 错误描述 | 程序处理建议 | UI 反馈建议 |
| :--- | :--- | :--- | :--- |
| `E001` | `RESOURCE_INSUFFICIENT` | 停止当前操作 | 弹出浮窗提示“资源不足” |