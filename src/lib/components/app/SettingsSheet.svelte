<script lang="ts">
  import * as Sheet from '$lib/components/ui/sheet/index.js';
  import * as Select from '$lib/components/ui/select/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import { Input } from '$lib/components/ui/input/index.js';
  import { Switch } from '$lib/components/ui/switch/index.js';
  import BellIcon from '@lucide/svelte/icons/bell';
  import FolderOpenIcon from '@lucide/svelte/icons/folder-open';
  import Trash2Icon from '@lucide/svelte/icons/trash-2';
  import {
    clearAudioCache,
    getLogFileStatus,
    listLogRecords,
    selectDirectory,
    sendTestNotification,
  } from '$lib/settingsApi';
  import type { Locale } from '$lib/i18n/types';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type {
    LogFileKind,
    LogFileStatus,
    LogLevel,
    LogViewerRecord,
    OutputFormat,
  } from '$lib/types';

  interface Props {
    open?: boolean;
    format?: OutputFormat;
    outputDir?: string;
    downloadLyrics?: boolean;
    notifyOnDownloadComplete?: boolean;
    notifyOnPlaybackChange?: boolean;
    logLevel?: LogLevel;
    locale?: Locale;
    logRefreshToken?: number;
    notifyInfo: (message: string) => void;
    notifyError: (message: string) => void;
    onOutputDirChange: (outputDir: string) => boolean | Promise<boolean>;
  }

  let {
    open = $bindable(false),
    format = $bindable<OutputFormat>('flac'),
    outputDir = $bindable(''),
    downloadLyrics = $bindable(true),
    notifyOnDownloadComplete = $bindable(true),
    notifyOnPlaybackChange = $bindable(true),
    logLevel = $bindable<LogLevel>('error'),
    locale = $bindable<Locale>('zh-CN'),
    logRefreshToken = 0,
    notifyInfo,
    notifyError,
    onOutputDirChange,
  }: Props = $props();

  let logFileKind = $state<LogFileKind>('session');
  let logRecords = $state<LogViewerRecord[]>([]);
  let logFileStatus = $state<LogFileStatus | null>(null);
  let logViewerLoading = $state(false);
  let logViewerError = $state('');
  let logRequestSeq = 0;
  let isSendingTestNotification = $state(false);
  let isClearingAudioCache = $state(false);
  let lastLoadedWhileOpen = $state(false);

  const formatOptions: { value: OutputFormat; label: string }[] = [
    { value: 'flac', label: 'FLAC（无损压缩）' },
    { value: 'wav', label: 'WAV（无损）' },
    { value: 'mp3', label: 'MP3' },
  ];

  const logLevelOptions: { value: LogLevel; label: string }[] = [
    { value: 'error', label: 'Error（仅严重错误）' },
    { value: 'warn', label: 'Warn（警告及以上）' },
    { value: 'info', label: 'Info（信息及以上）' },
    { value: 'debug', label: 'Debug（记录全部调试信息）' },
  ];

  async function refreshLogs(kind = logFileKind) {
    const requestSeq = ++logRequestSeq;
    logViewerLoading = true;
    logViewerError = '';
    try {
      const [page, status] = await Promise.all([
        listLogRecords({ kind, limit: 100 }),
        getLogFileStatus(),
      ]);
      if (requestSeq !== logRequestSeq || !open) {
        return;
      }
      logRecords = page.records;
      logFileStatus = status;
      logFileKind = kind;
    } catch (error) {
      if (requestSeq !== logRequestSeq || !open) {
        return;
      }
      logViewerError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSeq === logRequestSeq) {
        logViewerLoading = false;
      }
    }
  }

  async function handleSelectDirectory() {
    const currentOutputDir = outputDir;
    const dir = await selectDirectory(currentOutputDir);
    if (!dir || dir === currentOutputDir) {
      return;
    }

    outputDir = dir;
    const saved = await onOutputDirChange(dir);
    if (!saved) {
      outputDir = currentOutputDir;
      notifyError('保存下载目录失败，已恢复为之前的设置。');
    }
  }

  async function handleClearAudioCache() {
    if (isClearingAudioCache) return;
    isClearingAudioCache = true;
    try {
      const removed = await clearAudioCache();
      notifyInfo(
        removed > 0
          ? `已清除 ${removed} 个音频缓存文件`
          : '当前没有可清除的音频缓存'
      );
    } catch (error) {
      notifyError(
        `清除音频缓存失败：${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      isClearingAudioCache = false;
    }
  }

  async function handleSendTestNotification() {
    if (isSendingTestNotification) return;
    isSendingTestNotification = true;
    try {
      await sendTestNotification();
      notifyInfo('测试通知已请求发送，请观察系统通知中心或终端日志。');
    } catch (error) {
      notifyError(
        `发送测试通知失败：${error instanceof Error ? error.message : String(error)}`
      );
    } finally {
      isSendingTestNotification = false;
    }
  }

  $effect(() => {
    if (!open) {
      lastLoadedWhileOpen = false;
      logRequestSeq += 1;
      logViewerLoading = false;
      return;
    }

    if (lastLoadedWhileOpen) {
      return;
    }

    lastLoadedWhileOpen = true;
    void refreshLogs(logFileKind);
  });

  $effect(() => {
    const refreshToken = logRefreshToken;
    if (!open || !lastLoadedWhileOpen || refreshToken === 0) {
      return;
    }

    void refreshLogs(logFileKind);
  });

  const localeLabels = $derived.by(() => {
    void localeState.current;
    return {
      languageLabel: m.settings_language_label(),
      zhCN: m.settings_language_zh_cn(),
      enUS: m.settings_language_en_us(),
    };
  });

  const localeOptions = $derived([
    { value: 'zh-CN' as Locale, label: localeLabels.zhCN },
    { value: 'en-US' as Locale, label: localeLabels.enUS },
  ]);

  const currentLocaleLabel = $derived(
    localeOptions.find((option) => option.value === locale)?.label ??
      localeLabels.zhCN
  );

  const currentFormatLabel = $derived(
    formatOptions.find((option) => option.value === format)?.label ?? 'FLAC'
  );

  const currentLogLevelLabel = $derived(
    logLevelOptions.find((option) => option.value === logLevel)?.label ??
      'Error'
  );
</script>

<Sheet.Root bind:open>
  <Sheet.Content
    class="app-side-sheet settings-sheet gap-0 overflow-hidden border-[var(--settings-border)] bg-[var(--surface-sheet)] p-0 text-[var(--text-primary)] shadow-[0_24px_64px_rgba(15,23,42,0.18)] backdrop-blur-xl"
  >
    <Sheet.Header class="settings-sheet-header">
      <Sheet.Title>下载设置</Sheet.Title>
      <Sheet.Description>音频格式、通知和缓存管理</Sheet.Description>
    </Sheet.Header>

    <div class="settings-sheet-body">
      <section class="settings-section">
        <div class="settings-section-heading">
          <h3>基础偏好</h3>
        </div>

        <div class="settings-field-grid">
          <label class="settings-field" for="locale-select">
            <span>{localeLabels.languageLabel}</span>
            <Select.Root type="single" bind:value={locale}>
              <Select.Trigger
                id="locale-select"
                class="h-9 w-full border-[var(--settings-border)] bg-[var(--settings-control-bg)]"
              >
                {currentLocaleLabel}
              </Select.Trigger>
              <Select.Content>
                {#each localeOptions as option (option.value)}
                  <Select.Item value={option.value} label={option.label} />
                {/each}
              </Select.Content>
            </Select.Root>
          </label>

          <label class="settings-field" for="format-select">
            <span>输出格式</span>
            <Select.Root type="single" bind:value={format}>
              <Select.Trigger
                id="format-select"
                class="h-9 w-full border-[var(--settings-border)] bg-[var(--settings-control-bg)]"
              >
                {currentFormatLabel}
              </Select.Trigger>
              <Select.Content>
                {#each formatOptions as option (option.value)}
                  <Select.Item value={option.value} label={option.label} />
                {/each}
              </Select.Content>
            </Select.Root>
          </label>

          <label class="settings-field" for="log-level-select">
            <span>持久化日志等级</span>
            <Select.Root type="single" bind:value={logLevel}>
              <Select.Trigger
                id="log-level-select"
                class="h-9 w-full border-[var(--settings-border)] bg-[var(--settings-control-bg)]"
              >
                {currentLogLevelLabel}
              </Select.Trigger>
              <Select.Content>
                {#each logLevelOptions as option (option.value)}
                  <Select.Item value={option.value} label={option.label} />
                {/each}
              </Select.Content>
            </Select.Root>
          </label>

          <div class="settings-field">
            <label for="output-dir">保存位置</label>
            <div class="settings-path-row">
              <Input
                id="output-dir"
                class="h-9 border-[var(--settings-border)] bg-[var(--settings-control-bg)]"
                readonly
                value={outputDir}
              />
              <Button
                class="h-9 shrink-0"
                onclick={() => void handleSelectDirectory()}
              >
                <FolderOpenIcon data-icon="inline-start" />
                选择
              </Button>
            </div>
          </div>
        </div>
      </section>

      <section class="settings-section">
        <div class="settings-section-heading">
          <h3>通知与附加文件</h3>
          <Button
            variant="secondary"
            disabled={isSendingTestNotification}
            onclick={() => void handleSendTestNotification()}
          >
            <BellIcon data-icon="inline-start" />
            {isSendingTestNotification ? '发送中' : '测试'}
          </Button>
        </div>

        <div class="settings-toggle-list">
          <label class="settings-toggle">
            <span>
              <strong>歌词文件</strong>
              <small>有歌词时，在音频旁生成同名 `.lrc`。</small>
            </span>
            <Switch bind:checked={downloadLyrics} />
          </label>

          <label class="settings-toggle">
            <span>
              <strong>下载完成通知</strong>
              <small>下载任务完成时显示通知。</small>
            </span>
            <Switch bind:checked={notifyOnDownloadComplete} />
          </label>

          <label class="settings-toggle">
            <span>
              <strong>播放切换通知</strong>
              <small>播放新歌曲时显示通知。</small>
            </span>
            <Switch bind:checked={notifyOnPlaybackChange} />
          </label>
        </div>
      </section>

      <section class="settings-section settings-action-section">
        <div class="settings-section-heading">
          <div>
            <h3>音乐缓存</h3>
            <p>播放时的音频缓存保存在系统缓存目录。</p>
          </div>
          <Button
            variant="secondary"
            disabled={isClearingAudioCache}
            onclick={() => void handleClearAudioCache()}
          >
            <Trash2Icon data-icon="inline-start" />
            {isClearingAudioCache ? '清除中' : '清除'}
          </Button>
        </div>
      </section>

      <section class="settings-section">
        <div class="settings-section-heading settings-log-heading">
          <div>
            <h3>日志与诊断</h3>
            <p>本次运行日志会在正常退出时按等级写入持久化日志。</p>
          </div>
          <div class="settings-segment" aria-label="日志文件类型">
            <button
              type="button"
              class:active={logFileKind === 'session'}
              onclick={() => void refreshLogs('session')}
            >
              本次运行
            </button>
            <button
              type="button"
              class:active={logFileKind === 'persistent'}
              onclick={() => void refreshLogs('persistent')}
            >
              持久化
            </button>
          </div>
        </div>

        <p class="settings-log-status">
          session: {logFileStatus?.hasSessionLog ? '可用' : '暂无'} · persistent:
          {logFileStatus?.hasPersistentLog ? '可用' : '暂无'}
        </p>

        {#if logViewerLoading}
          <div class="settings-empty-state">正在加载日志…</div>
        {:else if logViewerError}
          <div class="settings-error-state">
            {logViewerError}
          </div>
        {:else if logRecords.length > 0}
          <div class="settings-log-list">
            {#each logRecords as record (record.id)}
              <article class="settings-log-record">
                <div class="settings-log-meta">
                  <span>{record.level}</span>
                  <time>{record.ts}</time>
                </div>
                <p class="settings-log-message">{record.message}</p>
                <p class="settings-log-source">
                  {record.domain} · {record.code}
                </p>
                {#if record.details}
                  <p class="settings-log-details">
                    {record.details}
                  </p>
                {/if}
              </article>
            {/each}
          </div>
        {:else}
          <div class="settings-empty-state">当前没有可显示的日志记录。</div>
        {/if}
      </section>
    </div>
  </Sheet.Content>
</Sheet.Root>

<style>
  :global(.settings-sheet) {
    --settings-border: color-mix(in srgb, var(--border) 78%, white 22%);
    --settings-section-bg: color-mix(
      in srgb,
      var(--bg-secondary) 76%,
      transparent
    );
    --settings-control-bg: color-mix(
      in srgb,
      var(--bg-primary) 54%,
      transparent
    );
    --settings-row-bg: color-mix(in srgb, var(--bg-primary) 42%, transparent);
    --settings-row-hover-bg: color-mix(
      in srgb,
      var(--bg-primary) 56%,
      transparent
    );
  }

  :global(.settings-sheet-header) {
    padding: 18px 48px 14px 18px;
    border-bottom: 1px solid var(--settings-border);
    background: linear-gradient(
      180deg,
      color-mix(in srgb, var(--surface-tint-strong) 72%, transparent),
      transparent
    );
  }

  .settings-sheet-body {
    display: flex;
    min-height: 0;
    flex: 1;
    flex-direction: column;
    gap: 12px;
    overflow-y: auto;
    padding: 14px 14px 18px;
  }

  .settings-section {
    display: grid;
    gap: 12px;
    border: 1px solid var(--settings-border);
    border-radius: 8px;
    background: var(--settings-section-bg);
    padding: 12px;
  }

  .settings-section-heading {
    display: flex;
    align-items: flex-start;
    justify-content: space-between;
    gap: 12px;
  }

  .settings-section-heading h3 {
    margin: 0;
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 700;
    letter-spacing: 0;
  }

  .settings-section-heading p {
    margin: 3px 0 0;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }

  .settings-field-grid {
    display: grid;
    gap: 10px;
  }

  .settings-field {
    display: grid;
    gap: 6px;
    min-width: 0;
    color: var(--text-secondary);
    font-size: 12px;
    font-weight: 600;
  }

  .settings-path-row {
    display: grid;
    grid-template-columns: minmax(0, 1fr) auto;
    gap: 8px;
  }

  .settings-toggle-list {
    display: grid;
    overflow: hidden;
    border: 1px solid var(--settings-border);
    border-radius: 8px;
  }

  .settings-toggle {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
    min-height: 58px;
    padding: 10px 12px;
    background: var(--settings-row-bg);
    cursor: pointer;
    transition: background var(--motion-fast) var(--ease-standard);
  }

  .settings-toggle + .settings-toggle {
    border-top: 1px solid var(--settings-border);
  }

  .settings-toggle:hover {
    background: var(--settings-row-hover-bg);
  }

  .settings-toggle span {
    display: grid;
    gap: 3px;
    min-width: 0;
  }

  .settings-toggle strong {
    color: var(--text-primary);
    font-size: 13px;
    font-weight: 600;
  }

  .settings-toggle small {
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.35;
  }

  .settings-action-section {
    padding-block: 13px;
  }

  .settings-log-heading {
    align-items: center;
  }

  .settings-segment {
    display: inline-grid;
    grid-template-columns: repeat(2, minmax(0, 1fr));
    overflow: hidden;
    border: 1px solid var(--settings-border);
    border-radius: 8px;
    background: var(--settings-row-bg);
    padding: 2px;
  }

  .settings-segment button {
    height: 26px;
    border: 0;
    border-radius: 6px;
    background: transparent;
    color: var(--text-secondary);
    font: inherit;
    font-size: 12px;
    font-weight: 600;
    cursor: pointer;
    transition:
      background var(--motion-fast) var(--ease-standard),
      color var(--motion-fast) var(--ease-standard);
  }

  .settings-segment button.active {
    background: var(--accent);
    color: white;
  }

  .settings-log-status {
    margin: -4px 0 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }

  .settings-log-list {
    display: grid;
    gap: 8px;
    max-height: 240px;
    overflow-y: auto;
    border: 1px solid var(--settings-border);
    border-radius: 8px;
    background: var(--settings-row-bg);
    padding: 8px;
  }

  .settings-log-record {
    display: grid;
    gap: 4px;
    border: 1px solid var(--settings-border);
    border-radius: 7px;
    background: color-mix(in srgb, var(--bg-primary) 52%, transparent);
    padding: 8px 10px;
  }

  .settings-log-meta {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 8px;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.35;
  }

  .settings-log-meta span {
    font-weight: 700;
    text-transform: uppercase;
  }

  .settings-log-message {
    margin: 0;
    color: var(--text-primary);
    font-size: 12px;
    font-weight: 600;
    line-height: 1.45;
  }

  .settings-log-source,
  .settings-log-details {
    margin: 0;
    color: var(--text-secondary);
    font-size: 11px;
    line-height: 1.4;
  }

  .settings-log-details {
    white-space: pre-wrap;
    overflow-wrap: anywhere;
  }

  .settings-empty-state,
  .settings-error-state {
    border: 1px solid var(--settings-border);
    border-radius: 8px;
    background: var(--settings-row-bg);
    padding: 14px 12px;
    color: var(--text-secondary);
    font-size: 12px;
    line-height: 1.45;
  }

  .settings-error-state {
    border-color: color-mix(in srgb, var(--destructive) 40%, transparent);
    background: color-mix(in srgb, var(--destructive) 10%, transparent);
    color: var(--destructive);
  }

  @media (max-width: 420px) {
    .settings-path-row,
    .settings-section-heading {
      grid-template-columns: 1fr;
    }

    .settings-section-heading {
      display: grid;
    }

    .settings-log-heading {
      align-items: stretch;
    }
  }
</style>
