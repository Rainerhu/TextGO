# 当前需求
（无）

# 已完成需求
1. 增加启用禁用每条规则的能力，支持单独启禁用规则
   - Rule 类型新增 `disabled` 属性
   - matcher.ts 中跳过 disabled 规则
   - 快捷键页面每条规则行增加禁用/启用按钮，禁用后规则行半透明显示

2. 增加在每个启用的快捷键下面，添加文件夹的能力，可将多条规则放在一个文件夹里
   - Rule 类型新增 `group` 字段，Shortcut 类型新增 `groups` 字段
   - Binder 组件新增"文件夹"输入框
   - 工具栏中同组规则合并为文件夹按钮，hover 展开子规则

3. 工具栏布局支持水平/垂直两种方向
   - Shortcut 类型新增 `toolbarLayout` 字段
   - 快捷键页面工具栏模式下显示布局切换按钮
   - 工具栏页面根据 layout 参数切换 flex 方向
   - 垂直模式下图标和分隔线自动旋转

4. ai对话框能调节尺寸，记忆尺寸，可配置默认尺寸
   - popup 窗口启用 resizable
   - 新增 popupRememberSize / popupSize / popupDefaultSize 持久化状态
   - 通用设置页面新增"弹出窗口"设置组

5. 可一键复制AI的输出结果
   - AI 对话模式标题栏新增复制按钮

6. 按规则级别的应用过滤
   - Rule 类型新增 `showOnlyApps` 和 `noShowApps` 字段
   - 后端 mouse/keyboard handler 在事件中传递当前 appId
   - 前端 shortcut.ts 新增 wildcardMatch 和 filterRulesByApp 函数
   - Binder 组件新增"仅在应用"和"排除应用"输入框
   - 支持通配符匹配（* 和 ?）
