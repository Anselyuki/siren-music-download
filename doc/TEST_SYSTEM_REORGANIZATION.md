# 测试体系重组方案

## 1. 背景

当前仓库的主要问题不是“测试太多”，而是部分 Rust 场景测试与生产代码混在同一源码文件底部，导致：

- 生产文件过长，阅读和导航成本升高
- 测试职责边界不清，难以快速定位“纯逻辑”与“行为场景”
- 状态机 / 持久化 / 扫描类测试与实现耦合过深，影响后续维护

本方案的目标是在**不改变运行时行为、不调整对外契约、不为了测试迁移而扩大生产代码可见性**的前提下，建立更清晰的测试分层。

本方案同时受 [doc/REVIEW_RULES.md](REVIEW_RULES.md) 约束，尤其遵循以下规则：

- 行为保持优先
- 不为测试便利扩大可见性
- diff 小而明确
- 测试整理服务于“更快定位行为覆盖”

## 2. 分层原则

### 2.1 内联单元测试

保留在源码文件内的 `#[cfg(test)]` 测试，只负责覆盖以下内容：

- 纯函数
- 私有 helper
- 小范围边界值行为
- 明显依赖私有状态或私有实现细节的断言

适用标准：如果迁移该测试会要求把私有函数、私有字段或 `pub(crate)` 边界继续向外暴露，则该测试应继续内联。

### 2.2 crate 级场景测试

放在 `crates/<crate>/tests/` 下的测试，负责覆盖以下内容：

- 通过公开 API 即可观察的行为场景
- 状态恢复、状态聚合、重试/取消等领域行为
- 跨多步调用才能看清的规则

这类测试按“场景域”命名，而不是按源码文件命名。

### 2.3 契约测试

后续阶段可考虑补充围绕 command / event / 共享结构的契约测试，用于守住：

- Tauri command 参数/返回值
- 事件载荷
- 文档契约与实现之间的一致性

本轮不执行。

### 2.4 前端测试

后续阶段可考虑补充：

- Vitest 单元测试：纯逻辑、store、cache、轻量 UI 行为
- 组件测试：关键交互组件
- 少量 E2E：主流程冒烟

本轮不执行。

## 3. 当前仓库的阶段划分

## Phase 1：先处理 `siren-core`

本阶段只处理已经具备良好 crate 边界的 `siren-core`。

### 执行范围

- 文档新增：`doc/TEST_SYSTEM_REORGANIZATION.md`
- 测试迁移：`crates/siren-core/src/download/service.rs`
- 新增 integration tests：`crates/siren-core/tests/download_service_scenarios.rs`

### 迁移原则

迁出到 integration tests 的测试应满足：

- 可以通过 `siren_core::DownloadService` 的公开 API 表达
- 不需要访问私有字段或私有 helper
- 断言的是行为场景，而不是内部实现过程

继续保留在 `service.rs` 内联的测试包括：

- 时间格式辅助逻辑
- 选择任务标题生成逻辑
- 依赖私有状态的重试 / finish 边界行为
- 依赖内部还原细节的断言

### Phase 1 已验证的边界

#### `download/service.rs`

本轮实际迁移后，已经确认以下测试**适合外移**到 `crates/siren-core/tests/download_service_scenarios.rs`：

- `restores_service_from_manager_snapshot`
- `recomputes_job_status_from_restored_tasks`
- `can_retry_restored_failed_task`

同时也已确认以下测试**不适合外移**，应继续保留内联：

- `keeps_cancelled_status_when_finishing_cancelled_job`
- `ignores_retry_for_active_task_until_worker_exits`

原因不是它们“不重要”，而是它们的断言依赖 `DownloadService` 的内部执行态构造，例如 `active_job_id`、`active_task_id` 与内部运行中状态。仅靠公开 `DownloadManagerSnapshot` 无法无损重建这些条件；如果为了迁移它们而扩大生产代码可见性，会违反 [doc/REVIEW_RULES.md](REVIEW_RULES.md) 的约束。

#### `local_inventory/mod.rs`

本轮也已确认，以下这类测试**适合整批外移**到 `crates/siren-core/tests/local_inventory_badges.rs`：

- 候选路径生成与匹配规则测试
- `track_badge_from_matches(...)` 的规则测试
- `aggregate_album_download_badge(...)` 的聚合测试
- `album_badge_from_evidence(...)` 的保守提示测试
- `is_downloaded_status(...)` / `badge_for_detected_file(...)` 这类公开规则函数测试

这批测试可以安全外移的原因是：

- 断言对象全部是公开函数返回值
- 不依赖私有 helper 或私有字段
- 不需要构造内部运行态
- 外移后仍然保持“规则契约测试”的语义，没有改变覆盖重点

#### `audio.rs`

本轮已确认，`audio.rs` 适合采用**部分外移**的方式：

- 保留在 `crates/siren-core/tests/audio_flac.rs` 的，是 `save_audio(...) + tag_flac(...)` 这种公开音频工作流测试
- 保留在 `crates/siren-core/src/audio.rs` 内联的，是 `detect_image_mime(...)`、`encode_cover_as_jpeg(...)` 这类更偏 helper 的测试

