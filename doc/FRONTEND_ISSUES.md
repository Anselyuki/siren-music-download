# 前端问题清单

> 基于 2026-04-16 代码审查，`src/App.svelte` 当前 **3319 行**。
> 配套重构方案见 [APP_SVELTE_SPLIT_PLAN_B.md](./APP_SVELTE_SPLIT_PLAN_B.md)。

---

## 问题一览

| # | 优先级 | 类型 | 问题 | 涉及行 |
|---|---|---|---|---|
| 1 | 🔴 高 | 性能 | 下载进度事件每次重建整个 `downloadManager` 对象 | ~1458 |
| 2 | 🔴 高 | 体验 | `alert()` 滥用，11 处阻塞式弹窗 | 多处 |
| 3 | 🟡 中 | 代码质量 | 死状态 `hasPrevious` / `hasNext` 从未被消费 | 113–114 |
| 4 | 🟡 中 | 代码质量 | `downloadingSongCid` / `downloadingAlbumCid` 与 `downloadManager` 状态重复 | 128–129 |
| 5 | 🟡 中 | 可维护性 | 设置面板状态声明与其他状态相距 1500+ 行 | 1669 |
| 6 | 🟡 中 | 正确性 | `downloadLyrics` 偏好写回存在初始化竞态窗口 | 1315 |
| 7 | 🟢 低 | 可维护性 | 动效工具函数（10+）内联在 App.svelte，与 AudioPlayer 存在重复 | 1057–1249 |
| 8 | 🟢 低 | 可维护性 | 设置面板、下载面板、骨架屏 UI 全部内联，无对应组件文件 | 模板区 |

---

## 详细说明

### 1. 下载进度事件重建整个 `downloadManager`（性能）

**位置**：`src/App.svelte` ~1458 行，`download-task-progress` 事件处理器。

**问题**：每次进度事件都执行：

```ts
downloadManager = { ...downloadManager, jobs: updatedJobs };
```

这会重建整个 `DownloadManagerSnapshot` 对象，触发所有消费 `downloadManager` 的 `$derived` 和模板重新求值。10+ 任务并行下载时，高频进度事件会导致掉帧。

**修复方向**：按 Plan B §7 拆分为三个独立 `$state`：

```ts
let jobs = $state<DownloadJobSnapshot[]>([]);
let taskSpeedMap = $state(new Map<string, number>());
let managerMeta = $state<Omit<DownloadManagerSnapshot, 'jobs'> | null>(null);
```

进度事件只更新 `taskSpeedMap` 和对应 task 的字节字段，不触碰 `jobs` 结构。

---

### 2. `alert()` 滥用（用户体验）

**位置**：11 处，分布在下载创建、缓存清理、通知测试等操作的回调中。

**问题**：原生 `alert()` 阻塞主线程，样式与应用完全不一致，无法自动消失。

**涉及场景**：
- 单曲/整专/批量下载重复创建提示（~806、~894、~911 行）
- 下载创建失败（~810、~898、~915 行）
- 清除音频缓存成功/失败（~1692、~1699 行）
- 发送测试通知成功/失败（~1710、~1713 行）
- 清理下载历史（~2050、~2054 行）

**修复方向**：实现轻量 toast/snackbar 组件，替换所有 `alert()` 调用。

---

### 3. 死状态 `hasPrevious` / `hasNext`（代码质量）

**位置**：`src/App.svelte` 113–114 行声明，518–519 行从后端同步赋值。

**问题**：传给 `AudioPlayer` 的实际是 `playerHasPrevious` / `playerHasNext`（第 175–176 行，基于本地队列长度的 `$derived`），后端同步的 `hasPrevious` / `hasNext` 从未被消费，是无效状态。

**修复方向**：删除 `hasPrevious`、`hasNext` 两个 `$state` 及 `syncPlayerState` 中对应的赋值行。

---

### 4. 下载"创建中"状态与 `downloadManager` 重复（代码质量）

