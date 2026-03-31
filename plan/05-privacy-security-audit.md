# TextGO 隐私安全审计

## 结论：TextGO 不会主动将用户本地信息发送到远端服务器

项目中没有任何遥测（telemetry）、数据分析（analytics）、用户追踪（tracking）代码。
没有后台静默上报机制。所有数据存储在本地（`.settings.dat`）。

---

## 详细分析

### 1. 会发生网络通信的场景（全部由用户主动触发）

| 场景 | 触发方式 | 发送的数据 | 目标服务器 |
|------|---------|-----------|-----------|
| AI 对话 | 用户配置 Prompt 规则并触发 | 选中文本 + 剪贴板文本 + 提示词 | 用户配置的 LLM 提供商 API |
| 网页搜索 | 用户配置 Searcher 规则并触发 | 选中文本（拼接到 URL 中） | 用户配置的搜索网站 |
| 检查更新 | 用户点击"检查更新"按钮 | 当前版本号（HTTP 请求头） | GitHub Releases |
| 自定义脚本 | 用户编写并执行脚本 | 取决于脚本内容 | 取决于脚本内容 |

### 2. AI 对话功能

LLM 客户端（`src/lib/llm/`）支持 7 种提供商 + 自定义提供商：
- 本地：Ollama（`127.0.0.1:11434`）、LM Studio（`127.0.0.1:1234`）
- 云端：OpenAI、Anthropic、Google Gemini、xAI、OpenRouter、自定义

发送的数据：`messages`（对话历史，包含选中文本）、`model`、`temperature` 等参数。

关键点：
- 只有用户主动配置了 API Key 并创建了 Prompt 规则后才会触发
- 用户可以选择纯本地的 Ollama/LM Studio，完全不经过外网
- 没有默认启用的云端 AI 功能

### 3. 脚本执行功能

用户自定义脚本（JS/Python/Shell）可以访问 `fetch` API 和系统命令，
理论上可以发送任意数据到任意服务器。但这完全由用户自己编写和控制。

`evaluator.ts` 中暴露了 `fetch` 给前端 JS 脚本：
```js
(window as any)._fetch = fetch;
```

### 4. 更新检查

通过 `tauri-plugin-updater` 访问：
```
https://github.com/C5H12O5/TextGO/releases/latest/download/latest.json
```
仅下载一个 JSON 文件比对版本号，不上传任何用户数据。
且需要用户手动点击"检查更新"按钮（除非开启了"自动检查更新"设置）。

### 5. 数据存储

所有设置、规则、历史记录、API Key 存储在本地文件 `.settings.dat` 中。
API Key 使用加密存储（`src/lib/utils.ts` 中的 `encrypt`/`decrypt`）。
没有任何云同步功能。

### 6. 不存在的风险

- ❌ 没有遥测/分析 SDK（无 Sentry、Mixpanel、PostHog 等）
- ❌ 没有后台数据上报
- ❌ 没有用户行为追踪
- ❌ 没有广告 SDK
- ❌ ML 分类模型完全在本地运行（TensorFlow.js，模型文件在 `static/` 目录）
- ❌ 自然语言检测（franc）和编程语言检测（guesslang）都是本地库

### 7. HTTP 权限配置

`capabilities/default.json` 中 HTTP 权限完全开放：
```json
{ "url": "http://*:*" }, { "url": "https://*:*" }
```
这是为了支持用户自定义的 LLM API 地址和搜索 URL，不是用于后台通信。
