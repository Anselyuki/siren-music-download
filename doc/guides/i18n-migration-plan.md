# i18n 前后端异步解耦迁移计划

> 目标：在不打断现有下载、播放、搜索与设置链路的前提下，把当前中文硬编码 UI 逐步迁移为可维护的多语言体系；前后端分别落地，各自管理自己的文案资源，当前语言统一跟随后端 `AppPreferences.locale`。
>
> 最后更新：2026-04-28
>
> **当前进度**：PR1 ✅ → PR2a ✅ → PR2b ✅ → PR2c ✅（用户入口正式开放条件已满足，文档同步完成）

## 1. 迁移范围与原则

### 目标范围

首轮建议支持：

- `zh-CN`：当前默认语言，作为基准文案
- `en-US`：第一种新增语言，用于验证英文长度、复数、单位、系统通知和错误提示链路

首轮需要覆盖：

1. 前端可见 UI 文案：按钮、标题、空状态、加载态、筛选项、表单说明、aria / title / alt
2. 前端运行时反馈：toast、下载任务状态、播放器状态、搜索状态、格式化标签
3. 后端用户可见消息：系统通知、偏好校验错误、搜索/库存/下载暴露给前端的用户提示
4. 用户语言偏好：读取、保存、启动恢复、切换后即时生效

首轮不要求覆盖：

- 专辑名、歌曲名、艺术家名、歌词、上游 API 返回的内容数据
- 日志 key、内部错误 key、Rust / TS 类型名、Tauri command 名称
- rustdoc、开发文档、README 的多语言版本
- 构建产物安装包元信息的多语言本地化

### 总体原则

1. 先建立语言基础设施，再迁移文案；不要边抽文案边临时拼接工具函数。
2. 前端 UI 文案优先由前端翻译层负责；后端只处理自己直接发出的系统通知与用户可见错误。
3. 后端跨 IPC 返回的数据尽量使用稳定状态码、枚举和结构化参数；不要让前端解析中文字符串。
4. 翻译 key 按业务域组织，避免以组件文件名作为唯一分组。
5. 所有新增文案都必须从第一天起同时写入 `zh-CN` 和 `en-US`。
6. 数量、单位、进度、任务摘要必须走参数化消息；不要手写字符串拼接。
7. `AppPreferences.locale` 是唯一语言来源；前端只镜像后端偏好，不读取 `navigator.language`、不使用 localStorage/IndexedDB 单独保存语言。

### 单一语言来源契约

语言状态只有一条主链路：

```text
后端偏好文件 / 默认值
  → get_preferences / set_preferences
  → AppPreferences.locale
  → 前端 Svelte locale mirror
  → Paraglide runtime + document.documentElement.lang
```

约束：

1. 后端负责 `locale` 的默认值、持久化、校验和旧偏好迁移。
2. 前端负责把后端返回的 `AppPreferences.locale` 应用到 Svelte 状态和 Paraglide runtime。
3. 前端不得从 `navigator.language`、localStorage、IndexedDB、URL query 或其他浏览器侧来源推断当前语言。
4. 用户在设置面板选择语言时，只是提交偏好变更请求；只有 `set_preferences` 成功返回后的 `AppPreferences.locale` 才能成为新的当前语言。
5. 如果 `get_preferences` 暂时失败，前端只能使用内存兜底语言渲染错误/加载界面，不得把兜底值写入前端存储或反向覆盖后端偏好。

### 技术选型决策

本计划固定采用“双端成熟库”方案：

1. 前端使用 `@inlang/paraglide-js`。
2. 后端使用 Rust Fluent 生态，首轮直接引入 `fluent-templates`。
3. 前后端异步解耦实现：前端 Paraglide 和后端 Fluent 各自独立落地，不互相调用运行时，也不等待对方文案迁移完成。
4. 两端只通过 `AppPreferences.locale` 契约同步当前语言，不共享同一套文案资源，不分别读取语言偏好。
5. 前端文案进入 Paraglide message 文件；后端系统通知、偏好校验和后端用户错误进入 Fluent `.ftl` 文件。

该决策用于避免后续计划修订反复在“成熟库 / 自研 / 静态映射”之间摆动。除非出现明确阻断因素，不再把以下方案作为首轮落地选项：

- 前端自研完整 i18n runtime。
- 前端改用 `svelte-i18n` 或 `i18next`。
- 后端先做零依赖静态映射表、后续再迁移 Fluent。
- 后端用户可见文案继续散落在 `match` / `format!` / 裸字符串中。

允许的修订范围仅限：

1. 根据实际安装的 Paraglide JS 版本校准 Vite 插件、runtime API 和 message 文件格式。
2. 根据 `fluent-templates` 的实际 API 调整 loader 写法、资源路径和 fallback 实现。
3. 调整迁移批次和 PR 边界，但不能改变前端 Paraglide、后端 Fluent 的方向。

## 2. 当前状态快照

当前前端共有约 `129` 个 `.ts/.svelte` 文件，其中约 `19` 个包含中文用户文案。中文文案集中在以下区域：

| 区域           | 主要文件                                                                                                                                        | 特征                                          |
| -------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------- |
| 壳层与主装配   | `src/App.svelte`                                                                                                                                | 初始化失败、刷新失败、toast、少量注释         |
| 播放器         | `src/lib/components/AudioPlayer.svelte`、`src/lib/components/app/PlayerFlyoutStack.svelte`                                                      | aria、状态标签、歌词/队列空状态、下载按钮文案 |
| 专辑与搜索     | `src/lib/components/app/AlbumSidebar.svelte`、`src/lib/components/app/AlbumDetailPanel.svelte`、`src/lib/components/SongRow.svelte`             | 搜索范围、加载态、选择态、下载动作            |
| 设置面板       | `src/lib/components/app/SettingsSheet.svelte`                                                                                                   | 表单标签、说明、按钮、缓存/通知/日志文案      |
| 下载任务面板   | `src/lib/components/app/DownloadTasksSheet.svelte`、`src/lib/features/download/controller.svelte.ts`、`src/lib/features/download/formatters.ts` | 任务状态、筛选项、进度、摘要、错误提示        |
| 通用标签       | `src/lib/downloadBadge.ts`                                                                                                                      | 下载状态 badge                                |
| 后端通知与错误 | `src-tauri/src/notification/mod.rs`、`src-tauri/src/preferences.rs`、`src-tauri/src/search/index.rs`                                            | 系统通知、偏好校验、搜索错误                  |
| 核心库透传文案 | `crates/siren-core/src/download/service.rs`                                                                                                     | 下载批次标题，如“已选 N 首 / N 张专辑”        |

当前难点不是静态按钮文案，而是这几类动态文案：

- `x 首歌曲`、`跨 x 张专辑` 这类复数和单位
- `下载中 (current/total)`、`x/y 首已结束` 这类进度摘要
- `来自《albumName》` 这类带内容名称的模板
- `crates/siren-core` 生成并透传给前端的下载批次标题
- toast 与 command 错误中混合的后端原始错误
- 后端系统通知直接在 Rust 中生成中文正文

## 3. 迁移切片与依赖关系

### 优先级切片

