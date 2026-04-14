<script lang="ts">
  import { onMount, tick } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { AnimatePresence, motion } from '@humanspeak/svelte-motion';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { OverlayScrollbarsComponentRef } from 'overlayscrollbars-svelte';
  import type { EventListeners, OverlayScrollbars, PartialOptions } from 'overlayscrollbars';
  import {
    getAlbums, getAlbumDetail, getDefaultOutputDir, selectDirectory,
    playSong, pausePlayback, resumePlayback,
    seekCurrentPlayback, getPlayerState, clearAudioCache, extractImageTheme,
    getSongLyrics, downloadSong
  } from '$lib/api';
  import { clearCache } from '$lib/cache';
  import type {
    Album,
    AlbumDetail,
    OutputFormat,
    SongEntry,
    PlayerState,
    PlaybackContext,
    PlaybackQueueEntry,
  } from '$lib/types';
  import { applyThemePalette, DEFAULT_THEME_PALETTE } from '$lib/theme';
  import { motionStyles } from '$lib/actions/motionStyles';
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import SongRow from '$lib/components/SongRow.svelte';
  import AudioPlayer from '$lib/components/AudioPlayer.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';

  // Minimum display time (ms) to prevent animation flash on fast loads
  const MIN_DISPLAY_MS = 260;
  const DETAIL_SKELETON_DELAY_MS = 140;
  const PANEL_DURATION = 0.18;
  const HERO_DURATION = 0.22;
  const HERO_DELAY = 0.03;
  const LIST_DURATION = 0.2;
  const LIST_DELAY = 0.07;
  const CONTENT_MASK_DURATION = 0.14;
  const OVERLAY_DURATION = 0.16;
  const SHEET_DURATION = 0.22;
  const PLAYER_DOCK_DURATION = 0.22;
  const DEFAULT_ALBUM_STAGE_ASPECT_RATIO = 16 / 9;
  const ALBUM_STAGE_BASE_VIEWPORT_RATIO = 1 / 3;
  const ALBUM_STAGE_COLLAPSE_SCROLL_RANGE = 260;
  const ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE = 220;
  const DOWNLOAD_LYRICS_PREF_KEY = 'siren:download-lyrics-sidecar';

  const delay = (ms: number) => new Promise(resolve => setTimeout(resolve, ms));

  type RepeatMode = 'all' | 'one';

  interface PlayerSong {
    cid: string;
    name: string;
    artists: string[];
    coverUrl: string | null;
  }

  interface LyricLine {
    id: string;
    time: number | null;
    text: string;
  }

  let albums = $state<Album[]>([]);
  let selectedAlbum = $state<AlbumDetail | null>(null);
  let selectedAlbumCid = $state<string | null>(null);
  let outputDir = $state('');
  let format = $state<OutputFormat>('flac');
  let loadingAlbums = $state(false);
  let loadingDetail = $state(false);
  let errorMsg = $state('');

  // Audio player state (synced from Rust backend via Tauri events)
  let currentSong = $state<PlayerSong | null>(null);
  let isPlaying = $state(false);
  let isPaused = $state(false);
  let isLoading = $state(false);
  let hasPrevious = $state(false);
  let hasNext = $state(false);
  let progress = $state(0);
  let duration = $state(0);
  let shuffleEnabled = $state(false);
  let repeatMode = $state<RepeatMode>('all');
  let playbackEntries = $state<PlaybackQueueEntry[]>([]);
  let playbackOrder = $state<PlaybackQueueEntry[]>([]);
  let playbackIndex = $state(-1);
  let lyricsOpen = $state(false);
  let playlistOpen = $state(false);
  let lyricsLoading = $state(false);
  let lyricsError = $state('');
  let lyricsLines = $state<LyricLine[]>([]);
  let lyricsSongCid = $state<string | null>(null);
  let downloadingSongCid = $state<string | null>(null);
  // Track which song is currently being loaded to prevent duplicate play calls
  let playingCid = $state<string | null>(null);
  let albumRequestSeq = $state(0);
  let themeRequestSeq = 0;
  let lyricRequestSeq = 0;
  let playbackEndRequestSeq = 0;
  let lastPlaybackSnapshot = {
    cid: null as string | null,
    active: false,
  };
  let prefersReducedMotion = $state(false);
  let showDetailSkeleton = $state(false);
  let contentEl = $state<HTMLElement | null>(null);
  let contentScrollbar = $state<OverlayScrollbarsComponentRef<'main'> | null>(null);
  let albumStageEl = $state<HTMLElement | null>(null);
  let isMacOS = $state(false);
  let detailSkeletonTimer: ReturnType<typeof setTimeout> | null = null;
  let albumStageAspectRatio = $state(DEFAULT_ALBUM_STAGE_ASPECT_RATIO);
  let albumStageWidth = $state(0);
  let viewportHeight = $state(0);
  let albumStageCollapseOffset = $state(0);
  let albumStageScrollTop = $state(0);
  let albumStageMotionFrame = 0;
  let pendingAlbumStageCollapseOffset = 0;
  let pendingAlbumStageScrollTop = 0;

  const playerHasPrevious = $derived.by(() => playbackOrder.length > 1);
  const playerHasNext = $derived.by(() => playbackOrder.length > 1);

  const activeLyricIndex = $derived.by(() => {
    let activeIndex = -1;

    for (let index = 0; index < lyricsLines.length; index += 1) {
      const lineTime = lyricsLines[index]?.time;
      if (lineTime === null || lineTime === undefined) continue;
      if (progress + 0.08 >= lineTime) {
        activeIndex = index;
      } else {
        break;
      }
    }

    return activeIndex;
  });

  function setContentViewport(instance: OverlayScrollbars) {
    const viewport = instance.elements().viewport;
    if (contentEl !== viewport) {
      contentEl = viewport;
    }
  }

  const overlayScrollbarOptions = $derived.by((): PartialOptions => ({
    scrollbars: {
      theme: 'os-theme-app',
      autoHide: prefersReducedMotion ? 'leave' : 'move',
      autoHideDelay: prefersReducedMotion ? 160 : 720,
      autoHideSuspend: true,
      dragScroll: true,
      clickScroll: false,
    },
  }));

  const contentScrollbarEvents = $derived.by((): EventListeners => ({
    initialized(instance) {
      setContentViewport(instance);
      handleContentScroll();
    },
    updated(instance) {
      setContentViewport(instance);
    },
    destroyed() {
      contentEl = null;
    },
    scroll(instance) {
      setContentViewport(instance);
      handleContentScroll();
    },
  }));

  function resetContentScroll() {
    resetAlbumStageMotion();
    contentEl?.scrollTo({ top: 0, behavior: prefersReducedMotion ? 'auto' : 'smooth' });
  }

  interface ImageMeta {
    aspectRatio: number;
  }

  function getImageMeta(image: HTMLImageElement): ImageMeta | null {
    const width = image.naturalWidth || image.width;
    const height = image.naturalHeight || image.height;

    if (!width || !height) {
      return null;
    }

    return {
      aspectRatio: width / height,
    };
  }

  function preloadImage(src: string | null | undefined): Promise<ImageMeta | null> {
    if (!src) return Promise.resolve(null);

    return new Promise((resolve) => {
      const image = new Image();
      let settled = false;

      const finish = (meta: ImageMeta | null) => {
        if (settled) return;
        settled = true;
        resolve(meta);
      };

      image.decoding = 'async';
      image.onload = () => finish(getImageMeta(image));
      image.onerror = () => finish(null);
      image.src = src;

      if (image.complete) {
        queueMicrotask(() => finish(getImageMeta(image)));
      }
    });
  }

  async function preloadAlbumArtwork(album: AlbumDetail): Promise<number | null> {
    const meta = await preloadImage(album.coverDeUrl ?? album.coverUrl ?? null);
    return meta?.aspectRatio ?? null;
  }

  function setAlbumStageAspectRatio(value: number | null | undefined) {
    if (value && Number.isFinite(value) && value > 0) {
      albumStageAspectRatio = value;
      return;
    }

    albumStageAspectRatio = DEFAULT_ALBUM_STAGE_ASPECT_RATIO;
  }

  function clamp(value: number, min: number, max: number): number {
    return Math.min(max, Math.max(min, value));
  }

  function getSelectedAlbumCoverUrl(): string | null {
    return selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;
  }

  function normalizePlayerSong(state: PlayerState): PlayerSong | null {
    if (!state.songCid) return null;

    return {
      cid: state.songCid,
      name: state.songName ?? '',
      artists: state.artists,
      coverUrl: state.coverUrl ?? null,
    };
  }

  function buildAlbumPlaybackEntries(album: AlbumDetail | null): PlaybackQueueEntry[] {
    if (!album) return [];

    const coverUrl = album.coverUrl ?? album.coverDeUrl ?? null;
    return album.songs.map((entry) => ({
      cid: entry.cid,
      name: entry.name,
      artists: entry.artists,
      coverUrl,
    }));
  }

  function buildSinglePlaybackEntry(song: PlayerSong): PlaybackQueueEntry {
    return {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: song.coverUrl,
    };
  }

  function shufflePlaybackEntries(entries: PlaybackQueueEntry[], currentCid: string | null): PlaybackQueueEntry[] {
    if (entries.length <= 1) return [...entries];

    const rest = [...entries];
    let pinnedEntry: PlaybackQueueEntry | null = null;

    if (currentCid) {
      const pinnedIndex = rest.findIndex((entry) => entry.cid === currentCid);
      if (pinnedIndex >= 0) {
        pinnedEntry = rest.splice(pinnedIndex, 1)[0];
      }
    }

    for (let index = rest.length - 1; index > 0; index -= 1) {
      const swapIndex = Math.floor(Math.random() * (index + 1));
      [rest[index], rest[swapIndex]] = [rest[swapIndex], rest[index]];
    }

    return pinnedEntry ? [pinnedEntry, ...rest] : rest;
  }

  function applyPlaybackQueue(entries: PlaybackQueueEntry[], currentCid: string | null) {
    playbackEntries = [...entries];

    if (!entries.length) {
      playbackOrder = [];
      playbackIndex = -1;
      return;
    }

    playbackOrder = shuffleEnabled ? shufflePlaybackEntries(entries, currentCid) : [...entries];
    playbackIndex = currentCid
      ? playbackOrder.findIndex((entry) => entry.cid === currentCid)
      : 0;

    if (playbackIndex < 0) {
      playbackIndex = 0;
    }
  }

  function buildPlaybackContext(order: PlaybackQueueEntry[], currentIndex: number): PlaybackContext | undefined {
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

  function syncPlaybackQueueWithSong(song: PlayerSong | null) {
    if (!song) {
      playbackIndex = -1;
      return;
    }

    const currentOrderIndex = playbackOrder.findIndex((entry) => entry.cid === song.cid);
    if (currentOrderIndex >= 0) {
      playbackIndex = currentOrderIndex;
      return;
    }

    const currentSourceIndex = playbackEntries.findIndex((entry) => entry.cid === song.cid);
    if (currentSourceIndex >= 0) {
      applyPlaybackQueue(playbackEntries, song.cid);
      return;
    }

    applyPlaybackQueue([buildSinglePlaybackEntry(song)], song.cid);
  }

  function parseLyricText(source: string): LyricLine[] {
    const lines = source
      .split(/\r?\n/)
      .map((line) => line.trim())
      .filter(Boolean);
    const parsed: LyricLine[] = [];

    for (const rawLine of lines) {
      const matches = [...rawLine.matchAll(/\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g)];
      const text = rawLine.replace(/\[(\d{1,2}):(\d{2})(?:\.(\d{1,3}))?\]/g, '').trim() || '♪';

      if (!matches.length) {
        parsed.push({
          id: `plain-${parsed.length}`,
          time: null,
          text,
        });
        continue;
      }

      for (const match of matches) {
        const minutes = Number(match[1] ?? 0);
        const seconds = Number(match[2] ?? 0);
        const fractionText = match[3] ?? '0';
        const fraction = Number(`0.${fractionText.padEnd(3, '0')}`);
        parsed.push({
          id: `${minutes}:${seconds}:${fractionText}:${parsed.length}`,
          time: minutes * 60 + seconds + fraction,
          text,
        });
      }
    }

    return parsed.sort((left, right) => {
      if (left.time === null && right.time === null) return 0;
      if (left.time === null) return 1;
      if (right.time === null) return -1;
      return left.time - right.time;
    });
  }

  async function loadLyrics(songCid: string) {
    const requestSeq = ++lyricRequestSeq;
    lyricsSongCid = songCid;
    lyricsLoading = true;
    lyricsError = '';
    lyricsLines = [];

    try {
      const lyricText = await getSongLyrics(songCid);
      if (requestSeq !== lyricRequestSeq) return;

      if (!lyricText) {
        lyricsLoading = false;
        return;
      }

      lyricsLines = parseLyricText(lyricText);
    } catch (error) {
      if (requestSeq !== lyricRequestSeq) return;
      lyricsError = error instanceof Error ? error.message : String(error);
    } finally {
      if (requestSeq === lyricRequestSeq) {
        lyricsLoading = false;
      }
    }
  }

  function syncPlayerState(state: PlayerState) {
    currentSong = normalizePlayerSong(state);
    isPlaying = state.isPlaying;
    isPaused = state.isPaused;
    isLoading = state.isLoading;
    hasPrevious = state.hasPrevious;
    hasNext = state.hasNext;
    progress = state.progress;
    duration = state.duration;

    if (!state.isLoading) {
      playingCid = null;
    }

    syncPlaybackQueueWithSong(currentSong);
  }

  async function playQueueEntry(
    entry: PlaybackQueueEntry,
    order = playbackOrder,
    index = order.findIndex((candidate) => candidate.cid === entry.cid),
    options: { forceRestart?: boolean } = {},
  ) {
    if (index < 0) return;

    playbackOrder = [...order];
    playbackIndex = index;

    if (!options.forceRestart) {
      if (currentSong?.cid === entry.cid && isPaused) {
        await handleResumePlayback();
        return;
      }

      if (currentSong?.cid === entry.cid && (isPlaying || isLoading)) {
        return;
      }

      if (playingCid === entry.cid) {
        return;
      }
    }

    playingCid = entry.cid;

    try {
      await playSong(entry.cid, entry.coverUrl ?? undefined, buildPlaybackContext(order, index));
    } catch (error) {
      playingCid = null;
      console.error('[ERROR] Failed to play song from queue:', error);
    }
  }

  function resolveWrappedQueueIndex(step: 1 | -1): number {
    if (!playbackOrder.length) return -1;
    if (playbackIndex < 0) return step > 0 ? 0 : playbackOrder.length - 1;
    return (playbackIndex + step + playbackOrder.length) % playbackOrder.length;
  }

  function handleShuffleChange(next: boolean) {
    shuffleEnabled = next;
    if (!playbackEntries.length) return;

    const currentCid = currentSong?.cid ?? playbackEntries[0]?.cid ?? null;
    applyPlaybackQueue(playbackEntries, currentCid);
  }

  function handleRepeatModeChange(next: RepeatMode) {
    repeatMode = next;
  }

  function toggleLyricsPanel() {
    if (!currentSong) return;
    lyricsOpen = !lyricsOpen;
    if (lyricsOpen) {
      playlistOpen = false;
    }
  }

  function togglePlaylistPanel() {
    if (!currentSong) return;
    playlistOpen = !playlistOpen;
    if (playlistOpen) {
      lyricsOpen = false;
    }
  }

  async function performSongDownload(songCid: string) {
    if (downloadingSongCid) return null;

    downloadingSongCid = songCid;
    try {
      return await downloadSong(songCid, outputDir, format, downloadLyrics);
    } finally {
      if (downloadingSongCid === songCid) {
        downloadingSongCid = null;
      }
    }
  }

  async function handleCurrentSongDownload() {
    if (!currentSong) return;
    try {
      const outputPath = await performSongDownload(currentSong.cid);
      if (!outputPath) return;
      alert(`已下载到：\n${outputPath}`);
    } catch (error) {
      console.error('[ERROR] Failed to download current song:', error);
      alert(`下载失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async function handlePlaybackEnded(songCid: string) {
    const requestSeq = ++playbackEndRequestSeq;
    const currentIndex = playbackOrder.findIndex((entry) => entry.cid === songCid);
    if (currentIndex < 0) return;

    if (repeatMode === 'one') {
      await playQueueEntry(playbackOrder[currentIndex], playbackOrder, currentIndex, { forceRestart: true });
      return;
    }

    const nextIndex = (currentIndex + 1) % playbackOrder.length;
    if (requestSeq !== playbackEndRequestSeq) return;
    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex, { forceRestart: true });
  }

  function flushAlbumStageMotion() {
    albumStageMotionFrame = 0;

    if (albumStageCollapseOffset !== pendingAlbumStageCollapseOffset) {
      albumStageCollapseOffset = pendingAlbumStageCollapseOffset;
    }

    if (albumStageScrollTop !== pendingAlbumStageScrollTop) {
      albumStageScrollTop = pendingAlbumStageScrollTop;
    }
  }

  function scheduleAlbumStageMotion(
    next: {
      collapseOffset?: number;
      scrollTop?: number;
    },
    immediate = false,
  ) {
    pendingAlbumStageCollapseOffset = next.collapseOffset ?? pendingAlbumStageCollapseOffset;
    pendingAlbumStageScrollTop = next.scrollTop ?? pendingAlbumStageScrollTop;

    if (immediate || prefersReducedMotion || typeof window === 'undefined') {
      if (albumStageMotionFrame) {
        cancelAnimationFrame(albumStageMotionFrame);
        albumStageMotionFrame = 0;
      }
      flushAlbumStageMotion();
      return;
    }

    if (albumStageMotionFrame) {
      return;
    }

    albumStageMotionFrame = requestAnimationFrame(() => {
      flushAlbumStageMotion();
    });
  }

  function resetAlbumStageMotion() {
    if (albumStageMotionFrame) {
      cancelAnimationFrame(albumStageMotionFrame);
      albumStageMotionFrame = 0;
    }

    pendingAlbumStageCollapseOffset = 0;
    pendingAlbumStageScrollTop = 0;
    albumStageCollapseOffset = 0;
    albumStageScrollTop = 0;
  }

  function syncAlbumStageWidth() {
    albumStageWidth = albumStageEl?.clientWidth ?? 0;
  }

  function syncViewportHeight() {
    viewportHeight = window.innerHeight || 0;
  }

  const albumStageFullHeight = $derived.by(() => {
    if (!albumStageWidth || !albumStageAspectRatio) {
      return 0;
    }

    return albumStageWidth / albumStageAspectRatio;
  });

  const albumStageBaseHeight = $derived.by(() => {
    if (!albumStageWidth) {
      return 0;
    }

    return Math.min(albumStageFullHeight, viewportHeight * ALBUM_STAGE_BASE_VIEWPORT_RATIO);
  });

  const albumStageCollapseProgress = $derived.by(() =>
    clamp(albumStageCollapseOffset / ALBUM_STAGE_COLLAPSE_SCROLL_RANGE, 0, 1)
  );

  const albumStageRevealProgress = $derived.by(() =>
    1 - albumStageCollapseProgress
  );

  const albumStageSolidifyProgress = $derived.by(() =>
    Math.max(
      albumStageCollapseProgress,
      clamp(albumStageScrollTop / ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE, 0, 1)
    )
  );

  const albumStageHeight = $derived.by(() => {
    if (!albumStageBaseHeight) {
      return 0;
    }

    return albumStageBaseHeight + (albumStageFullHeight - albumStageBaseHeight) * albumStageRevealProgress;
  });

  const albumStageStyle = $derived.by(() => `--album-stage-aspect-ratio: ${albumStageAspectRatio}`);

  type MotionTarget = Record<string, string | number>;

  function motionTransition(duration: number, delay = 0) {
    return {
      duration: prefersReducedMotion ? 0 : duration,
      delay: prefersReducedMotion ? 0 : delay,
      ease: 'easeOut',
    };
  }

  function fadeEnter(opacity = 0): MotionTarget {
    return prefersReducedMotion ? { opacity: 1 } : { opacity };
  }

  function fadeExit(opacity = 0): MotionTarget {
    return { opacity };
  }

  function axisEnter(axis: 'x' | 'y', offset: number): MotionTarget {
    return prefersReducedMotion ? { opacity: 1 } : { opacity: 0, [axis]: offset };
  }

  function axisAnimate(axis: 'x' | 'y'): MotionTarget {
    return { opacity: 1, [axis]: 0 };
  }

  function axisExit(axis: 'x' | 'y', offset: number): MotionTarget {
    return prefersReducedMotion ? { opacity: 0 } : { opacity: 0, [axis]: offset };
  }

  const interactiveTransition = $derived.by(() => ({
    duration: prefersReducedMotion ? 0 : 0.16,
    ease: 'easeOut',
  } as const));

  const albumStageMotionHeight = $derived.by(() => (
    albumStageHeight > 0 ? albumStageHeight : Math.max(albumStageBaseHeight || 0, 280)
  ));

  const albumStageMediaHeight = $derived.by(() => `${albumStageMotionHeight}px`);
  const albumStageScrimOpacity = $derived.by(() => Math.max(0.58, 1 - albumStageSolidifyProgress * 0.34));
  const albumStageImageOpacity = $derived.by(() => 1 - albumStageSolidifyProgress * 0.54);
  const albumStageImageTransform = $derived.by(() =>
    prefersReducedMotion
      ? 'translateZ(0) scale(1)'
      : `translateZ(0) scale(${1 + albumStageRevealProgress * 0.006 + albumStageSolidifyProgress * 0.012})`
  );
  const albumStageSolidifyOpacity = $derived.by(() => albumStageSolidifyProgress);

  function modeButtonAnimate(active: boolean): MotionTarget {
    return active
      ? {
          backgroundColor: 'var(--accent)',
          color: '#ffffff',
          boxShadow: '0 10px 22px rgba(var(--accent-rgb), 0.22)',
        }
      : {
          backgroundColor: 'rgba(15, 23, 42, 0)',
          color: 'rgba(31, 41, 55, 0.72)',
          boxShadow: '0 0 0 rgba(var(--accent-rgb), 0)',
        };
  }

  function modeButtonHover(active: boolean): MotionTarget | undefined {
    if (active) {
      return prefersReducedMotion ? undefined : { y: -1 };
    }

    return {
      backgroundColor: 'rgba(15, 23, 42, 0.06)',
      color: 'var(--text-primary)',
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function toolbarButtonAnimate(active = false, accented = false, disabled = false): MotionTarget {
    return {
      opacity: disabled ? 0.42 : 1,
      backgroundColor: active ? 'var(--accent-light)' : 'rgba(255, 255, 255, 0.78)',
      color: active || accented ? 'var(--accent)' : 'var(--text-primary)',
      boxShadow: 'inset 0 1px 0 rgba(255, 255, 255, 0.94)',
    };
  }

  function toolbarButtonHover(disabled = false): MotionTarget | undefined {
    if (disabled) {
      return undefined;
    }

    return {
      backgroundColor: 'rgba(var(--accent-rgb), 0.1)',
      boxShadow: '0 10px 22px rgba(var(--accent-rgb), 0.14)',
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function appButtonAnimate(primary = false, disabled = false): MotionTarget {
    return primary
      ? {
          backgroundColor: disabled ? 'var(--bg-tertiary)' : 'var(--accent)',
          color: disabled ? 'var(--text-tertiary)' : '#ffffff',
          boxShadow: disabled ? '0 0 0 rgba(var(--accent-rgb), 0)' : '0 10px 24px rgba(var(--accent-rgb), 0.16)',
          opacity: disabled ? 0.72 : 1,
        }
      : {
          backgroundColor: 'var(--bg-tertiary)',
          color: 'var(--text-primary)',
          boxShadow: '0 0 0 rgba(var(--accent-rgb), 0)',
          opacity: disabled ? 0.42 : 1,
        };
  }

  function appButtonHover(primary = false, disabled = false): MotionTarget | undefined {
    if (disabled) return undefined;

    return primary
      ? {
          backgroundColor: 'var(--accent-hover)',
          boxShadow: '0 10px 24px rgba(var(--accent-rgb), 0.2)',
          ...(prefersReducedMotion ? {} : { y: -1 }),
        }
      : {
          backgroundColor: 'var(--hover-bg-elevated)',
          boxShadow: '0 8px 20px rgba(15, 23, 42, 0.08)',
          ...(prefersReducedMotion ? {} : { y: -1 }),
        };
  }

  function settingsCloseAnimate(): MotionTarget {
    return {
      backgroundColor: 'var(--bg-tertiary)',
      color: 'var(--text-secondary)',
    };
  }

  function settingsCloseHover(): MotionTarget {
    return {
      backgroundColor: 'var(--hover-bg-elevated)',
      color: 'var(--text-primary)',
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function fieldAnimate(): MotionTarget {
    return {
      backgroundColor: 'var(--bg-tertiary)',
      borderColor: 'var(--border)',
      color: 'var(--text-primary)',
      boxShadow: '0 0 0 0 rgba(var(--accent-rgb), 0)',
    };
  }

  function fieldHover(): MotionTarget {
    return {
      borderColor: 'var(--text-tertiary)',
    };
  }

  function fieldFocus(): MotionTarget {
    return {
      borderColor: 'var(--accent)',
      backgroundColor: 'var(--accent-light)',
      boxShadow: '0 0 0 3px rgba(var(--accent-rgb), 0.12)',
    };
  }

  function fieldMotion(hovered: boolean, focused: boolean): MotionTarget {
    if (focused) return fieldFocus();
    if (hovered) return fieldHover();
    return fieldAnimate();
  }

  function armDetailSkeleton() {
    if (detailSkeletonTimer) {
      clearTimeout(detailSkeletonTimer);
    }

    showDetailSkeleton = false;
    detailSkeletonTimer = setTimeout(() => {
      if (loadingDetail) {
        showDetailSkeleton = true;
      }
    }, DETAIL_SKELETON_DELAY_MS);
  }

  function clearDetailSkeleton() {
    if (detailSkeletonTimer) {
      clearTimeout(detailSkeletonTimer);
      detailSkeletonTimer = null;
    }
    showDetailSkeleton = false;
  }

  $effect(() => {
    const paletteRequestSeq = ++themeRequestSeq;
    const artworkUrl = selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;

    if (!artworkUrl) {
      applyThemePalette(DEFAULT_THEME_PALETTE);
      return;
    }

    void (async () => {
      try {
        const palette = await extractImageTheme(artworkUrl);
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(palette);
      } catch (e) {
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(DEFAULT_THEME_PALETTE);
        console.error('[ERROR] Failed to extract album theme:', e);
      }
    })();
  });

  $effect(() => {
    if (!albumStageEl) return;

    syncAlbumStageWidth();

    if (typeof ResizeObserver === 'undefined') return;

    const observer = new ResizeObserver(() => {
      syncAlbumStageWidth();
    });

    observer.observe(albumStageEl);

    return () => observer.disconnect();
  });

  $effect(() => {
    if (typeof window === 'undefined' || !downloadLyricsPrefReady) return;

    try {
      window.localStorage.setItem(DOWNLOAD_LYRICS_PREF_KEY, downloadLyrics ? '1' : '0');
    } catch {
      // ignore storage failures
    }
  });

  onMount(() => {
    isMacOS = /Mac|iPhone|iPad|iPod/.test(navigator.platform) || navigator.userAgent.includes('Mac');

    let unlistenState: (() => void) | null = null;
    let unlistenProgress: (() => void) | null = null;
    const mediaQuery = window.matchMedia('(prefers-reduced-motion: reduce)');

    function updateReducedMotionPreference() {
      prefersReducedMotion = mediaQuery.matches;
    }

    function handleWindowResize() {
      syncViewportHeight();
      syncAlbumStageWidth();
    }

    async function initialize() {
      loadingAlbums = true;
      try {
        const stored = window.localStorage.getItem(DOWNLOAD_LYRICS_PREF_KEY);
        if (stored !== null) {
          downloadLyrics = stored === '1';
        }
      } catch {
        // ignore storage failures
      } finally {
        downloadLyricsPrefReady = true;
      }

      try {
        [albums, outputDir] = await Promise.all([
          getAlbums(),
          getDefaultOutputDir(),
        ]);
        // Auto-select the first album on startup
        if (albums.length > 0) {
          await handleSelectAlbum(albums[0]);
        }
      } catch (e) {
        errorMsg = e instanceof Error ? e.message : String(e);
        console.error('[ERROR] Failed to load albums:', e);
      } finally {
        loadingAlbums = false;
      }

      unlistenState = await listen<PlayerState>('player-state-changed', (event) => {
        syncPlayerState(event.payload);
      });

      unlistenProgress = await listen<PlayerState>('player-progress', (event) => {
        const state = event.payload;
        progress = state.progress;
        isPlaying = state.isPlaying;
        isPaused = state.isPaused;
        duration = state.duration;
      });

      try {
        syncPlayerState(await getPlayerState());
      } catch {
        // Player not playing on startup
      }
    }

    syncViewportHeight();
    updateReducedMotionPreference();
    mediaQuery.addEventListener('change', updateReducedMotionPreference);
    window.addEventListener('resize', handleWindowResize, { passive: true });
    void initialize();

    return () => {
      clearDetailSkeleton();
      if (albumStageMotionFrame) {
        cancelAnimationFrame(albumStageMotionFrame);
      }
      unlistenState?.();
      unlistenProgress?.();
      mediaQuery.removeEventListener('change', updateReducedMotionPreference);
      window.removeEventListener('resize', handleWindowResize);
    };
  });

  $effect(() => {
    const songCid = currentSong?.cid ?? null;

    if (!songCid) {
      lyricRequestSeq += 1;
      lyricsSongCid = null;
      lyricsLines = [];
      lyricsError = '';
      lyricsLoading = false;
      lyricsOpen = false;
      playlistOpen = false;
      lastPlaybackSnapshot = { cid: null, active: false };
      return;
    }

    if (songCid === lyricsSongCid) {
      return;
    }

    void loadLyrics(songCid);
  });

  $effect(() => {
    const songCid = currentSong?.cid ?? null;
    const playbackActive = isPlaying || isPaused || isLoading;
    const hasReachedEnd =
      !!songCid &&
      duration > 0 &&
      progress >= Math.max(0, duration - 0.35);
    const shouldAutoAdvance =
      !!songCid &&
      songCid === lastPlaybackSnapshot.cid &&
      lastPlaybackSnapshot.active &&
      !playbackActive &&
      hasReachedEnd;

    lastPlaybackSnapshot = {
      cid: songCid,
      active: playbackActive,
    };

    if (shouldAutoAdvance) {
      void handlePlaybackEnded(songCid);
    }
  });

  async function handleSelectAlbum(album: Album) {
    if (album.cid === selectedAlbumCid && !loadingDetail) {
      return;
    }

    const requestSeq = ++albumRequestSeq;
    selectedAlbumCid = album.cid;
    loadingDetail = true;
    if (!selectedAlbum) {
      armDetailSkeleton();
    } else {
      clearDetailSkeleton();
    }

    const startTime = Date.now();
    try {
      const detail = await getAlbumDetail(album.cid);
      if (requestSeq !== albumRequestSeq) return;
      const artworkAspectRatio = await preloadAlbumArtwork(detail);
      if (requestSeq !== albumRequestSeq) return;
      selectedAlbum = detail;
      setAlbumStageAspectRatio(artworkAspectRatio);
      errorMsg = '';
      await tick();
      resetContentScroll();
    } catch (e) {
      if (requestSeq !== albumRequestSeq) return;
      errorMsg = e instanceof Error ? e.message : String(e);
      console.error('[ERROR] Failed to load album detail:', e);
    } finally {
      if (requestSeq !== albumRequestSeq) return;
      // Ensure minimum display time so animations aren't interrupted
      const elapsed = Date.now() - startTime;
      if (elapsed < MIN_DISPLAY_MS) {
        await delay(MIN_DISPLAY_MS - elapsed);
      }
      if (requestSeq !== albumRequestSeq) return;
      clearDetailSkeleton();
      loadingDetail = false;
    }
  }

  function handleContentScroll() {
    if (loadingDetail) {
      scheduleAlbumStageMotion({ scrollTop: 0 }, true);
      return;
    }

    const nextScrollTop = Math.max(0, contentEl?.scrollTop ?? 0);
    const nextCollapseOffset =
      nextScrollTop > 0 && pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ? ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        : undefined;

    scheduleAlbumStageMotion({
      scrollTop: nextScrollTop,
      collapseOffset: nextCollapseOffset,
    });
  }

  function handleContentWheel(event: WheelEvent) {
    if (loadingDetail || !contentEl) {
      return;
    }

    const atTop = (contentEl.scrollTop ?? 0) <= 0.5;

    if (!atTop) {
      return;
    }

    if (event.deltaY > 0 && pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE) {
      event.preventDefault();
      scheduleAlbumStageMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ),
        scrollTop: 0,
      });
      return;
    }

    if (event.deltaY < 0 && pendingAlbumStageCollapseOffset > 0) {
      event.preventDefault();
      scheduleAlbumStageMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
        ),
        scrollTop: 0,
      });
    }
  }

  let settingsOpen = $state(false);
  let isClearingAudioCache = $state(false);
  let downloadLyrics = $state(true);
  let downloadLyricsPrefReady = $state(false);
  let isFormatHovered = $state(false);
  let isFormatFocused = $state(false);
  let isOutputDirHovered = $state(false);
  let isOutputDirFocused = $state(false);

  async function handleSelectDirectory() {
    const dir = await selectDirectory(outputDir);
    if (dir) outputDir = dir;
  }

  async function handleClearAudioCache() {
    if (isClearingAudioCache) return;
    isClearingAudioCache = true;
    try {
      const removed = await clearAudioCache();
      alert(removed > 0 ? `已清除 ${removed} 个音频缓存文件` : '当前没有可清除的音频缓存');
    } catch (e) {
      console.error('[ERROR] Failed to clear audio cache:', e);
      alert(`清除音频缓存失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      isClearingAudioCache = false;
    }
  }

  async function handleSongDownload(song: SongEntry) {
    try {
      const outputPath = await performSongDownload(song.cid);
      if (!outputPath) return;
      alert(`已下载到：\n${outputPath}`);
    } catch (error) {
      console.error('[ERROR] Failed to download song:', error);
      alert(`下载失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }

  async function handlePlay(song: SongEntry) {
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];

    applyPlaybackQueue(entries, song.cid);

    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;

    await playQueueEntry(nextOrder[nextIndex], nextOrder, nextIndex);
  }

  async function handlePausePlayback() {
    try {
      await pausePlayback();
    } catch (e) {
      console.error('[ERROR] Failed to pause playback:', e);
    }
  }

  async function handleResumePlayback() {
    try {
      await resumePlayback();
    } catch (e) {
      console.error('[ERROR] Failed to resume playback:', e);
    }
  }

  async function handleSeekPlayback(positionSecs: number) {
    if (!duration || duration <= 0 || isLoading) return;
    try {
      await seekCurrentPlayback(positionSecs);
    } catch (e) {
      console.error('[ERROR] Failed to seek playback:', e);
    }
  }

  async function handlePlayNext() {
    if (!playerHasNext) return;

    const nextIndex = resolveWrappedQueueIndex(1);
    if (nextIndex < 0) return;

    await playQueueEntry(playbackOrder[nextIndex], playbackOrder, nextIndex);
  }

  async function handlePlayPrevious() {
    if (!currentSong) return;

    if (progress > 3 && !isLoading) {
      await handleSeekPlayback(0);
      return;
    }

    const previousIndex = resolveWrappedQueueIndex(-1);
    if (previousIndex < 0) return;

    await playQueueEntry(playbackOrder[previousIndex], playbackOrder, previousIndex);
  }

  // Refresh cache and reload current album
  let isRefreshing = $state(false);

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;
    const requestSeq = ++albumRequestSeq;

    // Clear cache
    clearCache();

    // Reload current album if selected
    if (selectedAlbumCid) {
      const currentAlbumCid = selectedAlbumCid;
      loadingDetail = true;
      if (!selectedAlbum) {
        armDetailSkeleton();
      } else {
        clearDetailSkeleton();
      }
      try {
        const detail = await getAlbumDetail(currentAlbumCid);
        if (requestSeq === albumRequestSeq) {
          const artworkAspectRatio = await preloadAlbumArtwork(detail);
          if (requestSeq === albumRequestSeq) {
            setAlbumStageAspectRatio(artworkAspectRatio);
          }
        }
        if (requestSeq === albumRequestSeq) {
          selectedAlbum = detail;
          await tick();
          resetContentScroll();
        }
      } catch (e) {
        if (requestSeq === albumRequestSeq) {
          console.error('[ERROR] Failed to reload album:', e);
        }
      } finally {
        if (requestSeq === albumRequestSeq) {
          clearDetailSkeleton();
          loadingDetail = false;
        }
      }
    }

    // Brief delay to show spinning state
    await delay(400);
    isRefreshing = false;
  }
</script>

{#if isMacOS}
  <div class="macos-window-drag-region" data-tauri-drag-region aria-hidden="true"></div>
{/if}

<div class="container" class:macos-overlay={isMacOS}>
  <!-- 专辑列表侧边栏 -->
  <OverlayScrollbarsComponent
    element="aside"
    class="sidebar"
    data-overlayscrollbars-initialize
    options={overlayScrollbarOptions}
    defer
  >
    {#if isMacOS}
      <div class="sidebar-drag-region" data-tauri-drag-region aria-hidden="true"></div>
    {/if}
    <h2 class="section-title">专辑</h2>
    {#if loadingAlbums}
      <div class="loading"><span>正在加载专辑...</span><MotionSpinner className="inline-loading-spinner" reducedMotion={prefersReducedMotion} /></div>
    {:else if errorMsg && albums.length === 0}
      <div class="empty-state">
        <div class="empty-icon">⚠️</div>
        <div class="empty-text">加载失败</div>
        <div class="empty-text" style="margin-top: 8px; font-size: 12px;">{errorMsg}</div>
      </div>
    {:else}
      <div class="album-list">
        {#each albums as album}
          <AlbumCard
            {album}
            selected={selectedAlbumCid === album.cid}
            reducedMotion={prefersReducedMotion}
            onclick={() => handleSelectAlbum(album)}
          />
        {/each}
      </div>
    {/if}
  </OverlayScrollbarsComponent>

  <section class="main-region">
    {#if isMacOS}
      <div class="main-drag-region" data-tauri-drag-region aria-hidden="true"></div>
    {/if}

    <div class="top-actions">
      <div class="top-toolbar" role="toolbar" aria-label="页面操作">
        <motion.button
          class="toolbar-icon-btn"
          onclick={handleRefresh}
          disabled={isRefreshing}
          aria-label="刷新缓存"
          title="刷新缓存"
          animate={toolbarButtonAnimate(false, false, isRefreshing)}
          whileHover={toolbarButtonHover(isRefreshing)}
          whileTap={!prefersReducedMotion && !isRefreshing ? { y: 0, scale: 0.96, opacity: 0.92 } : undefined}
          transition={interactiveTransition}
        >
          <motion.svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" animate={isRefreshing && !prefersReducedMotion ? { rotate: 360 } : { rotate: 0 }} transition={{ duration: prefersReducedMotion ? 0 : 0.9, ease: 'linear', repeat: isRefreshing && !prefersReducedMotion ? Infinity : 0 }}>
            <path d="M21 12a9 9 0 1 1-6.86-8.72"/>
            <polyline points="21 3 21 12 12 12"/>
          </motion.svg>
        </motion.button>

        <motion.button
          class={`toolbar-icon-btn${settingsOpen ? ' active' : ''}`}
          onclick={() => settingsOpen = !settingsOpen}
          aria-label="下载设置"
          aria-pressed={settingsOpen}
          title="下载设置"
          animate={toolbarButtonAnimate(settingsOpen, false, false)}
          whileHover={toolbarButtonHover(false)}
          whileTap={prefersReducedMotion ? undefined : { y: 0, scale: 0.96, opacity: 0.92 }}
          transition={interactiveTransition}
        >
          <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12.22 2h-.44a2 2 0 0 0-2 2v.18a2 2 0 0 1-1 1.73l-.43.25a2 2 0 0 1-2 0l-.15-.08a2 2 0 0 0-2.73.73l-.22.38a2 2 0 0 0 .73 2.73l.15.1a2 2 0 0 1 1 1.72v.51a2 2 0 0 1-1 1.74l-.15.09a2 2 0 0 0-.73 2.73l.22.38a2 2 0 0 0 2.73.73l.15-.08a2 2 0 0 1 2 0l.43.25a2 2 0 0 1 1 1.73V20a2 2 0 0 0 2 2h.44a2 2 0 0 0 2-2v-.18a2 2 0 0 1 1-1.73l.43-.25a2 2 0 0 1 2 0l.15.08a2 2 0 0 0 2.73-.73l.22-.39a2 2 0 0 0-.73-2.73l-.15-.08a2 2 0 0 1-1-1.74v-.5a2 2 0 0 1 1-1.74l.15-.09a2 2 0 0 0 .73-2.73l-.22-.38a2 2 0 0 0-2.73-.73l-.15.08a2 2 0 0 1-2 0l-.43-.25a2 2 0 0 1-1-1.73V4a2 2 0 0 0-2-2z"/>
            <circle cx="12" cy="12" r="3"/>
          </svg>
        </motion.button>
      </div>
    </div>

    <!-- 歌曲列表内容区 -->
    <OverlayScrollbarsComponent
      element="main"
      class={`content${currentSong ? ' content-with-player' : ''}${loadingDetail && selectedAlbum ? ' content-pending' : ''}`}
      data-overlayscrollbars-initialize
      bind:this={contentScrollbar}
      options={overlayScrollbarOptions}
      events={contentScrollbarEvents}
      defer
      onwheel={handleContentWheel}
      aria-busy={loadingDetail}
    >
      <AnimatePresence mode="wait">
        {#if loadingDetail && showDetailSkeleton}
          <motion.section
            key={`loading-${albumRequestSeq}`}
            class="album-panel album-panel-loading"
            initial={fadeEnter()}
            animate={{ opacity: 1 }}
            exit={fadeExit()}
            transition={motionTransition(PANEL_DURATION)}
          >
            <div class="album-stage" bind:this={albumStageEl} style={albumStageStyle}>
              <div class="album-stage-frame">
                <div
                  class="album-stage-media album-stage-media-loading"
                  style:height={albumStageMediaHeight}
                >
                  <div class="album-stage-media-content">
                    <MotionPulseBlock className="album-stage-skeleton loading-cover" reducedMotion={prefersReducedMotion} />
                  </div>
                  <div
                    class="album-stage-media-scrim"
                    aria-hidden="true"
                    style:opacity={albumStageScrimOpacity}
                  ></div>
                  <div class="album-stage-media-border" aria-hidden="true"></div>
                  <div class="album-stage-divider" aria-hidden="true"></div>
                </div>
              </div>
            </div>
            <motion.div
              class="album-detail-card"
              initial={fadeEnter()}
              animate={{ opacity: 1 }}
              exit={fadeExit()}
              transition={motionTransition(PANEL_DURATION)}
            >
              <div class="album-hero">
                <motion.div
                  class="album-hero-info"
                  initial={fadeEnter()}
                  animate={{ opacity: 1 }}
                  exit={fadeExit()}
                  transition={motionTransition(HERO_DURATION, HERO_DELAY)}
                >
                  <MotionPulseBlock className="album-hero-title loading-text" reducedMotion={prefersReducedMotion} />
                  <MotionPulseBlock className="album-hero-sub loading-text-sub" reducedMotion={prefersReducedMotion} delay={0.14} />
                </motion.div>
              </div>
              <motion.div
                class="loading album-loading"
                initial={fadeEnter()}
                animate={{ opacity: 1 }}
                exit={fadeExit()}
                transition={motionTransition(LIST_DURATION, LIST_DELAY)}
              >
                <span>正在加载歌曲...</span><MotionSpinner className="inline-loading-spinner" reducedMotion={prefersReducedMotion} />
              </motion.div>
            </motion.div>
          </motion.section>
        {:else if selectedAlbum}
          <motion.section
            key={selectedAlbum.cid}
            class="album-panel"
            initial={fadeEnter()}
            animate={{ opacity: 1 }}
            exit={fadeExit()}
            transition={motionTransition(PANEL_DURATION)}
          >
            <div class="album-stage" bind:this={albumStageEl} style={albumStageStyle}>
              <div class="album-stage-frame">
                <div
                  class="album-stage-media"
                  style:height={albumStageMediaHeight}
                >
                  <div class="album-stage-media-content">
                    <img
                      class="album-stage-image"
                      src={selectedAlbum.coverDeUrl ?? selectedAlbum.coverUrl}
                      alt="{selectedAlbum.name} banner"
                      loading="eager"
                      style:opacity={albumStageImageOpacity}
                      style:transform={albumStageImageTransform}
                    />
                    <div
                      class="album-stage-solidify"
                      aria-hidden="true"
                      style:opacity={albumStageSolidifyOpacity}
                    ></div>
                  </div>
                  <div
                    class="album-stage-media-scrim"
                    aria-hidden="true"
                    style:opacity={albumStageScrimOpacity}
                  ></div>
                  <div class="album-stage-media-border" aria-hidden="true"></div>
                  <div class="album-stage-divider" aria-hidden="true"></div>
                </div>
              </div>
            </div>
            <motion.div
              class="album-detail-card"
              initial={fadeEnter()}
              animate={{ opacity: 1 }}
              exit={fadeExit()}
              transition={motionTransition(PANEL_DURATION)}
            >
              <div class="album-hero">
                <motion.div
                  class="album-hero-info"
                  initial={axisEnter('y', 14)}
                  animate={axisAnimate('y')}
                  exit={axisExit('y', 8)}
                  transition={motionTransition(HERO_DURATION, HERO_DELAY)}
                >
                  {#if selectedAlbum.belong}
                    <span class="album-belong-tag">{selectedAlbum.belong.toUpperCase()}</span>
                  {/if}
                  <h1 class="album-hero-title">{selectedAlbum.name}</h1>
                  {#if selectedAlbum.artists && selectedAlbum.artists.length > 0}
                    <p class="album-hero-artists">{selectedAlbum.artists.join(', ')}</p>
                  {/if}
                  {#if selectedAlbum.intro}
                    <p class="album-hero-intro">{selectedAlbum.intro}</p>
                  {/if}
                  <div class="album-hero-meta">
                    <span class="album-song-count">{selectedAlbum.songs.length} 首歌曲</span>
                  </div>
                </motion.div>
              </div>
              <motion.div
                class="song-list"
                initial={axisEnter('y', 10)}
                animate={axisAnimate('y')}
                exit={fadeExit()}
                transition={motionTransition(LIST_DURATION, LIST_DELAY)}
              >
                {#each selectedAlbum.songs as song, i (song.cid)}
                  <SongRow
                    {song}
                    index={i}
                    isPlaying={currentSong?.cid === song.cid && (isPlaying || isPaused)}
                    isDownloading={downloadingSongCid === song.cid}
                    reducedMotion={prefersReducedMotion}
                    onclick={() => handlePlay(song)}
                    onDownload={() => handleSongDownload(song)}
                  />
                {/each}
              </motion.div>
            </motion.div>
          </motion.section>
        {/if}
      </AnimatePresence>

      {#if !loadingDetail && !selectedAlbum}
        <h1 class="page-title">选择专辑</h1>
        <p class="page-subtitle">从左侧选择一个专辑以查看歌曲</p>
      {/if}

      <AnimatePresence>
        {#if loadingDetail && selectedAlbum}
          <motion.div
            key={`content-mask-${albumRequestSeq}`}
            class="content-loading-mask"
            aria-hidden="true"
            initial={fadeEnter()}
            animate={{ opacity: 1 }}
            exit={fadeExit()}
            transition={motionTransition(CONTENT_MASK_DURATION)}
          >
            <MotionSpinner className="content-loading-mask-spinner" reducedMotion={prefersReducedMotion} />
          </motion.div>
        {/if}
      </AnimatePresence>
    </OverlayScrollbarsComponent>

    <AnimatePresence>
      {#if currentSong}
        <motion.div
          key="player-dock"
          class="player-dock"
          initial={axisEnter('y', 18)}
          animate={axisAnimate('y')}
          exit={fadeExit()}
          transition={motionTransition(PLAYER_DOCK_DURATION)}
        >
          <div class="player-dock-stack" data-panel={lyricsOpen ? 'lyrics' : playlistOpen ? 'playlist' : 'none'}>
            <AnimatePresence initial={false}>
              {#if lyricsOpen}
                <motion.section
                  key="player-lyrics"
                  class="player-flyout"
                  data-panel="lyrics"
                  initial={axisEnter('y', 12)}
                  animate={axisAnimate('y')}
                  exit={axisExit('y', 8)}
                  transition={motionTransition(0.18)}
                >
                  <div class="player-flyout-header">
                    <div>
                      <p class="player-flyout-eyebrow">歌词</p>
                      <h3 class="player-flyout-title">{currentSong.name}</h3>
                    </div>
                    <span class="player-flyout-count">{lyricsLines.length > 0 ? `${lyricsLines.length} 行` : '歌词'}</span>
                  </div>

                  {#if lyricsLoading}
                    <div class="player-flyout-empty">正在加载歌词…</div>
                  {:else if lyricsError}
                    <div class="player-flyout-empty">{lyricsError}</div>
                  {:else if lyricsLines.length > 0}
                    <div class="player-lyrics-list">
                      {#each lyricsLines as line, index (line.id)}
                        <p class={`player-lyric-line${index === activeLyricIndex ? ' active' : ''}`}>{line.text}</p>
                      {/each}
                    </div>
                  {:else}
                    <div class="player-flyout-empty">这首歌暂时没有歌词。</div>
                  {/if}
                </motion.section>
              {:else if playlistOpen}
                <motion.section
                  key="player-playlist"
                  class="player-flyout"
                  data-panel="playlist"
                  initial={axisEnter('y', 12)}
                  animate={axisAnimate('y')}
                  exit={axisExit('y', 8)}
                  transition={motionTransition(0.18)}
                >
                  <div class="player-flyout-header">
                    <div>
                      <p class="player-flyout-eyebrow">播放列表</p>
                      <h3 class="player-flyout-title">当前队列</h3>
                    </div>
                    <span class="player-flyout-count">{playbackOrder.length} 首</span>
                  </div>

                  {#if playbackOrder.length > 0}
                    <div class="player-playlist-list">
                      {#each playbackOrder as entry, index (entry.cid)}
                        <button
                          type="button"
                          class={`player-playlist-item${entry.cid === currentSong?.cid ? ' active' : ''}`}
                          onclick={() => { void playQueueEntry(entry, playbackOrder, index); }}
                        >
                          <span class="player-playlist-index">{String(index + 1).padStart(2, '0')}</span>
                          <span class="player-playlist-meta">
                            <span class="player-playlist-name">{entry.name}</span>
                            <span class="player-playlist-artists">{entry.artists.join(' · ')}</span>
                          </span>
                        </button>
                      {/each}
                    </div>
                  {:else}
                    <div class="player-flyout-empty">当前没有可播放的队列。</div>
                  {/if}
                </motion.section>
              {/if}
            </AnimatePresence>

          <AudioPlayer
            song={currentSong}
            {isPlaying}
            {isPaused}
            hasPrevious={playerHasPrevious}
            hasNext={playerHasNext}
            {progress}
            {duration}
            {isLoading}
            isShuffled={shuffleEnabled}
            {repeatMode}
            lyricsActive={lyricsOpen}
            playlistActive={playlistOpen}
            reducedMotion={prefersReducedMotion}
            onPrevious={handlePlayPrevious}
            onTogglePlay={isPlaying ? handlePausePlayback : handleResumePlayback}
            onSeek={handleSeekPlayback}
            onNext={handlePlayNext}
            onShuffleChange={handleShuffleChange}
            onRepeatModeChange={handleRepeatModeChange}
            onToggleLyrics={toggleLyricsPanel}
            onTogglePlaylist={togglePlaylistPanel}
            onDownload={handleCurrentSongDownload}
          />
          </div>
        </motion.div>
      {/if}
    </AnimatePresence>
  </section>

  <!-- Download settings panel (slide-in from right) -->
  <AnimatePresence>
    {#if settingsOpen}
      <motion.div
        key="settings-overlay"
        class="settings-overlay"
        role="button"
        tabindex="-1"
        onclick={() => settingsOpen = false}
        onkeydown={(e) => e.key === 'Escape' && (settingsOpen = false)}
        initial={fadeEnter()}
        animate={{ opacity: 1 }}
        exit={fadeExit()}
        transition={motionTransition(OVERLAY_DURATION)}
      ></motion.div>
      <motion.div
        key="settings-panel"
        class="settings-panel"
        role="dialog"
        aria-modal="true"
        aria-labelledby="settings-title"
        initial={axisEnter('x', 24)}
        animate={axisAnimate('x')}
        exit={axisExit('x', 18)}
        transition={motionTransition(SHEET_DURATION)}
      >
        <div class="settings-header">
          <h2 class="settings-title" id="settings-title">下载设置</h2>
          <motion.button
            class="settings-close"
            onclick={() => settingsOpen = false}
            aria-label="关闭"
            animate={settingsCloseAnimate()}
            whileHover={settingsCloseHover()}
            whileTap={prefersReducedMotion ? undefined : { y: 0, scale: 0.96, opacity: 0.92 }}
            transition={interactiveTransition}
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </motion.button>
        </div>

        <div class="form-group">
          <label class="form-label" for="format-select">输出格式</label>
          <select
            id="format-select"
            class="form-select"
            bind:value={format}
            use:motionStyles={{ animate: fieldMotion(isFormatHovered, isFormatFocused), transition: interactiveTransition, reducedMotion: prefersReducedMotion }}
            onmouseenter={() => { isFormatHovered = true; }}
            onmouseleave={() => { isFormatHovered = false; }}
            onfocus={() => { isFormatFocused = true; }}
            onblur={() => { isFormatFocused = false; }}
          >
            <option value="flac">FLAC（无损压缩）</option>
            <option value="wav">WAV（无损）</option>
            <option value="mp3">MP3</option>
          </select>
        </div>

        <div class="form-group">
          <label class="form-label" for="output-dir">保存位置</label>
          <input
            id="output-dir"
            type="text"
            class="form-input"
            readonly
            value={outputDir}
            use:motionStyles={{ animate: fieldMotion(isOutputDirHovered, isOutputDirFocused), transition: interactiveTransition, reducedMotion: prefersReducedMotion }}
            onmouseenter={() => { isOutputDirHovered = true; }}
            onmouseleave={() => { isOutputDirHovered = false; }}
            onfocus={() => { isOutputDirFocused = true; }}
            onblur={() => { isOutputDirFocused = false; }}
          />
          <motion.button
            class="btn"
            onclick={handleSelectDirectory}
            style="width: 100%; margin-top: 8px;"
            animate={appButtonAnimate(false, false)}
            whileHover={appButtonHover(false, false)}
            whileTap={prefersReducedMotion ? undefined : { y: 0, scale: 0.98, opacity: 0.94 }}
            transition={interactiveTransition}
          >
            📁 选择文件夹
          </motion.button>
        </div>

        <div class="form-group">
          <label class="settings-switch" for="download-lyrics">
            <span class="settings-switch-copy">
              <span class="form-label settings-switch-label">歌词文件</span>
              <span class="form-help">有歌词时，在音频文件旁生成同名 `.lrc`。</span>
            </span>
            <span class="settings-switch-control">
              <input
                id="download-lyrics"
                class="settings-switch-input"
                type="checkbox"
                bind:checked={downloadLyrics}
              />
              <span class="settings-switch-track" aria-hidden="true">
                <span class="settings-switch-thumb"></span>
              </span>
            </span>
          </label>
        </div>

        <div class="form-group">
          <div class="form-label">音频缓存</div>
          <p class="form-help">播放时会边下边播，并把完整音频缓存到系统缓存目录。</p>
          <motion.button
            class="btn"
            onclick={handleClearAudioCache}
            disabled={isClearingAudioCache}
            style="width: 100%; justify-content: center; margin-top: 8px;"
            animate={appButtonAnimate(false, isClearingAudioCache)}
            whileHover={appButtonHover(false, isClearingAudioCache)}
            whileTap={!prefersReducedMotion && !isClearingAudioCache ? { y: 0, scale: 0.98, opacity: 0.94 } : undefined}
            transition={interactiveTransition}
          >
            {isClearingAudioCache ? '正在清除缓存...' : '清除音频缓存'}
          </motion.button>
        </div>
      </motion.div>
    {/if}
  </AnimatePresence>
</div>
