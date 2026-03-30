# 剪贴板回退方案绕开网站检测分析

## 问题本质

当 Windows UIA (`IUIAutomationTextPattern::GetSelection`) 或 macOS AXAPI (`AXSelectedText`) 
无法获取选中文本时，TextGO 回退到"模拟复制 + 读取剪贴板"方案。

这个回退方案会触发浏览器的 `copy` 事件，被网站的 JavaScript 拦截。

## 技术分析：为什么无法完全绕开

### 浏览器的 copy 事件触发机制

浏览器（Chrome/Firefox/Edge）的 `copy` 事件触发链：
```
用户按键 / 系统模拟按键
    ↓
操作系统将按键事件发送到浏览器窗口
    ↓
浏览器内核识别为"复制"操作（无论是 Ctrl+C 还是 Ctrl+Insert）
    ↓
触发 JavaScript 的 copy 事件
    ↓
如果网站调用了 event.preventDefault()，复制被阻止
```

关键点：**浏览器不区分 Ctrl+C 和 Ctrl+Insert**，两者都会触发相同的 `copy` 事件。
当前代码在 Windows 上用 `Ctrl+Insert` 而非 `Ctrl+C`，但这并不能绕开检测。

### 已尝试/已知的所有读取选中文本的方式

| 方式 | 是否绕开 JS 检测 | 对浏览器是否有效 | 说明 |
|------|-----------------|-----------------|------|
| UIA TextPattern | ✅ 完全绕开 | ⚠️ 部分有效 | Chrome/Edge 支持较好，但某些页面元素不暴露 TextPattern |
| macOS AXAPI | ✅ 完全绕开 | ⚠️ 部分有效 | 需要启用 AXEnhancedUserInterface |
| Win32 WM_GETTEXT + EM_GETSEL | ✅ 完全绕开 | ❌ 不适用 | 只对原生 Win32 控件有效，浏览器不是原生控件 |
| 模拟 Ctrl+C / Ctrl+Insert | ❌ 触发 copy 事件 | ✅ 有效 | 当前回退方案 |
| WM_COPY 消息 | ❌ 触发 copy 事件 | ✅ 有效 | 等效于 Ctrl+C |
| IAccessible / MSAA | ✅ 完全绕开 | ❌ 不适用 | 浏览器不通过 MSAA 暴露选中文本 |
| IAccessible2 | ✅ 完全绕开 | ⚠️ Firefox 支持 | Chrome 不支持 IA2 |

### 结论

**对于浏览器场景，当 UIA/AXAPI 失败时，没有已知的方式可以在不触发 JS copy 事件的情况下读取选中文本。**

这是因为浏览器的渲染引擎是一个独立的沙箱环境，外部进程无法直接访问其 DOM 状态。
UIA/AXAPI 能工作是因为浏览器主动将无障碍信息暴露给操作系统，但这个暴露是有限的。

## 已实施的改进方案

### 方案 1：提高 UIA 原生 API 的成功率 ✅ 已实现

增强 Windows UIA 的 `get_selection` 实现：
- 当焦点元素的 TextPattern 失败时，从前台窗口根元素搜索 Document 控件
- 在 Document 控件上尝试获取 TextPattern 和选中文本
- 这覆盖了浏览器中焦点元素不直接暴露 TextPattern 但 Document 控件暴露的场景

改动文件：`src-tauri/src/platform/windows.rs`

### 方案 2：增加"仅使用原生 API"选项 ✅ 已实现

新增 `nativeSelectionOnly` 设置，开启后：
- 仅通过 UIA/AXAPI 读取选中文本
- 当原生 API 失败时直接返回空字符串
- **完全不模拟任何复制按键**，彻底避免触发网站检测

改动文件：
- `src-tauri/src/lib.rs` — 新增 `NATIVE_SELECTION_ONLY` 全局状态
- `src-tauri/src/commands/shortcut.rs` — 新增 `set_native_selection_only` 命令
- `src-tauri/src/commands/selection.rs` — `get_selection` 中检查标志位
- `src/lib/stores.svelte.ts` — 新增 `nativeSelectionOnly` 持久化状态
- `src/routes/(main)/settings/mouse/+page.svelte` — 新增开关 UI
- `messages/en.json` / `messages/zh-CN.json` — 新增翻译

### 推荐使用方式

对于需要在禁止复制的网站上使用 TextGO 的用户：

1. 开启"仅使用原生 API"— 完全避免触发 copy 事件
2. 如果原生 API 在某些场景下无法获取文本，配合"延迟读取"使用
3. 在大多数现代浏览器（Chrome/Edge）中，UIA 的 TextPattern 对网页内容支持良好
