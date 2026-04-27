<script lang="ts">
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import { fade } from 'svelte/transition';
  import { OverlayScrollbarsComponent } from 'overlayscrollbars-svelte';
  import type { EventListeners, PartialOptions } from 'overlayscrollbars';
  import type { AlbumDetail, SongEntry } from '$lib/types';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import AlbumStage from '$lib/components/app/AlbumStage.svelte';
  import AlbumDetailSkeleton from '$lib/components/app/AlbumDetailSkeleton.svelte';
  import AlbumDetailPanel from '$lib/components/app/AlbumDetailPanel.svelte';

  type SongDownloadState = 'idle' | 'creating' | 'queued' | 'running';

  interface Props {
    loadingDetail: boolean;
    showDetailSkeleton: boolean;
    albumRequestSeq: number;
    selectedAlbum: AlbumDetail | null;
    selectedAlbumArtworkUrl: string | null;
    currentSongCid: string | null;
    isPlaybackActive: boolean;
    downloadingAlbumCid: string | null;
    selectionModeEnabled: boolean;
    selectedSongCids: string[];
    reducedMotion: boolean;
    overlayScrollbarOptions: PartialOptions;
    contentScrollbarEvents: EventListeners;
    onContentWheel: (event: WheelEvent) => void;
    albumStageStyle: string;
    albumStageMediaHeight: string;
    albumStageScrimOpacity: number;
    albumStageImageOpacity: number;
    albumStageImageTransform: string;
    albumStageSolidifyOpacity: number;
    albumStageElement?: HTMLElement | null;
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

  let {
    loadingDetail,
    showDetailSkeleton,
    albumRequestSeq: _albumRequestSeq,
    selectedAlbum,
    selectedAlbumArtworkUrl,
    currentSongCid,
    isPlaybackActive,
    downloadingAlbumCid,
    selectionModeEnabled,
    selectedSongCids,
    reducedMotion,
    overlayScrollbarOptions,
    contentScrollbarEvents,
    onContentWheel,
    albumStageStyle,
    albumStageMediaHeight,
    albumStageScrimOpacity,
    albumStageImageOpacity,
    albumStageImageTransform,
    albumStageSolidifyOpacity,
    albumStageElement = $bindable<HTMLElement | null>(null),
    onToggleSelectionMode,
    onSelectAllSongs,
    onDeselectAllSongs,
    onInvertSongSelection,
    onDownloadAlbum,
    onDownloadSelection,
    onPlaySong,
    onDownloadSong,
    onToggleSongSelection,
    isSongSelected,
    getSongDownloadState,
    isSongDownloadInteractionBlocked,
    hasAlbumDownloadJob,
    isSelectionDownloadDisabled,
    isCurrentSelectionCreating,
    hasCurrentSelectionJob,
  }: Props = $props();

  function dur(base: number): number {
    return reducedMotion ? 0 : base;
  }

  const emptyLabels = $derived.by(() => {
    void localeState.current;
    return {
      title: m.library_workspace_select_album(),
      hint: m.library_workspace_select_album_hint(),
    };
  });
</script>

<OverlayScrollbarsComponent
  element="div"
  class="h-full"
  data-overlayscrollbars-initialize
  options={overlayScrollbarOptions}
  events={contentScrollbarEvents}
  defer
  onwheel={onContentWheel}
  aria-busy={loadingDetail}
>
  {#if loadingDetail && showDetailSkeleton}
    <section
      class="album-panel album-panel-loading"
      in:fade={{ duration: dur(180) }}
      out:fade={{ duration: dur(180) }}
    >
      <AlbumStage
        loading={true}
        {reducedMotion}
        stageStyle={albumStageStyle}
        mediaHeight={albumStageMediaHeight}
        scrimOpacity={albumStageScrimOpacity}
        bind:element={albumStageElement}
      />
      <AlbumDetailSkeleton {reducedMotion} />
    </section>
  {:else if selectedAlbum}
    {#key selectedAlbum.cid}
      <section
        class="album-panel"
        in:fade={{ duration: dur(180) }}
        out:fade={{ duration: dur(180) }}
      >
        <AlbumStage
          albumName={selectedAlbum.name}
          artworkUrl={selectedAlbumArtworkUrl}
          {reducedMotion}
          stageStyle={albumStageStyle}
          mediaHeight={albumStageMediaHeight}
          scrimOpacity={albumStageScrimOpacity}
          imageOpacity={albumStageImageOpacity}
          imageTransform={albumStageImageTransform}
          solidifyOpacity={albumStageSolidifyOpacity}
          bind:element={albumStageElement}
        />
        <AlbumDetailPanel
          album={selectedAlbum}
          {currentSongCid}
          {isPlaybackActive}
          {downloadingAlbumCid}
          {selectionModeEnabled}
          {selectedSongCids}
          {reducedMotion}
          {onToggleSelectionMode}
          {onSelectAllSongs}
          {onDeselectAllSongs}
          {onInvertSongSelection}
          {onDownloadAlbum}
          {onDownloadSelection}
          {onPlaySong}
          {onDownloadSong}
          {onToggleSongSelection}
          {isSongSelected}
          {getSongDownloadState}
          {isSongDownloadInteractionBlocked}
          {hasAlbumDownloadJob}
          {isSelectionDownloadDisabled}
          {isCurrentSelectionCreating}
          {hasCurrentSelectionJob}
        />
      </section>
    {/key}
  {/if}

  {#if !loadingDetail && !selectedAlbum}
    <h1 class="page-title">{emptyLabels.title}</h1>
    <p class="page-subtitle">{emptyLabels.hint}</p>
  {/if}

  {#if loadingDetail && selectedAlbum}
    <div
      class="content-loading-mask"
      aria-hidden="true"
      in:fade={{ duration: dur(140) }}
      out:fade={{ duration: dur(140) }}
    >
      <MotionSpinner className="content-loading-mask-spinner" {reducedMotion} />
    </div>
  {/if}
</OverlayScrollbarsComponent>