这样拆分的原因是：

- FLAC 转换与标签写入是明确的公开工作流，适合作为 integration tests 维护
- 封面 MIME 识别和 JPEG 归一化虽然也是公开函数，但更像底层实现 helper；如果把它们一并迁到 `tests/`，会提高它们作为“稳定外部契约”的维护预期

因此，`audio.rs` 更适合采用 **工作流外移、helper 留内联** 的混合模式。

#### `api.rs`

`api.rs` 当前**不建议继续外移**现有测试，至少不应在这一轮测试整理里做。

原因是当前黑盒测试依赖了只存在于内联测试里的构造入口：

- `new_for_test(base_url, capacity)`

这使得现有测试可以稳定控制：

- mock server base URL
- cache capacity
- 缓存命中 / 不缓存失败 / LRU 驱逐等行为

如果直接把这些测试迁到 crate-level `tests/`，通常会面临两种选择：

1. 为测试新增或放宽生产可见性接口
2. 重写测试方式，改变当前测试控制面的结构

这两种选择都不适合作为本轮“只做测试体系整理”的范围。因此，`api.rs` 目前应继续保留内联测试，除非后续单独立项做 API client 的可测试边界重构。

## Phase 2：评估 `src-tauri` 的可测试边界

> 更新（2026-04-22）：本节中的“binary-only crate”判断是 Phase 2 边界重构执行前的评估记录。
> 当前 `src-tauri` 已完成 `src/lib.rs` + `[lib]` 引入，形态已调整为 **library + binary 双目标**；
> 但本节关于“不要机械迁移现有测试”“不要为测试便利扩大可见性”的结论仍然有效。

关注文件：

- [src-tauri/src/download_session.rs](../src-tauri/src/download_session.rs)
- [src-tauri/src/local_inventory.rs](../src-tauri/src/local_inventory.rs)
- [src-tauri/Cargo.toml](../src-tauri/Cargo.toml)
- [src-tauri/src/main.rs](../src-tauri/src/main.rs)

以下为改造前的评估记录；当前仅第 1 步 crate 形态调整已单独完成，其余测试迁移仍不在本轮内执行，原因如下：

1. 在这轮评估发生时，`src-tauri` 还是 binary-only crate
2. 相关测试依赖私有实现细节或 `pub(crate)` 边界
3. 即使当前已具备 library target，若直接外移到 crate-level `tests/`，通常仍需要额外设计稳定测试边界，而不是机械迁移
4. 这会违反 [doc/REVIEW_RULES.md](REVIEW_RULES.md) 中“不为测试便利扩大可见性”的规则

因此，`src-tauri` 的测试外移必须作为**单独的边界重构议题**评审，而不是夹在本轮测试整理里一起做。

### Phase 2 已验证的边界

#### 当前 crate 结构约束

在这轮评估发生时，`src-tauri` 是 **binary-only crate**：

- [src-tauri/Cargo.toml](../src-tauri/Cargo.toml) 只有 `[[bin]]`
- [src-tauri/src/main.rs](../src-tauri/src/main.rs) 通过 `mod download_session;`、`mod local_inventory;` 等方式声明内部模块
- 当时没有 `src/lib.rs`，因此 crate-level integration tests 无法直接导入这些模块

当前状态更新：

- `src-tauri` 已拥有 `src/lib.rs` 与 `[lib]` target
- [main.rs](../src-tauri/src/main.rs) 已收缩为 Tauri 启动、wiring 与 command 注册入口
- 当前仍未开始把 `download_session.rs` / `local_inventory.rs` 现有内联测试机械迁到 `src-tauri/tests/`

这意味着：

- 现有 `#[cfg(test)]` 测试之所以能工作，是因为它们和实现位于同一模块内部
- 虽然 crate 形态问题已经解决，但 `src-tauri/tests/*.rs` 仍不应直接复用依赖私有实现细节的测试写法

#### `download_session.rs`

[download_session.rs](../src-tauri/src/download_session.rs) 现有测试大量依赖以下内部对象：

- `DownloadSessionStore`（`pub(crate)`）
- `normalize_restored_snapshot(...)`（private）
- `apply_retention(...)`（private）

因此，以下测试应继续保留内联：

- `normalizes_interrupted_tasks_on_restore`
- `retention_keeps_most_recent_terminal_jobs`
- `concurrent_saves_do_not_race_on_temp_file`（更接近内部保存锁与临时文件实现语义）

其中一部分文件系统 roundtrip 测试，理论上将来可以迁出，但前提不是“直接搬”，而是先设计一个稳定的持久化 facade。

#### `local_inventory.rs`

[local_inventory.rs](../src-tauri/src/local_inventory.rs) 现有测试几乎都依赖内部实现细节，例如：

- `LocalInventoryService`（`pub(crate)`）
- `collect_local_audio_evidence(...)`（private）
- `track_badge_for_song(...)`（private）
- `complete_scan(...)`（private）
- `ScanCollectionOutcome`（private）

