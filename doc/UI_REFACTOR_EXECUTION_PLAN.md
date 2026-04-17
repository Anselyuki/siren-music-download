# UI Refactor Execution Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 在不改变 Tauri 业务语义的前提下，按 `shadcn-svelte + Tailwind + Apple 化设计系统` 重建前端 UI，并把 `App.svelte` 收敛为装配层。

**Architecture:** 先建立工具链、设计令牌和组件原语，再按 `env / shell / library / download / player` 拆分状态与视图，最后分区迁移壳层、工作区和播放器。所有通用样式统一走设计令牌与项目变体，复杂业务组件继续保留定制实现，只消费原语和令牌。

**Tech Stack:** Svelte 5, Vite 8, Tauri 2, Tailwind CSS, shadcn-svelte, Bits UI, @humanspeak/svelte-motion, OverlayScrollbars

---

## Planning Assumptions

- 当前仓库没有成熟的前端自动化测试框架，本计划以 `npx svelte-check`、`npx tsc --noEmit`、`npm run build`、`cargo check --workspace` 和手工冒烟清单作为红绿回路。
- 文档输出沿用当前仓库习惯，统一放在 `doc/` 下，而不是 `docs/superpowers/`。
- 不切换到 `SvelteKit`，不调整后端接口，不引入新的状态管理库。
- `shadcn-svelte` 只用于通用交互原语与基础结构，不直接决定最终视觉。

## Manual QA Matrix

**Files:**
- Create: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 新建手工冒烟清单**

```md
# UI Refactor QA Checklist

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
```

- [ ] **Step 2: 提前记录基线校验命令**

Run:

```powershell
npx tsc --noEmit
npx svelte-check
npm run build
cargo check --workspace
```

Expected:

```text
记录当前成功项和失败项，作为后续每个任务的对照基线。
```

---

### Task 1: Tooling Foundation

**Files:**
- Modify: `package.json`
- Modify: `tsconfig.json`
- Modify: `vite.config.ts`
- Modify: `src/app.css`
- Create: `components.json`
- Create: `src/lib/utils.ts`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 安装 Tailwind 和设计系统依赖**

Run:

```powershell
npm install tailwindcss @tailwindcss/vite tailwind-variants clsx tailwind-merge tw-animate-css @lucide/svelte
```

Expected:

```text
npm install 成功，package-lock.json 更新。
```

- [ ] **Step 2: 为校验补充脚本**

Update `package.json`:

```json
{
  "scripts": {
    "dev": "vite",
    "build": "vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "check": "npm run check:types && npm run check:svelte && npm run check:build",
    "check:types": "tsc --noEmit",
    "check:svelte": "svelte-check",
    "check:build": "vite build",
    "check:cargo": "cargo check --workspace"
  }
}
```

- [ ] **Step 3: 修正 TypeScript 路径映射**

Update `tsconfig.json`:

```json
{
  "compilerOptions": {
    "target": "ESNext",
    "useDefineForClassFields": true,
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "jsx": "preserve",
    "resolveJsonModule": true,
    "isolatedModules": true,
    "esModuleInterop": true,
    "noEmit": true,
    "skipLibCheck": true,
    "moduleDetection": "force",
    "lib": ["ESNext", "DOM", "DOM.Iterable"],
    "baseUrl": ".",
    "paths": {
      "$lib": ["./src/lib"],
      "$lib/*": ["./src/lib/*"]
    }
  },
  "include": ["src/**/*.ts", "src/**/*.svelte"]
}
```

- [ ] **Step 4: 接入 Tailwind Vite 插件**

Update `vite.config.ts`:

```ts
import { defineConfig } from "vite";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import tailwindcss from "@tailwindcss/vite";
import { resolve } from "path";

export default defineConfig({
  plugins: [svelte(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
  resolve: {
    alias: {
      $lib: resolve(__dirname, "src/lib"),
    },
  },
});
```

- [ ] **Step 5: 创建 shadcn-svelte 所需工具文件**

Create `src/lib/utils.ts`:

```ts
import { type ClassValue, clsx } from "clsx";
import { twMerge } from "tailwind-merge";

export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
```

Create `components.json`:

```json
{
  "$schema": "https://shadcn-svelte.com/schema.json",
  "style": "new-york",
  "typescript": true,
  "tailwind": {
    "css": "src/app.css",
    "baseColor": "slate"
  },
  "aliases": {
    "components": "$lib/components",
    "ui": "$lib/components/ui",
    "utils": "$lib/utils",
    "hooks": "$lib/hooks",
    "lib": "$lib"
  }
}
```