| 优先级 | 目标         | 内容                                                                                                                         |
| ------ | ------------ | ---------------------------------------------------------------------------------------------------------------------------- |
| P0     | 打通可测链路 | `AppPreferences.locale`、Paraglide 生成、Svelte 5 locale mirror、设置面板语言切换验证入口、clean checkout 下 `bun run check` |
| P1     | 用户可见闭环 | 主要静态 UI、基础 formatter、download badge、测试通知、偏好校验错误、语言入口正式开放条件检查                                |
| P2     | 深水区       | 下载任务动态摘要、播放器动态状态、后端用户错误、搜索/库存/下载错误治理                                                       |
| P3     | 长期治理     | 结构化错误、自动中文硬编码扫描、未使用 key 检查、更多语言                                                                    |

### 异步解耦实现策略

阶段 1 建立 `AppPreferences.locale` 契约后，前端线和后端线可以异步并行推进：

- 前端线只依赖 `get_preferences` / `set_preferences` 返回的 `AppPreferences.locale`，不依赖后端 Fluent 是否已经迁移完成。
- 后端线只依赖同一个 `AppPreferences.locale`，不依赖前端 Paraglide 文案迁移进度。
- 两端不共享文案资源、不互相调用 i18n runtime；唯一同步点是偏好契约和最终验收。
- 两条线可以拆 PR、错峰合入；普通用户入口在开放条件满足前保持隐藏，避免半迁移体验暴露。

### 阶段依赖

```text
阶段 0 文件级文案粗清单
  ↓
阶段 1A AppPreferences.locale 契约 + Paraglide 基础设施 + CI 生成
  ↓
阶段 1B 设置面板语言切换验证入口 + 1-2 个低风险文案
  ├─ 前端线：阶段 2 静态 UI → 阶段 3A formatter / badge
  └─ 后端线：阶段 5 Fluent 通知 → 阶段 6A core 透传文案 / 偏好校验 / 测试通知相关错误
  ↓
阶段 7/8 验收与治理

阶段 3B 下载任务摘要 / 播放器动态状态 + 阶段 6B 搜索/库存/下载错误治理可在 PR3+ 继续推进
```

约束：`AppPreferences.locale` 必须在阶段 1A 先落地，并作为唯一语言来源。设置面板语言选择 UI 前移到阶段 1B，作为直观手动测试入口；调试或正式入口都必须走 `set_preferences`，否则阶段 2/3 无法端到端验证语言切换和启动恢复。

### PR 映射

| PR   | 对应阶段                         | 目的                                                   | 是否暴露用户入口                    |
| ---- | -------------------------------- | ------------------------------------------------------ | ----------------------------------- |
| PR1  | 阶段 0 + 阶段 1A/1B + 少量阶段 2 | 打通偏好契约、设置面板切换、前端生成、响应式和 CI 链路 | 可作为开发/测试入口，正式曝光可延后 |
| PR2a | 阶段 2 + 阶段 3A                 | 完成主要前端静态文案、formatter 和 badge               | 否                                  |
| PR2b | 阶段 5 + 阶段 6A                 | 落地后端 Fluent、通知、core 透传文案和偏好错误本地化   | 否                                  |
| PR2c | 过渡期入口开放条件检查           | 将已实现的设置面板语言选择正式开放                     | 是，满足开放条件后暴露              |
| PR3+ | 阶段 3B + 阶段 6B + 阶段 8       | 深入动态文案、后端错误和治理规则                       | 是                                  |

## 4. 目标架构

### 前端目标结构

前端采用 `@inlang/paraglide-js`，不自研完整翻译运行时。Paraglide 负责 message 编译、类型安全函数生成、参数类型约束和 tree-shaking；项目只保留语言状态、偏好同步和少量格式化封装。

建议新增或生成以下结构：

```text
messages/
├── zh-CN.json                   # Paraglide 源文案：中文基准
└── en-US.json                   # Paraglide 源文案：英文

src/lib/paraglide/               # Paraglide 生成目录，由构建脚本生成，不手写业务逻辑
└── messages.js                  # 生成的 message 函数入口，实际结构以 Paraglide 输出为准

src/lib/i18n/
├── index.ts                     # 对外入口：locale、applyBackendLocale、initI18n、格式化导出
├── locale.svelte.ts             # Svelte 5 响应式语言状态与 Paraglide runtime 同步
├── formatters.ts                # number / byte / speed / date / duration 等格式化
└── types.ts                     # 唯一定义 Locale、SupportedLocale 等项目侧类型
```

推荐调用方式：

```ts
import * as m from '$lib/paraglide/messages';

m.download_job_status_running({ current: 2, total: 12 });
m.download_task_count({ count: 3 });
formatByteSize(bytes, locale);
formatSpeed(bytesPerSec, locale);
```

前端语言状态建议满足：

1. 初始化只调用后端 `get_preferences()`，从返回的 `AppPreferences.locale` 设置前端镜像状态。
2. 首次启动默认值由后端偏好系统决定，当前固定为 `zh-CN`；前端不读取系统语言，也不自行推断首次语言。
3. `set_preferences()` 成功返回更新后的 `AppPreferences` 后，前端再应用返回的 `locale`；不把 UI 选择值当作最终语言来源。
4. `document.documentElement.lang` 跟随后端返回的 `AppPreferences.locale` 更新。
5. 生成目录只由 Paraglide 管理，不在业务改动中手写修改。
6. Paraglide runtime 只作为 message 求值时的语言来源；Svelte 组件重渲染由项目侧 `$state` 镜像驱动。
7. 如果 `get_preferences()` 失败，前端可以临时使用内存默认 `zh-CN` 保底渲染，但不得写入任何前端侧语言持久化；恢复后必须重新跟随后端偏好。

`initI18n()` 的职责只应是调用 `get_preferences()` 并应用返回的 `locale`；它不能做浏览器语言探测，也不能读取或写入前端侧语言缓存。

前端启动流程：

1. 创建内存态 `BOOTSTRAP_LOCALE_FALLBACK = 'zh-CN'`，只用于偏好返回前的临时渲染。
2. 应用启动时调用 `get_preferences()`。
3. 成功返回后调用 `applyBackendLocale(preferences.locale)`。
4. 之后所有语言变更都通过 `set_preferences()` 发起，并以后端返回的新偏好作为最终结果。
5. 失败时展示本地兜底错误界面或保留当前镜像语言，等待用户重试；不得读取浏览器系统语言补偿。

> 注意：本项目不是 SvelteKit，不使用 `@inlang/paraglide-sveltekit`。Paraglide runtime 的语言状态是它自己的运行时状态；调用 Paraglide 的语言切换 API 本身不会让 Svelte 5 组件自动重新渲染。必须通过项目侧响应式 locale state 显式触发重新求值。具体 API 名称和导入路径必须以实际安装版本为准。

### 后端目标结构

后端首轮直接采用 Rust Fluent 生态，使用 `fluent-templates`。这样可以避免先做静态映射、后续再迁移 `.ftl` 的二次成本；即使首轮文案数量不多，也统一用 Fluent 管理系统通知、偏好校验和后端用户错误。后端不复用前端 Paraglide 文案文件，两端只共享 `Locale` 字符串契约；默认值、校验和偏好 fallback 由后端负责。

后端结构：

```text
src-tauri/locales/
├── zh-CN/main.ftl               # Fluent 中文文案
└── en-US/main.ftl               # Fluent 英文文案

src-tauri/src/i18n/
├── mod.rs                       # Locale 类型、Fluent loader、公共翻译入口
└── format.rs                    # 后端通知需要的少量数字/任务摘要格式化
```

后端语言来源：

