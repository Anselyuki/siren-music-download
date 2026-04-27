# i18n 文件级文案粗清单

> 本文档记录待迁移文件范围和当前状态，不维护独立 key 级清单。
> 具体 message key 以 Paraglide message 文件、后端 Fluent `.ftl` 文件和对应 PR description 为准。
>
> 首轮语言：`zh-CN`（基准）、`en-US`
>
> 最后更新：2026-04-27

## 前端文案清单

| 文件                                                  | 业务域   | 主要类型                 | 估算条数 | 状态    |
| ----------------------------------------------------- | -------- | ------------------------ | -------- | ------- |
| `src/App.svelte`                                      | shell    | toast / dynamic          | 4        | pending |
| `src/lib/components/app/TopToolbar.svelte`            | shell    | title / aria             | 3        | pending |
| `src/lib/components/app/AlbumWorkspaceContent.svelte` | library  | static                   | 2        | pending |
| `src/lib/components/app/AlbumDetailSkeleton.svelte`   | library  | static                   | 1        | pending |
| `src/lib/components/app/AlbumSidebar.svelte`          | library  | static / dynamic / aria  | 14       | pending |
| `src/lib/components/app/AlbumDetailPanel.svelte`      | library  | static / dynamic         | 12       | pending |
| `src/lib/components/SongRow.svelte`                   | library  | aria / title / dynamic   | 8        | pending |
| `src/lib/components/AudioPlayer.svelte`               | player   | aria / dynamic           | 14       | pending |
| `src/lib/components/app/PlayerFlyoutStack.svelte`     | player   | static / aria            | 4        | pending |
| `src/lib/components/app/SettingsSheet.svelte`         | settings | static / toast / dynamic | 28       | pending |
| `src/lib/components/app/DownloadTasksSheet.svelte`    | download | static / aria            | 18       | pending |
| `src/lib/features/download/controller.svelte.ts`      | download | toast / dynamic          | 10       | pending |
| `src/lib/features/download/formatters.ts`             | download | dynamic                  | 9        | pending |
| `src/lib/downloadBadge.ts`                            | common   | static                   | 6        | pending |

## 后端文案清单

| 文件                                        | 业务域       | 主要类型 | 估算条数 | 状态    |
| ------------------------------------------- | ------------ | -------- | -------- | ------- |
| `src-tauri/src/notification/mod.rs`         | notification | backend  | 4        | pending |
| `src-tauri/src/notification/macos.rs`       | notification | backend  | 2        | pending |
| `src-tauri/src/notification/desktop.rs`     | notification | backend  | 2        | pending |
| `src-tauri/src/preferences.rs`              | preferences  | backend  | 12       | pending |
| `crates/siren-core/src/download/service.rs` | download     | backend  | 5        | pending |

## 不翻译的内容

- 专辑名、歌曲名、艺术家名、歌词（上游 API 返回的内容数据）
- 日志 key、内部错误 key、Rust / TS 类型名、Tauri command 名称
- rustdoc、开发文档、README
- 构建产物安装包元信息

## Fallback 策略

- 前端：Paraglide message 缺 key 时编译期报错；运行时缺语言回退 `zh-CN`
- 后端：Fluent 目标语言缺 key 时回退 `zh-CN`；`zh-CN` 仍缺失时返回 message id
- 参数缺失：Paraglide 编译期类型检查；Fluent 使用 `{$param}` 原样输出
