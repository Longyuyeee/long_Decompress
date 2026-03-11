# Game Data Architect (游戏数据架构师)

这是一个专门为您设计的、面向技术落地的游戏系统策划与数据建模技能。它是 `game-planner` 的黄金搭档，负责将“好玩的想法”转化为“严谨的代码与数据”。

## 核心功能 (Core Features)
- **技术栈自适应 (Language-Aware)**：根据您选择的编程语言（C#, C++, TS, Python...）自动调整数据类型和命名规范。
- **结构化数据建模 (Data Modeling)**：提供 JSON、Markdown 表格、以及基于选定语言的类/结构体定义。
- **关系图谱可视化 (System Relationship)**：使用 Mermaid.js 绘制系统拓扑图，揭示数据流转与耦合关系。
- **对抗式风险审计 (Adversarial Audit)**：主动寻找数据的边界情况、数值溢出风险、以及 UI 显示的极端情况。

## 如何配合使用 (How to Use)
1. **获取主案**：首先由 `game-planner` 产出核心玩法或系统策划草案。
2. **启动数据化**：
   - `gemini --skill game-data-architect "这是刚才生成的战斗系统策划案，请帮我进行数据建模"`
3. **环境确认**：它会先问您：“您计划使用什么编程语言和存储格式？”
4. **输出详细文档**：
   - 它会自动生成 `01_system_data.md` (包含代码、表格、JSON 假数据)。
   - 它会绘制 `04_logic_flow.md` (系统耦合图)。

## 文档结构
- `01_system_data.md`: [单系统] 数据定义文档 (含逻辑、类型、字段说明)
- `02_data_dictionary.md`: 全局数据字典 (枚举值、单位定义)
- `03_ui_spec.md`: UI 交互数据需求表
- `04_logic_flow.md`: Mermaid 逻辑流转与系统耦合图
- `05_mock_data.json`: 假数据 (Mock Data) 示例模板

## 架构师的质问 (Sample Questions)
“如果玩家在网络延迟 500ms 的情况下触发了‘反击’，你的数据包是否包含时间戳校准字段？”