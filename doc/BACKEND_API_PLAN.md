# 后端 API 实施计划

## 目标

基于当前代码结构，分阶段把下载能力从“单次阻塞命令”升级为“统一下载任务系统”，为以下能力提供后端基础：

- 批量下载
- 下载进度 UI
- 下载任务状态管理
- 取消 / 重试
- 结构化错误反馈

本计划强调 API-first，优先完成 Rust 后端领域建模和 Tauri 契约，再推动前端接入。

## 现状总结

### 当前优势

1. `crates/siren-core/src/downloader.rs` 已有 `download_song()`。
2. `crates/siren-core/src/downloader.rs` 已有 `download_album()`。
3. `DownloadProgress` 已可表达歌曲级与批次级进度。
4. 播放器已经证明“命令 + 完整快照事件”模式可行。
5. 前端已有成熟的 Tauri invoke / event 使用模式。

### 当前缺口

1. `src-tauri/src/main.rs` 没有下载任务管理器。
2. Tauri 层没有下载相关事件。
3. `src/lib/types.ts` 没有下载任务类型。
4. 前端仍依赖 `download_song -> string` 的同步结果模型。
5. 错误状态仍是字符串，不适合复杂任务 UI。

## 设计总原则

1. 复用现有下载核心，不重写 `siren-core` 下载流程。
2. 在共享库和 Tauri 层之间明确职责边界，避免下载任务系统继续堆进 `main.rs`。
3. 第一阶段采用单 worker 串行执行。
4. 与播放器保持一致，优先使用完整快照同步。
5. 先建立稳定契约，再改前端 UI。

## 库职责拆分方案

这一轮先冻结职责边界，避免实现过程中继续扩大 `src-tauri/src/main.rs` 和 `crates/siren-core/src/downloader.rs` 的耦合。

### 冻结职责边界

#### `crates/siren-core`

负责平台无关的领域能力：

- 上游 API 客户端
- 音频格式识别、保存和标签写入
- 下载任务模型
- 下载任务规划
- 下载执行 worker
- 下载服务 façade
- 结构化错误与重试 / 取消语义

#### `src-tauri`

负责应用壳层和 Tauri 集成：

- Tauri commands
- Tauri events
- `AppState` 组合与生命周期
- 平台路径策略，例如默认下载目录
- 播放缓存、媒体会话、窗口管理
- 把共享库下载事件桥接为前端可消费的 Tauri 事件

### 冻结目标结构

#### `siren-core`

```text
crates/siren-core/src/
├── api.rs
├── audio.rs
└── download/
    ├── mod.rs
    ├── model.rs
    ├── planner.rs
    ├── worker.rs
    ├── service.rs
    └── error.rs
```

#### `src-tauri`

```text
src-tauri/src/
├── app_state.rs
├── commands/
│   ├── mod.rs
│   ├── library.rs
│   ├── playback.rs
│   └── downloads.rs
├── downloads/
│   ├── mod.rs
│   ├── bridge.rs
│   └── events.rs
├── audio_cache.rs
├── theme.rs
└── player/
```

### 迁移原则

1. 先抽命令层，再拆下载层，不在一个提交里同时改 command 注册和下载服务实现。
2. 先保留兼容 façade，再切 UI，避免一次性破坏单曲下载功能。
3. `get_default_output_dir` 和缓存目录逻辑继续留在 `src-tauri`。

## 实施阶段

## Phase 0：接口规格冻结

### 目标

在正式编码前，先冻结下载任务系统的共享概念和命名。

### 输出

完整的类型定义、命令、事件、状态机规则和冻结决策已统一维护在 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md)。

### 完成定义

- 字段命名统一采用 `camelCase`
- Rust / TS 类型语义一致
- 命令返回结构与事件载荷关系明确
- 文档、实现、前端桥接使用同一套命名

## Phase 1：建立下载任务领域模型

### 目标

先在 `siren-core` 内引入下载任务系统的核心状态、规划与服务结构，再由 `src-tauri` 做桥接。

### 主要工作