1. `AppPreferences.locale`
2. 启动时从偏好文件读取；偏好文件不存在时由后端创建默认 `zh-CN`
3. 缺失或非法时由后端校验/迁移为 `zh-CN`
4. 后端系统通知和偏好校验使用当前偏好语言

后端优先处理自己无法交给前端的文案：

- 系统通知标题/正文
- 偏好校验错误
- 后端主动发出的 `AppErrorEvent.userMessage`
- command 返回给前端且会直接展示的用户错误

Fluent `.ftl` 示例：

```ftl
notification-download-completed = 下载完成
notification-download-album-completed = 专辑下载完成（{ $count } 首歌曲）
notification-download-album-partial = 专辑下载完成（{ $completed } 首成功，{ $failed } 首失败）
preferences-output-dir-not-exists = 保存路径不存在
```

后端 Fluent 约定：

1. message id 使用 kebab-case，并按业务域前缀组织，例如 `notification-*`、`preferences-*`、`search-*`。
2. `.ftl` 文件只存用户可见文案，不存日志 key、内部错误 key 或 Rust 类型名。
3. 后端 `tr()` 统一负责参数注入和 fallback，不允许业务代码直接读取 `.ftl` 文件。
4. 目标语言缺 key 时回退 `zh-CN`；`zh-CN` 仍缺失时返回稳定 fallback message 或 message id，不能 panic。

## 5. 阶段 0：准备与约束固化

### 目标

在正式改代码前明确迁移边界，避免迁移过程中出现“同一类文案一半走 i18n，一半继续硬编码”的状态。

### 前端任务

1. 建立文件级文案粗清单，至少覆盖当前含中文的 `19` 个前端文件。
2. 按文件标记主要类型：静态 UI、动态模板、toast、aria、title、alt、格式化标签。
3. 按文件标记所属业务域：`common`、`library`、`player`、`download`、`settings`、`shell`、`errors`。
4. 识别不应翻译的内容：专辑名、歌曲名、艺术家名、路径、日志 key、错误 code。

### 后端任务

1. 扫描 Rust 中真正会暴露给用户的中文字符串，范围包括 `src-tauri/src/**/*.rs` 和 `crates/**/src/**/*.rs`，尤其要覆盖 `crates/siren-core`；排除注释、rustdoc 和测试样例。
2. 区分后端用户文案与内部日志文案。
3. 对 `crates/siren-core` 单独评估：凡是通过 Tauri command、事件、下载快照或错误链路透传到前端的 `String`，都按后端用户文案处理；当前已知 `crates/siren-core/src/download/service.rs` 的 `selection_job_title()` 会生成用户可见下载批次标题。
4. 标记当前通过 `Result<T, String>` 返回并会被前端展示的错误。
5. 标记系统通知文案入口。

### 产出

- 新增 `doc/guides/i18n-inventory.md` 作为文件级粗清单，仅用于记录待迁移文件范围和当前状态。
- 阶段 0 不维护独立 key 级清单，避免文档和代码脱节。
- 文件级清单最小列定义：`文件 | 业务域 | 主要类型 | 估算条数 | 状态`。
- `类型` 取值建议：`static`、`dynamic`、`aria`、`title`、`alt`、`toast`、`backend`。
- `状态` 取值建议：`pending`、`done`、`skipped`。
- 具体文案到 message key 的映射直接落在 Paraglide message 文件、后端 Fluent `.ftl` 文件和对应 PR description 中；如果 Paraglide 当前格式支持注释或 metadata 字段，可以记录简短来源说明，否则不要在严格 JSON 中写注释。
- 明确首轮语言：`zh-CN`、`en-US`。
- 明确 fallback 策略：缺 key、缺语言、参数缺失时如何处理。

示例：

```markdown
| 文件                                       | 业务域 | 主要类型     | 估算条数 | 状态    |
| ------------------------------------------ | ------ | ------------ | -------- | ------- |
| `src/lib/components/app/TopToolbar.svelte` | shell  | title / aria | 3        | pending |
```

### 验收

- 文件级文案粗清单已提交到仓库。
- 所有新增文案都能明确归属到某个业务域。
- 团队知道哪些字符串不应该被翻译。
- 后续阶段迁移文件时，只更新文件级状态；不要求补 key 级文档表格。
- 迁移完成度以后续脚本扫描中文硬编码残留为准，而不是手工维护清单为准。
- 后续代码迁移不会改变业务状态机和 IPC 契约。

## 6. 阶段 1：locale 契约、Paraglide 基础设施与 CI 生成

### 目标

先落地唯一语言契约和可响应式更新的前端翻译层，但不大规模替换业务文案，也不引入后端 Fluent。后端在本阶段只负责 `AppPreferences.locale` 的默认值、校验、序列化和返回契约。

### 推荐实现

优先采用 `@inlang/paraglide-js`，理由是它与 Vite/TypeScript 项目贴合，能在构建期生成类型安全 message 函数，减少传统 `t('key')` 字符串 key 写错、参数漏传和未使用文案难发现的问题。

### 版本校准 Spike（硬性前置）

阶段 1 不能直接按本文示例实现。必须先做一个独立 Spike，在当前仓库安装并跑通 Paraglide hello world，然后把确认后的 API 回写到本文档。

Spike 必须完成：

1. 运行 `bun add -D @inlang/paraglide-js@latest`。
2. 查看 CLI help 与官方 Vite 文档，确认当前版本的命令、Vite 插件导入名和配置字段。
3. 创建最小 `zh-CN` / `en-US` message，跑通生成命令和 Vite dev/build。
4. 确认生成目录结构、message 函数入口、runtime 语言切换 API。
5. 确认 message format JSON 的 schema、复数声明语法和参数类型生成行为。
6. 删除生成目录后运行 `bun run check`，验证生成脚本不会依赖本地残留文件。
7. 将校准后的最终 Vite 配置、生成命令、runtime 导入路径和 message 示例回写本计划；未回写前不得进入正式迁移。

基础能力：

1. 安装并配置 `@inlang/paraglide-js@latest`。
2. 在动手前运行本地命令或查官方 Vite 文档，确认当前安装版本的 CLI、Vite 插件、runtime API、生成目录结构；本文档中的 API 示例只作为形态参考。
3. 在 `vite.config.ts` 集成当前版本的 Paraglide Vite 插件，确认 dev server 和 production build 都能生成 `$lib/paraglide/`。
4. 建立 `zh-CN` / `en-US` message 源文件。
5. 生成 `$lib/paraglide/messages` 与语言 runtime 入口；实际路径以当前版本输出为准。
6. 把 `src/lib/paraglide/` 加入 `.gitignore`，生成目录不提交。
7. 确保 `bun run check` 在无预生成目录时也能通过；必须增加 `i18n:generate` 脚本，并让 `check:types` / `check:svelte` 在执行前显式生成 Paraglide 输出。
8. `Locale = 'zh-CN' | 'en-US'`，只在 `src/lib/i18n/types.ts` 定义一次。
9. `BOOTSTRAP_LOCALE_FALLBACK = 'zh-CN'`，只用于前端等待后端偏好返回前的内存兜底。
10. `SUPPORTED_LOCALES`。
11. `locale` Svelte 5 响应式镜像状态，来源只能是后端返回的 `AppPreferences.locale`。
12. `applyBackendLocale(locale)` 只负责应用后端返回值：同步 Svelte state、Paraglide runtime 和 `document.documentElement.lang`，不负责持久化。
13. 在本阶段提前给 `AppPreferences` 增加 `locale` 字段，并让启动初始化通过 `get_preferences` 从后端读取 locale；设置面板语言选择 UI 必须作为阶段 1B 验证入口前移实现。
14. `Intl.NumberFormat`、`Intl.DateTimeFormat` 包装；数量文案优先使用 Paraglide variants 的 `plural` formatter。

