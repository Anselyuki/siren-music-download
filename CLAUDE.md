# CLAUDE.md

This file provides guidance to Claude Code when working with this repository.

## 项目概览

- 技术栈：Rust + Tauri 2 + Vite + Svelte 5
- 形态：跨平台桌面应用（macOS / Windows / Linux）
- 当前重点：专辑浏览、在线播放、当前曲目下载已经接通；批量下载和下载进度还没完成

## 常用命令

```bash
npm install
npm run tauri:dev
npm run build
npm run tauri:build

cargo check --workspace
cargo fmt --all
cargo clippy --workspace --all-targets

# 文档
cargo doc -p siren_core --no-deps
cargo doc -p siren-music-download --bin siren-music-download --no-deps --document-private-items
```

## 仓库结构

```text
Cargo workspace
├── src-tauri/               # Tauri 后端二进制 crate
│   └── src/
│       ├── main.rs          # Tauri command 入口
│       ├── audio_cache.rs   # 流式播放缓存
│       ├── theme.rs         # 封面取色
│       └── player/          # 播放器实现
└── crates/
    └── siren-core/          # 共享 Rust 核心库
        └── src/
            ├── api.rs       # 上游 HTTP API 客户端
            ├── audio.rs     # 音频格式检测 / 保存 / FLAC 标记
            ├── downloader.rs # 下载流程
            └── lib.rs       # 对外导出
```

前端位于仓库根目录：

- `src/App.svelte`：主界面和状态编排
- `src/lib/api.ts`：Tauri command bridge
- `src/lib/types.ts`：前后端共享数据结构的 TS 版本
- `src/lib/components/`：播放器、专辑卡片、曲目行等组件

## 后端 command 清单

`src-tauri/src/main.rs` 当前注册了这些 Tauri command：

- `get_albums`
- `get_album_detail`
- `get_song_detail`
- `extract_image_theme`
- `get_default_output_dir`
- `play_song`
- `stop_playback`
- `pause_playback`
- `resume_playback`
- `seek_current_playback`
- `play_next`
- `play_previous`
- `get_player_state`
- `set_playback_volume`
- `download_song`
- `clear_audio_cache`

播放器事件：

- `player-state-changed`
- `player-progress`

## 当前实现状态

### 已完成

- 专辑列表和曲目详情加载
- Tauri command + event 通信链路
- 在线播放、暂停、恢复、拖动进度
- 上一首 / 下一首
- 当前专辑上下文播放
- 底部播放器、歌词面板、播放队列面板
- 系统媒体会话同步
- 封面主题色提取
- 流式播放缓存与缓存清理
- 当前播放曲目单曲下载
- FLAC 元数据和封面写入

### 未完成

- 工具栏多选批量下载
- 下载进度 UI
- 更完整的错误提示和任务状态管理

## 代码层约定

- 后端“端点”指的是 Tauri command，不是 HTTP server route
- 共享数据结构优先在 Rust 侧定义，再让前端 `types.ts` 保持形状一致
- 如果改了 command 参数、返回值或事件载荷，要同步更新：
  - `src/lib/api.ts`
  - `src/lib/types.ts`
  - `README.md`
  - `src-tauri` / `siren_core` 中对应的 rustdoc