1. 在 `crates/siren-core/src/download/` 下定义模型、错误、planner、worker、service。
2. 保留当前 `crates/siren-core/src/downloader.rs` 作为兼容 façade，内部逐步转调新模块。
3. 在 `src-tauri` 中新增 `app_state.rs`、`commands/`、`downloads/bridge.rs`。
4. 在 `AppState` 中新增下载服务与 Tauri 事件桥接依赖。
5. 设计 job / task ID 生成策略。

### 涉及文件

- `crates/siren-core/src/lib.rs`
- `crates/siren-core/src/downloader.rs`
- `crates/siren-core/src/download/mod.rs`
- `crates/siren-core/src/download/model.rs`
- `crates/siren-core/src/download/planner.rs`
- `crates/siren-core/src/download/worker.rs`
- `crates/siren-core/src/download/service.rs`
- `crates/siren-core/src/download/error.rs`
- `src-tauri/src/app_state.rs`
- `src-tauri/src/commands/mod.rs`
- `src-tauri/src/commands/downloads.rs`
- `src-tauri/src/downloads/bridge.rs`
- `src-tauri/src/downloads/events.rs`

### 完成定义

- 共享库能在内存中维护 job / task 快照
- 可创建单曲 job 和专辑 job 的初始结构
- Tauri 壳层不直接承载下载状态机实现
- 不要求此阶段真正执行下载

## Phase 2：接入执行器与事件系统

### 目标

让下载任务真正执行，并对外发出稳定事件。

### 主要工作

1. 在 `src-tauri/src/downloads/events.rs` 中定义事件名：
   - `download-manager-state-changed`
   - `download-job-updated`
   - `download-task-progress`
2. 在 `crates/siren-core/src/download/service.rs` 内部接入单 worker。
3. 串行消费 job / task，并通过回调或桥接层把共享库领域事件转发给 Tauri。
4. 调用 `download_song()` 与 `download_album()` 时，利用 `DownloadProgress` 更新 task 状态。
5. 把状态变化映射为：
   - job 完整快照事件
   - manager 概览快照事件
   - 高频进度事件

### 状态推进建议

#### 单曲任务

- `queued`
- `preparing`
- `downloading`
- `writing`
- `completed | failed | cancelled`

#### 专辑任务

- Job 负责聚合多个 task
- 任一 task 失败但其他继续完成时，Job 最终为 `partiallyFailed`

### 涉及文件

- `src-tauri/src/downloads/events.rs`
- `src-tauri/src/downloads/manager.rs`
- `crates/siren-core/src/downloader.rs`
- `src-tauri/src/main.rs`

### 完成定义

- 单曲 job 可真正执行
- 专辑 job 可真正执行
- 前端可通过事件拿到进度信息
- job 完成后能获取输出路径和错误信息

## Phase 3：暴露 Tauri command 契约

### 目标

把下载任务系统正式暴露给前端。

### 建议新增 commands

1. `create_download_job(request)`
2. `list_download_jobs()`
3. `get_download_job(jobId)`
4. `cancel_download_job(jobId)`
5. `cancel_download_task(jobId, taskId)`
6. `retry_download_job(jobId)`
7. `retry_download_task(jobId, taskId)`
8. `clear_download_history()`

说明：

- song / album / selection 统一通过 `create_download_job` 的 `kind` 和请求字段表达。
- 不再单独增加 `enqueue_album_download`，避免长期维护双入口。

### 迁移策略

建议采用短期兼容、尽快收口的方式：

1. 第一阶段可暂时保留旧 `download_song`。
2. 新前端接入后，逐步让 UI 全部改走任务接口。
3. 最终移除或内部废弃旧同步命令。

### 涉及文件

- `src-tauri/src/main.rs`
- `src/lib/api.ts`
- `src/lib/types.ts`

### 完成定义

- 所有下载相关入口都能通过新接口创建任务
- 前端不再必须等待下载完成的 Promise 才能更新 UI

## Phase 4：前端最小接入

### 目标

以前端最小成本验证后端 API 设计是否合理。

### 主要工作

1. 在 `src/lib/types.ts` 加入下载任务类型。
2. 在 `src/lib/api.ts` 加入下载任务 commands 和事件桥接。
3. 在 `src/App.svelte` 中引入下载任务状态：
   - 任务列表
   - 当前活动任务
   - 失败结果