### Vite 插件配置

阶段 1 必须修改 `vite.config.ts`，把 Paraglide 放进 Vite 插件链。下面示例必须在安装当前版本后按官方文档校准；插件名称、选项名、runtime 导出路径都以实际版本为准。

```ts
import { paraglideVitePlugin } from '@inlang/paraglide-js';

export default defineConfig({
  plugins: [
    paraglideVitePlugin({
      project: './project.inlang',
      outdir: './src/lib/paraglide',
    }),
    svelte(),
    tailwindcss(),
  ],
});
```

验收点：

1. `bun run dev` 首次启动能生成 `src/lib/paraglide/`。
2. `bun run build` / `bun run check` 期间不依赖已提交的生成目录。
3. 删除 `src/lib/paraglide/` 后重新运行 dev/build/check 仍能恢复。
4. CI 中 `bun run check` 在 clean checkout、无预生成目录的情况下通过。

### 生成脚本策略

不要只依赖 Vite 插件生成文件，因为当前 `check` 顺序中 `check:types` 和 `check:svelte` 早于 `vite build`。建议在 `package.json` 增加显式生成脚本，并让类型检查和 Svelte 检查先生成：

```json
{
  "scripts": {
    "i18n:generate": "<按 Spike 校准后的 Paraglide 生成命令>",
    "check:types": "bun run i18n:generate && tsc --noEmit",
    "check:svelte": "bun run i18n:generate && bunx svelte-check",
    "check:build": "vite build"
  }
}
```

如果 Spike 确认当前 Paraglide Vite 插件能覆盖 `vite build`，`check:build` 可以不重复生成；但 `check:types` / `check:svelte` 必须能在 clean checkout 下独立通过。

### message 命名建议

```text
common_action_cancel
common_action_retry
common_state_loading
library_sidebar_title
library_search_placeholder
player_control_play
player_control_pause
download_job_status_running
download_job_summary_selection_scope
settings_notification_test_button
errors_download_create_failed
```

命名规则：

- 使用稳定英文语义命名，不使用中文拼音或中文内容。
- 第一段是业务域。
- 中间段是语义区域。
- 最后一段描述用途。
- 动态消息使用 Paraglide 参数，不把参数含义编码进 key。

### 前端文件建议

| 文件或目录                      | 责任                                      |
| ------------------------------- | ----------------------------------------- |
| `messages/zh-CN.json`           | Paraglide 中文基准文案                    |
| `messages/en-US.json`           | Paraglide 英文文案                        |
| `src/lib/paraglide/`            | Paraglide 生成目录，不手写业务逻辑        |
| `src/lib/i18n/locale.svelte.ts` | 当前语言状态、初始化、切换和 runtime 同步 |
| `src/lib/i18n/formatters.ts`    | byte、speed、date、duration、count 格式化 |
| `src/lib/i18n/index.ts`         | 项目侧 i18n 统一导出                      |
| `project.inlang/settings.json`  | Paraglide / inlang 配置                   |

### Locale 契约同步

`Locale` 的跨端契约必须显式同步，不能让 Rust enum 和 TS union 各自漂移。

推荐做法：

1. 后端 `AppPreferences.locale` 使用 Rust enum，而不是裸 `String`。
2. Rust enum 每个 variant 显式 `serde(rename = "zh-CN")` / `serde(rename = "en-US")`，不要依赖 `rename_all` 推导大小写。
3. 前端 `src/lib/i18n/types.ts` 定义 `export type Locale = 'zh-CN' | 'en-US'`，并与 `src/lib/types.ts` 的 `AppPreferences.locale` 对齐。
4. 新增语言时必须同时更新 Rust enum、TS union、Paraglide messages、Fluent `.ftl`、`SUPPORTED_LOCALES` 和契约文档。
5. 阶段 1 至少补一个序列化/反序列化测试，确认 `zh-CN` / `en-US` 在 TOML、Tauri command 返回值和 TS 类型中保持一致。
6. 前端不得在 `src/lib/i18n` 之外定义第二套 locale 来源；设置面板只提交偏好更新请求，最终显示语言以后端返回的 `AppPreferences.locale` 为准。
7. 后端是 locale 默认值和合法性判断的 owner；前端 TS union 只是 IPC 契约镜像，不允许扩展出后端不支持的语言。

### Paraglide + 纯 Svelte 5 响应式集成

本项目采用纯 Svelte 5 + Vite，不走 SvelteKit adapter。locale 的唯一事实来源是后端 `AppPreferences.locale`；前端 Svelte 5 `$state` 只是后端偏好的运行时镜像，Paraglide runtime 是该镜像的派生同步目标。

推荐规则：

1. `src/lib/i18n/locale.svelte.ts` 持有唯一响应式 locale 镜像，例如 `current = $state<Locale>(BOOTSTRAP_LOCALE_FALLBACK)`；该初始值只用于等待后端偏好返回前的内存兜底。
2. `applyBackendLocale(locale)` 是唯一应用语言的入口，只接受后端 `get_preferences` / `set_preferences` 返回的 locale。
3. `applyBackendLocale(locale)` 内部同步项目侧 `$state`、当前 Paraglide runtime 的语言切换 API 和 `document.documentElement.lang`；API 名称、参数和导入路径以实际安装版本为准。
4. UI 选择语言时不得直接调用 runtime 切换；必须先提交 `set_preferences`，成功返回后再调用 `applyBackendLocale(updated.locale)`。
5. 组件内不要假设 `m.xxx()` 会自动响应 Paraglide runtime 变化；message 求值必须显式依赖项目侧 locale 镜像 state。
6. 常用做法是在组件中用 `$derived.by()` 包装动态文案，并在 derived 内读取 `i18n.current` 以建立依赖。

示例：

```ts
// src/lib/i18n/locale.svelte.ts
// 具体导入路径和函数名以当前 Paraglide 版本生成结果为准
import { setLocale as setParaglideLocale } from '$lib/paraglide/runtime';
import { BOOTSTRAP_LOCALE_FALLBACK, type Locale } from './types';

class LocaleState {
  current = $state<Locale>(BOOTSTRAP_LOCALE_FALLBACK);

  applyBackendLocale(locale: Locale) {
    if (this.current === locale) return;
    this.current = locale;
    setParaglideLocale(locale);
    document.documentElement.lang = locale;
  }
}

export const localeState = new LocaleState();
```

```svelte
<script lang="ts">
  import * as m from '$lib/paraglide/messages';
  import { localeState } from '$lib/i18n';

  const title = $derived.by(() => {
    localeState.current;
    return m.library_sidebar_title();
  });
</script>

<h2>{title}</h2>
```

对于只在模板中直接调用 message 函数的简单组件，仍需通过 `$derived`、`{#key localeState.current}` 或其他显式读取方式建立 locale 依赖。

### 降低组件样板的推荐模式

