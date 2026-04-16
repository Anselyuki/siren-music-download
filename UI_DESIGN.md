# 界面设计说明

## 布局

当前界面不是早期的固定三栏，而是下面这套结构：

```text
┌──────────────┬──────────────────────────────────────────┐
│ 专辑侧栏     │ 主内容区                                 │
│ AlbumCard ×N │ 顶部工具栏                              │
│              │ 专辑舞台 / 曲目列表 / 内容滚动区         │
│              │                                          │
│              │ 底部播放器 Dock                          │
│              │ ├─ 传输控制 / 进度 / 下载                │
│              │ └─ 可展开歌词面板 / 播放队列面板         │
└──────────────┴──────────────────────────────────────────┘
                         ▲
                         └── 右侧下载设置面板按需滑出
```

## 主要组件

- `App.svelte`：应用入口，负责专辑加载、模式切换、播放状态同步、歌词与队列面板、下载设置面板
- `AlbumCard.svelte`：左侧专辑列表项
- `SongRow.svelte`：曲目行，依据模式在“勾选下载”和“单击播放”之间切换交互
- `AudioPlayer.svelte`：底部播放器 Dock，包含播放控制、进度条、乱序 / 循环、歌词、队列和当前曲目下载入口

## 交互模式

顶部工具栏提供两种模式：

- `download`：单击曲目只勾选，不触发播放，用于后续批量下载
- `player`：单击曲目立即播放，并把当前专辑曲目构造成播放上下文

多选模式（download）已接通后端下载任务系统，提供全选、清空、反选和”下载所选歌曲”按钮。

## 播放状态流

前端通过 Tauri command 拉起播放，通过 Tauri event 持续同步状态：

```text
App.svelte
  ├─ invoke('play_song')
  ├─ invoke('pause_playback')
  ├─ invoke('resume_playback')
  ├─ invoke('seek_current_playback')
  ├─ invoke('play_next' / 'play_previous')
  └─ listen('player-state-changed' / 'player-progress')

src-tauri/src/main.rs
  └─ AudioPlayer
       ├─ cpal 输出
       ├─ symphonia 解码
       ├─ souvlaki 媒体会话
       └─ emit PlayerState
```

事件载荷统一是 `PlayerState`，这样前端在一次同步里就能拿到：

- 当前曲目
- 播放 / 暂停 / 加载状态
- 上一首 / 下一首是否可用
- 当前进度与总时长
- 当前音量

## 视觉与主题

- 全局配色通过 CSS 变量驱动，分别覆盖亮色和暗色系统主题
- 当前专辑封面会通过 `extract_image_theme` 提取主题色，驱动播放器、舞台和部分高亮控件
- 减弱动画模式由 `prefers-reduced-motion` 驱动，动画时长和滚动行为会收敛

## 下载相关 UI

右侧设置面板负责：

- 输出格式选择：`wav` / `flac` / `mp3`
- 输出目录选择：通过 `@tauri-apps/plugin-dialog` 选择本地目录
- 清理播放缓存：调用 `clear_audio_cache`

当前下载能力分成三条链路：

1. 播放器下载按钮：调用 `create_download_job`（kind=song），下载当前播放曲目
2. 专辑页"下载整张专辑"按钮：调用 `create_download_job`（kind=album），一键整专下载
3. 多选模式批量下载：进入多选模式后，通过全选/清空/反选按钮批量勾选，点击"下载所选歌曲"调用 `create_download_job`（kind=selection）

工具栏下载图标可打开独立下载面板，展示任务列表、进度、取消/重试按钮和历史清理。

## 设计文档和实现的差异

这个文件现在以代码为准，和仓库早期说明相比有两点变化：

- 下载设置不再是固定第三栏，而是右侧滑出面板
- 播放器已经不只是底部简易控制条，而是带歌词、队列、下载和更多播放控制的 Dock
