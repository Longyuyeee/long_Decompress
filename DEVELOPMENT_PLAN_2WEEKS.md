# Long-Decompress 2周稳健发布计划

> **产品定位**: 华丽动效 × 全能格式 × 智能密码 × 批量自动化  
> **开始日期**: 2026-07-12  
> **目标发布**: 2026-07-26 (14天)

---

## 🎯 核心目标

### 差异化竞争力
1. **格式覆盖率最广** - 支持 13 种压缩格式 + 40+ 解压格式
2. **密码智能匹配** - 密码本自动尝试，优先级匹配
3. **批量工业化** - 配置组模板 + 任务队列自动化
4. **UI 体验领先** - 13 种主题 + 扁平动效 + 玻璃拟态

### 用户可定制性
- ✅ 所有配置参数可调整（格式、级别、密码策略、分卷、固实归档等）
- ✅ 主题系统可扩展（13 种内置主题 + 自定义色彩）
- ✅ 动画风格和强度可选（3 种风格 × 4 档强度）

---

## 📅 Week 1: 功能完整 + 安全加固

### Day 1-2 (7/12-7/13): 批量配置组系统 🔥

#### 后端开发
- [x] **数据模型** - `models/compression_profile.rs`
  - ✅ CompressionProfile 结构体（id, name, icon, description, config, auto_apply, password_strategy, stats）
  - ✅ CompressionConfig（format, level, password, split, solid_archive, filename_template）
  - ✅ AutoApplyRule（模式匹配、文件类型过滤、大小范围）
  - ✅ PasswordStrategy（None/Fixed/FromVault/AutoGenerate）
  - ✅ 5 个内置默认配置组

- [ ] **服务层** - `services/compression_profile_service.rs`
  - 配置组 CRUD（创建、读取、更新、删除）
  - 配置组排序（拖拽排序持久化）
  - 统计信息更新（使用次数、成功率、处理量）
  - 自动匹配逻辑（根据文件类型/大小推荐配置组）

- [ ] **持久化** - JSON 文件存储
  - 存储路径：`{APP_DATA}/profiles/profiles.json`
  - 初始化时加载默认配置组
  - 增量保存（避免全量覆盖）

- [ ] **Tauri 命令** - `commands/compression_profile.rs`
  ```rust
  #[tauri::command]
  async fn get_profiles() -> Result<Vec<CompressionProfile>, String>
  
  #[tauri::command]
  async fn create_profile(profile: CompressionProfile) -> Result<String, String>
  
  #[tauri::command]
  async fn update_profile(id: String, profile: CompressionProfile) -> Result<(), String>
  
  #[tauri::command]
  async fn delete_profile(id: String) -> Result<(), String>
  
  #[tauri::command]
  async fn reorder_profiles(ids: Vec<String>) -> Result<(), String>
  
  #[tauri::command]
  async fn apply_profile_to_task(profile_id: String, file_paths: Vec<String>) -> Result<(), String>
  
  #[tauri::command]
  async fn suggest_profile_for_file(file_path: String) -> Result<Option<CompressionProfile>, String>
  ```

#### 前端开发
- [ ] **Store 扩展** - `stores/profile.ts`
  - 配置组状态管理
  - 本地缓存 + 远程同步
  - 配置组排序状态

- [ ] **配置组选择器** - `components/profiles/ProfileSelector.vue`
  - 卡片网格布局（图标 + 名称 + 描述 + 统计）
  - 快捷应用按钮
  - 拖拽排序
  - 悬浮预览详细配置

- [ ] **配置组管理器** - `components/profiles/ProfileManager.vue`
  - 左侧列表（可搜索、可拖拽）
  - 右侧详细编辑面板
  - 所有参数可调整（格式、级别、密码策略、分卷、固实、文件名模板）
  - 自动应用规则配置

- [ ] **增强 CompressionSettingsPanel.vue**
  - 顶部添加"选择配置组"快捷按钮
  - 应用配置组后自动填充参数
  - "保存为新配置组"按钮

#### 视觉设计
- 配置组卡片：玻璃拟态 + 悬浮阴影
- 图标：支持 emoji 或自定义 SVG
- 统计徽章：使用次数、成功率
- 拖拽视觉反馈：半透明 + 虚线框

---

### Day 3 (7/14): 密码智能匹配增强 ⚠️

#### 后端优化
- [ ] **验证现有流程** - `services/password_attempt_service.rs`
  - 测试自动尝试是否完整触发
  - 检查优先级排序逻辑
  - 确认失败降级流程

- [ ] **优先级策略**
  - 按使用频率排序（最近成功的密码优先）
  - 按分类匹配（文件名关联的分类优先）
  - 按密码复杂度（简单 → 复杂）

