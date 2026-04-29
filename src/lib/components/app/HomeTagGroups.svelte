<script lang="ts">
  import type { TagDimension, TagGroup } from '$lib/types';

  interface Props {
    dimensions: TagDimension[];
    groups: TagGroup[];
    selectedDimensionKey: string | null;
    onSelectDimension: (key: string) => void;
  }

  let { dimensions, groups, selectedDimensionKey, onSelectDimension }: Props =
    $props();
</script>

<section class="tag-groups" aria-label="标签分组">
  <h2 class="section-title">标签</h2>

  {#if dimensions.length === 0}
    <p class="empty-hint">暂无标签数据</p>
  {:else}
    <div class="dimension-chips" role="tablist">
      {#each dimensions as dim (dim.key)}
        <button
          class="dimension-chip"
          class:active={selectedDimensionKey === dim.key}
          type="button"
          role="tab"
          aria-selected={selectedDimensionKey === dim.key}
          onclick={() => onSelectDimension(dim.key)}
        >
          {dim.label}
        </button>
      {/each}
    </div>

    {#if groups.length === 0 && selectedDimensionKey !== null}
      <div class="skeleton-list">
        {#each Array(3) as _, i (i)}
          <div class="skeleton-row"></div>
        {/each}
      </div>
    {:else}
      <ul class="group-list" role="list">
        {#each groups as group (group.value)}
          <li class="group-item">
            <div class="group-header">
              <span class="group-name">{group.value}</span>
              <span class="group-count">{group.albums.length} 张专辑</span>
            </div>

            <div class="group-albums">
              {#each group.albums.slice(0, 8) as album (album.cid)}
                <div class="mini-cover-wrapper" title={album.name}>
                  <img
                    src={album.coverUrl}
                    alt={album.name}
                    class="mini-cover"
                    loading="lazy"
                  />
                </div>
              {/each}
              {#if group.albums.length > 8}
                <span class="overflow-badge">+{group.albums.length - 8}</span>
              {/if}
            </div>
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .tag-groups {
    display: flex;
    flex-direction: column;
    gap: 0.75rem;
  }

  .section-title {
    font-family: var(--font-display);
    font-size: 1.125rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
    margin: 0;
  }

  .dimension-chips {
    display: flex;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .dimension-chip {
    padding: 0.25rem 0.75rem;
    border-radius: 9999px;
    border: none;
    font-family: var(--font-body);
    font-size: 0.75rem;
    font-weight: 500;
    cursor: pointer;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    transition:
      background 0.15s ease,
      color 0.15s ease;
  }

  .dimension-chip:hover {
    background: var(--surface-secondary, rgba(255, 255, 255, 0.08));
  }

  .dimension-chip.active {
    background: rgba(var(--accent-rgb), 0.15);
    color: var(--accent);
  }

  .group-list {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .group-item {
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
    border-radius: 10px;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.625rem;
  }

  .group-header {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }

  .group-name {
    font-family: var(--font-body);
    font-size: 0.9375rem;
    font-weight: 600;
    color: var(--text-primary, #fff);
  }

  .group-count {
    font-family: var(--font-body);
    font-size: 0.75rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
  }

  .group-albums {
    display: flex;
    align-items: center;
    gap: 0.375rem;
    flex-wrap: wrap;
  }

  .mini-cover-wrapper {
    width: 40px;
    height: 40px;
    border-radius: 5px;
    overflow: hidden;
    flex-shrink: 0;
  }

  .mini-cover {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .overflow-badge {
    font-family: var(--font-body);
    font-size: 0.75rem;
    color: var(--text-secondary, rgba(255, 255, 255, 0.6));
    padding: 0 0.25rem;
  }

  .skeleton-list {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .skeleton-row {
    height: 80px;
    border-radius: 10px;
    background: var(--surface-secondary, rgba(255, 255, 255, 0.04));
    animation: pulse 1.5s ease-in-out infinite;
  }

  .empty-hint {
    font-family: var(--font-body);
    font-size: 0.8125rem;
    color: var(--text-tertiary, rgba(255, 255, 255, 0.4));
    margin: 0;
  }

  @keyframes pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.5;
    }
  }
</style>
