<script lang="ts">
  import AudioPlayer from '$lib/components/AudioPlayer.svelte';
  import type { ComponentProps } from 'svelte';

  let props: ComponentProps<typeof AudioPlayer> = $props();
</script>

<div class="pointer-events-none absolute inset-x-0 bottom-0 z-[140] px-0">
  <div
    class="player-dock-glow pointer-events-auto w-full border-x-0 border-b-0 border-t border-white/50 bg-[var(--surface-dock)] p-0 backdrop-blur-xl"
  >
    <AudioPlayer {...props} />
  </div>
</div>

<style>
  .player-dock-glow {
    position: relative;
    isolation: isolate;
    box-shadow: inset 0 1px 0 rgba(255, 255, 255, 0.36);
  }

  .player-dock-glow :global(.am-player) {
    position: relative;
    z-index: 1;
    width: 100%;
    border-radius: 0;
    border: 0;
  }
</style>
