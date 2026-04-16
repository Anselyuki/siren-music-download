# UI Refactor QA Checklist

## Baseline Notes

- `cargo build` currently fails before frontend build because `src-tauri/tauri.conf.json` points `frontendDist` to `../dist`, and `dist/` does not exist in a fresh worktree until `npm run build` completes.
- Initial `npm install` in the worktree may be slow enough to hit the default command timeout. Retry if it times out once.
- `npm run check:types` passes after adding `$lib` path mapping and TypeScript deprecation suppression.
- `npm run check:build` passes after installing the Windows rolldown native binding.
- `npm run check:svelte` still reports 21 pre-existing type issues, mainly:
  - `src/lib/components/SongRow.svelte` implicit event parameter types
  - `src/App.svelte` nullability around `downloadManager`
  - `src/App.svelte` motion transition typing mismatch
- These `svelte-check` failures are the current baseline and must trend downward during later tasks.

## Core Flows

- [ ] 首屏可加载专辑列表
- [ ] 切换专辑后详情刷新正常
- [ ] 单曲播放 / 暂停 / 恢复 / seek 正常
- [ ] 上一首 / 下一首 / 乱序 / 循环正常
- [ ] 歌词面板显示和高亮正常
- [ ] 播放列表面板显示和切歌正常
- [ ] 单曲下载正常
- [ ] 整专下载正常
- [ ] 多选下载正常
- [ ] 下载取消 / 重试 / 清理历史正常
- [ ] 设置项持久化正常

## Visual Checks

- [ ] 亮色主题正常
- [ ] 暗色主题正常
- [ ] 动态专辑主题色正常
- [ ] 右侧面板玻璃材质正常
- [ ] 底部 Dock 玻璃材质正常
- [ ] 主工作区保持干净，无大面积玻璃化
- [ ] Apple 化排版层级正常
- [ ] reduced-motion 正常

## Completion Notes

- Refactor complete date: 2026-04-17
- Final verification commands passed:
  - `npm run check`
  - `npm run check:cargo`
- Remaining follow-ups:
  - `cargo check` 仍有一个既有 warning：`src-tauri/src/notification/desktop.rs` 里的 `Manager` 未使用