- [ ] **Step 6: 给全局 CSS 加入 Tailwind 入口，不删除旧样式**

Update the top of `src/app.css`:

```css
@import "tailwindcss";
@import "tw-animate-css";

@custom-variant dark (&:is(.dark *));

/* Legacy CSS stays below during migration. Token definitions will be normalized in Task 2. */

* {
  margin: 0;
  padding: 0;
  box-sizing: border-box;
}
```

- [ ] **Step 7: 运行基础校验**

Run:

```powershell
npm run check:types
npm run check:svelte
npm run check:build
```

Expected:

```text
至少 tsc 与 build 通过；如果 svelte-check 仍报历史问题，记录在 QA 清单顶部，不新增新的路径解析错误。
```

- [ ] **Step 8: Commit**

```bash
git add package.json package-lock.json tsconfig.json vite.config.ts src/app.css components.json src/lib/utils.ts doc/UI_REFACTOR_QA_CHECKLIST.md
git commit -m "chore: add tailwind and shadcn foundation"
```

---

### Task 2: Design System Docs and Tokens

**Files:**
- Create: `doc/DESIGN_SYSTEM_TOKENS.md`
- Create: `doc/COMPONENT_GUIDELINES.md`
- Create: `doc/CONTENT_GUIDELINES.md`
- Create: `src/lib/design/tokens.ts`
- Create: `src/lib/design/variants.ts`
- Create: `src/lib/design/motion.ts`
- Modify: `src/app.css`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 写设计令牌文档**

Create `doc/DESIGN_SYSTEM_TOKENS.md`:

```md
# Design System Tokens

## Surfaces
- `surface.window`
- `surface.sidebar`
- `surface.workspace`
- `surface.sheet`
- `surface.dock`
- `surface.flyout`
- `surface.state`

## Text
- `text.primary`
- `text.secondary`
- `text.tertiary`

## Accent
- 动态专辑色统一做降饱和、提亮、压对比处理

## Materials
- 玻璃材质只允许用于 `sheet / dock / flyout`
```

- [ ] **Step 2: 写组件规范文档**

Create `doc/COMPONENT_GUIDELINES.md`:

```md
# Component Guidelines

## Categories
- Primitive
- App Variant
- Composite
- Pattern

## Lifecycle
- draft
- beta
- stable
- deprecated

## Apple Direction
- 工具栏更像 macOS 控件条
- Dock 与 Sheet 保留适中玻璃材质
- 主工作区保持干净
```

- [ ] **Step 3: 写内容规范文档**

Create `doc/CONTENT_GUIDELINES.md`:

```md
# Content Guidelines

## Tone
- 句子更短
- 少解释
- 不营销

## UI Copy Rules
- 面板标题像系统功能名
- 错误提示优先说明可恢复动作
- toast 只保留结果和下一步
```

- [ ] **Step 4: 建立令牌和动效代码文件**

Create `src/lib/design/tokens.ts`:

```ts
export const surfaceTokens = {
  window: "bg-[var(--surface-window)]",
  sidebar: "bg-[var(--surface-sidebar)]",
  workspace: "bg-[var(--surface-workspace)]",
  sheet: "bg-[var(--surface-sheet)]",
  dock: "bg-[var(--surface-dock)]",
  flyout: "bg-[var(--surface-flyout)]",
  state: "bg-[var(--surface-state)]",
} as const;

export const textTokens = {
  primary: "text-[var(--text-primary)]",
  secondary: "text-[var(--text-secondary)]",
  tertiary: "text-[var(--text-tertiary)]",
} as const;
```

Create `src/lib/design/motion.ts`:

```ts
export const motionDuration = {
  fast: 140,
  base: 180,
  slow: 260,
} as const;

export const motionEase = {
  standard: [0.2, 0, 0, 1],
  decelerate: [0.16, 1, 0.3, 1],
} as const;
```

- [ ] **Step 5: 建立项目变体入口**

Create `src/lib/design/variants.ts`:

