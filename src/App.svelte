<script lang="ts">
  import { createAppRuntime } from '$lib/features/shell/appRuntime.svelte';
  import TopToolbar from '$lib/components/app/TopToolbar.svelte';
  import StatusToastHost from '$lib/components/app/StatusToastHost.svelte';
  import AlbumSidebarContainer from '$lib/components/app/AlbumSidebarContainer.svelte';
  import AlbumWorkspace from '$lib/components/app/AlbumWorkspace.svelte';
  import AlbumWorkspaceContent from '$lib/components/app/AlbumWorkspaceContent.svelte';
  import PlayerFlyoutStack from '$lib/components/app/PlayerFlyoutStack.svelte';
  import AppSideSheets from '$lib/components/app/AppSideSheets.svelte';
  import HomeView from '$lib/components/app/HomeView.svelte';

  const runtime = createAppRuntime();
</script>

{#if runtime.isMacOS}
  <div
    class="macos-window-drag-region"
    data-tauri-drag-region
    aria-hidden="true"
  ></div>
{/if}

<StatusToastHost />

<div class="app-shell" class:macos-overlay={runtime.isMacOS}>
  <AlbumSidebarContainer
    isMacOS={runtime.isMacOS}
    currentView={runtime.currentView}
    albums={runtime.albums}
    selectedAlbumCid={runtime.selectedAlbumCid}
    reducedMotion={runtime.prefersReducedMotion}
    loadingAlbums={runtime.loadingAlbums}
    errorMsg={runtime.errorMsg}
    searchQuery={runtime.librarySearchQuery}
    searchScope={runtime.librarySearchScope}
    searchLoading={runtime.librarySearchLoading}
    searchResponse={runtime.librarySearchResponse}
    onNavigateHome={runtime.shellStore.navigateToHome}
    onSearchQueryChange={runtime.libraryController.setSearchQuery}
    onSearchScopeChange={runtime.libraryController.setSearchScope}
    onSelect={runtime.handleSelectAlbum}
    onSelectSearchResult={runtime.handleSelectSearchResult}
  />

  <section class="main-region">
    {#if runtime.isMacOS}
      <div
        class="main-drag-region"
        data-tauri-drag-region
        aria-hidden="true"
      ></div>
    {/if}

    <TopToolbar
      activeDownloadCount={runtime.activeDownloadCount}
      isRefreshing={runtime.isRefreshing}
      settingsOpen={runtime.settingsOpen}
      downloadPanelOpen={runtime.downloadPanelOpen}
      onRefresh={runtime.handleRefresh}
      onOpenDownloads={runtime.handleToggleDownloads}
      onOpenSettings={runtime.handleToggleSettings}
    />

    {#if runtime.currentView === 'home'}
      <HomeView {runtime} />
    {:else}
      <AlbumWorkspace
        currentSong={runtime.currentSong}
        loadingDetail={runtime.loadingDetail}
        selectedAlbum={runtime.selectedAlbum}
      >
        <AlbumWorkspaceContent
          loadingDetail={runtime.loadingDetail}
          showDetailSkeleton={runtime.showDetailSkeleton}
          albumRequestSeq={runtime.albumRequestSeq}
          selectedAlbum={runtime.selectedAlbum}
          selectedAlbumArtworkUrl={runtime.selectedAlbumArtworkUrl}
          currentSongCid={runtime.currentSong?.cid ?? null}
          isPlaybackActive={runtime.isPlaying || runtime.isPaused}
          downloadingAlbumCid={runtime.downloadingAlbumCid}
          selectionModeEnabled={runtime.selectionModeEnabled}
          selectedSongCids={runtime.selectedSongCids}
          reducedMotion={runtime.prefersReducedMotion}
          overlayScrollbarOptions={runtime.overlayScrollbarOptions}
          contentScrollbarEvents={runtime.contentScrollbarEvents}
          onContentWheel={runtime.handleContentWheel}
          albumStageStyle={runtime.albumStageStyle}
          albumStageMediaHeight={runtime.albumStageMediaHeight}
          albumStageScrimOpacity={runtime.albumStageScrimOpacity}
          albumStageImageOpacity={runtime.albumStageImageOpacity}
          albumStageImageTransform={runtime.albumStageImageTransform}
          albumStageSolidifyOpacity={runtime.albumStageSolidifyOpacity}
          bind:albumStageElement={runtime.albumStageElement}
          onToggleSelectionMode={runtime.toggleSelectionMode}
          onSelectAllSongs={runtime.selectAllSongs}
          onDeselectAllSongs={runtime.deselectAllSongs}
          onInvertSongSelection={runtime.invertSongSelection}
          onDownloadAlbum={runtime.downloadController.handleAlbumDownload}
          onDownloadSelection={runtime.handleDownloadSelection}
          onPlaySong={runtime.handlePlay}
          onDownloadSong={runtime.downloadController.handleSongDownload}
          onToggleSongSelection={runtime.toggleSongSelection}
          isSongSelected={runtime.isSongSelected}
          getSongDownloadState={runtime.downloadController.getSongDownloadState}
          isSongDownloadInteractionBlocked={runtime.downloadController
            .isSongDownloadInteractionBlocked}
          hasAlbumDownloadJob={runtime.hasAlbumDownloadJob}
          isSelectionDownloadDisabled={runtime.downloadController
            .isSelectionDownloadActionDisabled}
          isCurrentSelectionCreating={runtime.downloadController
            .isCurrentSelectionCreating}
          hasCurrentSelectionJob={runtime.hasCurrentSelectionJob}
        />
      </AlbumWorkspace>
    {/if}

    <PlayerFlyoutStack
      song={runtime.currentSong}
      isPlaying={runtime.isPlaying}
      isPaused={runtime.isPaused}
      hasPrevious={runtime.playerHasPrevious}
      hasNext={runtime.playerHasNext}
      progress={runtime.progress}
      duration={runtime.duration}
      isLoading={runtime.isLoading}
      reducedMotion={runtime.prefersReducedMotion}
      isShuffled={runtime.shuffleEnabled}
      repeatMode={runtime.repeatMode}
      lyricsOpen={runtime.lyricsOpen}
      playlistOpen={runtime.playlistOpen}
      lyricsLoading={runtime.lyricsLoading}
      lyricsError={runtime.lyricsError}
      lyricsLines={runtime.lyricsLines}
      activeLyricIndex={runtime.activeLyricIndex}
      playbackOrder={runtime.playbackOrder}
      downloadState={runtime.currentSongDownloadState}
      downloadDisabled={runtime.currentSongDownloadDisabled}
      onPrevious={runtime.playerController.playPrevious}
      onTogglePlay={runtime.isPlaying
        ? runtime.playerController.pause
        : runtime.playerController.resume}
      onSeek={runtime.playerController.seek}
      onNext={runtime.playerController.playNext}
      onShuffleChange={runtime.playerController.toggleShuffle}
      onRepeatModeChange={runtime.playerController.toggleRepeat}
      onToggleLyrics={runtime.playerController.toggleLyrics}
      onTogglePlaylist={runtime.playerController.togglePlaylist}
      onDownload={runtime.handleCurrentSongDownload}
      onPlayQueueEntry={runtime.playerController.playQueueEntry}
    />

    <AppSideSheets
      SettingsSheetView={runtime.SettingsSheetView}
      DownloadTasksSheetView={runtime.DownloadTasksSheetView}
      bind:settingsOpen={runtime.shellStore.settingsOpen}
      bind:downloadPanelOpen={runtime.shellStore.downloadPanelOpen}
      bind:format={runtime.settingsState.format}
      bind:outputDir={runtime.settingsState.outputDir}
      bind:downloadLyrics={runtime.settingsState.downloadLyrics}
      bind:notifyOnDownloadComplete={
        runtime.settingsState.notifyOnDownloadComplete
      }
      bind:notifyOnPlaybackChange={runtime.settingsState.notifyOnPlaybackChange}
      bind:logLevel={runtime.settingsState.logLevel}
      bind:locale={runtime.settingsState.locale}
      settingsLogRefreshToken={runtime.settingsState.settingsLogRefreshToken}
      notifyInfo={runtime.notifyInfo}
      notifyError={runtime.notifyError}
      onOutputDirChange={runtime.handleOutputDirChange}
      jobs={runtime.filteredDownloadJobs}
      hasDownloadHistory={runtime.hasDownloadHistory}
      bind:searchQuery={runtime.downloadController.searchQuery}
      bind:scopeFilter={runtime.downloadController.scopeFilter}
      bind:statusFilter={runtime.downloadController.statusFilter}
      bind:kindFilter={runtime.downloadController.kindFilter}
      canClearDownloadHistory={runtime.downloadController
        .canClearDownloadHistory}
      getJobProgress={runtime.downloadController.getJobProgress}
      getJobProgressText={runtime.downloadController.getJobProgressText}
      getJobStatusLabel={runtime.downloadController.getJobStatusLabel}
      getJobKindLabel={runtime.downloadController.getJobKindLabel}
      getJobSummaryLabel={runtime.downloadController.getJobSummaryLabel}
      getJobDisplayTitle={runtime.downloadController.getJobDisplayTitle}
      getJobErrorSummary={runtime.downloadController.getJobErrorSummary}
      isJobActive={runtime.downloadController.isJobActive}
      canCancelTask={runtime.downloadController.canCancelTask}
      canRetryTask={runtime.downloadController.canRetryTask}
      getTaskErrorLabel={runtime.downloadController.getTaskErrorLabel}
      getTaskStatusLabel={runtime.downloadController.getTaskStatusLabel}
      onClearDownloadHistory={runtime.downloadController
        .handleClearDownloadHistory}
      onCancelDownloadJob={runtime.downloadController.handleCancelDownloadJob}
      onRetryDownloadJob={runtime.downloadController.handleRetryDownloadJob}
      onCancelDownloadTask={runtime.downloadController.handleCancelDownloadTask}
      onRetryDownloadTask={runtime.downloadController.handleRetryDownloadTask}
    />
  </section>
</div>