不要要求每条静态文案都单独写一个 `$derived.by()`，但也不要在模板里直接调用一个普通 `t()` helper 来读取 locale。Svelte 5 不会追踪普通函数内部读取的 `$state`，只有在 `$derived` / `$derived.by()` / `$effect` 或模板显式依赖中读取才会建立响应式依赖。

推荐模式：

1. 高频、状态密集组件：使用一个 `$derived.by()` 聚合多个 label，返回对象。适用 `AudioPlayer.svelte`、`SongRow.svelte`、下载任务行等。
2. 低频、面板型组件：允许在局部容器使用 `{#key localeState.current}` 包裹，让内部 message 调用在语言切换时自然重建。适用 `SettingsSheet.svelte`、`DownloadTasksSheet.svelte`、帮助说明区等。
3. 动态模板和复用纯函数：优先把 message 调用收敛到 formatter / label 函数中，由调用方在 `$derived.by()` 内建立 locale 依赖。

聚合 label 示例：

```svelte
<script lang="ts">
  const labels = $derived.by(() => {
    localeState.current;
    return {
      cancel: m.common_action_cancel(),
      retry: m.common_action_retry(),
      title: m.settings_title(),
    };
  });
</script>

<button>{labels.cancel}</button>
```

`{#key}` 示例：

```svelte
{#key localeState.current}
  <SettingsSheetContent />
{/key}
```

默认推荐：静态低频区域用 `{#key}` 降低迁移成本；高频交互区域用聚合 `$derived.by()` 控制重算范围。

### 性能约束

1. Paraglide message 函数可以在普通渲染路径中调用；它是构建期生成函数，不应成为主要性能瓶颈。
2. 不要在高频事件处理里反复生成长文案，例如下载进度 tick、播放进度 tick、滚动事件和 resize 事件。
3. 高频区域避免用大范围 `{#key localeState.current}` 重建，例如播放器、歌曲列表、下载任务列表；这些区域优先使用聚合 `$derived.by()` 控制重算粒度。
4. 低频区域可以用 `{#key}` 简化迁移，例如设置面板、下载任务筛选区、帮助说明区。
5. 后端 Fluent 只用于低频用户可见消息，例如通知、偏好校验、command 错误；不要在下载进度事件流中生成本地化长文案。

### Paraglide message 文件格式

首轮使用 Paraglide 当前版本推荐的 message format JSON。下面示例表达目标结构，具体 schema、声明语法和文件路径必须以实际安装版本的官方文档与生成器校准。简单文案可以写成 key-value；包含复数、数字和日期格式化的文案使用 declarations / selectors / match 结构。

简单示例：

```json
{
  "$schema": "https://inlang.com/schema/inlang-message-format",
  "common_action_cancel": "取消",
  "settings_notification_test_button": "发送测试通知",
  "download_job_summary_partial": "{completed} 首成功，{failed} 首失败"
}
```

复数策略只保留意图，不在本计划固化具体 JSON 语法：

- 中文可以让 `one` / `other` 对应相同文案，这是有意为之；中文没有英语式名词复数变化。
- 英文必须区分 `one` / `other`，例如 `1 song` / `2 songs`。
- 具体 declarations / selectors / match 写法以阶段 1 Spike 校准后的 Paraglide message format 为准，并在校准后回写本节示例。

### 复数与格式化策略

1. 数量相关用户文案优先使用 Paraglide variants 和 `plural` formatter。
2. 数字、日期、时间如出现在完整句子里，优先使用 Paraglide 的 `number` / `datetime` formatter。
3. 文件大小、下载速度、播放时长这类项目级格式化仍保留在 `src/lib/i18n/formatters.ts`，因为它们通常不是自然语言句子，而是单位格式化工具。
4. 不再采用“`Intl.PluralRules` 或 `tCount()` 二选一”的模糊策略；只有 Paraglide message format 无法覆盖的项目级单位格式化，才在 `formatters.ts` 中封装 `Intl`。

### 验收

- 能在任意 Svelte 组件中调用生成的 message 函数并随语言切换刷新。
- `zh-CN` 和 `en-US` message key 完全一致。
- message 参数缺失或名称错误能被 TypeScript 或生成流程发现。
- 生成目录不需要手工维护。
- clean checkout 下执行 `bun run check` 能自动生成或预生成 Paraglide 输出并通过。
- 不修改任何业务行为。

## 7. 阶段 1B：语言偏好与设置面板切换入口

### 目标

把设置面板语言切换前移到共享前置阶段，尽早提供一个肉眼可验证的端到端入口。这个入口用于验证 `SettingsSheet → set_preferences → 后端偏好持久化 → AppPreferences.locale → 前端 locale mirror → Paraglide runtime` 链路。

优先级约束：

1. 语言切换 UI 不再等到主要静态文案全部迁移后才实现。
2. PR1 必须能通过设置面板切换 `zh-CN` / `en-US`，并验证刷新、重启和保存失败行为。
3. 如果担心半迁移体验，可以先用开发/测试 gating 控制普通用户可见性；但实现位置仍应在 `SettingsSheet.svelte`，不要另做命令行或隐藏本地存储入口。

### 前端任务

1. 在 `SettingsSheet.svelte` 中新增语言选择项。
2. 当前语言展示值来自后端偏好镜像，即 `settingsState.locale` / `src/lib/i18n/locale.svelte.ts`，不得读取浏览器系统语言或前端本地存储。
3. 用户切换语言时先调用 `set_preferences` 提交完整偏好；请求成功返回后，用返回的 `AppPreferences.locale` 调用 `applyBackendLocale()` 更新 UI。
4. 保存请求处理中可以显示 pending 选择值，但全局 UI 语言不要在成功返回前切换。
5. 保存失败时丢弃 pending 选择值，保持当前后端镜像语言不变，并显示本地化错误。
6. 设置面板如需要本地表单态，只能表示“待提交选择”，不能作为语言事实来源。

### 后端任务

1. `AppPreferences.locale` 字段应已在阶段 1A 完成；本阶段只补齐设置面板保存链路和必要的契约文档。
2. 后端偏好默认值为 `zh-CN`。
3. `validate()` 校验 locale 是否属于支持列表。
4. 偏好文件缺失 `locale` 时由后端自动补默认值，前端不做独立默认持久化。
5. `get_preferences` / `set_preferences` 返回结构与 `src/lib/types.ts` 对齐。

### 契约更新

需要同步更新：

- `src-tauri/src/preferences.rs`
- `src/lib/types.ts`
- `src/lib/api.ts` 中相关类型说明或调用注释
- `doc/reference/backend-api-contract.md` 的 `AppPreferences`
- 设置面板文档或前端指南中关于设置项的说明

### 验收

- 首次启动由后端偏好默认 `zh-CN`，前端不会隐式按系统语言写入偏好。
- 前端初始化语言只来自 `get_preferences` 返回值；没有额外浏览器侧语言探测或前端持久化读取。
- 切换英文后，关闭并重新打开应用仍为英文。
- `set_preferences` 失败时 UI 语言不提前变化。
- 非法 locale 不会写入偏好文件。
- 旧偏好文件能无感迁移。

### PR1 手动测试办法

PR1 至少迁移 `SettingsSheet.svelte` 中语言选择项自身，以及 `TopToolbar.svelte` 或 `AlbumWorkspaceContent.svelte` 中 1-2 处低风险文案，用来直观看到语言切换结果。

测试步骤：

