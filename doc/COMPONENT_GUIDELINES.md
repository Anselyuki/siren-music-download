# Component Guidelines

## Categories

- Primitive
  基础交互原语，直接来自 `shadcn-svelte / Bits UI`
- App Variant
  在原语之上包一层项目视觉和状态约束
- Composite
  面向单个业务区域的复合组件
- Pattern
  跨多个组件复用的结构模式

## Lifecycle

- `draft`
  仅局部试验，不允许全局复用
- `beta`
  可用于新界面，但 API 和视觉仍可能调整
- `stable`
  可以作为默认方案复用
- `deprecated`
  不再推荐使用，只保留迁移窗口

## Stable Criteria

- 已接入设计令牌
- 已定义变体边界
- 已满足键盘可达性
- 已写入组件说明
- 已至少被两个场景复用

## Apple Direction

- 工具栏更像 macOS 控件条
- Dock 与 Sheet 保留适中玻璃材质
- 主工作区保持干净
- 动态专辑色默认降饱和
- 列表区优先效率，标题区优先内容层级

## Primitive Candidates

- Button
- Badge
- Sheet
- Select
- Switch
- Scroll Area
- Progress
- Tooltip
- Dialog
- Alert Dialog
- Sonner
- Skeleton
- Tabs
- Slider

## App Variant Candidates

- ToolbarIconButton
- SegmentedModeControl
- AppBadge
- SheetSectionHeader
- DockUtilityButton
- ListRowActionButton
- TonalStatusBadge

## Composite Candidates

- TopToolbar
- AlbumSidebar
- AlbumWorkspace
- PlayerDock
- SettingsSheet
- DownloadTasksSheet
- LyricsFlyout
- PlaylistFlyout
