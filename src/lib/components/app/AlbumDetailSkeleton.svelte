<script lang="ts">
  import { fade } from 'svelte/transition';
  import MotionPulseBlock from '$lib/components/MotionPulseBlock.svelte';
  import MotionSpinner from '$lib/components/MotionSpinner.svelte';

  interface Props {
    reducedMotion: boolean;
  }

  let props: Props = $props();

  function fadeDuration(base: number): number {
    return props.reducedMotion ? 0 : base;
  }
</script>

<div
  class="album-detail-card"
  in:fade={{ duration: fadeDuration(180) }}
  out:fade={{ duration: fadeDuration(180) }}
>
  <div class="album-hero">
    <div
      class="album-hero-info"
      in:fade={{
        duration: fadeDuration(220),
        delay: props.reducedMotion ? 0 : 30,
      }}
      out:fade={{ duration: fadeDuration(220) }}
    >
      <MotionPulseBlock
        className="album-hero-title loading-text"
        reducedMotion={props.reducedMotion}
      />
      <MotionPulseBlock
        className="album-hero-sub loading-text-sub"
        reducedMotion={props.reducedMotion}
        delay={0.14}
      />
    </div>
  </div>
  <div
    class="loading album-loading"
    in:fade={{
      duration: fadeDuration(200),
      delay: props.reducedMotion ? 0 : 70,
    }}
    out:fade={{ duration: fadeDuration(200) }}
  >
    <span>正在加载歌曲...</span><MotionSpinner
      className="inline-loading-spinner"
      reducedMotion={props.reducedMotion}
    />
  </div>
</div>
