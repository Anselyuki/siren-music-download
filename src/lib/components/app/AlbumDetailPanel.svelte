<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import SongRow from '$lib/components/SongRow.svelte';
  import {
    getDownloadBadgeLabel,
    shouldShowDownloadBadge,
  } from '$lib/downloadBadge';
  import type { AlbumDetail, SongEntry } from '$lib/types';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    album: AlbumDetail;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    downloadingAlbumCid: string | null;
    selectionModeEnabled: boolean;
    selectedSongCids: string[];
    reducedMotion: boolean;
    onToggleSelectionMode: () => void;
    onSelectAllSongs: () => void;
    onDeselectAllSongs: () => void;
    onInvertSongSelection: () => void;
    onDownloadAlbum: (albumCid: string) => void | Promise<void>;
    onDownloadSelection: (songCids: string[]) => void | Promise<void>;
    onPlaySong: (song: SongEntry) => void | Promise<void>;
    onDownloadSong: (songCid: string) => void | Promise<void>;
    onToggleSongSelection: (songCid: string) => void;
    isSongSelected: (songCid: string) => boolean;
    getSongDownloadState: (songCid: string) => SongDownloadState;
    isSongDownloadInteractionBlocked: (songCid: string) => boolean;
    hasAlbumDownloadJob: (albumCid: string) => boolean;
    isSelectionDownloadDisabled: (songCids: string[]) => boolean;
    isCurrentSelectionCreating: (songCids: string[]) => boolean;
    hasCurrentSelectionJob: (songCids: string[]) => boolean;
  }

  let props: Props = $props();

  function dur(base: number): number {
    return props.reducedMotion ? 0 : base;
  }

  const selectedSongCount = $derived.by(() => props.selectedSongCids.length);
  const selectedSongsLabel = $derived.by(() => {
    if (selectedSongCount === 0) return '未选择歌曲';
    if (selectedSongCount === 1) return '已选择 1 首';
    return `已选择 ${selectedSongCount} 首`;
  });
  const isAlbumDownloadCreating = $derived.by(
    () => props.downloadingAlbumCid === props.album.cid
  );
  const hasAlbumDownloadJob = $derived.by(() =>
    props.hasAlbumDownloadJob(props.album.cid)
  );
  const isAlbumDownloadDisabled = $derived.by(
    () => isAlbumDownloadCreating || hasAlbumDownloadJob
  );
  const isAllSongsSelected = $derived.by(
    () => selectedSongCount === props.album.songs.length
  );
  const canInvertSelection = $derived.by(() => props.album.songs.length > 0);
  const isSelectionCreating = $derived.by(() =>
    props.isCurrentSelectionCreating(props.selectedSongCids)
  );
  const hasCurrentSelectionJob = $derived.by(() =>
    props.hasCurrentSelectionJob(props.selectedSongCids)
  );
  const isSelectionDownloadDisabled = $derived.by(() =>
    props.isSelectionDownloadDisabled(props.selectedSongCids)
  );
</script>

<div
  class="album-detail-card"
  class:is-reduced-motion={props.reducedMotion}
  in:fade={{ duration: dur(220) }}
  out:fade={{ duration: dur(220) }}
