<script lang="ts">
  import type { SongEntry } from '$lib/types';
  import {
    getDownloadBadgeLabel,
    shouldShowDownloadBadge,
  } from '$lib/downloadBadge';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';
  interface Props {
    song: SongEntry;
    index: number;
    isPlaying?: boolean;
    downloadState?: SongDownloadState;
    downloadDisabled?: boolean;
    selectionMode?: boolean;
    isSelected?: boolean;
    selectionDisabled?: boolean;
    reducedMotion?: boolean;
    onclick?: () => void;
    onDownload?: () => void;
    onToggleSelection?: () => void;
  }
  let {
    song,
    index,
    isPlaying = false,
    downloadState = 'idle',
    downloadDisabled = false,
    selectionMode = false,
    isSelected = false,
    selectionDisabled = false,
    reducedMotion = false,
    onclick,
    onDownload,
    onToggleSelection,
  }: Props = $props();
  let isHovered = $state(false);
  let isFocused = $state(false);
  const showEmphasis = $derived.by(
    () => isPlaying || isHovered || isFocused || isSelected
  );
  const showPlayIndicator = $derived.by(
    () => isPlaying || isHovered || isFocused
  );
  const showDownloadedBadge = $derived.by(() =>
    shouldShowDownloadBadge(song.download.downloadStatus)
  );
  const downloadedBadgeLabel = $derived.by(() =>
    getDownloadBadgeLabel(song.download.downloadStatus)
  );
  const isBusy = $derived.by(() => downloadState !== 'idle');
  const isDownloadDisabled = $derived.by(
    () => isBusy || downloadDisabled || selectionMode
  );
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      downloadCreatingAria: m.common_download_creating_aria({
        name: song.name,
      }),
      downloadQueuedAria: m.common_download_queued_aria({ name: song.name }),
      downloadRunningAria: m.common_download_running_aria({ name: song.name }),
      downloadIdleAria: m.common_download_idle_aria({ name: song.name }),
      downloadCreatingTitle: m.common_download_creating_title(),
      downloadQueuedTitle: m.common_download_queued_title(),
      downloadRunningTitle: m.common_download_running_title(),
      downloadIdleTitle: m.common_download_idle_title(),
      deselectAria: m.common_selection_deselect_aria({ name: song.name }),
      selectAria: m.common_selection_select_aria({ name: song.name }),
      playAria: m.library_song_row_play_aria({ name: song.name }),
    };
  });
  const downloadButtonLabel = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return labels.downloadCreatingAria;
      case 'queued':
        return labels.downloadQueuedAria;
      case 'running':
        return labels.downloadRunningAria;
      default:
        return labels.downloadIdleAria;
    }
  });
  const downloadButtonTitle = $derived.by(() => {
    switch (downloadState) {
      case 'creating':
        return labels.downloadCreatingTitle;
      case 'queued':
        return labels.downloadQueuedTitle;
      case 'running':
        return labels.downloadRunningTitle;
      default:
        return labels.downloadIdleTitle;
    }
  });
  function handleRowActivate() {
    if (selectionMode) {
      if (!selectionDisabled) {
        onToggleSelection?.();
      }
      return;
    }
    onclick?.();
  }
</script>

<div
  class="song-row"
  class:is-selection-mode={selectionMode}
  class:is-selected={isSelected}
  class:is-playing={isPlaying}
  class:is-hovered={isHovered || isFocused}
  class:is-reduced-motion={reducedMotion}
  data-song-cid={song.cid}
  role="button"
  tabindex="0"
  aria-label={selectionMode
    ? isSelected
      ? labels.deselectAria
      : labels.selectAria
    : labels.playAria}
  onclick={handleRowActivate}
  onmouseenter={() => {
    isHovered = true;
  }}
  onmouseleave={() => {
    isHovered = false;
  }}
  onfocusin={() => {
    isFocused = true;
  }}
  onfocusout={() => {
    isFocused = false;
  }}
  onkeydown={(e: KeyboardEvent) => {
    if (e.key === 'Enter' || e.key === ' ') {
      e.preventDefault();
      handleRowActivate();
    }
  }}