1. 运行 `bun run tauri:dev` 启动应用。
2. 打开设置面板，确认语言选择项初始显示为后端返回的 `zh-CN`。
3. 将语言切换为 `English`，确认保存请求成功后，设置面板语言项、已迁移的顶部按钮或空状态文案立即变为英文。
4. 关闭应用并重新运行 `bun run tauri:dev`，确认仍保持英文，证明偏好由后端持久化。
5. 切回 `简体中文`，确认同一批文案立即恢复中文。
6. 手动制造一次保存失败场景，例如临时让 `set_preferences` 返回错误或提交非法 locale，确认全局 UI 语言不会提前切换，pending 选择被回滚，并展示当前语言下的错误提示。

通过标准：

- 切换语言不需要刷新页面。
- 重启后语言保持不变。
- 浏览器系统语言、localStorage、IndexedDB 不影响结果。
- 未迁移区域允许继续显示中文，但已迁移的测试点必须稳定跟随设置面板选择。

## 8. 阶段 2：前端静态 UI 文案迁移

### 目标

先迁移风险最低的静态界面文案，验证 key 组织、组件响应式更新和英文布局。

本阶段属于前端线，只依赖阶段 1A/1B 的 `AppPreferences.locale` 读取、设置面板切换验证入口和前端 Paraglide 生成，不依赖后端 Fluent 完成。

### 优先顺序

1. `TopToolbar.svelte`
   - `刷新缓存`
   - `下载任务`
   - `下载设置`
2. `AlbumWorkspaceContent.svelte`
   - `选择专辑`
   - `从左侧选择一个专辑以查看歌曲`
3. `AlbumDetailSkeleton.svelte`
   - `正在加载歌曲...`
4. `AlbumSidebar.svelte`
   - 标题、placeholder、搜索范围选项、加载态、空状态
5. `SettingsSheet.svelte`
   - 标题、说明、表单 label、按钮、帮助文本
6. `DownloadTasksSheet.svelte`
   - 标题、说明、筛选项、空状态、按钮

### 注意事项

- `aria-label`、`title`、`alt` 必须一起迁移，不只迁移可见文本。
- Select option 的 `label` 应使用翻译后的 label，但 `value` 保持稳定枚举值。
- 不要把 `value`、状态码、任务类型枚举改成翻译后的字符串。
- 英文较长的按钮需要检查最小宽度、换行和截断策略。

### 验收

- 切换语言后，所有已迁移的可见文案、tooltip、aria 同步更新。
- `bun run check` 通过。
- 中英文下主界面、设置面板、下载任务面板没有明显截断或重叠。

## 9. 阶段 3：前端动态文案与格式化迁移

### 目标

迁移下载、播放器、搜索、toast 中的动态模板，消除字符串拼接导致的复数和语序问题。

本阶段属于前端线，只处理前端生成和展示的文案；后端返回的用户错误在阶段 6 单独治理。

### 下载域

重点文件：

- `src/lib/features/download/controller.svelte.ts`
- `src/lib/features/download/formatters.ts`
- `src/lib/components/app/DownloadTasksSheet.svelte`
- `src/lib/components/app/AlbumDetailPanel.svelte`
- `src/lib/components/SongRow.svelte`
- `src/lib/downloadBadge.ts`

迁移文件范围以 `doc/guides/i18n-inventory.md` 的文件级粗清单为准；具体 message key 以 Paraglide message 文件和 PR description 为准，不再维护独立 key 级清单。

迁移要求：

- 数量文案统一使用 Paraglide variants 和 `plural` formatter。
- byte 和 speed 继续使用 `src/lib/i18n/formatters.ts`，内部基于 `Intl.NumberFormat`，不要硬编码小数格式。
- 状态 label 函数需要调用 Paraglide 生成的 message 函数，或接收明确的 message 函数组合作为依赖。
- 如果某个 label 函数被多个组件复用，优先保留函数，但让函数内部走 Paraglide message，不再返回硬编码中文。

### 播放器域

重点文件：

- `src/lib/components/AudioPlayer.svelte`
- `src/lib/components/app/PlayerFlyoutStack.svelte`
- `src/lib/features/player/controller.svelte.ts`

迁移文件范围以 `doc/guides/i18n-inventory.md` 的文件级粗清单为准；歌曲名、艺术家名不翻译，只作为参数插入模板。

播放器约束：

- 语言切换只允许热更新播放器 UI 文案，例如 `aria-label`、tooltip、循环模式标签、歌词/队列空状态、下载按钮文案。
- 语言切换不得触发播放器组件大范围 `{#key}` 重建，不得重建后端播放会话，不得重置 `PlayerState`、播放队列、进度、音量、暂停/播放状态。
- `AudioPlayer.svelte` 和 `PlayerFlyoutStack.svelte` 优先用聚合 `$derived.by()` 更新 labels；不要把整个播放器根节点包进 `{#key localeState.current}`。

### 搜索与专辑域

重点文件：

- `src/lib/components/app/AlbumSidebar.svelte`
- `src/lib/features/library/controller.svelte.ts`
- `src/App.svelte`

迁移文件范围以 `doc/guides/i18n-inventory.md` 的文件级粗清单为准；搜索结果中的专辑名、歌曲名、艺术家名不翻译。

### 验收

- 中英文动态消息语序自然，不出现中文单位残留。
- `1 song` / `2 songs` 等复数场景正确。
- 文件大小、速度、日期和数字格式跟随语言。
- toast 中不再出现由前端硬编码的中文前缀。

## 10. 阶段 5：后端系统通知本地化

### 目标

落地后端 Fluent 基础设施，并迁移后端直接发出的系统通知，保证下载完成通知、播放切换通知、测试通知跟随用户语言。

本阶段属于后端线，可以和前端静态/动态文案迁移并行。它只读取 `AppPreferences.locale`，不依赖前端 Paraglide message key 或组件迁移进度。

### 重点文件

- `src-tauri/src/notification/mod.rs`
- `src-tauri/src/notification/desktop.rs`
- `src-tauri/src/notification/macos.rs`
- `src-tauri/src/preferences.rs`

### 迁移内容

1. 下载完成通知：
   - 单曲下载完成
   - 专辑/多选下载完成
   - 部分失败：成功数、失败数
2. 播放切换通知：
   - 歌曲名和艺术家名不翻译
   - 空艺术家时保持空正文或使用本地化 fallback，需明确选择
3. 测试通知：
   - 标题
   - 正文
4. 通知投递失败仍写内部英文日志，不需要本地化日志 key。
5. 窗口标题首轮保持产品名不本地化；如果后续需要运行时本地化窗口标题，应通过后端 Fluent 文案和 Tauri `set_title` 统一处理。

### 后端实现建议

1. 首轮直接引入 `fluent-templates`。
2. 新增 `Locale` 类型，能从 `AppPreferences.locale` 转换。
3. 在 `src-tauri/locales/{locale}/main.ftl` 维护系统通知文案，并预留 `preferences-*`、`search-*` 等后端用户错误 message id。
4. 新增 Fluent loader，并提供 `tr(locale, key, args)` 风格入口。
5. `tr()` 必须有 fallback：目标语言缺 key 时回退 `zh-CN`，仍缺失时返回稳定 fallback message 或 key。
6. `notify_download_completed`、`notify_playback_changed`、`notify_test` 在生成 title/body 前读取当前偏好语言。
7. 不把翻译后的系统通知正文回传给前端作为业务状态。
8. 如果后续需要系统 locale 探测或资源嵌入增强，也应由后端偏好初始化流程统一处理，再评估叠加 `i18n-embed`；首轮不强制引入。