- [ ] **密码规则生成器** - `services/password_rule_generator.rs`
  - 基于文件名的规则（filename + 后缀）
  - 基于日期的规则（YYYYMMDD 组合）
  - 常用模式库（123456, password123 等）

#### 前端集成
- [ ] **密码尝试进度显示** - `components/passwords/PasswordAttemptProgress.vue`
  - 实时显示："正在尝试密码本中的 3/12 个密码..."
  - 进度条 + 当前尝试的密码提示（脱敏显示）
  - 失败后提示手动输入

- [ ] **密码匹配策略选择** - 设置面板
  - 启用/禁用自动尝试
  - 最大尝试次数（默认 20）
  - 失败后行为（停止/继续/提示）

---

### Day 4 (7/15): 安全修复 - 密码暴露 🔴 P0

#### 代码审计
- [ ] **扫描所有 CLI 调用** - 搜索关键字 `-p`
  - `services/universal_engine.rs` - 7z 已修复（环境变量）
  - `services/rar_support.rs` - RAR/unrar 需要验证
  - `services/compression_service.rs` - 检查所有外部命令调用

- [ ] **修复方案**
  - 7z: 使用 `_7ZIP_PASSWORD` 环境变量 ✅
  - RAR: 优先使用原生 `unrar` crate，避免 CLI
  - WinRAR: 如果必须用 CLI，使用临时文件传密码（`-p@password.txt`）

#### 测试验证
- [ ] **进程列表测试**
  ```bash
  # 启动压缩任务后立即检查
  ps aux | grep -E '7z|rar|unrar|winrar'
  # 或 Windows
  Get-Process | Where-Object {$_.ProcessName -match '7z|rar|winrar'}
  ```

- [ ] **日志脱敏检查**
  - 确保所有日志中的密码已脱敏（`***`）
  - 检查错误堆栈中是否有密码泄露

---

### Day 5 (7/16): 密码库导入导出前端连接

#### 后端验证
- [x] `commands/encrypted_password.rs` - `export_passwords_command` 已实现
- [x] `commands/encrypted_password.rs` - `import_passwords_command` 已实现
- [x] `composables/useTauriCommands.ts` - `exportPasswords`/`importPasswords` 已封装

#### 前端补全
- [ ] **导出功能** - `views/PasswordVaultView.vue`
  - 导出格式选择（JSON 加密 / JSON 明文 / CSV）
  - 加密选项（密码保护导出文件）
  - 保存位置选择
  - 导出进度提示

- [ ] **导入功能**
  - 文件选择（支持拖拽）
  - 格式自动识别
  - 冲突处理策略：
    - 跳过已存在的密码
    - 覆盖已存在的密码
    - 合并（保留两者，重命名重复项）
  - 导入预览（显示将要导入的条目数）

- [ ] **导入导出模态框** - `components/passwords/ImportExportModal.vue`
  - 统一入口，选项卡切换导入/导出
  - 历史记录（最近导入导出的文件）

---

## 📅 Week 2: UI 华丽化 + 动效打磨

### Day 6 (7/17): 主题系统扩展 🎨

#### 新增 8 种主题
- [ ] **实现新主题** - `styles/design-tokens.css`
  ```css
  /* 6. Nord (极地冰原) */
  .mode-nord { --bg-main: #2e3440; --dynamic-accent: #88c0d0; }
  
  /* 7. Dracula (德古拉夜) */
  .mode-dracula { --bg-main: #282a36; --dynamic-accent: #bd93f9; }
  
  /* 8. Solarized Light (阳光护眼) */
  .mode-solarized-light { --bg-main: #fdf6e3; --dynamic-accent: #268bd2; }
  
  /* 9. Tokyo Night (东京之夜) */
  .mode-tokyo-night { --bg-main: #1a1b26; --dynamic-accent: #7aa2f7; }
  
  /* 10. Monokai (经典编辑器风) */
  .mode-monokai { --bg-main: #272822; --dynamic-accent: #a6e22e; }
  
  /* 11. Gruvbox (复古暖调) */
  .mode-gruvbox { --bg-main: #282828; --dynamic-accent: #fe8019; }
  
  /* 12. One Dark (Atom 风格) */
  .mode-one-dark { --bg-main: #282c34; --dynamic-accent: #61afef; }
  
  /* 13. Material Ocean (海洋蓝调) */
  .mode-material-ocean { --bg-main: #0f111a; --dynamic-accent: #82aaff; }
  ```