>
  {#if selectionMode}
    <button
      type="button"
      class="song-selection-toggle"
      class:is-selected={isSelected}
      disabled={selectionDisabled}
      aria-label={isSelected ? labels.deselectAria : labels.selectAria}
      aria-pressed={isSelected}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        onToggleSelection?.();
      }}><span class="song-selection-dot"></span></button
    >
  {/if}
  <div class="song-number" class:is-emphasis={showEmphasis}>{index + 1}</div>
  <div class="song-info">
    <div class="song-name" class:is-emphasis={showEmphasis}>{song.name}</div>
    <div class="song-artists">{song.artists.join(' · ')}</div>
  </div>
  <div
    class="song-play-indicator"
    class:is-playing={isPlaying}
    class:is-visible={showPlayIndicator}
  >
    <svg class="play-indicator-icon" viewBox="0 0 24 24" aria-hidden="true">
      {#if isPlaying}<rect x="7.15" y="5.95" width="3.4" height="12.1" rx="1.25"
        ></rect><rect x="13.45" y="5.95" width="3.4" height="12.1" rx="1.25"
        ></rect>{:else}<path d="M8.2 6.3v11.4L17.35 12z"></path>{/if}
    </svg>
  </div>
  <div class="song-actions">
    {#if showDownloadedBadge}<span class="song-download-badge"
        >{downloadedBadgeLabel}</span
      >{/if}
    <button
      type="button"
      class="song-download-button"
      class:is-busy={isBusy}
      disabled={isDownloadDisabled}
      aria-label={downloadButtonLabel}
      title={downloadButtonTitle}
      onclick={(event: MouseEvent) => {
        event.stopPropagation();
        onDownload?.();
      }}
    >
      <svg class="download-icon" viewBox="0 0 24 24" aria-hidden="true">
        {#if downloadState === 'creating' || downloadState === 'running'}<circle
            class="download-spinner-ring"
            cx="12"
            cy="12"
            r="8"
          ></circle>{:else if downloadState === 'queued'}<circle
            cx="12"
            cy="12"
            r="2.5"
          ></circle><circle cx="12" cy="6" r="1.5" opacity="0.4"
          ></circle><circle cx="12" cy="18" r="1.5" opacity="0.4"
          ></circle>{:else}<path d="M12 4v12m0 0-4-4m4 4 4-4"></path><path
            d="M4 17v2a1 1 0 0 0 1 1h14a1 1 0 0 0 1-1v-2"
          ></path>{/if}
      </svg>
    </button>
  </div>
</div>

<style>
  .song-row {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 10px 16px;
    border-radius: 14px;
    cursor: pointer;
    outline: none;
    background: rgba(15, 23, 42, 0);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0);
    transition:
      background-color 0.16s ease-out,
      box-shadow 0.16s ease-out;
  }
  .song-row:not(.is-reduced-motion):active {
    transform: scale(0.996);
  }
  .song-row.is-hovered:not(.is-playing):not(.is-selected) {
    background: rgba(15, 23, 42, 0.04);
  }
  .song-row.is-playing {
    background: rgba(var(--accent-rgb), 0.1);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.08);
  }
  .song-row.is-selected {
    background: rgba(var(--accent-rgb), 0.12);
    box-shadow: inset 0 0 0 1px rgba(var(--accent-rgb), 0.12);
  }
  .song-row.is-reduced-motion {
    transition: none;
  }
  .song-number {
    width: 28px;
    text-align: center;
    font-size: 13px;
    font-weight: 500;
    color: var(--text-tertiary);
    flex-shrink: 0;
    transition:
      color 0.16s ease-out,
      opacity 0.16s ease-out;
  }
  .song-number.is-emphasis {
    color: var(--accent);
    opacity: 0.86;
  }
  .song-row.is-reduced-motion .song-number {
    transition: none;
  }
  .song-info {
    flex: 1;
    min-width: 0;
  }
  .song-name {
    font-size: 14px;
    font-weight: 500;
    color: var(--text-primary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    transition: color 0.16s ease-out;
  }
  .song-name.is-emphasis {
    color: var(--accent);
  }
  .song-row.is-reduced-motion .song-name {
    transition: none;
  }
  .song-artists {
    font-size: 12px;
    color: var(--text-secondary);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    margin-top: 2px;
  }
  .song-play-indicator {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    opacity: 0;
    background: rgba(15, 23, 42, 0.05);
    color: var(--text-secondary);
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
    transition:
      opacity 0.16s ease-out,
      transform 0.16s ease-out,
      background-color 0.16s ease-out,
      color 0.16s ease-out,
      box-shadow 0.16s ease-out;
  }
  .song-play-indicator.is-visible:not(.is-playing) {
    opacity: 1;
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
  .song-play-indicator.is-playing {
    opacity: 1;
    background: var(--accent);
    color: #ffffff;
    box-shadow: 0 10px 20px rgba(var(--accent-rgb), 0.18);
  }
  .song-play-indicator:not(.is-visible):not(.is-playing) {
    transform: scale(0.92);
  }
  .song-row.is-reduced-motion .song-play-indicator {
    transition: none;
    transform: scale(1);
  }
  .play-indicator-icon {
    width: 16px;
    height: 16px;
    fill: currentColor;
    stroke: none;
  }
  .song-actions {
    display: flex;
    align-items: center;
    gap: 8px;
    flex-shrink: 0;
  }
  .song-download-badge {
    display: inline-flex;
    align-items: center;
    padding: 3px 8px;
    border-radius: 999px;
    font-size: 11px;
    line-height: 1;
    color: var(--accent);
    background: rgba(var(--accent-rgb), 0.1);
    border: 1px solid rgba(var(--accent-rgb), 0.12);
  }
  .song-download-button {
    appearance: none;
    width: 32px;
    height: 32px;
    border-radius: 50%;
    border: none;
    background: transparent;
    color: var(--text-tertiary);
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    transition:
      background-color 0.16s ease-out,
      color 0.16s ease-out;
  }
  .song-download-button:hover:not(:disabled) {
    background: rgba(var(--accent-rgb), 0.1);
    color: var(--accent);
  }
  .song-download-button:disabled {
    opacity: 0.42;
    cursor: default;
  }
  .song-download-button.is-busy {
    color: var(--accent);
    opacity: 1;
  }
  .song-row.is-reduced-motion .song-download-button {
    transition: none;
  }
  .download-icon {
    width: 18px;
    height: 18px;
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-linecap: round;
    stroke-linejoin: round;
  }
  .download-spinner-ring {
    fill: none;
    stroke: currentColor;
    stroke-width: 2;
    stroke-dasharray: 20 30;
    animation: download-spin 0.9s linear infinite;
  }
  @keyframes download-spin {
    to {
      transform: rotate(360deg);
    }
  }
  .song-selection-toggle {
    appearance: none;
    width: 24px;
    height: 24px;
    border-radius: 50%;
    border: 2px solid var(--text-tertiary);
    background: transparent;
    cursor: pointer;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
    flex-shrink: 0;
    transition:
      border-color 0.16s ease-out,
      background-color 0.16s ease-out;
  }
  .song-selection-toggle.is-selected {
    border-color: var(--accent);
    background: var(--accent);
  }
  .song-selection-toggle:disabled {
    opacity: 0.42;
    cursor: default;
  }
  .song-row.is-reduced-motion .song-selection-toggle {
    transition: none;
  }
  .song-selection-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: transparent;
    transition: background-color 0.16s ease-out;
  }
  .song-selection-toggle.is-selected .song-selection-dot {
    background: white;
  }
  .song-row.is-reduced-motion .song-selection-dot {
    transition: none;
  }
</style>
