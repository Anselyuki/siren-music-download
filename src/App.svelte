<script lang="ts">
  import { tick } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { AnimatePresence, motion } from "@humanspeak/svelte-motion";
  import { OverlayScrollbarsComponent } from "overlayscrollbars-svelte";
  import type { OverlayScrollbarsComponentRef } from "overlayscrollbars-svelte";
  import type {
    EventListeners,
    OverlayScrollbars,
    PartialOptions,
  } from "overlayscrollbars";
  import {
    getAlbums,
    getAlbumDetail,
    getDefaultOutputDir,
    playSong,
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback,
    getPlayerState,
    clearResponseCache,
    extractImageTheme,
    getImageDataUrl,
    getSongLyrics,
    createDownloadJob,
    listDownloadJobs,
    cancelDownloadJob,
    cancelDownloadTask,
    retryDownloadJob,
    retryDownloadTask,
    clearDownloadHistory,
    getPreferences,
    setPreferences,
    getLocalInventorySnapshot,
    searchLibrary,
  } from "$lib/api";
  import {
    clearCache,
    createInventoryCacheTag,
    invalidateByTag,
    warmCacheManager,
  } from "$lib/cache";
  import type {
    Album,
    AlbumDetail,
    OutputFormat,
    SongEntry,
    PlayerState,
    PlaybackQueueEntry,
    DownloadJobSnapshot,
    DownloadManagerSnapshot,
    DownloadTaskProgressEvent,
    CreateDownloadJobRequest,
    DownloadTaskSnapshot,
    AppPreferences,
    LocalInventorySnapshot,
    AppErrorEvent,
    LogLevel,
    DownloadHistoryScopeFilter,
    DownloadHistoryStatusFilter,
    DownloadHistoryKindFilter,
    LibrarySearchScope,
    SearchLibraryResponse,
    SearchLibraryResultItem,
  } from "$lib/types";
  import { applyThemePalette, DEFAULT_THEME_PALETTE } from "$lib/theme";
  import { getDownloadBadgeLabel, shouldShowDownloadBadge } from "$lib/downloadBadge";
  import { motionStyles } from "$lib/actions/motionStyles";
  import { envStore } from "$lib/features/env/store.svelte";
  import { shellStore } from "$lib/features/shell/store.svelte";
  import { createLibraryController } from "$lib/features/library/controller.svelte";
  import { createPlayerController } from "$lib/features/player/controller.svelte";
  import { createDownloadController } from "$lib/features/download/controller.svelte";
  import { clamp, preloadImage } from "$lib/features/library/helpers";
  import { buildAlbumPlaybackEntries, getSelectedAlbumCoverUrl } from "$lib/features/library/selectors";
  import { toast } from "svelte-sonner";
  import AlbumCard from "$lib/components/AlbumCard.svelte";
  import SongRow from "$lib/components/SongRow.svelte";
  import AudioPlayer from "$lib/components/AudioPlayer.svelte";
  import MotionSpinner from "$lib/components/MotionSpinner.svelte";
  import MotionPulseBlock from "$lib/components/MotionPulseBlock.svelte";
  import TopToolbar from "$lib/components/app/TopToolbar.svelte";
  import StatusToastHost from "$lib/components/app/StatusToastHost.svelte";
  import AlbumSidebar from "$lib/components/app/AlbumSidebar.svelte";
  import AlbumWorkspace from "$lib/components/app/AlbumWorkspace.svelte";
  import PlayerDock from "$lib/components/app/PlayerDock.svelte";

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

  const delay = (ms: number) =>
    new Promise((resolve) => setTimeout(resolve, ms));

  type SongDownloadState = "idle" | "creating" | "queued" | "running";
  type SettingsSheetComponent = typeof import("$lib/components/app/SettingsSheet.svelte").default;
  type DownloadTasksSheetComponent = typeof import("$lib/components/app/DownloadTasksSheet.svelte").default;

  const libraryController = createLibraryController({
    delay,
    detailSkeletonDelayMs: DETAIL_SKELETON_DELAY_MS,
    minDetailDisplayMs: MIN_DISPLAY_MS,
    getAlbums,
    getAlbumDetail,
    searchLibrary,
    preloadAlbumArtwork,
    setAlbumStageAspectRatio,
    notifyError,
  });

  const playerController = createPlayerController({
    playSong: async (songCid, coverUrl, context) => {
      await playSong(songCid, coverUrl ?? undefined, context ?? undefined);
    },
    pausePlayback,
    resumePlayback,
    seekCurrentPlayback,
    getSongLyrics,
    notifyError,
  });

  const downloadController = createDownloadController({
    createDownloadJob,
    cancelDownloadJob,
    cancelDownloadTask,
    retryDownloadJob,
    retryDownloadTask,
    clearDownloadHistory,
    openDownloadPanel: async (resetFilters = false) => {
      if (resetFilters) {
        downloadController.resetFilters();
      }

      const loaded = await ensureDownloadTasksSheetLoaded();
      if (!loaded) {
        return;
      }

      downloadPanelOpen = true;
      settingsOpen = false;
    },
    getDownloadOptions: () => ({
      outputDir,
      format,
      downloadLyrics,
    }),
    notifyInfo,
    notifyError,
  });

  let outputDir = $state("");
  let format = $state<OutputFormat>("flac");
  let selectedSongCids = $state<string[]>([]);
  let selectionModeEnabled = $state(false);
  // Download job system state
  let downloadPanelOpen = $state(false);
  let SettingsSheetView = $state<SettingsSheetComponent | null>(null);
  let DownloadTasksSheetView = $state<DownloadTasksSheetComponent | null>(null);
  let settingsSheetLoader = $state<Promise<SettingsSheetComponent> | null>(null);
  let downloadTasksSheetLoader = $state<Promise<DownloadTasksSheetComponent> | null>(null);
  let themeRequestSeq = 0;
  let artworkRequestSeq = 0;
  let playerStateInitSeq = 0;
  let playerStateHydratedFromEvent = false;
  const prefersReducedMotion = $derived(envStore.prefersReducedMotion);
  const albums = $derived(libraryController.albums);
  const selectedAlbum = $derived(libraryController.selectedAlbum);
  const selectedAlbumCid = $derived(libraryController.selectedAlbumCid);
  const loadingAlbums = $derived(libraryController.loadingAlbums);
  const loadingDetail = $derived(libraryController.loadingDetail);
  const errorMsg = $derived(libraryController.errorMsg);
  const librarySearchQuery = $derived(libraryController.librarySearchQuery);
  const librarySearchScope = $derived(libraryController.librarySearchScope);
  const librarySearchLoading = $derived(libraryController.librarySearchLoading);
  const librarySearchResponse = $derived(libraryController.librarySearchResponse);
  const pendingScrollToSongCid = $derived(libraryController.pendingScrollToSongCid);
  const showDetailSkeleton = $derived(libraryController.showDetailSkeleton);
  const albumRequestSeq = $derived(libraryController.albumRequestSeq);
  const currentSong = $derived(playerController.currentSong);
  const isPlaying = $derived(playerController.isPlaying);
  const isPaused = $derived(playerController.isPaused);
  const isLoading = $derived(playerController.isLoading);
  const progress = $derived(playerController.progress);
  const duration = $derived(playerController.duration);
  const shuffleEnabled = $derived(playerController.shuffleEnabled);
  const repeatMode = $derived(playerController.repeatMode);
  const playbackOrder = $derived(playerController.playbackOrder);
  const lyricsOpen = $derived(playerController.lyricsOpen);
  const playlistOpen = $derived(playerController.playlistOpen);
  const lyricsLoading = $derived(playerController.lyricsLoading);
  const lyricsError = $derived(playerController.lyricsError);
  const lyricsLines = $derived(playerController.lyricsLines);
  const downloadingSongCid = $derived(downloadController.downloadingSongCid);
  const downloadingAlbumCid = $derived(downloadController.downloadingAlbumCid);
  const activeDownloadCount = $derived(downloadController.activeDownloadCount);
  const filteredDownloadJobs = $derived(downloadController.filteredJobs);
  const hasDownloadHistory = $derived(downloadController.hasDownloadHistory);
  let contentEl = $state<HTMLElement | null>(null);
  let contentScrollbar = $state<OverlayScrollbarsComponentRef<"main"> | null>(
    null,
  );
  let albumStageEl = $state<HTMLElement | null>(null);
  let selectedAlbumArtworkUrl = $state<string | null>(null);
  const isMacOS = $derived(envStore.isMacOS);
  let albumStageAspectRatio = $state(DEFAULT_ALBUM_STAGE_ASPECT_RATIO);
  let albumStageWidth = $state(0);
  const viewportHeight = $derived(envStore.viewportHeight);
  let albumStageCollapseOffset = $state(0);
  let albumStageScrollTop = $state(0);
  let albumStageMotionFrame = 0;
  let pendingAlbumStageCollapseOffset = 0;
  let pendingAlbumStageScrollTop = 0;

  const playerHasPrevious = $derived(playerController.playerHasPrevious);
  const playerHasNext = $derived(playerController.playerHasNext);

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

  const selectedSongCount = $derived.by(() => selectedSongCids.length);
  const selectedSongsLabel = $derived.by(() => {
    if (selectedSongCount === 0) return "未选择歌曲";
    if (selectedSongCount === 1) return "已选择 1 首";
    return `已选择 ${selectedSongCount} 首`;
  });

  function setContentViewport(instance: OverlayScrollbars) {
    const viewport = instance.elements().viewport;
    if (contentEl !== viewport) {
      contentEl = viewport;
    }
  }

  const overlayScrollbarOptions = $derived.by(
    (): PartialOptions => ({
      scrollbars: {
        theme: "os-theme-app",
        autoHide: prefersReducedMotion ? "leave" : "move",
        autoHideDelay: prefersReducedMotion ? 160 : 720,
        autoHideSuspend: true,
        dragScroll: true,
        clickScroll: false,
      },
    }),
  );

  const contentScrollbarEvents = $derived.by(
    (): EventListeners => ({
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
    }),
  );

  function resetContentScroll() {
    resetAlbumStageMotion();
    contentEl?.scrollTo({
      top: 0,
      behavior: prefersReducedMotion ? "auto" : "smooth",
    });
  }

  async function preloadAlbumArtwork(
    album: AlbumDetail,
  ): Promise<number | null> {
    const sourceUrl = album.coverDeUrl ?? album.coverUrl ?? null;
    if (!sourceUrl) return null;

    let resolvedUrl = sourceUrl;
    try {
      resolvedUrl = await getImageDataUrl(sourceUrl);
    } catch {
      resolvedUrl = sourceUrl;
    }

    const meta = await preloadImage(resolvedUrl);
    return meta?.aspectRatio ?? null;
  }

  function setAlbumStageAspectRatio(value: number | null | undefined) {
    if (value && Number.isFinite(value) && value > 0) {
      albumStageAspectRatio = value;
      return;
    }

    albumStageAspectRatio = DEFAULT_ALBUM_STAGE_ASPECT_RATIO;
  }


  function isSongSelected(songCid: string): boolean {
    return selectedSongCids.includes(songCid);
  }

  function toggleSongSelection(songCid: string) {
    if (selectedSongCids.includes(songCid)) {
      selectedSongCids = selectedSongCids.filter((cid) => cid !== songCid);
      return;
    }

    selectedSongCids = [...selectedSongCids, songCid];
  }

  function clearSongSelection() {
    selectedSongCids = [];
  }

  function selectAllSongs() {
    if (!selectedAlbum) return;
    selectedSongCids = selectedAlbum.songs.map((s: SongEntry) => s.cid);
  }

  function deselectAllSongs() {
    selectedSongCids = [];
  }

  function invertSongSelection() {
    if (!selectedAlbum) return;
    const allCids = new Set(
      selectedAlbum.songs.map((s: SongEntry) => s.cid),
    );
    const currentSelected = new Set(selectedSongCids);
    selectedSongCids = [...allCids].filter(
      (cid) => !currentSelected.has(cid),
    );
  }

  function toggleSelectionMode() {
    selectionModeEnabled = !selectionModeEnabled;
    if (!selectionModeEnabled) {
      clearSongSelection();
    }
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
    pendingAlbumStageCollapseOffset =
      next.collapseOffset ?? pendingAlbumStageCollapseOffset;
    pendingAlbumStageScrollTop = next.scrollTop ?? pendingAlbumStageScrollTop;

    if (immediate || prefersReducedMotion || typeof window === "undefined") {
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

    return Math.min(
      albumStageFullHeight,
      viewportHeight * ALBUM_STAGE_BASE_VIEWPORT_RATIO,
    );
  });

  const albumStageCollapseProgress = $derived.by(() =>
    clamp(albumStageCollapseOffset / ALBUM_STAGE_COLLAPSE_SCROLL_RANGE, 0, 1),
  );

  const albumStageRevealProgress = $derived.by(
    () => 1 - albumStageCollapseProgress,
  );

  const albumStageSolidifyProgress = $derived.by(() =>
    Math.max(
      albumStageCollapseProgress,
      clamp(albumStageScrollTop / ALBUM_STAGE_SOLIDIFY_SCROLL_RANGE, 0, 1),
    ),
  );

  const albumStageHeight = $derived.by(() => {
    if (!albumStageBaseHeight) {
      return 0;
    }

    return (
      albumStageBaseHeight +
      (albumStageFullHeight - albumStageBaseHeight) * albumStageRevealProgress
    );
  });

  const albumStageStyle = $derived.by(
    () => `--album-stage-aspect-ratio: ${albumStageAspectRatio}`,
  );

  type MotionTarget = Record<string, string | number>;

  function motionTransition(duration: number, delay = 0): any {
    const transition: any = {
      duration: prefersReducedMotion ? 0 : duration,
      delay: prefersReducedMotion ? 0 : delay,
      ease: "easeOut" as const,
    };

    return transition;
  }

  function fadeEnter(opacity = 0): MotionTarget {
    return prefersReducedMotion ? { opacity: 1 } : { opacity };
  }

  function fadeExit(opacity = 0): MotionTarget {
    return { opacity };
  }

  function axisEnter(axis: "x" | "y", offset: number): MotionTarget {
    return prefersReducedMotion
      ? { opacity: 1 }
      : { opacity: 0, [axis]: offset };
  }

  function axisAnimate(axis: "x" | "y"): MotionTarget {
    return { opacity: 1, [axis]: 0 };
  }

  function axisExit(axis: "x" | "y", offset: number): MotionTarget {
    return prefersReducedMotion
      ? { opacity: 0 }
      : { opacity: 0, [axis]: offset };
  }

  const interactiveTransition = $derived.by(
    () =>
      ({
        duration: prefersReducedMotion ? 0 : 0.16,
        ease: "easeOut",
      }) as const,
  );

  const albumStageMotionHeight = $derived.by(() =>
    albumStageHeight > 0
      ? albumStageHeight
      : Math.max(albumStageBaseHeight || 0, 280),
  );

  const albumStageMediaHeight = $derived.by(
    () => `${albumStageMotionHeight}px`,
  );
  const albumStageScrimOpacity = $derived.by(() =>
    Math.max(0.58, 1 - albumStageSolidifyProgress * 0.34),
  );
  const albumStageImageOpacity = $derived.by(
    () => 1 - albumStageSolidifyProgress * 0.54,
  );
  const albumStageImageTransform = $derived.by(() =>
    prefersReducedMotion
      ? "translateZ(0) scale(1)"
      : `translateZ(0) scale(${1 + albumStageRevealProgress * 0.006 + albumStageSolidifyProgress * 0.012})`,
  );
  const albumStageSolidifyOpacity = $derived.by(
    () => albumStageSolidifyProgress,
  );

  function modeButtonAnimate(active: boolean): MotionTarget {
    return active
      ? {
          backgroundColor: "var(--accent)",
          color: "#ffffff",
          boxShadow: "0 10px 22px rgba(var(--accent-rgb), 0.22)",
        }
      : {
          backgroundColor: "rgba(15, 23, 42, 0)",
          color: "rgba(31, 41, 55, 0.72)",
          boxShadow: "0 0 0 rgba(var(--accent-rgb), 0)",
        };
  }

  function modeButtonHover(active: boolean): MotionTarget | undefined {
    if (active) {
      return prefersReducedMotion ? undefined : { y: -1 };
    }

    return {
      backgroundColor: "rgba(15, 23, 42, 0.06)",
      color: "var(--text-primary)",
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function toolbarButtonAnimate(
    active = false,
    accented = false,
    disabled = false,
  ): MotionTarget {
    return {
      opacity: disabled ? 0.42 : 1,
      backgroundColor: active
        ? "var(--accent-light)"
        : "rgba(255, 255, 255, 0.78)",
      color: active || accented ? "var(--accent)" : "var(--text-primary)",
      boxShadow: "inset 0 1px 0 rgba(255, 255, 255, 0.94)",
    };
  }

  function toolbarButtonHover(disabled = false): MotionTarget | undefined {
    if (disabled) {
      return undefined;
    }

    return {
      backgroundColor: "rgba(var(--accent-rgb), 0.1)",
      boxShadow: "0 10px 22px rgba(var(--accent-rgb), 0.14)",
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function appButtonAnimate(primary = false, disabled = false): MotionTarget {
    return primary
      ? {
          backgroundColor: disabled ? "var(--bg-tertiary)" : "var(--accent)",
          color: disabled ? "var(--text-tertiary)" : "#ffffff",
          boxShadow: disabled
            ? "0 0 0 rgba(var(--accent-rgb), 0)"
            : "0 10px 24px rgba(var(--accent-rgb), 0.16)",
          opacity: disabled ? 0.72 : 1,
        }
      : {
          backgroundColor: "var(--bg-tertiary)",
          color: "var(--text-primary)",
          boxShadow: "0 0 0 rgba(var(--accent-rgb), 0)",
          opacity: disabled ? 0.42 : 1,
        };
  }

  function appButtonHover(
    primary = false,
    disabled = false,
  ): MotionTarget | undefined {
    if (disabled) return undefined;

    return primary
      ? {
          backgroundColor: "var(--accent-hover)",
          boxShadow: "0 10px 24px rgba(var(--accent-rgb), 0.2)",
          ...(prefersReducedMotion ? {} : { y: -1 }),
        }
      : {
          backgroundColor: "var(--hover-bg-elevated)",
          boxShadow: "0 8px 20px rgba(15, 23, 42, 0.08)",
          ...(prefersReducedMotion ? {} : { y: -1 }),
        };
  }

  function settingsCloseAnimate(): MotionTarget {
    return {
      backgroundColor: "var(--bg-tertiary)",
      color: "var(--text-secondary)",
    };
  }

  function settingsCloseHover(): MotionTarget {
    return {
      backgroundColor: "var(--hover-bg-elevated)",
      color: "var(--text-primary)",
      ...(prefersReducedMotion ? {} : { y: -1 }),
    };
  }

  function fieldAnimate(): MotionTarget {
    return {
      backgroundColor: "var(--bg-tertiary)",
      borderColor: "var(--border)",
      color: "var(--text-primary)",
      boxShadow: "0 0 0 0 rgba(var(--accent-rgb), 0)",
    };
  }

  function fieldHover(): MotionTarget {
    return {
      borderColor: "var(--text-tertiary)",
    };
  }

  function fieldFocus(): MotionTarget {
    return {
      borderColor: "var(--accent)",
      backgroundColor: "var(--accent-light)",
      boxShadow: "0 0 0 3px rgba(var(--accent-rgb), 0.12)",
    };
  }

  function fieldMotion(hovered: boolean, focused: boolean): MotionTarget {
    if (focused) return fieldFocus();
    if (hovered) return fieldHover();
    return fieldAnimate();
  }

  $effect(() => {
    const paletteRequestSeq = ++themeRequestSeq;
    const artworkUrl =
      selectedAlbum?.coverUrl ?? selectedAlbum?.coverDeUrl ?? null;

    if (!artworkUrl) {
      applyThemePalette(DEFAULT_THEME_PALETTE);
      return;
    }

    void (async () => {
      try {
        const palette = await extractImageTheme(artworkUrl);
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(palette);
      } catch {
        if (paletteRequestSeq !== themeRequestSeq) return;
        applyThemePalette(DEFAULT_THEME_PALETTE);
      }
    })();
  });

  $effect(() => {
    const sourceUrl = selectedAlbum?.coverDeUrl ?? selectedAlbum?.coverUrl ?? null;
    const requestSeq = ++artworkRequestSeq;

    if (!sourceUrl) {
      selectedAlbumArtworkUrl = null;
      return;
    }

    void (async () => {
      try {
        const dataUrl = await getImageDataUrl(sourceUrl);
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = dataUrl;
      } catch {
        if (requestSeq !== artworkRequestSeq) return;
        selectedAlbumArtworkUrl = null;
      }
    })();
  });

  // Auto-save preferences via unified preferences API (after initialization)
  $effect(() => {
    const _fmt = format;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _lyrics = downloadLyrics;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _notif = notifyOnDownloadComplete;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _playback = notifyOnPlaybackChange;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    const _logLevel = logLevel;
    if (!prefsReady) return;
    void savePreferences();
  });

  $effect(() => {
    if (!albumStageEl) return;

    syncAlbumStageWidth();

    if (typeof ResizeObserver === "undefined") return;

    const observer = new ResizeObserver(() => {
      syncAlbumStageWidth();
    });

    observer.observe(albumStageEl);

    return () => observer.disconnect();
  });

  async function bootstrapApp(shouldDispose: () => boolean) {
    try {
      await warmCacheManager();
    } catch {
      // Keep startup usable if IndexedDB warm start is unavailable.
    }

    if (shouldDispose()) {
      return;
    }

    try {
      const prefs = await getPreferences();
      if (shouldDispose()) {
        return;
      }
      outputDir = prefs.outputDir || outputDir;
      format = prefs.outputFormat || format;
      downloadLyrics = prefs.downloadLyrics;
      notifyOnDownloadComplete = prefs.notifyOnDownloadComplete;
      notifyOnPlaybackChange = prefs.notifyOnPlaybackChange;
      logLevel = prefs.logLevel;
      prefsReady = true;
    } catch {
      if (!shouldDispose()) {
        prefsReady = true;
      }
    }

    const defaultDirPromise = outputDir
      ? Promise.resolve("")
      : getDefaultOutputDir().catch(() => "");

    try {
      const albumList = await libraryController.loadAlbums({ shouldDispose });

      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir && !outputDir) {
        outputDir = defaultDir;
      }

      try {
        const snapshot = await getLocalInventorySnapshot();
        if (shouldDispose()) {
          return;
        }
        libraryController.initializeInventory(snapshot);
      } catch {
        if (!shouldDispose()) {
          libraryController.initializeInventory(null);
        }
      }

      if (albumList.length > 0 && !libraryController.selectedAlbumCid) {
        clearSongSelection();
        selectionModeEnabled = false;
        await libraryController.selectAlbum(albumList[0], {
          shouldDispose,
          afterSelect: async () => {
            await tick();
            resetContentScroll();
          },
        });
        if (shouldDispose()) {
          return;
        }
      }
    } catch {
      const defaultDir = await defaultDirPromise;
      if (shouldDispose()) {
        return;
      }
      if (defaultDir && !outputDir) {
        outputDir = defaultDir;
      }
    }

    try {
      const requestSeq = downloadController.beginHydrationAttempt();
      const manager = await listDownloadJobs();
      if (shouldDispose()) {
        return;
      }
      downloadController.applyManagerSnapshot(manager, requestSeq);
    } catch {
      // Download manager not available
    }

    try {
      const requestSeq = ++playerStateInitSeq;
      const playerState = await getPlayerState();
      if (shouldDispose()) {
        return;
      }
      if (requestSeq === playerStateInitSeq && !playerStateHydratedFromEvent) {
        playerController.syncPlayerState(playerState);
      }
    } catch {
      // Player not playing on startup
    }
  }

  async function subscribeToTauriEvents(shouldDispose: () => boolean) {
    const unlisteners: Array<() => void> = [];

    const cleanup = () => {
      while (unlisteners.length > 0) {
        unlisteners.pop()?.();
      }
    };

    async function register<T>(
      eventName: string,
      handler: (event: { payload: T }) => void | Promise<void>,
    ) {
      const unlisten = await listen<T>(eventName, async (event) => {
        if (shouldDispose()) {
          return;
        }
        await handler(event);
      });

      if (shouldDispose()) {
        unlisten();
        return false;
      }

      unlisteners.push(unlisten);
      return true;
    }

    try {
      if (!(await register<PlayerState>("player-state-changed", (event) => {
        playerStateHydratedFromEvent = true;
        playerController.syncPlayerState(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<PlayerState>("player-progress", (event) => {
        playerController.syncPlayerProgress(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<DownloadManagerSnapshot>(
        "download-manager-state-changed",
        (event) => {
          downloadController.applyManagerEvent(event.payload);
        },
      ))) {
        return cleanup;
      }

      if (!(await register<DownloadJobSnapshot>("download-job-updated", (event) => {
        downloadController.applyJobUpdate(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<DownloadTaskProgressEvent>("download-task-progress", (event) => {
        downloadController.applyTaskProgress(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<AppErrorEvent>("app-error-recorded", (event) => {
        handleAppErrorEvent(event.payload);
      }))) {
        return cleanup;
      }

      if (!(await register<LocalInventorySnapshot>(
        "local-inventory-state-changed",
        async (event) => {
          await libraryController.handleInventoryStateChanged(event.payload, {
            shouldDispose,
            invalidateInventoryCaches,
            onSelectionInvalidated: () => {
              clearSongSelection();
              selectionModeEnabled = false;
            },
          });
        },
      ))) {
        return cleanup;
      }

      return cleanup;
    } catch (error) {
      cleanup();
      const message = error instanceof Error ? error.message : String(error);
      throw new Error(`failed to subscribe tauri events: ${message}`);
    }
  }

  function teardownAppRuntime(unsubscribe: (() => void) | null) {
    shellStore.dispose();
    envStore.dispose();
    libraryController.dispose();
    playerController.dispose();
    downloadController.dispose();
    playerStateInitSeq += 1;
    playerStateHydratedFromEvent = false;
    if (albumStageMotionFrame) {
      cancelAnimationFrame(albumStageMotionFrame);
    }
    unsubscribe?.();
  }

  $effect(() => {
    libraryController.init();
    playerController.init();
    downloadController.init();
    envStore.init();
    shellStore.init();

    let disposed = false;
    let unsubscribe: (() => void) | null = null;

    void (async () => {
      try {
        const nextUnsubscribe = await subscribeToTauriEvents(() => disposed);
        if (disposed) {
          nextUnsubscribe();
          return;
        }
        unsubscribe = nextUnsubscribe;

        await bootstrapApp(() => disposed);
      } catch (error) {
        if (disposed) {
          return;
        }
        notifyError(
          `初始化应用失败：${error instanceof Error ? error.message : String(error)}`,
        );
      }
    })();

    return () => {
      disposed = true;
      teardownAppRuntime(unsubscribe);
    };
  });

  $effect(() => {
    playerController.syncPlaybackLifecycle();
  });

  $effect(() => {
    if (!pendingScrollToSongCid || !selectedAlbum || loadingDetail) {
      return;
    }

    const expectedSongCid = pendingScrollToSongCid;
    void tick().then(() => {
      if (pendingScrollToSongCid !== expectedSongCid || !contentEl) {
        return;
      }

      const row = contentEl.querySelector<HTMLElement>(
        `[data-song-cid="${CSS.escape(expectedSongCid)}"]`,
      );
      if (!row) {
        return;
      }

      row.scrollIntoView({
        behavior: prefersReducedMotion ? "auto" : "smooth",
        block: "center",
      });
      libraryController.clearPendingScrollToSong(expectedSongCid);
    });
  });

  async function handleSelectSearchResult(item: SearchLibraryResultItem) {
    const album = albums.find((candidate) => candidate.cid === item.albumCid);
    if (!album) {
      notifyError("未找到对应专辑，可能需要先刷新列表。");
      return;
    }

    libraryController.setPendingScrollToSong(item.kind === "song" ? item.songCid : null);
    clearSongSelection();
    selectionModeEnabled = false;
    await libraryController.selectAlbum(album, {
      afterSelect: async () => {
        await tick();
        resetContentScroll();
      },
    });
  }

  function handleContentScroll() {
    if (loadingDetail) {
      scheduleAlbumStageMotion({ scrollTop: 0 }, true);
      return;
    }

    const nextScrollTop = Math.max(0, contentEl?.scrollTop ?? 0);
    const nextCollapseOffset =
      nextScrollTop > 0 &&
      pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
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

    if (
      event.deltaY > 0 &&
      pendingAlbumStageCollapseOffset < ALBUM_STAGE_COLLAPSE_SCROLL_RANGE
    ) {
      event.preventDefault();
      scheduleAlbumStageMotion({
        collapseOffset: clamp(
          pendingAlbumStageCollapseOffset + event.deltaY,
          0,
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE,
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
          ALBUM_STAGE_COLLAPSE_SCROLL_RANGE,
        ),
        scrollTop: 0,
      });
    }
  }

  let settingsOpen = $state(false);
  let downloadLyrics = $state(true);
  let notifyOnDownloadComplete = $state(true);
  let notifyOnPlaybackChange = $state(true);
  let logLevel = $state<LogLevel>("error");
  let isFormatHovered = $state(false);
  let isFormatFocused = $state(false);
  let isOutputDirHovered = $state(false);
  let isOutputDirFocused = $state(false);
  let settingsLogRefreshToken = $state(0);
  let prefsReady = $state(false);

  function handleAppErrorEvent(event: AppErrorEvent) {
    notifyError(event.message);
    if (settingsOpen) {
      settingsLogRefreshToken += 1;
    }
  }

  async function invalidateInventoryCaches(
    inventoryVersion: string | null | undefined,
  ) {
    await invalidateByTag(createInventoryCacheTag(inventoryVersion));
  }

  async function savePreferences(): Promise<boolean> {
    const prefs: AppPreferences = {
      outputFormat: format,
      outputDir,
      downloadLyrics,
      notifyOnDownloadComplete,
      notifyOnPlaybackChange,
      logLevel,
    };
    try {
      const updated = await setPreferences(prefs);
      // Sync from returned values (backend may have normalized them)
      format = updated.outputFormat;
      outputDir = updated.outputDir;
      downloadLyrics = updated.downloadLyrics;
      notifyOnDownloadComplete = updated.notifyOnDownloadComplete;
      notifyOnPlaybackChange = updated.notifyOnPlaybackChange;
      logLevel = updated.logLevel;
      return true;
    } catch (e) {
      notifyError(
        `保存设置失败：${e instanceof Error ? e.message : String(e)}`,
      );
      return false;
    }
  }

  function notifyInfo(message: string) {
    toast(message);
  }

  function notifyError(message: string) {
    toast.error(message);
  }

  async function ensureSettingsSheetLoaded(): Promise<boolean> {
    if (SettingsSheetView) {
      return true;
    }

    if (!settingsSheetLoader) {
      settingsSheetLoader = import("$lib/components/app/SettingsSheet.svelte")
        .then((module) => {
          SettingsSheetView = module.default;
          return module.default;
        })
        .finally(() => {
          settingsSheetLoader = null;
        });
    }

    try {
      await settingsSheetLoader;
      return true;
    } catch (error) {
      notifyError(
        `打开设置面板失败：${error instanceof Error ? error.message : String(error)}`,
      );
      return false;
    }
  }

  async function ensureDownloadTasksSheetLoaded(): Promise<boolean> {
    if (DownloadTasksSheetView) {
      return true;
    }

    if (!downloadTasksSheetLoader) {
      downloadTasksSheetLoader = import("$lib/components/app/DownloadTasksSheet.svelte")
        .then((module) => {
          DownloadTasksSheetView = module.default;
          return module.default;
        })
        .finally(() => {
          downloadTasksSheetLoader = null;
        });
    }

    try {
      await downloadTasksSheetLoader;
      return true;
    } catch (error) {
      notifyError(
        `打开下载任务面板失败：${error instanceof Error ? error.message : String(error)}`,
      );
      return false;
    }
  }

  async function openSettingsPanel() {
    if (settingsOpen) {
      settingsOpen = false;
      return;
    }

    const loaded = await ensureSettingsSheetLoaded();
    if (!loaded) {
      return;
    }

    settingsOpen = true;
    downloadPanelOpen = false;
  }


  async function handlePlay(song: SongEntry) {
    const sourceEntries = buildAlbumPlaybackEntries(selectedAlbum);
    const fallbackEntry: PlaybackQueueEntry = {
      cid: song.cid,
      name: song.name,
      artists: song.artists,
      coverUrl: getSelectedAlbumCoverUrl(selectedAlbum),
    };
    const entries = sourceEntries.length ? sourceEntries : [fallbackEntry];

    playerController.applyPlaybackQueue(entries, song.cid);

    const nextOrder = shuffleEnabled ? [...playbackOrder] : [...entries];
    const nextIndex = nextOrder.findIndex((entry) => entry.cid === song.cid);
    if (nextIndex < 0) return;

    await playerController.playQueueEntry(nextOrder[nextIndex], nextOrder, nextIndex);
  }

  // Refresh cache and reload current album
  let isRefreshing = $state(false);

  async function handleRefresh() {
    if (isRefreshing) return;
    isRefreshing = true;

    clearSongSelection();
    selectionModeEnabled = false;

    try {
      await clearCache();
      await clearResponseCache();
      await libraryController.reloadAlbumsAndRefreshCurrentSelection({
        afterSelect: async () => {
          await tick();
          resetContentScroll();
        },
      });
    } catch (e) {
      notifyError(`刷新专辑列表失败：${e instanceof Error ? e.message : String(e)}`);
    } finally {
      await delay(400);
      isRefreshing = false;
    }
  }
</script>

{#if isMacOS}
  <div
    class="macos-window-drag-region"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>
{/if}

<StatusToastHost />

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
      <div
        class="sidebar-drag-region"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
    {/if}
    <AlbumSidebar
      {albums}
      {selectedAlbumCid}
      reducedMotion={prefersReducedMotion}
      {loadingAlbums}
      {errorMsg}
      searchQuery={librarySearchQuery}
      searchScope={librarySearchScope}
      searchLoading={librarySearchLoading}
      searchResponse={librarySearchResponse}
      onSearchQueryChange={libraryController.setSearchQuery}
      onSearchScopeChange={libraryController.setSearchScope}
      onSelect={(album) => {
        clearSongSelection();
        selectionModeEnabled = false;
        return libraryController.selectAlbum(album, {
          afterSelect: async () => {
            await tick();
            resetContentScroll();
          },
        });
      }}
      onSelectSearchResult={handleSelectSearchResult}
    />
  </OverlayScrollbarsComponent>

  <section class="main-region">
    {#if isMacOS}
      <div
        class="main-drag-region"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
    {/if}

    <TopToolbar
      {activeDownloadCount}
      {isRefreshing}
      {settingsOpen}
      {downloadPanelOpen}
      onRefresh={handleRefresh}
      onOpenDownloads={async () => {
        const nextOpen = !downloadPanelOpen;
        if (nextOpen) {
          await downloadController.openPanel();
          return;
        }
        downloadPanelOpen = false;
      }}
      onOpenSettings={async () => {
        await openSettingsPanel();
      }}
    />


    <!-- 歌曲列表内容区 -->
    <AlbumWorkspace
      {currentSong}
      {loadingDetail}
      {selectedAlbum}
    >
      {#snippet children()}
    <OverlayScrollbarsComponent
      element="div"
      class="h-full"
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
            <div
              class="album-stage"
              bind:this={albumStageEl}
              style={albumStageStyle}
            >
              <div class="album-stage-frame">
                <div
                  class="album-stage-media album-stage-media-loading"
                  style:height={albumStageMediaHeight}
                >
                  <div class="album-stage-media-content">
                    <MotionPulseBlock
                      className="album-stage-skeleton loading-cover"
                      reducedMotion={prefersReducedMotion}
                    />
                  </div>
                  <div
                    class="album-stage-media-scrim"
                    aria-hidden="true"
                    style:opacity={albumStageScrimOpacity}
                  ></div>
                  <div
                    class="album-stage-media-border"
                    aria-hidden="true"
                  ></div>
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
                  <MotionPulseBlock
                    className="album-hero-title loading-text"
                    reducedMotion={prefersReducedMotion}
                  />
                  <MotionPulseBlock
                    className="album-hero-sub loading-text-sub"
                    reducedMotion={prefersReducedMotion}
                    delay={0.14}
                  />
                </motion.div>
              </div>
              <motion.div
                class="loading album-loading"
                initial={fadeEnter()}
                animate={{ opacity: 1 }}
                exit={fadeExit()}
                transition={motionTransition(LIST_DURATION, LIST_DELAY)}
              >
                <span>正在加载歌曲...</span><MotionSpinner
                  className="inline-loading-spinner"
                  reducedMotion={prefersReducedMotion}
                />
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
            <div
              class="album-stage"
              bind:this={albumStageEl}
              style={albumStageStyle}
            >
              <div class="album-stage-frame">
                <div
                  class="album-stage-media"
                  style:height={albumStageMediaHeight}
                >
                  <div class="album-stage-media-content">
                    <img
                      class="album-stage-image"
                      src={selectedAlbumArtworkUrl ?? undefined}
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
                  <div
                    class="album-stage-media-border"
                    aria-hidden="true"
                  ></div>
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
                  initial={axisEnter("y", 14)}
                  animate={axisAnimate("y")}
                  exit={axisExit("y", 8)}
                  transition={motionTransition(HERO_DURATION, HERO_DELAY)}
                >
                  {#if selectedAlbum.belong}
                    <span class="album-belong-tag"
                      >{selectedAlbum.belong.toUpperCase()}</span
                    >
                  {/if}
                  <h1 class="album-hero-title">{selectedAlbum.name}</h1>
                  {#if selectedAlbum.artists && selectedAlbum.artists.length > 0}
                    <p class="album-hero-artists">
                      {selectedAlbum.artists.join(", ")}
                    </p>
                  {/if}
                  {#if selectedAlbum.intro}
                    <p class="album-hero-intro">{selectedAlbum.intro}</p>
                  {/if}
                  <div class="album-hero-meta">
                    <span class="album-song-count"
                      >{selectedAlbum.songs.length} 首歌曲</span
                    >
                    {#if shouldShowDownloadBadge(selectedAlbum.download.downloadStatus)}
                      <span class="album-download-status-badge">
                        {getDownloadBadgeLabel(selectedAlbum.download.downloadStatus)}
                      </span>
                    {/if}
                  </div>
                  <div class="controls album-hero-actions">
                    <motion.button
                      class="btn btn-primary"
                      onclick={() => downloadController.handleAlbumDownload(selectedAlbum.cid)}
                      disabled={downloadingAlbumCid === selectedAlbum.cid ||
                        !!downloadController.findAlbumDownloadJob(selectedAlbum.cid)}
                      animate={appButtonAnimate(
                        true,
                        downloadingAlbumCid === selectedAlbum.cid ||
                          !!downloadController.findAlbumDownloadJob(selectedAlbum.cid),
                      )}
                      whileHover={appButtonHover(
                        true,
                        downloadingAlbumCid === selectedAlbum.cid ||
                          !!downloadController.findAlbumDownloadJob(selectedAlbum.cid),
                      )}
                      whileTap={!prefersReducedMotion &&
                      !(
                        downloadingAlbumCid === selectedAlbum.cid ||
                        !!downloadController.findAlbumDownloadJob(selectedAlbum.cid)
                      )
                        ? { y: 0, scale: 0.98, opacity: 0.94 }
                        : undefined}
                      transition={interactiveTransition}
                    >
                      {#if downloadingAlbumCid === selectedAlbum.cid}
                        正在创建任务...
                      {:else if downloadController.findAlbumDownloadJob(selectedAlbum.cid)}
                        已在队列中
                      {:else}
                        下载整张专辑
                      {/if}
                    </motion.button>
                    <motion.button
                      class="btn"
                      onclick={toggleSelectionMode}
                      animate={appButtonAnimate(false, false)}
                      whileHover={appButtonHover(false, false)}
                      whileTap={prefersReducedMotion
                        ? undefined
                        : { y: 0, scale: 0.98, opacity: 0.94 }}
                      transition={interactiveTransition}
                    >
                      {selectionModeEnabled ? "取消多选" : "多选下载"}
                    </motion.button>
                    {#if selectionModeEnabled}
                      <motion.button
                        class="btn"
                        onclick={selectAllSongs}
                        disabled={!selectedAlbum ||
                          selectedSongCount === selectedAlbum.songs.length}
                        animate={appButtonAnimate(
                          false,
                          !selectedAlbum ||
                            selectedSongCount === selectedAlbum.songs.length,
                        )}
                        whileHover={appButtonHover(
                          false,
                          !selectedAlbum ||
                            selectedSongCount === selectedAlbum.songs.length,
                        )}
                        whileTap={!prefersReducedMotion &&
                        selectedAlbum &&
                        selectedSongCount !== selectedAlbum.songs.length
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        全选
                      </motion.button>
                      <motion.button
                        class="btn"
                        onclick={deselectAllSongs}
                        disabled={selectedSongCount === 0}
                        animate={appButtonAnimate(false, selectedSongCount === 0)}
                        whileHover={appButtonHover(false, selectedSongCount === 0)}
                        whileTap={!prefersReducedMotion && selectedSongCount > 0
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        清空
                      </motion.button>
                      <motion.button
                        class="btn"
                        onclick={invertSongSelection}
                        disabled={!selectedAlbum ||
                          selectedAlbum.songs.length === 0}
                        animate={appButtonAnimate(
                          false,
                          !selectedAlbum || selectedAlbum.songs.length === 0,
                        )}
                        whileHover={appButtonHover(
                          false,
                          !selectedAlbum || selectedAlbum.songs.length === 0,
                        )}
                        whileTap={!prefersReducedMotion &&
                        selectedAlbum &&
                        selectedAlbum.songs.length > 0
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        反选
                      </motion.button>
                      <motion.button
                        class="btn btn-primary"
                        onclick={() =>
                          downloadController.handleSelectionDownload(selectedSongCids, {
                            afterCreated: () => {
                              clearSongSelection();
                              selectionModeEnabled = false;
                            },
                          })}
                        disabled={downloadController.isSelectionDownloadActionDisabled(selectedSongCids)}
                        animate={appButtonAnimate(
                          true,
                          downloadController.isSelectionDownloadActionDisabled(selectedSongCids),
                        )}
                        whileHover={appButtonHover(
                          true,
                          downloadController.isSelectionDownloadActionDisabled(selectedSongCids),
                        )}
                        whileTap={!prefersReducedMotion &&
                        !downloadController.isSelectionDownloadActionDisabled(selectedSongCids)
                          ? { y: 0, scale: 0.98, opacity: 0.94 }
                          : undefined}
                        transition={interactiveTransition}
                      >
                        {#if downloadController.isCurrentSelectionCreating(selectedSongCids)}
                          正在创建批量任务...
                        {:else if downloadController.getCurrentSelectionJob(selectedSongCids)}
                          已在队列中
                        {:else}
                          下载所选歌曲
                        {/if}
                      </motion.button>
                      <span class="album-selection-summary"
                        >{selectedSongsLabel}</span
                      >
                    {/if}
                  </div>
                </motion.div>
              </div>
              <motion.div
                class="song-list"
                initial={axisEnter("y", 10)}
                animate={axisAnimate("y")}
                exit={fadeExit()}
                transition={motionTransition(LIST_DURATION, LIST_DELAY)}
              >
                {#each selectedAlbum.songs as song, i (song.cid)}
                  <SongRow
                    {song}
                    index={i}
                    isPlaying={currentSong?.cid === song.cid &&
                      (isPlaying || isPaused)}
                    downloadState={downloadController.getSongDownloadState(song.cid)}
                    downloadDisabled={downloadController.isSongDownloadInteractionBlocked(
                      song.cid,
                    )}
                    selectionMode={selectionModeEnabled}
                    isSelected={isSongSelected(song.cid)}
                    selectionDisabled={downloadController.isCurrentSelectionCreating(selectedSongCids)}
                    reducedMotion={prefersReducedMotion}
                    onclick={() => handlePlay(song)}
                    onDownload={() => downloadController.handleSongDownload(song.cid)}
                    onToggleSelection={() => toggleSongSelection(song.cid)}
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
            <MotionSpinner
              className="content-loading-mask-spinner"
              reducedMotion={prefersReducedMotion}
            />
          </motion.div>
        {/if}
      </AnimatePresence>
    </OverlayScrollbarsComponent>
      {/snippet}
    </AlbumWorkspace>

    <AnimatePresence>
      {#if currentSong}
        <motion.div
          key="player-dock"
          initial={axisEnter("y", 18)}
          animate={axisAnimate("y")}
          exit={fadeExit()}
          transition={motionTransition(PLAYER_DOCK_DURATION)}
        >
          <div
            class="player-dock-stack"
            data-panel={lyricsOpen
              ? "lyrics"
              : playlistOpen
                ? "playlist"
                : "none"}
          >
            <AnimatePresence initial={false}>
              {#if lyricsOpen}
                <motion.section
                  key="player-lyrics"
                  class="player-flyout"
                  data-panel="lyrics"
                  initial={axisEnter("y", 12)}
                  animate={axisAnimate("y")}
                  exit={axisExit("y", 8)}
                  transition={motionTransition(0.18)}
                >
                  <div class="player-flyout-header">
                    <div>
                      <p class="player-flyout-eyebrow">歌词</p>
                      <h3 class="player-flyout-title">{currentSong.name}</h3>
                    </div>
                    <span class="player-flyout-count"
                      >{lyricsLines.length > 0
                        ? `${lyricsLines.length} 行`
                        : "歌词"}</span
                    >
                  </div>

                  {#if lyricsLoading}
                    <div class="player-flyout-empty">正在加载歌词…</div>
                  {:else if lyricsError}
                    <div class="player-flyout-empty">{lyricsError}</div>
                  {:else if lyricsLines.length > 0}
                    <div class="player-lyrics-list">
                      {#each lyricsLines as line, index (line.id)}
                        <p
                          class={`player-lyric-line${index === activeLyricIndex ? " active" : ""}`}
                        >
                          {line.text}
                        </p>
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
                  initial={axisEnter("y", 12)}
                  animate={axisAnimate("y")}
                  exit={axisExit("y", 8)}
                  transition={motionTransition(0.18)}
                >
                  <div class="player-flyout-header">
                    <div>
                      <p class="player-flyout-eyebrow">播放列表</p>
                      <h3 class="player-flyout-title">当前队列</h3>
                    </div>
                    <span class="player-flyout-count"
                      >{playbackOrder.length} 首</span
                    >
                  </div>

                  {#if playbackOrder.length > 0}
                    <div class="player-playlist-list">
                      {#each playbackOrder as entry, index (entry.cid)}
                        <button
                          type="button"
                          class={`player-playlist-item${entry.cid === currentSong?.cid ? " active" : ""}`}
                          onclick={() => {
                            void playerController.playQueueEntry(entry, playbackOrder, index);
                          }}
                        >
                          <span class="player-playlist-index"
                            >{String(index + 1).padStart(2, "0")}</span
                          >
                          <span class="player-playlist-meta">
                            <span class="player-playlist-name"
                              >{entry.name}</span
                            >
                            <span class="player-playlist-artists"
                              >{entry.artists.join(" · ")}</span
                            >
                          </span>
                        </button>
                      {/each}
                    </div>
                  {:else}
                    <div class="player-flyout-empty">
                      当前没有可播放的队列。
                    </div>
                  {/if}
                </motion.section>
              {/if}
            </AnimatePresence>

            <PlayerDock
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
              downloadState={currentSong
                ? downloadController.getSongDownloadState(currentSong.cid)
                : "idle"}
              downloadDisabled={currentSong
                ? downloadController.isSongDownloadInteractionBlocked(currentSong.cid)
                : false}
              reducedMotion={prefersReducedMotion}
              onPrevious={playerController.playPrevious}
              onTogglePlay={isPlaying
                ? playerController.pause
                : playerController.resume}
              onSeek={playerController.seek}
              onNext={playerController.playNext}
              onShuffleChange={playerController.toggleShuffle}
              onRepeatModeChange={playerController.toggleRepeat}
              onToggleLyrics={playerController.toggleLyrics}
              onTogglePlaylist={playerController.togglePlaylist}
              onDownload={() => {
                if (currentSong) {
                  return downloadController.handleSongDownload(currentSong.cid);
                }
              }}
            />
          </div>
        </motion.div>
      {/if}
    </AnimatePresence>
  </section>

  {#if SettingsSheetView}
    <SettingsSheetView
      bind:open={settingsOpen}
      bind:format
      bind:outputDir
      bind:downloadLyrics
      bind:notifyOnDownloadComplete
      bind:notifyOnPlaybackChange
      bind:logLevel
      logRefreshToken={settingsLogRefreshToken}
      {notifyInfo}
      {notifyError}
      onOutputDirChange={() => savePreferences()}
    />
  {/if}

  {#if DownloadTasksSheetView}
    <DownloadTasksSheetView
      bind:open={downloadPanelOpen}
      jobs={filteredDownloadJobs}
      hasDownloadHistory={hasDownloadHistory}
      bind:searchQuery={downloadController.searchQuery}
      bind:scopeFilter={downloadController.scopeFilter}
      bind:statusFilter={downloadController.statusFilter}
      bind:kindFilter={downloadController.kindFilter}
      canClearDownloadHistory={downloadController.canClearDownloadHistory}
      getJobProgress={downloadController.getJobProgress}
      getJobProgressText={downloadController.getJobProgressText}
      getJobStatusLabel={downloadController.getJobStatusLabel}
      getJobKindLabel={downloadController.getJobKindLabel}
      getJobSummaryLabel={downloadController.getJobSummaryLabel}
      getJobErrorSummary={downloadController.getJobErrorSummary}
      isJobActive={downloadController.isJobActive}
      canCancelTask={downloadController.canCancelTask}
      canRetryTask={downloadController.canRetryTask}
      getTaskErrorLabel={downloadController.getTaskErrorLabel}
      getTaskStatusLabel={downloadController.getTaskStatusLabel}
      onClearDownloadHistory={downloadController.handleClearDownloadHistory}
      onCancelDownloadJob={downloadController.handleCancelDownloadJob}
      onRetryDownloadJob={downloadController.handleRetryDownloadJob}
      onCancelDownloadTask={downloadController.handleCancelDownloadTask}
      onRetryDownloadTask={downloadController.handleRetryDownloadTask}
    />
  {/if}

</div>