#### 主题选择器
- [ ] **华丽主题网格** - `components/settings/ThemeSelector.vue`
  - 4 列网格布局
  - 每个主题卡片显示：
    - 颜色预览条（背景/卡片/强调色）
    - 主题名称
    - 选中标记
  - 悬浮时放大（scale-105）
  - 点击时平滑过渡（0.7s）

- [ ] **"跟随系统"选项**
  - 检测系统主题（Windows/macOS）
  - 自动切换 light/dark

---

### Day 7 (7/18): 动画系统 🎬

#### 动画引擎
- [ ] **动画配置** - `utils/animations.ts`
  ```typescript
  // 3 种风格
  const ANIMATION_STYLES = {
    elegant: { easing: 'cubic-bezier(0.4, 0, 0.2, 1)', duration: 0.3 },
    bouncy: { easing: 'cubic-bezier(0.34, 1.56, 0.64, 1)', duration: 0.5 },
    snappy: { easing: 'cubic-bezier(0.25, 0.46, 0.45, 0.94)', duration: 0.15 },
  }
  
  // 4 档强度
  const ANIMATION_INTENSITY = {
    off: { duration: 0, effects: [] },
    reduced: { duration: 0.15, effects: ['fade'] },
    normal: { duration: 0.3, effects: ['fade', 'transform'] },
    enhanced: { duration: 0.5, effects: ['fade', 'transform', 'blur', 'glow'] },
  }
  ```

- [ ] **动画 Composable** - `composables/useAnimation.ts`
  - 读取用户设置（风格 + 强度）
  - 提供动画类名生成器
  - 自动降级（检测低配设备）

#### 核心动画
- [ ] **任务列表动画** - `components/tasks/AeroTable.vue`
  - 进入动画：stagger 错开（每项延迟 50ms）
  - 进度条：渐变流动（shimmer 效果）
  - 完成动画：绿色波纹扩散

- [ ] **模态框动画**
  - 弹出：从中心缩放 + 淡入
  - 关闭：缩小 + 淡出
  - 背景：模糊渐入

- [ ] **按钮微交互**
  - 悬浮：轻微上移 + 阴影加深
  - 按下：scale-95 + 阴影减弱
  - 点击：涟漪效果（ripple）

---

### Day 8-9 (7/19-7/20): 配置组管理界面 📋

#### 主界面开发
- [ ] **配置组管理模态框** - `components/profiles/ConfigurationProfilesModal.vue`
  - 全屏模态框（80% 视口）
  - 左侧：配置组列表（30% 宽度）
    - 可搜索（按名称/描述）
    - 可拖拽排序（vue-draggable-next）
    - 悬浮预览统计
  - 右侧：详细编辑面板（70% 宽度）
    - 基础信息（名称、图标、描述）
    - 压缩配置（所有参数可调）
    - 密码策略选择器
    - 自动应用规则
    - 快捷键绑定（可选）
  - 底部：保存/取消/删除按钮

#### 交互功能
- [ ] **拖拽应用**
  - 拖拽配置组到文件 = 立即应用
  - 拖拽配置组到任务列表 = 批量应用

- [ ] **快捷操作**
  - 双击配置组 = 应用到当前任务
  - 右键菜单：编辑/复制/删除/导出

- [ ] **统计面板**
  - 使用次数、成功率、处理量
  - 最近使用时间
  - 推荐徽章（高成功率）

---

### Day 10-11 (7/21-7/22): UI 组件美化 ✨

#### 格式选择器重构
- [ ] **华丽网格卡片** - `CompressionSettingsPanel.vue`
  - 从按钮 → 升级为卡片网格
  - 每个格式卡片：
    - 大图标（emoji 或 SVG）
    - 格式名称 + 扩展名
    - 能力标签（密码🔐/分卷✂️/固实📦）
    - 悬浮动效（旋转 + 放大）
    - 选中光晕（渐变背景 + 脉冲）

#### 任务列表增强
- [ ] **AeroTable 动效升级** - `components/tasks/AeroTable.vue`
  - 进入动画（从底部滑入）
  - 进度条渐变（彩虹流动）
  - 状态指示器动画：
    - 等待：灰色脉冲
    - 进行中：蓝色转圈 + shimmer
    - 完成：绿色对勾弹跳 + 波纹
    - 错误：红色抖动 + 闪烁

#### 文件拖放区美化
- [ ] **EnhancedFileDropzone 增强**
  - 拖入时：背景辉光效果（从边缘扩散）
  - 文件图标悬浮动画
  - 支持格式滚动提示（跑马灯）
  - 拖放预览（显示即将添加的文件数）

---

### Day 12 (7/23): 设置面板完善 ⚙️

#### 动画设置
- [ ] **动画控制面板** - `views/SettingsView.vue`
  - 风格选择器（3 个按钮：优雅/弹性/锐利）
  - 强度滑块（0-3，带标签）
  - 实时预览区域（点击触发动画演示）

