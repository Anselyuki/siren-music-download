<script lang="ts">
  import { Input } from '$lib/components/ui/input/index.js';
  import { Button } from '$lib/components/ui/button/index.js';
  import SearchIcon from '@lucide/svelte/icons/search';
  import AlbumCard from '$lib/components/AlbumCard.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';
  import * as m from '$lib/paraglide/messages.js';
  import { localeState } from '$lib/i18n';
  import type {
    Album,
    LibraryIndexState,
    SearchLibraryResponse,
    SearchLibraryResultItem,
    LibrarySearchScope,
  } from '$lib/types';

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    reducedMotion: boolean;
    loadingAlbums?: boolean;
    errorMsg?: string;
    searchQuery?: string;
    searchScope?: LibrarySearchScope;
    searchLoading?: boolean;
    searchResponse?: SearchLibraryResponse | null;
    onSearchQueryChange: (query: string) => void;
    onSearchScopeChange: (scope: LibrarySearchScope) => void;
    onSelect: (album: Album) => void;
    onSelectSearchResult: (item: SearchLibraryResultItem) => void;
  }

  let {
    albums,
    selectedAlbumCid,
    reducedMotion,
    loadingAlbums = false,
    errorMsg = '',
    searchQuery = '',
    searchScope = 'all',
    searchLoading = false,
    searchResponse = null,
    onSearchQueryChange,
    onSearchScopeChange,
    onSelect,
    onSelectSearchResult,
  }: Props = $props();

  const scopeOptions = $derived.by(() => {
    void localeState.current;
    return [
      {
        value: 'all' as LibrarySearchScope,
        // "ALL" 是固定品牌文案，不走 i18n
        label: 'ALL',
      },
      {
        value: 'albums' as LibrarySearchScope,
        label: m.library_search_scope_albums(),
      },
      {
        value: 'songs' as LibrarySearchScope,
        label: m.library_search_scope_songs(),
      },
    ];
  });

  const trimmedSearchQuery = $derived.by(() => searchQuery.trim());
  const isSearchMode = $derived.by(() => trimmedSearchQuery.length > 0);
  const searchIndexState = $derived.by<LibraryIndexState>(
    () => searchResponse?.indexState ?? 'notReady'
  );
  const isSearchIndexBuilding = $derived.by(
    () => isSearchMode && !searchLoading && searchIndexState === 'building'
  );

  const labels = $derived.by(() => {
    void localeState.current;
    return {
      searchAria: m.library_search_aria(),
      loadingAlbums: m.library_loading_albums(),
      loadFailed: m.library_load_failed(),
      indexBuildingTitle: m.library_search_index_building_title(),
      indexBuildingAria: m.library_search_index_building_aria(),
      indexBuildingValuetext: m.library_search_index_building_valuetext(),
      indexBuildingHint: m.library_search_index_building_hint(),
      resultKindAlbum: m.library_search_result_kind_album(),
      resultKindSong: m.library_search_result_kind_song(),
      noResults: m.library_search_no_results(),
    };
  });

  const searchStatusMessage = $derived.by(() => {
    if (!isSearchMode) return '';
    void localeState.current;
    if (searchLoading) return m.library_search_searching();
    switch (searchIndexState) {
      case 'stale':
        return m.library_search_index_stale();
      case 'notReady':
        return m.library_search_index_not_ready();
      default:
        return '';
    }
  });
  const activeScopeLabel = $derived.by(
    () =>
      scopeOptions.find((option) => option.value === searchScope)?.label ??
      'ALL'
  );

  function cycleSearchScope() {
    const currentIndex = scopeOptions.findIndex(
      (option) => option.value === searchScope
    );
    const nextIndex = (currentIndex + 1) % scopeOptions.length;
    onSearchScopeChange(scopeOptions[nextIndex]?.value ?? 'all');
  }
</script>