### 验收

- `zh-CN` / `en-US` 的 `.ftl` message id 一致。
- 设置为英文后，测试通知显示英文。
- 下载完成通知随语言变化。
- 部分失败通知的数量和语序正确。
- 播放通知中歌曲名、艺术家名保持原始内容。

## 11. 阶段 6：后端用户错误、事件与 core 透传文案治理

### 目标

减少前端直接展示后端中文字符串的情况，为后续更多语言和更稳定的错误处理打基础。

本阶段属于后端线，可以独立于前端 UI 文案迁移推进。前端只需要继续展示 command 返回的用户错误，不需要理解后端 Fluent message id。

### 当前问题

部分 Rust command 和服务层会返回中文 `String`，前端捕获后直接拼接展示。例如偏好校验、搜索索引、路径校验等场景。`crates/siren-core` 也可能通过下载快照、事件或错误链路向 `src-tauri` 透传用户可见字符串，例如当前 `selection_job_title()` 会生成“已选 N 首 / N 张专辑”这类下载批次标题。这样会导致：

1. 前端无法根据当前语言重新渲染历史错误。
2. 英文 UI 中可能混入中文后端错误。
3. 前端无法可靠区分错误类型，只能展示字符串。

### 首轮策略

首轮只做短期兼容：保留 `Result<T, String>` 和现有事件结构，但后端根据当前 `AppPreferences.locale` 通过 Fluent 返回本地化错误字符串。

优点：改动小，适合先完成端到端语言切换。  
缺点：错误仍不是结构化契约，前端不能重新翻译历史错误。

适用：偏好校验、测试通知失败、搜索失败等当前低频用户错误。

### 后续演进记录

结构化错误不进入首轮迁移，只作为后续增强记录。目标形态可以是：

```text
AppUserError {
  code: string,
  messageKey: string,
  params: Record<string, string | number | boolean>,
  fallbackMessage: string
}
```

前端优先使用 `messageKey + params` 本地化；缺失时展示 `fallbackMessage`。后续再逐步把 command 返回错误和 `AppErrorEvent` 统一到结构化错误模型，减少裸 `String`。

### 首轮迁移顺序

1. `crates/siren-core` 透传文案评估与迁移：先覆盖 `crates/siren-core/src/download/service.rs` 的 `selection_job_title()`；推荐把下载批次标题改为结构化 title parts，由 `src-tauri` 按 `AppPreferences.locale` 通过 Fluent 生成最终文案。如果短期不改结构，也必须确保 core 生成标题时能接收明确 locale / translator，而不是继续硬编码中文。
2. 偏好校验错误：路径非法、路径不存在、格式不支持、日志等级不支持。
3. 测试通知和通知权限相关错误。
4. 搜索错误：关键词为空、关键词过长、索引构建失败。
5. 下载和本地库存错误保持现有结构，先按当前 locale 通过 Fluent 返回本地化字符串；结构化 `messageKey + params` 留到后续演进。
6. 通用 app error event 首轮不改结构，只治理其中会直接展示给用户的 `userMessage`。

### 验收

- 英文 UI 中不再出现后端生成的中文用户错误。
- 英文 UI 中不再出现 `crates/siren-core` 透传的中文下载批次标题或用户错误。
- 前端不需要解析中文字符串来判断错误类型。
- 现有日志 key 和内部错误 details 不受影响。

## 12. 过渡期共存策略

### 目标

允许分阶段迁移，不要求一次性清空所有中文硬编码，同时避免用户在半迁移状态下看到破碎的多语言体验。

### 策略

1. 语言基础设施和设置面板语言选择可以先合入，但在可见 UI 覆盖不足前，可以通过开发/测试 gating 控制普通用户可见性。
2. 开发期应直接使用设置面板语言选择触发 `set_preferences` 来切换 locale，用于验证已迁移区域；不要再另做前端本地调试入口。
3. 正式向普通用户开放设置面板语言选择前，至少完成最小 PR 范围内的静态 UI、`formatters.ts`、`downloadBadge.ts`、测试通知和关键 toast。
4. 过渡期允许未迁移组件继续显示中文，但必须能被文件级粗清单或中文硬编码扫描覆盖。
5. 新增文案不得继续硬编码中文，必须直接进入 Paraglide 或后端 Fluent `.ftl`。

### 用户可见入口开放条件

- 设置面板、下载任务面板、专辑侧栏和顶部工具栏已完成静态文案迁移。
- 下载 badge、基础数量/单位格式化已完成迁移。
- 后端测试通知和偏好校验错误已能按 locale 输出。
- 手动验收确认英文界面没有大面积中文残留。

## 13. 阶段 7：英文布局、可访问性与回归验证

### 目标

确认语言切换不是只替换字符串，而是在真实 UI 中可用。

### 重点检查界面

1. 左侧专辑栏：搜索输入、范围选择、搜索结果类型标签。
2. 主专辑页：下载整张专辑、多选下载、选择摘要、歌曲行下载按钮。
3. 底部播放器：控制按钮 aria、循环模式、歌词/队列/下载按钮。
4. 设置 Sheet：表单 label、说明、测试通知、缓存清理、日志区域。
5. 下载任务 Sheet：筛选 Select、任务状态、进度、取消/重试、空状态。
6. Toast：错误和信息提示。
7. 系统通知：测试通知、下载完成通知、播放切换通知。

### 验证命令

默认对齐仓库标准命令。Paraglide 生成流程必须接入 Vite 或 package script，确保 `bun run check` 能覆盖 message 生成、类型检查和前端构建。如果 `check:types` / `svelte-check` 早于 Vite 插件生成执行，需要在 `package.json` 增加显式 `i18n:generate` 脚本，并让 `check:types` / `check:svelte` 调用它。

```bash
bun run check
cargo test --workspace
```

如果本轮只改前端文案且未触及 Rust 行为，`cargo test --workspace` 可以作为发布前补充验证；涉及后端 i18n、偏好或通知时必须执行。

### 手动验收场景

1. 首次启动后端默认 `zh-CN`，前端跟随 `get_preferences` 返回值。
2. 设置中切换英文，UI 立即更新。
3. 关闭应用后重新打开，仍为英文。
4. 英文下搜索专辑、歌曲、艺术家。
5. 英文下创建单曲下载、整专下载、多选下载。
6. 英文下取消、重试、清理下载历史。
7. 英文下播放歌曲、切歌、打开歌词和播放队列。
8. 英文下发送测试通知。
9. 切回中文，以上关键界面恢复中文。

### 布局验收标准

- 无按钮文字互相覆盖。
- 无重要信息被不可见截断。
- 可接受单行省略，但必须保留 tooltip 或 aria 语义。
- Sheet 在窄宽度下可滚动访问全部内容。
- aria-label 跟随语言切换。

## 14. 阶段 8：治理规则与后续维护

### 新增文案规则

1. 新增前端用户可见文案必须通过 Paraglide message。
2. 新增 `zh-CN` message 时必须同步新增 `en-US` message。
3. 新增动态文案必须使用参数化模板。
4. 新增数量文案必须考虑单复数。
5. 新增后端用户错误必须优先给出 code / key，不优先给裸字符串。
6. 新增系统通知文案必须在后端 Fluent `.ftl` 文件中登记。
7. 当前仅支持 LTR 语言；后续新增 RTL 语言时，必须补充 `document.documentElement.dir` 切换、布局检查和截图验收。

