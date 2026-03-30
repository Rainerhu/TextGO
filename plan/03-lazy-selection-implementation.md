# 延迟读取选中文本功能实现记录

## 功能说明

新增"延迟读取选中文本"开关（Deferred Selection Reading / 延迟读取选中文本），
开启后鼠标选中文本时不会立即读取文本内容（不触发 Ctrl+C 或原生 API 读取），
而是等到用户点击工具栏上的操作按钮后才读取，从而避免在禁止复制的网站上触发检测。

## 改动文件

### 后端 (Rust)

| 文件 | 改动 |
|------|------|
| `src-tauri/src/lib.rs` | 新增全局状态 `LAZY_SELECTION: AtomicBool`；注册新命令 `set_lazy_selection_enabled` |
| `src-tauri/src/commands/shortcut.rs` | 新增 `set_lazy_selection_enabled` 命令函数 |
| `src-tauri/src/handlers/mouse.rs` | `emit_event` 函数中，当 `LAZY_SELECTION` 为 true 时跳过文本读取，直接发送空 selection 事件 |

### 前端 (TypeScript/Svelte)

| 文件 | 改动 |
|------|------|
| `src/lib/stores.svelte.ts` | 新增 `lazySelection` 持久化状态，变更时同步到后端 |
| `src/lib/shortcut.ts` | `handleShortcutEvent` 中处理 lazy 模式：toolbar 模式下跳过文本匹配直接显示所有规则；quiet 模式下先获取文本再执行 |
| `src/routes/toolbar/+page.svelte` | 新增 `isLazy` 状态；`executeAction` 中延迟获取选中文本 |
| `src/routes/(main)/settings/mouse/+page.svelte` | 新增"延迟读取选中文本"开关 UI |

### 国际化

| 文件 | 改动 |
|------|------|
| `messages/en.json` | 新增 `lazy_selection_settings`、`lazy_selection_enabled`、`lazy_selection_explain` |
| `messages/zh-CN.json` | 新增对应中文翻译 |

## 工作流程

### 开关关闭（默认行为，不变）
```
鼠标选中文本 → 自动读取选中文本 → 发送事件(含文本) → 规则匹配 → 显示工具栏/执行动作
```

### 开关开启（延迟读取）
```
鼠标选中文本 → 发送事件(空文本) → 显示工具栏(所有规则) → 用户点击按钮 → 读取选中文本 → 执行动作
```

## 绕开检测的原理

1. 鼠标选中文本时不触发任何读取操作（不模拟 Ctrl+C，不调用 UIA/AXAPI）
2. 网站的 `copy` 事件监听器不会被触发
3. 用户点击工具栏按钮时才读取，此时优先使用原生 API（UIA/AXAPI），不经过 JavaScript 层
4. 即使回退到剪贴板方法，也是在用户主动操作后才触发，时间上与自动触发有明显区别
