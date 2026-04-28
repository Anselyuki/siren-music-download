<script lang="ts">
  import { fade, fly } from 'svelte/transition';
  import PlayerDock from '$lib/components/app/PlayerDock.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
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
  const labels = $derived.by(() => {
    void localeState.current;
    return {
      lyricsEyebrow: m.player_lyrics_eyebrow(),
      lyricsLoading: m.player_lyrics_loading(),
      lyricsEmpty: m.player_lyrics_empty(),
      queueEyebrow: m.player_queue_eyebrow(),
      queueTitle: m.player_queue_title(),
      queueEmpty: m.player_queue_empty(),
    };
  });
  const lyricsCountLabel = $derived.by(() => {
    void localeState.current;
    return props.lyricsLines.length > 0
      ? m.player_lyrics_line_count({ count: props.lyricsLines.length })
      : labels.lyricsEyebrow;
  });
  const queueCountLabel = $derived.by(() => {
    void localeState.current;
    return m.player_queue_count({ count: props.playbackOrder.length });
  });
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
              <p class="player-flyout-eyebrow">{labels.lyricsEyebrow}</p>
              <h3 class="player-flyout-title">{props.song.name}</h3>
            </div>
            <span class="player-flyout-count">{lyricsCountLabel}</span>
          </div>
          {#if props.lyricsLoading}
            <div class="player-flyout-empty">{labels.lyricsLoading}</div>
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
            <div class="player-flyout-empty">{labels.lyricsEmpty}</div>
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
              <p class="player-flyout-eyebrow">{labels.queueEyebrow}</p>
              <h3 class="player-flyout-title">{labels.queueTitle}</h3>
            </div>
            <span class="player-flyout-count">{queueCountLabel}</span>
          </div>
          {#if props.playbackOrder.length > 0}
            <div class="player-playlist-list">
              {#each props.playbackOrder as entry, index (entry.cid)}
                <button
                  type="button"
                  class={`player-playlist-item${entry.cid === props.song?.cid ? ' active' : ''}`}
                  aria-label={m.player_queue_item_aria({
                    index: index + 1,
                    name: entry.name,
                  })}
                  aria-current={entry.cid === props.song?.cid
                    ? 'true'
                    : undefined}
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
                  <span class="player-playlist-meta"
                    ><span class="player-playlist-name">{entry.name}</span><span
                      class="player-playlist-artists"
                      >{entry.artists.join(' · ')}</span
                    ></span
                  >
                </button>
              {/each}
            </div>
          {:else}
            <div class="player-flyout-empty">{labels.queueEmpty}</div>
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