>
  <div class="album-hero">
    <div
      class="album-hero-info"
      in:fly={{ y: 14, duration: dur(220), delay: dur(30) }}
      out:fly={{ y: 8, duration: dur(220) }}
    >
      {#if props.album.belong}
        <span class="album-belong-tag">{props.album.belong.toUpperCase()}</span>
      {/if}
      <h1 class="album-hero-title">{props.album.name}</h1>
      {#if props.album.artists && props.album.artists.length > 0}
        <p class="album-hero-artists">{props.album.artists.join(', ')}</p>
      {/if}
      {#if props.album.intro}
        <p class="album-hero-intro">{props.album.intro}</p>
      {/if}
      <div class="album-hero-meta">
        <span class="album-song-count">{props.album.songs.length} 首歌曲</span>
        {#if shouldShowDownloadBadge(props.album.download.downloadStatus)}
          <span class="album-download-status-badge">
            {getDownloadBadgeLabel(props.album.download.downloadStatus)}
          </span>
        {/if}
      </div>
      <div class="controls album-hero-actions">
        <button
          type="button"
          class="btn btn-primary"
          class:is-disabled={isAlbumDownloadDisabled}
          onclick={() => props.onDownloadAlbum(props.album.cid)}
          disabled={isAlbumDownloadDisabled}
        >
          {#if isAlbumDownloadCreating}
            正在创建任务...
          {:else if hasAlbumDownloadJob}
            已在队列中
          {:else}
            下载整张专辑
          {/if}
        </button>
        <button type="button" class="btn" onclick={props.onToggleSelectionMode}>
          {props.selectionModeEnabled ? '取消多选' : '多选下载'}
        </button>
        {#if props.selectionModeEnabled}
          <button
            type="button"
            class="btn"
            class:is-disabled={isAllSongsSelected}
            onclick={props.onSelectAllSongs}
            disabled={isAllSongsSelected}
          >
            全选
          </button>
          <button
            type="button"
            class="btn"
            class:is-disabled={selectedSongCount === 0}
            onclick={props.onDeselectAllSongs}
            disabled={selectedSongCount === 0}
          >
            清空
          </button>
          <button
            type="button"
            class="btn"
            class:is-disabled={!canInvertSelection}
            onclick={props.onInvertSongSelection}
            disabled={!canInvertSelection}
          >
            反选
          </button>
          <button
            type="button"
            class="btn btn-primary"
            class:is-disabled={isSelectionDownloadDisabled}
            onclick={() => props.onDownloadSelection(props.selectedSongCids)}
            disabled={isSelectionDownloadDisabled}
          >
            {#if isSelectionCreating}
              正在创建批量任务...
            {:else if hasCurrentSelectionJob}
              已在队列中
            {:else}
              下载所选歌曲
            {/if}
          </button>
          <span class="album-selection-summary">{selectedSongsLabel}</span>
        {/if}
      </div>
    </div>
  </div>
  <div
    class="song-list"
    in:fly={{ y: 10, duration: dur(200), delay: dur(70) }}
    out:fade={{ duration: dur(200) }}
  >
    {#each props.album.songs as song, index (song.cid)}
      <SongRow
        {song}
        {index}
        isPlaying={props.currentSongCid === song.cid && props.isPlaybackActive}
        downloadState={props.getSongDownloadState(song.cid)}
        downloadDisabled={props.isSongDownloadInteractionBlocked(song.cid)}
        selectionMode={props.selectionModeEnabled}
        isSelected={props.isSongSelected(song.cid)}
        selectionDisabled={isSelectionCreating}
        reducedMotion={props.reducedMotion}
        onclick={() => props.onPlaySong(song)}
        onDownload={() => props.onDownloadSong(song.cid)}
        onToggleSelection={() => props.onToggleSongSelection(song.cid)}
      />
    {/each}
  </div>
</div>

<style>
  .btn {
    transition:
      background-color 0.16s ease-out,
      color 0.16s ease-out,
      box-shadow 0.16s ease-out,
      opacity 0.16s ease-out;
  }

  .btn:hover:not(:disabled):not(.is-reduced-motion *) {
    transform: translateY(-1px);
  }

  .btn:active:not(:disabled):not(.is-reduced-motion *) {
    transform: translateY(0) scale(0.98);
    opacity: 0.94;
  }

  .btn:not(.btn-primary):hover:not(:disabled) {
    background: var(--hover-bg-elevated);
    box-shadow: 0 8px 20px rgba(15, 23, 42, 0.08);
  }

  .btn-primary:hover:not(:disabled) {
    background: var(--accent-hover);
    box-shadow: 0 10px 24px rgba(var(--accent-rgb), 0.2);
  }

  .btn.is-disabled {
    opacity: 0.42;
  }

  .btn-primary.is-disabled {
    background: var(--bg-tertiary);
    color: var(--text-tertiary);
    box-shadow: 0 0 0 rgba(var(--accent-rgb), 0);
    opacity: 0.72;
  }

  .is-reduced-motion .btn {
    transition: none;
  }
</style>