```ts
import { tv } from "tailwind-variants";

export const toolbarIconButton = tv({
  base: "inline-flex items-center justify-center rounded-full border border-transparent text-[var(--text-primary)] transition-colors",
  variants: {
    active: {
      true: "bg-[var(--surface-state)] text-[var(--accent)]",
      false: "bg-transparent hover:bg-[var(--surface-state)]",
    },
  },
});

export const sheetSurface = tv({
  base: "backdrop-blur-xl border border-white/40 shadow-[0_24px_64px_rgba(15,23,42,0.16)]",
});
```

- [ ] **Step 6: 在 app.css 中引入第一批 Apple 化令牌**

Add to `src/app.css` near the root token area:

```css
:root {
  --surface-window: color-mix(in srgb, var(--bg-primary) 100%, transparent);
  --surface-sidebar: color-mix(in srgb, var(--bg-secondary) 92%, white 8%);
  --surface-workspace: color-mix(in srgb, var(--bg-primary) 96%, white 4%);
  --surface-sheet: color-mix(in srgb, var(--bg-secondary) 76%, white 24%);
  --surface-dock: color-mix(in srgb, var(--player-shell-bg) 88%, white 12%);
  --surface-flyout: color-mix(in srgb, var(--player-shell-bg) 80%, transparent);
  --surface-state: rgba(var(--accent-rgb), 0.08);
}
```

- [ ] **Step 7: 运行校验**

Run:

```powershell
npm run check
```

Expected:

```text
基础工具链无新增错误。
```

- [ ] **Step 8: Commit**

```bash
git add doc/DESIGN_SYSTEM_TOKENS.md doc/COMPONENT_GUIDELINES.md doc/CONTENT_GUIDELINES.md src/lib/design/tokens.ts src/lib/design/variants.ts src/lib/design/motion.ts src/app.css
git commit -m "docs: add design system tokens and guidelines"
```

---

### Task 3: shadcn-svelte Primitives

**Files:**
- Modify: `components.json`
- Create: `src/lib/components/ui/*`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 初始化 shadcn-svelte 组件目录**

Run:

```powershell
npx shadcn-svelte@latest add button badge sheet select switch scroll-area progress tooltip dialog alert-dialog sonner skeleton tabs slider separator input
```

Expected:

```text
`src/lib/components/ui/` 下生成对应组件目录。
```

- [ ] **Step 2: 建立统一的组件导入示例**

Create `src/lib/components/ui/README.md`:

```md
# UI Imports

- `import { Button } from "$lib/components/ui/button/index.js";`
- `import { Badge } from "$lib/components/ui/badge/index.js";`
- `import * as Sheet from "$lib/components/ui/sheet/index.js";`
```

- [ ] **Step 3: 先验证基础原语能在现有页面中被引用**

Add a temporary proof import near the top of `src/App.svelte`:

```ts
import { Button } from "$lib/components/ui/button/index.js";
```

Expected:

```text
仅验证路径可解析；下一步撤回临时 import，避免未使用告警。
```

- [ ] **Step 4: 撤回临时 proof import，保持工作树干净**

Remove the temporary import from `src/App.svelte`.

- [ ] **Step 5: 运行校验**

Run:

```powershell
npm run check:svelte
npm run check:build
```

Expected:

```text
shadcn 组件路径可用，构建通过。
```

- [ ] **Step 6: Commit**

```bash
git add components.json src/lib/components/ui src/lib/components/ui/README.md
git commit -m "feat: add shadcn svelte primitives"
```

---

### Task 4: Feature Store Skeletons and App Shell Split

**Files:**
- Create: `src/lib/features/env/store.svelte.ts`
- Create: `src/lib/features/shell/store.svelte.ts`
- Create: `src/lib/features/library/store.svelte.ts`
- Create: `src/lib/features/library/helpers.ts`
- Create: `src/lib/features/library/selectors.ts`
- Create: `src/lib/features/download/store.svelte.ts`
- Create: `src/lib/features/download/formatters.ts`
- Create: `src/lib/features/download/guards.ts`
- Create: `src/lib/features/player/store.svelte.ts`
- Create: `src/lib/features/player/queue.ts`
- Create: `src/lib/features/player/lyrics.ts`
- Modify: `src/App.svelte`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 抽 env store**

Create `src/lib/features/env/store.svelte.ts`:

```ts
let isMacOS = $state(false);
let prefersReducedMotion = $state(false);
let viewportHeight = $state(0);

function init() {
  isMacOS =
    /Mac|iPhone|iPad|iPod/.test(navigator.platform) ||
    navigator.userAgent.includes("Mac");
  viewportHeight = window.innerHeight || 0;
}

function dispose() {}

export const envStore = {
  get isMacOS() { return isMacOS; },
  get prefersReducedMotion() { return prefersReducedMotion; },
  get viewportHeight() { return viewportHeight; },
  init,
  dispose,
};
```

