<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { Button } from '$lib/components/ui/button/index.js';
  import { toolbarIconButton } from '$lib/design/variants';

  interface Props {
    activeDownloadCount: number;
    isRefreshing?: boolean;
    settingsOpen?: boolean;
    downloadPanelOpen?: boolean;
    onRefresh: () => void;
    onOpenDownloads: () => void;
    onOpenSettings: () => void;
  }

  let {
    activeDownloadCount,
    isRefreshing = false,
    settingsOpen = false,
    downloadPanelOpen = false,
    onRefresh,
    onOpenDownloads,
    onOpenSettings,
  }: Props = $props();

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      refresh: m.shell_toolbar_refresh(),
      downloads: m.shell_toolbar_downloads(),
      settings: m.shell_toolbar_settings(),
    };
  });
</script>

<div class="top-actions">
  <div
    class="flex items-center gap-2 rounded-full border border-white/50 bg-white/[0.62] p-2 shadow-[0_16px_36px_rgba(15,23,42,0.12)] backdrop-blur-xl"
  >
    <Button
      size="icon"
      variant="ghost"
      class={`text-base ${toolbarIconButton({ active: false })}`}
      onclick={onRefresh}
      disabled={isRefreshing}
      aria-label={labels.refresh}
      title={labels.refresh}
    >
      ↻
    </Button>

    <Button
      size="icon"
      variant="ghost"
      class={`relative text-base ${toolbarIconButton({ active: downloadPanelOpen })}`}
      onclick={onOpenDownloads}
      aria-label={labels.downloads}
      aria-pressed={downloadPanelOpen}
      title={labels.downloads}
    >
      ↓
      {#if activeDownloadCount > 0}
        <span class="toolbar-badge">{activeDownloadCount}</span>
      {/if}
    </Button>

    <Button
      size="icon"
      variant="ghost"
      class={`text-base ${toolbarIconButton({ active: settingsOpen })}`}
      onclick={onOpenSettings}
      aria-label={labels.settings}
      aria-pressed={settingsOpen}
      title={labels.settings}
    >
      ⚙
    </Button>
  </div>
</div>