**位置**：`src/App.svelte` 128–129 行（`downloadingSongCid` / `downloadingAlbumCid`），776–844 行使用。

**问题**：这两个状态用于在 `createDownloadJob` 返回前显示"创建中"的乐观 UI，但 `downloadManager.jobs` 里已有 `queued` 状态可以表达同样的语义。两套状态并存，逻辑分散，容易出现不一致（如创建失败时需要手动清空）。

**修复方向**：在 `createDownloadJob` 返回后立即刷新 `downloadManager`（或依赖 `download-manager-state-changed` 事件），用 `queued` 状态驱动 UI，移除乐观状态变量。

---

### 5. 设置面板状态声明位置漂移（可维护性）

**位置**：`src/App.svelte` 1669 行。

**问题**：`settingsOpen`、`isClearingAudioCache`、`downloadLyrics`、`notifyOnDownloadComplete` 等设置相关状态声明在脚本块中间，与文件顶部的其他状态相距 1500+ 行，阅读时需要大幅跳转。

**修复方向**：短期将设置相关状态集中到顶部状态区；长期按 Plan B Phase 4 抽取 `SettingsPanel.svelte`。

---

### 6. `downloadLyrics` 偏好写回竞态（正确性）

**位置**：`src/App.svelte` 1315 行（`$effect`），1383 行（`onMount` 读取）。

**问题**：`$effect` 用 `downloadLyricsPrefReady` 守卫写回 localStorage，但 `downloadLyricsPrefReady` 本身是 `$state`，在 `onMount` 异步完成前为 `false`。如果 Svelte 在 `onMount` 完成前触发 `$effect`（理论上不会，但依赖执行顺序），初始值 `true` 会覆盖用户存储的偏好。

当前实现依赖"$effect 在 onMount 之后运行"的隐式假设，不够显式。

**修复方向**：在 `onMount` 中同步读取 localStorage 后再设置 `downloadLyricsPrefReady = true`，确保写回 `$effect` 只在读取完成后激活（当前已基本如此，但应加注释说明时序依赖，避免后续改动破坏）。

---

### 7. 动效工具函数内联且存在重复（可维护性）

**位置**：`src/App.svelte` 1057–1249 行（`motionTransition`、`fadeEnter`、`fadeExit`、`axisEnter`、`axisExit`、`modeButtonAnimate`、`toolbarButtonAnimate`、`appButtonAnimate` 等 10+ 函数）。

**问题**：这些纯函数与业务逻辑混在同一文件。`AudioPlayer.svelte`（1089 行）中也有类似的动效辅助逻辑，存在重复定义。

**修复方向**：按 Plan B Phase 1 抽取到 `src/lib/features/shell/motion.ts`，两个文件共享同一套工具。

---

### 8. 面板 UI 全部内联（可维护性）

**位置**：App.svelte 模板区。

**问题**：
- 设置面板（约 200 行）
- 下载任务面板（约 150 行）
- 专辑舞台骨架屏（约 80 行）

全部内联在 App.svelte 模板中，没有对应的独立组件文件，导致模板区超过 1000 行。

**修复方向**：按 Plan B Phase 4 抽取 `SettingsPanel.svelte` 和 `DownloadTasksPanel.svelte`。

---

## 修复优先级路线图

```
立即可做（不依赖大重构）
├── 问题 3：删除死状态 hasPrevious / hasNext（5 分钟）
├── 问题 1：拆分 taskSpeedMap，修复进度事件性能（1-2 小时）
└── 问题 2：实现 toast 组件，替换 alert()（2-4 小时）

中期（配合 Plan B Phase 1）
├── 问题 7：抽动效工具函数到 motion.ts
└── 问题 5：集中设置状态声明位置

长期（Plan B Phase 2-6）
├── 问题 4：移除乐观下载状态
├── 问题 6：显式化偏好写回时序
└── 问题 8：抽取面板组件
```