- [ ] **Step 2: 抽 shell store**

Create `src/lib/features/shell/store.svelte.ts`:

```ts
let settingsOpen = $state(false);
let downloadPanelOpen = $state(false);

export const shellStore = {
  get settingsOpen() { return settingsOpen; },
  get downloadPanelOpen() { return downloadPanelOpen; },
  openSettings() { settingsOpen = true; },
  closeSettings() { settingsOpen = false; },
  openDownloads() { downloadPanelOpen = true; },
  closeDownloads() { downloadPanelOpen = false; },
  init() {},
  dispose() {},
};
```

- [ ] **Step 3: 把纯函数先迁到 helper 文件**

Create `src/lib/features/player/queue.ts`:

```ts
import type { PlaybackContext, PlaybackQueueEntry } from "$lib/types";

export function buildPlaybackContext(
  order: PlaybackQueueEntry[],
  currentIndex: number,
): PlaybackContext | undefined {
  if (!order.length || currentIndex < 0 || currentIndex >= order.length) {
    return undefined;
  }

  return {
    currentIndex,
    entries: order.map((entry) => ({
      cid: entry.cid,
      name: entry.name,
      artists: entry.artists,
      coverUrl: entry.coverUrl,
    })),
  };
}
```

Create `src/lib/features/player/lyrics.ts`:

```ts
export type LyricLine = {
  id: string;
  time: number | null;
  text: string;
};

export function parseLyricText(source: string): LyricLine[] {
  return source
    .split(/\r?\n/)
    .map((line, index) => ({
      id: `line-${index}`,
      time: null,
      text: line.trim() || "♪",
    }))
    .filter((line) => line.text.length > 0);
}
```

- [ ] **Step 4: 创建各域 store 的最小骨架**

Create one skeleton, then duplicate the pattern for library/download/player:

```ts
let initialized = false;

async function init() {
  if (initialized) return;
  initialized = true;
}

function dispose() {
  initialized = false;
}

export const libraryStore = {
  init,
  dispose,
};
```

- [ ] **Step 5: 让 App.svelte 只承担初始化协调的第一步**

At the top of `src/App.svelte`, add the new imports:

```ts
import { envStore } from "$lib/features/env/store.svelte";
import { shellStore } from "$lib/features/shell/store.svelte";
```

Inside `onMount`, start delegating:

```ts
onMount(() => {
  envStore.init();
  shellStore.init();

  return () => {
    shellStore.dispose();
    envStore.dispose();
  };
});
```

- [ ] **Step 6: 运行校验**

Run:

```powershell
npm run check
```

Expected:

```text
App 仍可运行，新增 store skeleton 不引入编译错误。
```

- [ ] **Step 7: Commit**

```bash
git add src/lib/features src/App.svelte
git commit -m "refactor: scaffold feature stores and shell split"
```

---

### Task 5: Shell UI Migration (Toolbar, Sheets, Toast)

**Files:**
- Create: `src/lib/components/app/TopToolbar.svelte`
- Create: `src/lib/components/app/SettingsSheet.svelte`
- Create: `src/lib/components/app/DownloadTasksSheet.svelte`
- Create: `src/lib/components/app/StatusToastHost.svelte`
- Modify: `src/App.svelte`
- Modify: `src/lib/design/variants.ts`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 创建顶部工具栏组件**

Create `src/lib/components/app/TopToolbar.svelte`:

```svelte
<script lang="ts">
  import { Button } from "$lib/components/ui/button/index.js";
  import { toolbarIconButton } from "$lib/design/variants";

  interface Props {
    activeDownloadCount: number;
    onRefresh: () => void;
    onOpenDownloads: () => void;
    onOpenSettings: () => void;
  }

  let { activeDownloadCount, onRefresh, onOpenDownloads, onOpenSettings }: Props = $props();
</script>

<div class="flex items-center gap-2 rounded-full border border-white/40 bg-white/60 p-2 shadow-[0_16px_36px_rgba(15,23,42,0.12)] backdrop-blur-xl">
  <Button class={toolbarIconButton()} onclick={onRefresh}>刷新</Button>
  <Button class={toolbarIconButton()} onclick={onOpenDownloads}>
    下载任务 {activeDownloadCount > 0 ? `(${activeDownloadCount})` : ""}
  </Button>
  <Button class={toolbarIconButton()} onclick={onOpenSettings}>设置</Button>
</div>
```