因此，当前这些测试不适合直接迁出：

- `track_badge` 子模块
- `scan` 子模块
- `provenance` 子模块

`enrich` 相关测试相对更接近服务行为，但目前仍依赖 `pub(crate)` service 和 private scan 完成路径，也不应在当前阶段强行外移。

### Phase 2 推荐方向

如果未来确实要为 `src-tauri` 增加 crate-level integration tests，推荐顺序是：

1. 先单独设计 `src-tauri/src/lib.rs`，让 `src-tauri` 拥有 library target
2. 让 `main.rs` 只负责 Tauri 启动、wiring 和 app setup
3. 只暴露少量真正值得稳定测试的高层边界
4. 保留 normalization / retention / scan pipeline / provenance 等内部实现测试继续内联

#### `src-tauri/src/lib.rs` 的最小边界设计

推荐采用 **library + binary 双目标**，但保持最小边界：

- `src-tauri/src/lib.rs` 负责成为后端模块的 crate root
- [main.rs](../src-tauri/src/main.rs) 继续只负责可执行入口和 Tauri wiring
- 引入 `lib.rs` 的目标是解决 crate 形态问题，而不是顺手放大内部可见性

`lib.rs` 适合接管的内容：

- `app_state`
- `audio_cache`
- `commands`
- `download_session`
- `downloads`
- `local_inventory`
- `local_inventory_provenance`
- `logging`
- `notification`
- `player`
- `preferences`
- `theme`

`main.rs` 应继续保留：

- `fn main()`
- Tauri `Builder` / plugin 注册
- window 初始化与生命周期处理
- command handler 注册
- `AppState::new(...)`
- `downloads::bridge::initialize(...)`
- `local_inventory::spawn_inventory_scan(...)`
- 退出时日志 flush

即使引入 `lib.rs`，以下内容仍应保持 crate-private：

- `app_state` 的大部分字段与方法
- `download_session` 的 normalization / retention helper
- `local_inventory` 的 scan / provenance / collect helper
- `downloads::bridge` 的执行循环与 write worker 细节
- 各种事件发射 helper、日志与持久化内部实现

如果后续确实需要更高层测试 seam，推荐只暴露非常窄的高层边界，例如：

- download session 的持久化 facade
- local inventory 的高层扫描入口
- 后台服务 bootstrap seam

不推荐暴露：

- helper 函数本身
- 内部状态归一化细节
- write worker / scan pipeline 的实现级对象

这个设计的核心原则是：

- 先让 `src-tauri` 从 binary-only 变成 library + binary
- 再按需抽高层 seam
- 不把 crate 结构调整和大规模测试迁移混在同一轮 diff 中

### Phase 2 明确不推荐

当前阶段不推荐：

- 直接把 [download_session.rs](../src-tauri/src/download_session.rs) 或 [local_inventory.rs](../src-tauri/src/local_inventory.rs) 的现有测试机械迁到 `tests/`
- 为了测试迁移把 private / `pub(crate)` 大面积改成 `pub`
- 在没有单独边界方案的情况下，把 crate 结构调整和测试迁移混进同一轮 diff

## Phase 3：补齐契约测试与前端测试

后续可考虑：

- Tauri command / event 契约测试
- 前端 Vitest 单元测试
- 组件测试
- 少量 E2E 冒烟测试

本轮不执行。

## 4. Phase 1 的目标文件布局

```text
crates/siren-core/
├── src/
│   └── download/
│       └── service.rs        # 保留私有 helper / 小范围内联测试
└── tests/
    └── download_service_scenarios.rs
```

原则：只有在测试 setup 重复已经明显影响可读性时，才新增最小化的 `tests/support/`。本轮优先避免过早抽象。

## 5. Phase 1 非目标（历史范围说明）

本轮明确不做：

- 不迁移 `src-tauri` 测试到 crate-level `tests/`
- 不把 `src-tauri` 改成 library + binary 双目标（该事项已在后续独立边界重构中单独完成，不属于 Phase 1 diff）
- 不新增前端测试框架
- 不修改业务规则、状态机语义、Tauri command、事件载荷或共享数据结构
- 不为测试迁移新增生产可见性
- 不顺带做与测试体系无关的清理

## 6. 验证要求

Phase 1 完成后，至少应验证：

1. `cargo test -p siren_core`
2. `cargo test -p siren_core --test download_service_scenarios`
3. `cargo test --workspace`

此外还应人工确认：

- `service.rs` 的内联测试块缩小，但私有逻辑覆盖仍在
- 没有为测试外移新增 `pub` / `pub(crate)`
- 文档描述与实际改动范围一致
- diff 没有混入无关逻辑修改

## 7. 审批关注点

进入评审时，应重点检查：

- 测试分组是否比之前更容易定位行为覆盖
- 新的 integration tests 是否只依赖公开 API
- 是否有测试因为迁移而偷偷放宽了生产代码边界
- 是否保持了原有断言语义，而不是借重组顺手改规则
- 是否严格把 `src-tauri` 留在后续阶段，没有混入结构性边界改动
