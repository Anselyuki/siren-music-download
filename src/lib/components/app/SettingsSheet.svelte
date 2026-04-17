<script lang="ts">
  import * as Sheet from "$lib/components/ui/sheet/index.js";
  import { Button } from "$lib/components/ui/button/index.js";
  import { Switch } from "$lib/components/ui/switch/index.js";
  import type { OutputFormat } from "$lib/types";

  interface Props {
    open?: boolean;
    format?: OutputFormat;
    outputDir?: string;
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    isSendingTestNotification?: boolean;
    isClearingAudioCache?: boolean;
    onSelectDirectory: () => void | Promise<void>;
    onSendTestNotification: () => void | Promise<void>;
    onClearAudioCache: () => void | Promise<void>;
  }

  let {
    open = $bindable(false),
    format = $bindable<OutputFormat>("flac"),
    outputDir = "",
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    isSendingTestNotification = false,
    isClearingAudioCache = false,
    onSelectDirectory,
    onSendTestNotification,
    onClearAudioCache,
  }: Props = $props();
</script>

<Sheet.Root bind:open>
  <Sheet.Content class="w-[340px] border-white/50 bg-[var(--surface-sheet)] text-[var(--text-primary)] backdrop-blur-xl">
    <Sheet.Header>
      <Sheet.Title>下载设置</Sheet.Title>
      <Sheet.Description>音频格式、通知和缓存管理</Sheet.Description>
    </Sheet.Header>

    <div class="space-y-6 py-2">
      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]" for="format-select">输出格式</label>
        <select
          id="format-select"
          class="w-full rounded-2xl border border-white/50 bg-white/40 px-3 py-2 text-sm outline-none"
          bind:value={format}
        >
          <option value="flac">FLAC（无损压缩）</option>
          <option value="wav">WAV（无损）</option>
          <option value="mp3">MP3</option>
        </select>
      </div>

      <div class="space-y-2">
        <label class="text-sm text-[var(--text-secondary)]" for="output-dir">保存位置</label>
        <input
          id="output-dir"
          class="w-full rounded-2xl border border-white/50 bg-white/35 px-3 py-2 text-sm outline-none"
          readonly
          value={outputDir}
        />
        <Button class="w-full" onclick={() => void onSelectDirectory()}>
          选择文件夹
        </Button>
      </div>

      <div class="space-y-4 rounded-[22px] border border-white/45 bg-white/28 p-4">
        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">歌词文件</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">有歌词时，在音频旁生成同名 `.lrc`。</p>
          </div>
          <Switch bind:checked={downloadLyrics} />
        </div>

        <div class="border-t border-white/40"></div>

        <div class="space-y-3">
          <p class="text-sm font-medium">系统通知</p>
          <p class="text-xs text-[var(--text-secondary)]">
            桌面端权限以系统结果为准，开发环境下可能和打包结果不一致。
          </p>
          <Button
            class="w-full"
            variant="secondary"
            disabled={isSendingTestNotification}
            onclick={() => void onSendTestNotification()}
          >
            {isSendingTestNotification ? "正在发送..." : "发送测试通知"}
          </Button>
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">下载完成通知</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">下载任务完成时显示通知。</p>
          </div>
          <Switch bind:checked={notifyOnDownloadComplete} />
        </div>

        <div class="flex items-center justify-between gap-4">
          <div class="min-w-0">
            <p class="text-sm font-medium">播放切换通知</p>
            <p class="mt-1 text-xs text-[var(--text-secondary)]">播放新歌曲时显示通知。</p>
          </div>
          <Switch bind:checked={notifyOnPlaybackChange} />
        </div>
      </div>

      <div class="space-y-3 rounded-[22px] border border-white/45 bg-white/25 p-4">
        <div>
          <p class="text-sm font-medium">音乐缓存</p>
          <p class="mt-1 text-xs text-[var(--text-secondary)]">播放时的音频缓存保存在系统缓存目录。</p>
        </div>
        <Button
          class="w-full"
          variant="secondary"
          disabled={isClearingAudioCache}
          onclick={() => void onClearAudioCache()}
        >
          {isClearingAudioCache ? "正在清除缓存..." : "清除音频缓存"}
        </Button>
      </div>
    </div>
  </Sheet.Content>
</Sheet.Root>
