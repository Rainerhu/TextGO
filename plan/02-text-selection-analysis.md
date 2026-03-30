# 文本选择机制分析 & 绕开网站复制拦截方案

## 当前文本读取机制

### 两层读取策略

TextGO 使用两层策略读取选中文本（`selection.rs`）：

#### 第一层：平台原生 API（优先）

**Windows - UI Automation (UIA)**
- 通过 COM 接口创建 `IUIAutomation` 实例
- 调用 `GetFocusedElement()` 获取当前焦点元素
- 通过 `IUIAutomationTextPattern::GetSelection()` 直接读取选中文本
- 这是操作系统级别的无障碍 API，直接从 UI 元素中读取文本数据

**macOS - Accessibility API (AXAPI)**
- 通过 `AXUIElementCreateSystemWide()` 获取系统级无障碍元素
- 读取 `AXFocusedUIElement` 获取焦点元素
- 读取 `AXSelectedText` 属性获取选中文本

#### 第二层：剪贴板回退（备用）

当原生 API 失败时，回退到剪贴板方法：
1. 备份当前剪贴板内容
2. 清空剪贴板
3. 模拟发送 Ctrl+C（复制命令）
4. 轮询剪贴板变化（5ms 间隔，最长 1000ms）
5. 提取复制到的文本
6. 恢复原始剪贴板内容

## 关于网站禁止复制的拦截分析

### 网站常见的禁止复制手段

1. **CSS `user-select: none`** - 禁止文本选择
2. **JavaScript `copy` 事件拦截** - 监听 copy 事件并 `preventDefault()`
3. **JavaScript `selectstart` 事件拦截** - 阻止选择开始
4. **右键菜单禁用** - `contextmenu` 事件拦截
5. **覆盖层遮挡** - 透明 div 覆盖在文本上方

### 当前机制能否绕开？

#### 第一层（原生 API）的绕开能力 ✅

**Windows UIA 方案：可以绕开大部分网站限制**
- UIA 是操作系统级别的无障碍 API，直接从浏览器的 UI 元素树中读取文本
- 它不经过 JavaScript 层，不受 `copy` 事件拦截影响
- 不受 CSS `user-select: none` 影响（UIA 读取的是 DOM 元素的文本内容）
- 但前提是浏览器暴露了 `IUIAutomationTextPattern` 接口

**macOS AXAPI 方案：可以绕开大部分网站限制**
- 同理，AXAPI 直接从浏览器的无障碍树读取 `AXSelectedText`
- 不经过 JavaScript 层

**局限性：**
- 如果用户无法在页面上选中文本（CSS `user-select: none` 阻止了鼠标选择），
  那么即使 UIA/AXAPI 能读取文本，也没有"选中的文本"可读
- 某些复杂的 Web 应用可能不完整暴露无障碍接口

#### 第二层（剪贴板回退）的绕开能力 ❌

- 模拟 Ctrl+C 会触发浏览器的 `copy` 事件
- 如果网站拦截了 `copy` 事件，这个方法会失败
- 如果网站使用 `user-select: none`，用户根本无法选中文本

### 核心问题

真正的问题不在于"读取"选中文本，而在于"选中"文本本身：
- 如果网站通过 CSS `user-select: none` 阻止了文本选择，用户鼠标拖动时不会产生选区
- TextGO 的鼠标事件检测依赖 I-Beam 光标检测，如果网站禁止选择，光标不会变成 I-Beam

## 解决方案

### 方案 A：利用原生 API 直接读取光标位置的文本（复杂度高）

通过 UIA/AXAPI 获取鼠标位置下的 UI 元素，直接读取其全部文本内容，而不依赖"选中"操作。
- 优点：完全绕开网站限制
- 缺点：无法精确获取用户想要的文本片段，只能获取整个元素的文本

### 方案 B：增加"延迟读取"开关（推荐，实用性高）

**核心思路：** 将"鼠标选择时自动读取文本"改为可选行为。

当开关关闭时：
- 鼠标选择文本时，不自动触发 `get_selection`，不模拟 Ctrl+C
- 用户手动选择对应功能（如点击工具栏按钮）后，才读取选中文本
- 这样可以避免在禁止复制的网站上触发检测

当开关开启时（默认）：
- 保持当前行为，鼠标选择后自动读取文本

**实现方式：**
- 在 `emit_event` 函数中，当检测到鼠标触发时，传递 `skip_selection: true`
- 前端收到事件后，显示工具栏但不立即读取文本
- 用户点击工具栏上的操作按钮时，才调用 `get_selection` 读取文本

### 方案 C：混合方案（推荐实现）

结合方案 A 和 B 的优点：

1. **优先使用原生 API**（当前已实现）- 这本身就能绕开大部分网站的 JS 层拦截
2. **增加"延迟读取"开关** - 对于原生 API 也无法处理的情况，提供手动触发选项
3. **I-Beam 光标检测可关闭**（当前已实现）- 关闭后即使网站改变了光标样式也能触发

## 结论

当前 TextGO 的第一层（原生 API）已经能绕开大部分网站的 JavaScript 层复制拦截，
因为 UIA/AXAPI 不经过 JavaScript 层。

真正的问题场景是：
1. 网站使用 `user-select: none` 阻止了文本选择本身
2. 剪贴板回退方案会触发网站的 `copy` 事件拦截

**推荐实现方案 B（延迟读取开关）**，因为：
- 实现简单，改动小
- 用户可以先选中文本，再手动触发读取，避免自动 Ctrl+C 被网站检测
- 对于 `user-select: none` 的情况，用户可以配合浏览器扩展解除限制后使用
