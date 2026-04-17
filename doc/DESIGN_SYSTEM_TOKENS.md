# Design System Tokens

## Token Goals

- 统一窗口、侧栏、主工作区、Sheet、Dock、Flyout 的表面语义
- 保留动态专辑主题色，但默认做降饱和、提亮、压对比处理
- 用轻边框、轻高光、轻阴影表达层级，不靠重卡片感
- 让 Apple 化风格可被代码直接消费，而不是停留在描述层

## Surface Tokens

- `surface.window`
  窗口级背景，最稳定的基础表面
- `surface.sidebar`
  左侧专辑区，接近 macOS 侧栏
- `surface.workspace`
  主工作区，保持干净和平整
- `surface.sheet`
  右侧设置和下载任务面板，玻璃材质最明显
- `surface.dock`
  底部播放器 Dock，材质感明显但不厚重
- `surface.flyout`
  歌词和播放列表浮层，比 Sheet 更轻
- `surface.state`
  下载中、失败、选中等状态表面

## Text Tokens

- `text.primary`
  页面主信息
- `text.secondary`
  次级说明、列表副信息
- `text.tertiary`
  最弱说明、空状态辅助信息

## Accent Tokens

- `accent.base`
  当前专辑主题色
- `accent.hover`
  主题色 hover 态
- `accent.tonal`
  低饱和低对比状态面

## Material Rules

- 玻璃材质只允许用于 `sheet / dock / flyout`
- 主工作区不允许整页玻璃化
- 高光通过 `border + inset highlight` 表达
- 阴影表达分层，不表达悬浮卡片厚重感

## Motion Tokens

- `motion.fast`
- `motion.base`
- `motion.slow`
- `motion.ease.standard`
- `motion.ease.decelerate`

## Density Rules

- 侧栏和列表区保留桌面工具效率
- 标题和面板区允许更 Apple 化的留白
- Dock 和浮层保持紧凑，不走 iOS 式过度留白
