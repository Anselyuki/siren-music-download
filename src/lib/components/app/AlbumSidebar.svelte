<script lang="ts">
  import AlbumCard from "$lib/components/AlbumCard.svelte";
  import MotionSpinner from "$lib/components/MotionSpinner.svelte";
  import type { Album } from "$lib/types";

  interface Props {
    albums: Album[];
    selectedAlbumCid: string | null;
    reducedMotion: boolean;
    loadingAlbums?: boolean;
    errorMsg?: string;
    onSelect: (album: Album) => void;
  }

  let {
    albums,
    selectedAlbumCid,
    reducedMotion,
    loadingAlbums = false,
    errorMsg = "",
    onSelect,
  }: Props = $props();
</script>

<div class="h-full">
  <h2 class="section-title">专辑</h2>
  {#if loadingAlbums}
    <div class="loading">
      <span>正在加载专辑...</span><MotionSpinner
        className="inline-loading-spinner"
        reducedMotion={reducedMotion}
      />
    </div>
  {:else if errorMsg && albums.length === 0}
    <div class="empty-state">
      <div class="empty-icon">⚠️</div>
      <div class="empty-text">加载失败</div>
      <div class="empty-text" style="margin-top: 8px; font-size: 12px;">
        {errorMsg}
      </div>
    </div>
  {:else}
    <div class="album-list">
      {#each albums as album (album.cid)}
        <AlbumCard
          {album}
          selected={selectedAlbumCid === album.cid}
          reducedMotion={reducedMotion}
          onclick={() => onSelect(album)}
        />
      {/each}
    </div>
  {/if}
</div>