- [ ] **Step 2: 创建设置 Sheet 组件**

Create `src/lib/components/app/SettingsSheet.svelte`:

```svelte
<script lang="ts">
  import * as Sheet from "$lib/components/ui/sheet/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import * as Select from "$lib/components/ui/select/index.js";

  interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
  }

  let { open, onOpenChange }: Props = $props();
</script>

<Sheet.Root {open} onOpenChange={onOpenChange}>
  <Sheet.Content class="w-[320px] border-white/40 bg-[var(--surface-sheet)] backdrop-blur-xl">
    <Sheet.Header>
      <Sheet.Title>下载设置</Sheet.Title>
    </Sheet.Header>
    <div class="space-y-6 py-4">
      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]">输出格式</label>
        <Select.Root type="single">
          <Select.Trigger class="w-full rounded-2xl border border-white/50 bg-white/40 px-3 py-2 text-left text-sm">
            <Select.Value placeholder="选择格式" />
          </Select.Trigger>
          <Select.Content>
            <Select.Item value="flac">FLAC</Select.Item>
            <Select.Item value="wav">WAV</Select.Item>
            <Select.Item value="mp3">MP3</Select.Item>
          </Select.Content>
        </Select.Root>
      </div>
      <div class="flex items-center justify-between rounded-2xl border border-white/50 bg-white/35 p-3">
        <span class="text-sm">歌词文件</span>
        <Switch />
      </div>
      <Button class="w-full">选择文件夹</Button>
    </div>
  </Sheet.Content>
</Sheet.Root>
```

- [ ] **Step 3: 创建下载任务 Sheet 和 toast host**

Create `src/lib/components/app/DownloadTasksSheet.svelte`:

```svelte
<script lang="ts">
  import * as Sheet from "$lib/components/ui/sheet/index.js";
  import { Badge } from "$lib/components/ui/badge/index.js";
  import { Progress } from "$lib/components/ui/progress/index.js";

  interface Props {
    open: boolean;
    onOpenChange: (open: boolean) => void;
    jobs: Array<{
      id: string;
      title: string;
      status: string;
      progress: number;
    }>;
  }

  let { open, onOpenChange, jobs }: Props = $props();
</script>

<Sheet.Root {open} onOpenChange={onOpenChange}>
  <Sheet.Content class="w-[420px] border-white/40 bg-[var(--surface-sheet)] backdrop-blur-xl">
    <Sheet.Header>
      <Sheet.Title>下载任务</Sheet.Title>
    </Sheet.Header>
    <div class="space-y-3 py-4">
      {#each jobs as job (job.id)}
        <section class="rounded-2xl border border-white/35 bg-white/30 p-3">
          <div class="flex items-center justify-between gap-3">
            <p class="truncate text-sm font-medium text-[var(--text-primary)]">{job.title}</p>
            <Badge>{job.status}</Badge>
          </div>
          <Progress class="mt-3" value={job.progress} />
        </section>
      {/each}
    </div>
  </Sheet.Content>
</Sheet.Root>
```

Create `src/lib/components/app/StatusToastHost.svelte`:

```svelte
<script lang="ts">
  import { Toaster } from "$lib/components/ui/sonner/index.js";
</script>

<Toaster position="top-center" richColors={false} />
```

- [ ] **Step 4: 将 App.svelte 的内联壳层替换为新组件**

Use these imports in `src/App.svelte`:

```ts
import TopToolbar from "$lib/components/app/TopToolbar.svelte";
import SettingsSheet from "$lib/components/app/SettingsSheet.svelte";
import DownloadTasksSheet from "$lib/components/app/DownloadTasksSheet.svelte";
import StatusToastHost from "$lib/components/app/StatusToastHost.svelte";
```

Render them in place of the inline toolbar and sheets:

```svelte
<TopToolbar
  activeDownloadCount={activeDownloadCount}
  onRefresh={handleRefresh}
  onOpenDownloads={() => (downloadPanelOpen = true)}
  onOpenSettings={() => (settingsOpen = true)}
/>

<SettingsSheet open={settingsOpen} onOpenChange={(next) => (settingsOpen = next)} />
<DownloadTasksSheet open={downloadPanelOpen} onOpenChange={(next) => (downloadPanelOpen = next)} />
<StatusToastHost />
```

