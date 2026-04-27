<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import PlayerDock from '$lib/components/app/PlayerDock.svelte';
  import type { PlaybackQueueEntry } from '$lib/types';
  import type { LyricLine } from '$lib/features/player/lyrics';

  interface Song {
    cid: string;
    name: string;
    artists: string[];
    coverUrl: string | null;
  }

  type RepeatMode = 'all' | 'one';
  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    song: Song | null;
    isPlaying: boolean;
    isPaused: boolean;
    hasPrevious: boolean;
    hasNext: boolean;
    progress: number;
    duration: number;
    isLoading: boolean;
    reducedMotion: boolean;
    isShuffled: boolean;
    repeatMode: RepeatMode;
    lyricsOpen: boolean;
    playlistOpen: boolean;
    lyricsLoading: boolean;
    lyricsError: string;
    lyricsLines: LyricLine[];
    activeLyricIndex: number;
    playbackOrder: PlaybackQueueEntry[];
    downloadState: SongDownloadState;
    downloadDisabled: boolean;
    onPrevious: () => void | Promise<void>;
    onTogglePlay: () => void | Promise<void>;
    onSeek: (positionSecs: number) => void | Promise<void>;
    onNext: () => void | Promise<void>;
    onShuffleChange: (next: boolean) => void | Promise<void>;
    onRepeatModeChange: (next: RepeatMode) => void | Promise<void>;
    onToggleLyrics: () => void | Promise<void>;
    onTogglePlaylist: () => void | Promise<void>;
    onDownload: () => void | Promise<void>;
    onPlayQueueEntry: (
      entry: PlaybackQueueEntry,
      order: PlaybackQueueEntry[],
      index: number
    ) => void | Promise<void>;
  }

  let props: Props = $props();

  function dur(base: number): number {
    return props.reducedMotion ? 0 : base;
  }
</script>

{#if props.song}
  <div
    class="player-dock-stack-wrapper"
    in:fly={{ y: 18, duration: dur(220) }}
    out:fade={{ duration: dur(220) }}
  >
    <div
      class="player-dock-stack"
      data-panel={props.lyricsOpen
        ? 'lyrics'
        : props.playlistOpen
          ? 'playlist'
          : 'none'}
    >
      {#if props.lyricsOpen}
        <section
          class="player-flyout"
          data-panel="lyrics"
          in:fly={{ y: 12, duration: dur(180) }}
          out:fly={{ y: 8, duration: dur(180) }}
        >
          <div class="player-flyout-header">
            <div>
              <p class="player-flyout-eyebrow">歌词</p>
              <h3 class="player-flyout-title">{props.song.name}</h3>
            </div>
            <span class="player-flyout-count"
              >{props.lyricsLines.length > 0
                ? `${props.lyricsLines.length} 行`
                : '歌词'}</span
            >
          </div>

          {#if props.lyricsLoading}
            <div class="player-flyout-empty">正在加载歌词…</div>
          {:else if props.lyricsError}
            <div class="player-flyout-empty">{props.lyricsError}</div>
          {:else if props.lyricsLines.length > 0}
            <div class="player-lyrics-list">
              {#each props.lyricsLines as line, index (line.id)}
                <p
                  class={`player-lyric-line${index === props.activeLyricIndex ? ' active' : ''}`}
                >
                  {line.text}
                </p>
              {/each}
            </div>
          {:else}
            <div class="player-flyout-empty">这首歌暂时没有歌词。</div>
          {/if}
        </section>
      {:else if props.playlistOpen}
        <section
          class="player-flyout"
          data-panel="playlist"
          in:fly={{ y: 12, duration: dur(180) }}
          out:fly={{ y: 8, duration: dur(180) }}
        >
          <div class="player-flyout-header">
            <div>
              <p class="player-flyout-eyebrow">播放列表</p>
              <h3 class="player-flyout-title">当前队列</h3>
            </div>
            <span class="player-flyout-count"
              >{props.playbackOrder.length} 首</span
            >
          </div>

          {#if props.playbackOrder.length > 0}
            <div class="player-playlist-list">
              {#each props.playbackOrder as entry, index (entry.cid)}
                <button
                  type="button"
                  class={`player-playlist-item${entry.cid === props.song?.cid ? ' active' : ''}`}
                  onclick={() => {
                    void props.onPlayQueueEntry(
                      entry,
                      props.playbackOrder,
                      index
                    );
                  }}
                >
                  <span class="player-playlist-index"
                    >{String(index + 1).padStart(2, '0')}</span
                  >
                  <span class="player-playlist-meta">
                    <span class="player-playlist-name">{entry.name}</span>
                    <span class="player-playlist-artists"
                      >{entry.artists.join(' · ')}</span
                    >
                  </span>
                </button>
              {/each}
            </div>
          {:else}
            <div class="player-flyout-empty">当前没有可播放的队列。</div>
          {/if}
        </section>
      {/if}

      <PlayerDock
        song={props.song}
        isPlaying={props.isPlaying}
        isPaused={props.isPaused}
        hasPrevious={props.hasPrevious}
        hasNext={props.hasNext}
        progress={props.progress}
        duration={props.duration}
        isLoading={props.isLoading}
        isShuffled={props.isShuffled}
        repeatMode={props.repeatMode}
        lyricsActive={props.lyricsOpen}
        playlistActive={props.playlistOpen}
        downloadState={props.downloadState}
        downloadDisabled={props.downloadDisabled}
        reducedMotion={props.reducedMotion}
        onPrevious={props.onPrevious}
        onTogglePlay={props.onTogglePlay}
        onSeek={props.onSeek}
        onNext={props.onNext}
        onShuffleChange={props.onShuffleChange}
        onRepeatModeChange={props.onRepeatModeChange}
        onToggleLyrics={props.onToggleLyrics}
        onTogglePlaylist={props.onTogglePlaylist}
        onDownload={props.onDownload}
      />
    </div>
  </div>
{/if}
