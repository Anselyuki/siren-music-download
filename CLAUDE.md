# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## 项目概览

- 技术栈：Rust + Tauri 2 + Vite + Svelte 5
- 形态：跨平台桌面应用（当前发布产物覆盖 macOS / Windows）
- 当前重点：下载、日志与本地库存链路已稳定；库内搜索 MVP 已落地，当前主线聚焦于在既有 controller + shell 架构上持续收敛实现与文档

## 常用命令

```bash
bun install
bun run tauri:dev
bun run format
bun run format:check
bun run lint
bun run check
bun run build
bun run tauri:build

cargo fmt --all
cargo check --workspace
cargo clippy --workspace --all-targets
cargo test --workspace

# 文档
cargo doc -p siren_core --no-deps
cargo doc -p siren-music-download --lib --no-deps --document-private-items
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

## 关键入口

- `src-tauri/src/main.rs`：Tauri 入口、command 注册与应用启动 wiring
- `src-tauri/src/app_state.rs`：后端共享状态组合，聚合播放器、下载、库存、偏好、日志与搜索服务
- `src/App.svelte`：前端根装配层，负责 controller 初始化、Tauri 事件订阅、跨域状态协调与壳层组件编排
- `src/lib/api.ts`：主 Tauri command bridge
- `src/lib/settingsApi.ts`：设置面板专用 IPC bridge
- `src/lib/types.ts`：前后端共享数据结构
- `src/lib/features/`：按 `env / library / player / download / shell` 划分的领域目录
- `src/lib/components/app/`：前端壳层组件目录

## 真相来源

- **后端契约真相**：`doc/reference/backend-api-contract.md`
- **前端架构真相**：`doc/guides/frontend-guide.md`
- **发布流程真相**：`.github/workflows/ci.yml`、`.github/workflows/distribute.yml` 与 `doc/process/release-process.md`
- **阶段记录**：`doc/history/backend-completed-phases.md`、`doc/history/backend-pending-phases.md`
- **技术决策背景**：`doc/history/decisions.md`

## 当前实现状态

- **已稳定**：Phase 1–10、下载历史增强、日志 viewer、偏好系统、本地库存标记链路、Phase 12A 库内搜索 MVP
- **已部分落地**：Phase 12B 的 `intro / belong` 命中表达、拼音召回与搜索排序增强
- **仍在演进**：Phase 11 的条件触发型后端增强、Phase 12B 剩余搜索增强、Phase 12C 歌词检索，以及前端 controller / 文档收敛

## 代码层约定

- 后端“端点”指的是 Tauri command，不是 HTTP server route
- 前端相关实现一律以 Svelte 5 为最高优先级：只要涉及新增前端代码、组件重构、状态组织、响应式写法、组合方式或语法选择，默认必须优先采用 Svelte 5 官方推荐模式；除非用户明确要求，否则不要为了延续旧习惯而主动回退到旧版写法或保守兼容模式
- 前端代码与 Markdown 文档默认使用 Prettier 统一格式化；前端静态规则检查默认使用 ESLint；Rust 代码格式化默认使用 `cargo fmt --all`；`bun run check` 默认包含格式、lint、类型、前端构建与 `cargo check --workspace`，`cargo test --workspace` 需单独执行
- 共享数据结构优先在 Rust 侧定义，再让前端 `types.ts` 保持形状一致
- 所有对外暴露的 API 都必须编写函数文档，且文档内容统一使用中文；函数文档至少要说明用途、入参语义、出参/返回值语义以及关键副作用或错误场景；对于层级较高、承担入口职责的 API，还应补充说明适用场景、何时使用、使用注意事项与必要的调用约束；如涉及明确契约边界，还应写清前置条件、状态约束、不变量、是否幂等、是否允许重试等信息；从调用者视角出发，在有必要时补充返回数据的稳定性/兼容性预期、常见调用顺序与最小可用示例；新增或修改对外 API 时同步补齐或更新对应文档；在可行时尽量补充文档测试
- 所有公开模块（尤其会进入 rustdoc 模块列表的 `pub mod`）都必须补充模块级 rustdoc，且文档内容统一使用中文；模块文档至少要概括该模块当前公开职责、主要暴露能力与典型使用场景，避免生成的模块列表只有名称没有说明；如果模块职责发生变化，要同步更新模块级 rustdoc，保证 rustdoc 首页、模块页与实际导出能力一致
- 如果改了 command 参数、返回值或事件载荷，要同步更新：
  - `src/lib/api.ts`
  - `src/lib/types.ts`
  - `README.md`
  - `src-tauri` / `siren_core` 中对应的 rustdoc
- 如果改了歌词、下载设置或播放器交互，同时检查 `src/App.svelte` 和 `src/lib/components/AudioPlayer.svelte` 的状态同步
- UI 展示组件不要直接调用 `invoke` / `listen`；统一走 bridge、controller 或具备明确边界的 shell 层
- 结构性重构、测试整理与文档补充默认视为行为保持变更；不要改业务分支语义、状态流转顺序、事件顺序、错误语义或日志 key
- 涉及并发、异步或后台任务时，不跨 `await` 持有锁，不改变 cancel / stop / worker 生命周期，也不改变资源清理顺序
- 测试整理优先按“内联单元测试 / crate 级场景测试 / 契约测试 / 前端测试”分层理解：依赖私有 helper、私有状态或内部执行态的测试继续保留内联；只有通过公开 API 就能稳定表达的行为场景，才适合迁到 `crates/<crate>/tests/`
- 不要为了测试迁移放大生产代码可见性；若外移测试会迫使 private / `pub(crate)` 边界继续外扩，应优先保留原地测试或单独设计高层测试 seam
- 涉及文件系统路径、缓存路径、下载输出路径或持久化路径的测试，不要写死平台分隔符；优先比较 `Path` / `PathBuf` 语义，或先做统一规范化后再比较，并避免只在 macOS 本地成立的断言
- 新增或整理测试时，优先按行为域、规则域、场景域分组，避免为了 DRY 过度抽象测试代码，也不要改变原有断言语义
- 未经用户明确指示，不要新建分支；默认在当前分支上工作，涉及分支切换、新建分支、基于分支的推送或 PR 准备时先确认
- 所有提交、PR 及相关 git / GitHub 协作文案一律使用中文
- 如果本轮改动属于测试整理、结构性重构或审批材料补充，优先对照 `doc/guides/review-rules.md` 中的通用规则，而不是把实现细节写进审批文档