#### 输入验证
- [ ] **表单验证** - `views/SettingsView.vue`
  - 线程数：1-32，默认 CPU 核心数
  - 缓存大小：100-10240 MB，默认 512
  - 默认压缩级别：0-9
  - 分卷大小：1-999999 MB

#### 快捷键配置
- [ ] **快捷键绑定** - 可选功能
  - 全局快捷键（压缩/解压）
  - 配置组快捷键（Ctrl+1~9）

---

### Day 13 (7/24): 全局微交互打磨 💎

#### 细节优化
- [ ] **输入框聚焦效果**
  - 边框发光（box-shadow: 0 0 0 3px primary/30%）
  - 内部光标渐入

- [ ] **勾选框动画**
  - 勾选时：对勾从无到有（路径动画）
  - 弹性动画（bouncy）

- [ ] **下拉菜单**
  - 展开：从顶部滑入 + 淡入
  - 收起：缩小 + 淡出

- [ ] **Tooltip 提示**
  - 延迟 500ms 显示
  - 箭头指向元素
  - 淡入淡出

#### 空状态设计
- [ ] **无任务时** - `views/CompressionView.vue`
  - 华丽插画（SVG 动画）
  - 快捷操作引导
  - 配置组快速入口

- [ ] **无密码时** - `views/PasswordVaultView.vue`
  - 引导添加第一个密码
  - 导入快捷入口

#### 加载状态
- [ ] **骨架屏** - `components/ui/Skeleton.vue`
  - 替代全局转圈圈
  - 保持布局稳定
  - shimmer 动画

---

### Day 14 (7/25): 全流程测试 + Bug 修复 🧪

#### 功能测试
- [ ] **压缩测试矩阵**
  - 20 个文件 × 13 种格式
  - 带密码/不带密码
  - 分卷/不分卷
  - 固实归档（7Z）

- [ ] **解压测试**
  - 50 个压缩包批量解压
  - 密码本自动匹配测试
  - 错误密码降级流程

- [ ] **配置组测试**
  - 创建/编辑/删除配置组
  - 批量应用到任务
  - 拖拽排序持久化
  - 统计信息准确性

#### 性能测试
- [ ] **内存占用**
  - 空闲时 < 200MB
  - 10 个并发任务时 < 500MB

- [ ] **动画性能**
  - 所有动画保持 60fps
  - 低配机器自动降级

#### UI 测试
- [ ] **主题切换**
  - 13 种主题无闪烁
  - 颜色过渡平滑（0.7s）

- [ ] **响应式布局**
  - 窗口缩放正常
  - 最小尺寸支持（800×600）

#### Bug 修复
- [ ] 修复发现的崩溃问题
- [ ] 修复 UI 显示异常
- [ ] 修复功能逻辑错误

---

## 📊 质量指标

### 代码质量
- [ ] Clippy 警告 < 50（当前 86）
- [ ] 所有 P0/P1 安全问题修复
- [ ] 核心测试覆盖率 > 80%

### 性能指标
- [ ] 启动时间 < 2s
- [ ] 压缩性能：ZIP-L6 约 50MB/s
- [ ] UI 响应 < 100ms（用户输入到界面更新）

### 用户体验
- [ ] 所有主要操作 < 3 步完成
- [ ] 错误提示清晰（附带解决方案）
- [ ] 无闪烁、无卡顿

---

## 🚀 发布检查清单

### P0 - 必须完成
- [ ] 安装器生成（WiX/DMG/deb/rpm）
- [ ] 密码不暴露在进程列表 ⚠️
- [ ] 核心功能测试通过（压缩/解压/密码/批量）
- [ ] 崩溃 Bug 清零

### P1 - 强烈建议
- [ ] 配置组系统完整可用
- [ ] 密码库导入导出连接
- [ ] 13 种主题实现
- [ ] 动画系统完整

### P2 - 可延后
- [ ] CI/CD 自动构建
- [ ] 11 个旧测试修复
- [ ] E2E 测试补充
- [ ] 英文文档

---

## 📝 技术债务

### 已知问题（不阻塞发布）
1. Clippy 警告（86 个，主要是代码风格）
2. 旧测试文件需要迁移（11 个）
3. 数据库无版本迁移系统
4. Toast 通知系统未统一

### 未来改进
1. 自动更新系统（签名密钥待配置）
2. 云端密码本同步
3. 插件系统（自定义格式支持）
4. AI 压缩参数推荐

---

**项目负责人**: Claude  
**最后更新**: 2026-07-12  
**版本**: v1.0.0-rc