4. 单曲下载按钮改为创建 job。
5. 增加最小下载状态展示：
   - 当前任务名
   - 完成数 / 总数
   - 当前歌曲进度
   - 完成 / 失败提示

### 不要求

- 第一版不要求完整下载中心 UI
- 第一版不要求复杂筛选和历史面板

### 完成定义

- 单曲下载可以展示实时进度
- 专辑下载可以展示总进度
- 失败能显示结构化信息

## Phase 5：任务增强能力

### 目标

在基础链路稳定后再加入增强功能。

### 主要工作

1. 支持取消整个 job。
2. 支持取消单个 task。
3. 支持重试整个 job。
4. 支持重试失败 task。
5. 增加历史清理能力。
6. 根据真实体验决定是否加入 session 内持久化。

### 完成定义

- 用户能处理失败任务
- 下载面板可展示完成、失败、取消等终态

## Phase 6：批量与多选入口扩展

### 目标

把统一任务系统扩展到未来产品入口。

### 主要工作

1. 增加“下载整张专辑”入口。
2. 增加多选下载请求构建逻辑。
3. 保证专辑下载、多选下载、单曲下载共用同一任务管线。

### 完成定义

- 所有下载能力统一接入 DownloadManager
- UI 侧不再区分“单曲下载逻辑”和“批量下载逻辑”

## 推荐命令与事件草案

命令、事件和错误模型的完整定义见 [BACKEND_API_CONTRACT.md](BACKEND_API_CONTRACT.md)。

## 测试策略

### Rust 单元测试

1. 下载状态流测试
2. Job / Task 聚合状态测试
3. 取消状态测试
4. 失败到 retry 的状态测试

### Rust 集成测试

1. 单曲任务创建和完成
2. 专辑任务创建和部分失败聚合
3. 事件发射节奏与快照结构

### 前端契约测试

1. TS 类型与后端字段一致性
2. 事件消费时的状态更新逻辑
3. 旧下载入口切换到任务入口后的行为验证

## 风险与缓解

### 风险 1：并发下载导致复杂度快速上升

缓解：

- 第一阶段固定单 worker 串行执行
- 并发作为后续增强项而不是基础项

### 风险 2：取消语义不清晰

缓解：

- 第一阶段只要求 cooperative cancellation
- 在网络下载块之间和歌曲边界生效
- 明确定义写文件阶段的行为

### 风险 3：旧下载接口长期存在造成双轨维护

缓解：

- 从一开始就把旧接口视为迁移兼容层
- 新 UI 优先只接新任务接口

### 风险 4：状态事件过于频繁导致前端负担过高

缓解：

- 进度事件只承载高频字段
- 大状态变化走完整快照事件

## 文件级实施清单

### Rust 后端

- `src-tauri/src/main.rs`
- `src-tauri/src/downloads/mod.rs`
- `src-tauri/src/downloads/types.rs`
- `src-tauri/src/downloads/state.rs`
- `src-tauri/src/downloads/events.rs`
- `src-tauri/src/downloads/manager.rs`
- `crates/siren-core/src/downloader.rs`

### 前端桥接

- `src/lib/api.ts`
- `src/lib/types.ts`
- `src/App.svelte`

## 每阶段完成后建议验证项

### Phase 1 后

- 能创建内存任务对象
- 类型结构稳定

### Phase 2 后

- 单曲 / 专辑任务能真正执行
- 事件能正确推送

### Phase 3 后

- 命令契约稳定
- 前后端接口可以独立联调

### Phase 4 后

- UI 可以看到任务进度
- 单曲与专辑下载可以共用任务系统

### Phase 5 后

- 取消 / 重试 / 清理形成完整闭环

## 建议的下一步执行顺序

1. 先实现 `downloads/types.rs` 和 `downloads/manager.rs`
2. 接着在 `main.rs` 中把 `AppState` 扩展为持有下载管理器
3. 再接入 commands / events
4. 然后更新 `src/lib/types.ts` 和 `src/lib/api.ts`
5. 最后改 `App.svelte` 的下载交互和 UI
