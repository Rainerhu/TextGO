# TextGO 项目架构分析

## 项目概述

TextGO 是一个跨平台（macOS/Windows）桌面文本处理工具，基于 Tauri v2 + Svelte 5 构建。
核心功能：检测用户在任意应用中选中的文本，根据可配置的规则自动执行相应操作（文本转换、脚本执行、AI 对话、网页搜索等）。

## 技术栈

- **前端**: Svelte 5 + TypeScript + TailwindCSS + CodeMirror
- **后端**: Rust (Tauri 2.10.3)
- **平台 API**: Windows UI Automation / macOS Accessibility API
- **全局事件监听**: rdev (鼠标/键盘事件)
- **键盘模拟**: enigo
- **剪贴板**: clipboard-rs
- **LLM 集成**: OpenAI, Anthropic, Google Gemini, Ollama, LM Studio, OpenRouter, XAI

## 核心架构

```
全局鼠标/键盘事件 (rdev)
    ↓
鼠标处理器 (handlers/mouse.rs) / 键盘处理器 (handlers/keyboard.rs)
    ↓
触发检测 (拖选、双击、Shift+点击、长按、快捷键)
    ↓
黑名单检查 (is_blocked)
    ↓
文本选择读取 (get_selection)
    ├→ 第一层: 平台原生 API (Windows UIA / macOS AXAPI)
    └→ 第二层: 剪贴板回退 (模拟 Ctrl+C + 轮询剪贴板)
    ↓
发送 "shortcut" 事件到前端
    ↓
前端快捷键处理器 (shortcut.ts)
    ↓
规则匹配 (文本类型分类)
    ↓
动作执行 (executor.ts 执行链)
    ├→ 脚本执行 (JS/Python/Shell/PowerShell)
    ├→ AI Prompt 生成
    ├→ URL 搜索
    └→ 文本转换 (内置函数)
    ↓
输出 (replace/popup/clipboard)
```

## 关键文件

### 后端 (Rust)
| 文件 | 职责 |
|------|------|
| `src-tauri/src/lib.rs` | 应用初始化、全局状态、事件监听器设置 |
| `src-tauri/src/handlers/mouse.rs` | 鼠标事件检测和触发逻辑 |
| `src-tauri/src/handlers/keyboard.rs` | 键盘快捷键事件处理 |
| `src-tauri/src/commands/selection.rs` | 文本提取（原生 API + 剪贴板回退） |
| `src-tauri/src/commands/clipboard.rs` | 剪贴板操作（读/写/备份/恢复） |
| `src-tauri/src/commands/keyboard.rs` | 键盘模拟（Ctrl+C/V/X） |
| `src-tauri/src/platform/windows.rs` | Windows UI Automation 实现 |
| `src-tauri/src/platform/macos.rs` | macOS Accessibility API 实现 |
| `src-tauri/src/commands/shortcut.rs` | 快捷键注册/注销管理 |

### 前端 (TypeScript/Svelte)
| 文件 | 职责 |
|------|------|
| `src/lib/stores.svelte.ts` | 持久化响应式状态管理 |
| `src/lib/shortcut.ts` | 快捷键事件监听和规则分发 |
| `src/lib/executor.ts` | 动作执行管道 |
| `src/lib/evaluator.ts` | JavaScript 代码求值 |
| `src/lib/matcher.ts` | 文本类型匹配 |
| `src/lib/classifier.ts` | ML 文本分类 |
| `src/lib/types.d.ts` | 类型定义 |
| `src/lib/constants.ts` | 常量定义 |

## 触发方式

| 触发方式 | 快捷键字符串 | 说明 |
|----------|-------------|------|
| 拖选 | `MouseClick+MouseMove` | 鼠标拖动距离 > 8px |
| 双击 | `MouseClick+MouseClick` | 500ms 内两次点击，距离 < 3px |
| Shift+点击 | `Shift+MouseClick` | 按住 Shift 键点击 |
| 长按 | `LongPress` | 鼠标按住超过配置时长（默认 1000ms） |
| 键盘快捷键 | 自定义组合键 | 通过 tauri_plugin_global_shortcut 注册 |

## 窗口

- `main`: 主设置窗口
- `toolbar`: 浮动工具栏（macOS 使用 NSPanel）
- `popup`: 结果弹出窗口