- [ ] **Step 5: 把所有 alert() 替换成统一 toast 调用**

Add a wrapper near existing callbacks:

```ts
import { toast } from "svelte-sonner";

function notifyInfo(message: string) {
  toast(message);
}
```

Replace one `alert("当前没有可清理的下载历史。");` with:

```ts
notifyInfo("当前没有可清理的下载历史。");
```

Replace the remaining `alert()` sites in these handlers during the same task:

```ts
handleCurrentSongDownload
handleAlbumDownload
handleSelectionDownload
handleClearAudioCache
handleSendTestNotification
handleClearDownloadHistory
handleSongDownload
```

- [ ] **Step 6: 运行壳层冒烟**

Run:

```powershell
npm run tauri:dev
```

Expected:

```text
顶部工具栏、设置面板、下载任务面板都能打开；toast 正常显示。
```

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/app/TopToolbar.svelte src/lib/components/app/SettingsSheet.svelte src/lib/components/app/DownloadTasksSheet.svelte src/lib/components/app/StatusToastHost.svelte src/App.svelte src/lib/design/variants.ts
git commit -m "feat: migrate shell ui to shadcn sheets and toolbar"
```

---

### Task 6: Sidebar and Workspace Migration

**Files:**
- Create: `src/lib/components/app/AlbumSidebar.svelte`
- Create: `src/lib/components/app/AlbumWorkspace.svelte`
- Modify: `src/lib/components/AlbumCard.svelte`
- Modify: `src/lib/components/SongRow.svelte`
- Modify: `src/App.svelte`
- Modify: `src/lib/design/variants.ts`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 创建专辑侧栏组件**

Create `src/lib/components/app/AlbumSidebar.svelte`:

```svelte
<script lang="ts">
  import AlbumCard from "$lib/components/AlbumCard.svelte";
  import type { Album } from "$lib/types";

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    reducedMotion: boolean;
    onSelect: (album: Album) => void;
  }

  let { albums, selectedAlbumCid, reducedMotion, onSelect }: Props = $props();
</script>