### 推荐检查

迁移完成度以脚本扫描为主，不以人工 key 清单为准。后续应增加轻量脚本检查；如果脚本落地，应接入 `bun run check` 或 CI：

1. 扫描 `src/**/*.svelte` 和 `src/**/*.ts` 中的中文硬编码，允许通过白名单排除歌曲名、艺术家名、测试样例和文档示例。
2. 扫描 `src-tauri/src/**/*.rs` 和 `crates/**/src/**/*.rs` 中非注释、非测试的中文字符串。
3. 运行 Paraglide 生成/校验流程，确认 `zh-CN` 与 `en-US` message key 一致。
4. 检查后端 Fluent `.ftl` 中 `zh-CN` 与 `en-US` 的 message id 一致。
5. 检查未使用 key。

### 文档更新

完成迁移后需要同步更新：

- `doc/guides/frontend-guide.md`：新增 Paraglide 目录、使用方式和组件文案规则。
- `doc/reference/backend-api-contract.md`：新增 `AppPreferences.locale` 和可能的结构化错误契约。
- `doc/history/decisions.md`：追加 i18n 技术决策，说明前端采用 Paraglide JS、后端采用 Fluent / `fluent-templates` 的原因。
- `README.md`：如对用户暴露语言切换能力，则补充功能说明。

## 15. 推荐实施顺序总览

```text
共享前置：
0. 文件级文案粗清单与边界确认
1A. AppPreferences.locale 最小持久化 + Paraglide 基础设施 + CI 生成
1B. 设置面板语言切换验证入口 + 1-2 个直观测试文案

前端线：
2F. 前端静态 UI 文案
3F. 前端动态文案与格式化基础

后端线：
2B. 后端系统通知本地化（Fluent）
3B. 后端用户错误短期本地化（先覆盖偏好校验 / 测试通知相关错误）

汇合：
5. 设置面板语言入口正式开放条件检查
6. 英文布局、可访问性与回归验证
7. 规则固化与文档同步
```

推荐不要跳过共享前置阶段。设置面板语言切换前移到 PR1，用来验证真实链路；前端线和后端线可以异步合入，不要求在同一个 PR 中完成。若 UI 覆盖不足，普通用户正式可见性可以等汇合验收通过后再开放。最小 PR 可以不完成所有动态文案，但必须包含动态文案基础设施和至少一批纯函数迁移，例如 `formatters.ts` 与 `downloadBadge.ts`，避免后续英文出现大量语序和单位问题。

## 16. 风险与缓解

| 风险                         | 影响                                                    | 缓解                                                                                           |
| ---------------------------- | ------------------------------------------------------- | ---------------------------------------------------------------------------------------------- |
| 英文文案变长导致布局挤压     | 播放器、按钮、筛选区可能错位                            | 先迁移高密度区域并做英文手测                                                                   |
| 后端仍返回中文错误           | 英文 UI 混入中文                                        | 短期按 locale 返回本地化字符串，中期改结构化错误                                               |
| message key 失控             | 后续维护困难                                            | 前端按 Paraglide message 命名规则治理，后端按 Fluent message id 治理                           |
| 字符串拼接残留               | 复数和语序错误                                          | 动态文案统一走参数化模板，前端用 Paraglide 参数，后端用 Fluent 变量                            |
| 前端出现第二套语言来源       | 刷新、保存失败或多窗口场景下语言状态漂移                | 只允许通过 `get_preferences` / `set_preferences` 返回的 `AppPreferences.locale` 更新前端镜像   |
| Paraglide 生成与类型检查竞态 | clean checkout 下 `tsc` / `svelte-check` 找不到生成文件 | 用 Vite 插件覆盖 build，并让 `check:types` / `check:svelte` 显式调用 `i18n:generate`           |
| 前端 message 函数异常        | 生成文件损坏或极端运行时异常导致局部 UI 渲染失败        | 生成流程纳入 `bun run check`；关键壳层文案可用薄封装捕获异常并回退 `zh-CN` 或稳定 fallback key |
| 语言切换触发大范围重建       | 切换语言时播放器、列表或任务面板一次性卡顿              | 高频区域避免大范围 `{#key}`，优先用聚合 `$derived.by()` 控制重算粒度                           |
| 语言切换中断播放状态         | 正在播放的歌曲、进度、队列或音量被重置                  | 播放器只热更新 labels；禁止对播放器根节点使用 `{#key locale}`，手测切换语言时播放不中断        |
| 系统通知和前端 toast 不一致  | 用户感知割裂                                            | 后端通知读取同一个 `AppPreferences.locale`                                                     |

## 17. 最小可交付版本定义

建议拆成四个小 PR，降低 review 和回滚成本。

### PR1：设置面板切换链路打通

1. `doc/guides/i18n-inventory.md` 文件级粗清单。
2. 前端 Paraglide JS 基础设施。
3. `AppPreferences.locale` 最小字段、默认值、启动读取和 TS 类型对齐。
4. `SettingsSheet.svelte` 中实现语言选择项，必须通过 `set_preferences` 更新后端偏好后生效。
5. 1-2 个低风险组件的静态文案迁移，例如 `TopToolbar.svelte`、`AlbumWorkspaceContent.svelte`，作为肉眼验证点。
6. clean checkout 下 `bun run check` 能生成 Paraglide 输出并通过。
7. 按阶段 1B 的手动测试办法验证切换、重启恢复和保存失败回滚。

PR1 目标是证明设置面板选择语言可以真实驱动后端偏好、Svelte 5 响应式镜像、Paraglide runtime 和已迁移文案刷新。

### PR2a：前端主要静态文案

1. `SettingsSheet`、`DownloadTasksSheet`、`AlbumSidebar` 静态文案迁移。
2. `formatters.ts` 和 `downloadBadge.ts` 这类纯函数文案迁移。
3. `zh-CN` / `en-US` Paraglide message key 一致性检查。
4. 保持设置面板语言选择走 `set_preferences`；如仍处半迁移状态，普通用户正式可见性继续受 gating 控制。

PR2a 目标是把前端主要静态区域和基础格式化迁移完成，避免和后端 Fluent 改动混在一个 review 中。

### PR2b：后端 Fluent、通知、core 透传文案与偏好错误

1. 后端 Fluent / `fluent-templates` 基础设施。
2. 测试通知本地化。
3. `crates/siren-core/src/download/service.rs` 下载批次标题治理。
4. 偏好校验错误本地化。
5. `zh-CN` / `en-US` `.ftl` message id 一致性检查。

PR2b 目标是形成后端本地化能力，独立验证通知、core 透传文案、偏好错误和 `.ftl` fallback。

### PR2c：用户入口正式开放

1. 复核设置面板语言选择 UI 的可见性、文案和交互状态。
2. 移除或放宽开发/测试 gating，正式向普通用户开放。
3. 复测语言选择通过 `set_preferences` 提交，并以后端返回的 `AppPreferences.locale` 更新前端镜像。
4. 保存失败保持当前后端镜像语言不变，并显示本地化错误提示。
5. 过渡期入口开放条件检查。
6. 手动验收中英文主流程。

PR2c 目标是在覆盖率达标后正式向用户暴露已经实现的语言切换入口，形成首个用户可见语言切换闭环。复杂下载任务动态摘要和结构化错误可以放到后续 PR。