<div class="sidebar-layout">
  <div class="sidebar-header">
    <div class="search-control-row">
      <SearchIcon class="library-search-icon" aria-hidden="true" />
      <Input
        value={searchQuery}
        placeholder=""
        aria-label={labels.searchAria}
        class="library-search-input"
        oninput={(event) => {
          const target = event.currentTarget as HTMLInputElement;
          onSearchQueryChange(target.value);
        }}
      />
      <Button
        variant="outline"
        size="icon"
        class="library-search-scope-button active:!translate-y-0"
        data-scope={searchScope}
        aria-label={m.library_search_scope_aria({ scope: activeScopeLabel })}
        title={m.library_search_scope_title({ scope: activeScopeLabel })}
        onclick={cycleSearchScope}
      >
        {activeScopeLabel}
      </Button>
    </div>
    <div class="library-search-divider" aria-hidden="true"></div>
  </div>

  <div class="sidebar-scroll-area">
    {#if loadingAlbums}
      <div class="loading">
        <span>{labels.loadingAlbums}</span><MotionSpinner
          className="inline-loading-spinner"
          {reducedMotion}
        />
      </div>
    {:else if errorMsg && albums.length === 0}
      <div class="empty-state">
        <div class="empty-icon">⚠️</div>
        <div class="empty-text">{labels.loadFailed}</div>
        <div class="empty-text" style="margin-top: 8px; font-size: 12px;">
          {errorMsg}
        </div>
      </div>
    {:else if isSearchMode}
      {#if isSearchIndexBuilding}
        <div class="search-status-card" aria-live="polite">
          <div class="search-status-title">{labels.indexBuildingTitle}</div>
          <div
            class="search-status-progress"
            role="progressbar"
            aria-label={labels.indexBuildingAria}
            aria-valuetext={labels.indexBuildingValuetext}
          >
            <div
              class={`search-status-progress-bar${reducedMotion ? ' is-reduced-motion' : ''}`}
            ></div>
          </div>
          <div class="search-status-hint">{labels.indexBuildingHint}</div>
        </div>
      {:else if searchStatusMessage}
        <div class="empty-state">
          <div class="empty-text">{searchStatusMessage}</div>
        </div>
      {:else if searchResponse && searchResponse.items.length > 0}
        <div class="album-list">
          {#each searchResponse.items as item, index (`${item.kind}:${item.albumCid}:${item.songCid ?? 'album'}:${index}`)}
            <button
              type="button"
              class={`search-result${selectedAlbumCid === item.albumCid ? ' is-selected' : ''}`}
              aria-label={m.library_search_result_aria({
                kind:
                  item.kind === 'album'
                    ? labels.resultKindAlbum
                    : labels.resultKindSong,
                name:
                  item.kind === 'song' && item.songTitle
                    ? item.songTitle
                    : item.albumTitle,
              })}
              onclick={() => onSelectSearchResult(item)}
            >
              <div class="search-result-kind">
                {item.kind === 'album'
                  ? labels.resultKindAlbum
                  : labels.resultKindSong}
              </div>
              <div class="search-result-title">
                {item.kind === 'song' && item.songTitle
                  ? item.songTitle
                  : item.albumTitle}
              </div>
              <div class="search-result-subtitle">
                {#if item.kind === 'song'}
                  <span>{item.albumTitle}</span>
                {/if}
                {#if item.artistLine}
                  <span>{item.artistLine}</span>
                {/if}
              </div>
            </button>
          {/each}
        </div>
      {:else}
        <div class="empty-state">
          <div class="empty-text">{labels.noResults}</div>
        </div>
      {/if}
    {:else}
      <div class="album-list">
        {#each albums as album (album.cid)}
          <AlbumCard
            {album}
            selected={selectedAlbumCid === album.cid}
            {reducedMotion}
            onclick={() => onSelect(album)}
          />
        {/each}
      </div>
    {/if}
  </div>
</div>

<style>
  .sidebar-layout {
    display: flex;
    flex-direction: column;
    width: 100%;
    min-width: 0;
    height: 100%;
  }

  .sidebar-header {
    flex-shrink: 0;
    min-width: 0;
  }

  .sidebar-scroll-area {
    flex: 1;
    width: 100%;
    min-width: 0;
    overflow-y: auto;
    min-height: 0;
    scrollbar-width: thin;
    padding-right: 0;
  }

  .sidebar-scroll-area::-webkit-scrollbar {
    width: 4px;
  }

  .sidebar-scroll-area::-webkit-scrollbar-track {
    background: transparent;
  }

  .sidebar-scroll-area::-webkit-scrollbar-thumb {
    border-radius: 4px;
    background: rgba(255, 255, 255, 0.28);
  }

  .sidebar-scroll-area::-webkit-scrollbar-thumb:hover {
    background: rgba(255, 255, 255, 0.42);
  }

  .search-control-row {
    position: relative;
    width: 100%;
    min-width: 0;
  }

  .sidebar-layout :global(.album-list) {
    width: 100%;
    min-width: 0;
  }

  .sidebar-layout :global(.album-card) {
    width: 100%;
    min-width: 0;
  }

  .library-search-divider {
    height: 1px;
    margin: 14px 4px 16px;
    background: linear-gradient(
      90deg,
      transparent,
      rgba(var(--accent-rgb), 0.34) 18%,
      rgba(var(--accent-rgb), 0.52) 50%,
      rgba(var(--accent-rgb), 0.34) 82%,
      transparent
    );
  }

  :global(.library-search-input) {
    height: 40px;
    padding-left: 34px;
    padding-right: 46px;
    border: 1px solid rgba(255, 255, 255, 0.48);
    border-radius: 12px;
    background:
      linear-gradient(
        180deg,
        rgba(255, 255, 255, 0.36),
        rgba(255, 255, 255, 0.2)
      ),
      rgba(255, 255, 255, 0.18);
    color: var(--text-primary);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.52),
      0 8px 18px rgba(15, 23, 42, 0.08);
  }

  :global(.library-search-icon) {
    position: absolute;
    top: 50%;
    left: 12px;
    z-index: 1;
    width: 16px;
    height: 16px;
    color: var(--text-tertiary);
    pointer-events: none;
    transform: translateY(-50%);
  }

  :global(.library-search-input:focus-visible) {
    border-color: rgba(var(--accent-rgb), 0.36);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.56),
      0 0 0 3px rgba(var(--accent-rgb), 0.14),
      0 10px 20px rgba(15, 23, 42, 0.1);
  }

  :global(.library-search-scope-button) {
    --scope-bg: var(--accent);
    --scope-bg-hover: var(--accent-hover);

    interpolate-size: allow-keywords;
    position: absolute;
    top: 4px;
    right: 4px;
    width: auto;
    min-width: 32px;
    height: 32px;
    padding: 0 8px;
    border: 1px solid color-mix(in srgb, var(--scope-bg) 72%, white 28%);
    border-radius: 8px;
    background: var(--scope-bg);
    box-shadow:
      inset 0 1px 0 rgba(255, 255, 255, 0.22),
      0 5px 12px color-mix(in srgb, var(--scope-bg) 24%, transparent);
    color: var(--accent-readable-foreground);
    font-size: 11px;
    font-weight: 800;
    letter-spacing: 0;
    line-height: 1;
    white-space: nowrap;
    transition:
      width 0.25s ease,
      transform 0.15s ease,
      background-color 0.2s ease,
      border-color 0.2s ease,
      box-shadow 0.2s ease;
  }

  :global(.library-search-scope-button[data-scope='albums']) {
    --scope-bg: oklch(from var(--accent) l c calc(h + 22));
    --scope-bg-hover: oklch(from var(--accent-hover) l c calc(h + 22));
  }

  :global(.library-search-scope-button[data-scope='songs']) {
    --scope-bg: oklch(from var(--accent) l c calc(h - 28));
    --scope-bg-hover: oklch(from var(--accent-hover) l c calc(h - 28));
  }

  :global(.library-search-scope-button:hover) {
    border-color: color-mix(in srgb, var(--scope-bg-hover) 78%, white 22%);
    background: var(--scope-bg-hover);
    color: var(--accent-hover-readable-foreground);
  }

  :global(.library-search-scope-button[data-scope='all']) {
    overflow: hidden;
    isolation: isolate;
  }

  :global(.library-search-scope-button[data-scope='all']::before) {
    content: '';
    position: absolute;
    top: 50%;
    left: 50%;
    width: 800px;
    height: 800px;
    background:
      radial-gradient(
          circle,
          rgba(255, 255, 255, 0.32) 0.7px,
          transparent 0.7px
        )
        0 0 / 3.5px 3.5px,
      linear-gradient(
        to right,
        #ff2080,
        #c830ff,
        #5050ff,
        #00a0ff,
        #00d4a0,
        #60e840,
        #e8d020,
        #ff8020,
        #ff2080
      );
    background-size:
      3.5px 3.5px,
      25% 100%;
    z-index: -1;
    pointer-events: none;
    opacity: 0;
    transform: translate(-50%, -50%) rotate(135deg);
    animation: scope-rainbow-slide 2.4s linear infinite;
    transition: opacity 0.3s ease;
  }

  :global(.library-search-scope-button[data-scope='all']:hover) {
    border-color: rgba(255, 255, 255, 0.48);
    background: transparent;
    color: #fff;
    text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
    -webkit-text-stroke: 2px rgba(0, 0, 0, 0.5);
    paint-order: stroke fill;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.32);
  }

  :global(.library-search-scope-button[data-scope='all']:hover::before) {
    opacity: 1;
  }

  :global(.library-search-scope-button:active) {
    transform: scaleX(0.92);
    box-shadow:
      inset 0 1px 2px rgba(15, 23, 42, 0.08),
      0 2px 6px rgba(15, 23, 42, 0.06);
  }

  .search-status-card {
    display: grid;
    gap: 10px;
    padding: 16px 14px;
    border-radius: 20px;
    border: 1px solid rgba(255, 255, 255, 0.22);
    background: rgba(255, 255, 255, 0.16);
  }

  .search-status-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-status-hint {
    font-size: 12px;
    color: var(--text-secondary);
  }

  .search-status-progress {
    position: relative;
    overflow: hidden;
    height: 8px;
    border-radius: 999px;
    background: rgba(255, 255, 255, 0.16);
    box-shadow: inset 0 0 0 1px rgba(255, 255, 255, 0.12);
  }

  .search-status-progress-bar {
    position: absolute;
    inset: 0;
    width: 42%;
    border-radius: inherit;
    background: linear-gradient(
      90deg,
      rgba(var(--accent-rgb), 0.28) 0%,
      rgba(var(--accent-rgb), 0.9) 45%,
      rgba(var(--accent-rgb), 0.32) 100%
    );
    animation: search-progress-slide 1.2s ease-in-out infinite;
  }

  .search-status-progress-bar.is-reduced-motion {
    width: 100%;
    opacity: 0.72;
    animation: none;
  }

  @keyframes search-progress-slide {
    0% {
      transform: translateX(-100%);
    }

    100% {
      transform: translateX(240%);
    }
  }

  @keyframes scope-rainbow-slide {
    from {
      transform: translate(-50%, -50%) rotate(135deg) translateX(0);
    }

    to {
      transform: translate(-50%, -50%) rotate(135deg) translateX(-25%);
    }
  }

  .search-result {
    width: 100%;
    display: grid;
    gap: 4px;
    padding: 12px 14px;
    border-radius: 18px;
    border: 1px solid rgba(255, 255, 255, 0.28);
    background: rgba(255, 255, 255, 0.22);
    text-align: left;
  }

  .search-result:not(.is-selected) {
    transition:
      background-color 0.16s ease,
      border-color 0.16s ease;
  }

  .search-result:hover:not(.is-selected),
  .search-result.is-selected {
    background: rgba(var(--accent-rgb), 0.1);
    border-color: rgba(var(--accent-rgb), 0.22);
  }

  .search-result.is-selected {
    transition: none;
  }

  .search-result-kind {
    font-size: 11px;
    color: var(--text-secondary);
  }

  .search-result-title {
    font-size: 14px;
    font-weight: 600;
    color: var(--text-primary);
  }

  .search-result-subtitle {
    display: flex;
    flex-wrap: wrap;
    gap: 6px;
    font-size: 12px;
    color: var(--text-secondary);
  }
</style>