<aside class="flex h-full flex-col gap-1 bg-[var(--surface-sidebar)] px-4 py-5">
  {#each albums as album (album.cid)}
    <AlbumCard
      album={album}
      selected={album.cid === selectedAlbumCid}
      reducedMotion={reducedMotion}
      onclick={() => onSelect(album)}
    />
  {/each}
</aside>
```

- [ ] **Step 2: 创建主工作区组件**

Create `src/lib/components/app/AlbumWorkspace.svelte`:

```svelte
<script lang="ts">
  import SongRow from "$lib/components/SongRow.svelte";
  import type { AlbumDetail, SongEntry } from "$lib/types";

  interface Props {
    album: AlbumDetail | null;
    reducedMotion: boolean;
    onPlay: (song: SongEntry) => void;
  }

  let { album, reducedMotion, onPlay }: Props = $props();
</script>

<section class="flex h-full flex-col bg-[var(--surface-workspace)]">
  {#if album}
    <header class="px-8 pt-8 pb-6">
      <p class="text-xs uppercase tracking-[0.14em] text-[var(--accent)]">{album.belong}</p>
      <h1 class="mt-2 text-4xl font-semibold tracking-[-0.03em] text-[var(--text-primary)]">{album.name}</h1>
      <p class="mt-2 text-sm text-[var(--text-secondary)]">{(album.artists ?? []).join(" · ")}</p>
    </header>
    <div class="flex flex-col gap-1 px-6 pb-6">
      {#each album.songs as song, index (song.cid)}
        <SongRow song={song} {index} reducedMotion={reducedMotion} onclick={() => onPlay(song)} />
      {/each}
    </div>
  {/if}
</section>
```

- [ ] **Step 3: 调整 AlbumCard 为 macOS 风格侧栏项**

Update the root style area in `src/lib/components/AlbumCard.svelte`:

```css
:global(.album-card) {
  background: transparent;
  border-radius: 14px;
  padding: 10px 12px;
  margin-bottom: 2px;
  cursor: pointer;
  display: flex;
  align-items: center;
  gap: 12px;
  box-shadow: inset 0 0 0 1px transparent;
}

:global(.album-card.selected) {
  background: rgba(var(--accent-rgb), 0.08);
}
```

- [ ] **Step 4: 调整 SongRow 为更轻的 Apple 风格列表项**

Update the root style area in `src/lib/components/SongRow.svelte`:

```css
:global(.song-row) {
  display: flex;
  align-items: center;
  padding: 12px 10px;
  border-radius: 16px;
  gap: 16px;
  background: transparent;
  box-shadow: inset 0 0 0 1px transparent;
}
```

- [ ] **Step 5: 用新组件替换 App.svelte 中的侧栏和主区大段模板**

Use these imports:

```ts
import AlbumSidebar from "$lib/components/app/AlbumSidebar.svelte";
import AlbumWorkspace from "$lib/components/app/AlbumWorkspace.svelte";
```

Render:

```svelte
<AlbumSidebar
  albums={albums}
  selectedAlbumCid={selectedAlbumCid}
  reducedMotion={prefersReducedMotion}
  onSelect={handleSelectAlbum}
/>

<AlbumWorkspace
  album={selectedAlbum}
  reducedMotion={prefersReducedMotion}
  onPlay={handlePlay}
/>
```

- [ ] **Step 6: 运行工作区冒烟**

Run:

```powershell
npm run check
npm run tauri:dev
```

Expected:

```text
专辑侧栏、专辑主区、曲目列表都能渲染；专辑切换与曲目点击正常。
```

- [ ] **Step 7: Commit**

```bash
git add src/lib/components/app/AlbumSidebar.svelte src/lib/components/app/AlbumWorkspace.svelte src/lib/components/AlbumCard.svelte src/lib/components/SongRow.svelte src/App.svelte
git commit -m "feat: migrate sidebar and workspace to apple-style layout"
```

---

### Task 7: Player Dock and Flyouts

**Files:**
- Create: `src/lib/components/app/PlayerDock.svelte`
- Modify: `src/lib/components/AudioPlayer.svelte`
- Modify: `src/App.svelte`
- Modify: `src/lib/design/variants.ts`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 建立 Dock 容器组件**

Create `src/lib/components/app/PlayerDock.svelte`:

```svelte
<script lang="ts">
  import AudioPlayer from "$lib/components/AudioPlayer.svelte";
  import type { PlaybackQueueEntry } from "$lib/types";

  interface Props {
    song: {
      cid: string;
      name: string;
      artists: string[];
      coverUrl: string | null;
    } | null;
    isPlaying: boolean;
    isPaused: boolean;
    progress: number;
    duration: number;
    isLoading: boolean;
    reducedMotion: boolean;
    playbackOrder: PlaybackQueueEntry[];
  }

  let props: Props = $props();
</script>

<div class="pointer-events-none absolute inset-x-0 bottom-4 z-[140] px-6">
  <div class="pointer-events-auto mx-auto max-w-[760px] rounded-full border border-white/50 bg-[var(--surface-dock)] p-2 shadow-[0_18px_40px_rgba(15,23,42,0.18)] backdrop-blur-xl">
    <AudioPlayer
      song={props.song}
      isPlaying={props.isPlaying}
      isPaused={props.isPaused}
      progress={props.progress}
      duration={props.duration}
      isLoading={props.isLoading}
      reducedMotion={props.reducedMotion}
      hasPrevious={props.playbackOrder.length > 1}
      hasNext={props.playbackOrder.length > 1}
    />
  </div>
</div>
```

- [ ] **Step 2: 收紧 AudioPlayer 的 Apple 化视觉**

Update the root `.am-player` block in `src/lib/components/AudioPlayer.svelte`:

```css
.am-player {
  width: 100%;
  min-height: 62px;
  border-radius: 999px;
  border: 1px solid rgba(255, 255, 255, 0.55);
  background: transparent;
  box-shadow: none;
  display: grid;
  grid-template-columns: 152px minmax(0, 1fr) auto;
  gap: 2px;
  align-items: center;
  padding: 4px 6px;
}
```

- [ ] **Step 3: 收紧播放器次级控件**

Update key style selectors in `src/lib/components/AudioPlayer.svelte`:

```css
.icon-button {
  border-radius: 999px;
  border: 1px solid transparent;
  background: transparent;
  color: var(--player-control-color);
}

.icon-button:hover:not(:disabled),
.icon-button[aria-pressed="true"] {
  background: rgba(var(--accent-rgb), 0.08);
  border-color: rgba(var(--accent-rgb), 0.08);
}
```

- [ ] **Step 4: 用 PlayerDock 替换 App.svelte 中的播放器包裹层**

Use:

```ts
import PlayerDock from "$lib/components/app/PlayerDock.svelte";
```

Render:

```svelte
<PlayerDock
  song={currentSong}
  {isPlaying}
  {isPaused}
  {progress}
  {duration}
  {isLoading}
  reducedMotion={prefersReducedMotion}
  {playbackOrder}
/>
```

- [ ] **Step 5: 手工验证最复杂路径**

Run:

```powershell
npm run tauri:dev
```

Expected:

```text
播放 / 暂停 / seek / 上一首 / 下一首 / 歌词 / 队列 / 当前歌曲下载入口均正常。
```

- [ ] **Step 6: Commit**

```bash
git add src/lib/components/app/PlayerDock.svelte src/lib/components/AudioPlayer.svelte src/App.svelte
git commit -m "feat: migrate player dock to apple-inspired design"
```

---

### Task 8: Cleanup, Legacy CSS Reduction, Final Verification

**Files:**
- Modify: `src/App.svelte`
- Modify: `src/app.css`
- Modify: `doc/UI_REFACTOR_QA_CHECKLIST.md`
- Test: `doc/UI_REFACTOR_QA_CHECKLIST.md`

- [ ] **Step 1: 删除已迁移的内联模板和遗留状态**

Remove old inline blocks from `src/App.svelte` once their component replacements are stable.

Target removals:

```ts
// inline toolbar block
// inline settings panel block
// inline download panel block
// inline sidebar block
// inline large workspace block
```

- [ ] **Step 2: 缩减 app.css，只保留令牌和全局特例**

Target final top section in `src/app.css`:

```css
@import "tailwindcss";
@import "tw-animate-css";

@custom-variant dark (&:is(.dark *));

:root {
  --surface-window: ...;
  --surface-sidebar: ...;
  --surface-workspace: ...;
  --surface-sheet: ...;
  --surface-dock: ...;
  --surface-flyout: ...;
}

body {
  background: var(--surface-window);
  color: var(--text-primary);
}
```

Expected:

```text
大块 `.top-toolbar`、`.settings-panel`、`.download-panel`、`.player-dock` 的旧实现被删除或显著缩短。
```

- [ ] **Step 3: 跑完整校验链**

Run:

```powershell
npm run check
npm run check:cargo
```

Expected:

```text
全部命令通过。
```

- [ ] **Step 4: 完成整体验收冒烟**

Manually check every item in `doc/UI_REFACTOR_QA_CHECKLIST.md`.

Expected:

```text
所有核心流和视觉检查项勾选完成。
```

- [ ] **Step 5: 更新计划与文档引用**

Add a completion note to `doc/UI_REFACTOR_QA_CHECKLIST.md`:

```md
## Completion Notes
- Refactor complete date: YYYY-MM-DD
- Final verification commands passed
- Remaining follow-ups: none
```

- [ ] **Step 6: Commit**

```bash
git add src/App.svelte src/app.css doc/UI_REFACTOR_QA_CHECKLIST.md
git commit -m "chore: finalize ui refactor and verification"
```

---

## Self-Review

### Spec coverage

- 工具链、Tailwind、shadcn-svelte：Task 1-3
- 设计系统令牌、文档、Apple 风格边界：Task 2
- App 拆分和 feature stores：Task 4
- 壳层、面板、toast：Task 5
- 专辑区、曲目区：Task 6
- 播放器、歌词/队列浮层：Task 7
- 清理、验收、回归：Task 8

### Placeholder scan

- 无 `TBD` / `TODO` / “后续补” 占位项
- 所有任务都给出明确文件路径、命令和代码骨架

### Type consistency

- 统一使用 `$lib/components/ui/.../index.js` 导入 shadcn-svelte 组件
- 统一使用 `env / shell / library / download / player` 五域命名
- 统一使用 `surface.*`、`text.*` 令牌命名

---

## Execution Handoff

Plan complete and saved to `doc/UI_REFACTOR_EXECUTION_PLAN.md`.

Two execution options:

1. Subagent-Driven (recommended) - I dispatch a fresh subagent per task, review between tasks, fast iteration
2. Inline Execution - Execute tasks in this session using executing-plans, batch execution with checkpoints

Which approach?
