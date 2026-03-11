# Game Planner (金牌游戏策划)

这是一个专门为您设计的、具备深度调研与批判性思维的游戏策划技能。它不仅是一个文档生成器，更是一个能为您提供灵感、对标市场、并指出设计漏洞的合作伙伴。

## 核心功能 (Core Features)
- **对话式引导 (Dialogic Guidance)**：通过深度的反向提问，帮助您从模糊的想法中提炼出核心玩法。
- **市场调研 (Market Research)**：主动使用 `google_web_search` 和 `web_fetch` 调研同赛道的成功作品（如《哈迪斯》、《塞尔达》、《杀戮尖塔》等）。
- **专业模板 (Professional Templates)**：提供从核心概念 (GDD) 到战斗、系统、经济、叙事的完整专业模板。
- **批判性纠偏 (Critical Feedback)**：作为一个资深制作人，它会主动指出您设计中的数值通胀、流程冗余等风险。

## 如何使用 (How to Use)
1. **激活技能**：在对话中输入 `/skills reload` 刷新技能列表（如果是新安装）。
2. **启动对话**：
   - `gemini --skill game-planner "我想做一个赛博朋克风格的类银河恶魔城游戏"`
   - `gemini --skill game-planner "帮我分析一下最近很火的《幻兽帕鲁》，看看我们可以借鉴什么？"`
3. **输出文档**：
   - 它会自动根据 `templates/` 下的格式生成 Markdown 文档。

## 文档结构
- `01_concept.md`: 核心概念文档 (GDD 0.1)
- `02_system.md`: 通用系统设计
- `03_combat.md`: 战斗系统详述
- `04_economy.md`: 数值与经济系统
- `05_narrative.md`: 叙事与关卡设计
- `06_market.md`: 市场调研报告

## 开发者建议 (Pro-Tip)
当您觉得想法卡住时，直接对它说：“作为一个制作人，你觉得这个玩法哪里最无聊？给我三个改进建议。”