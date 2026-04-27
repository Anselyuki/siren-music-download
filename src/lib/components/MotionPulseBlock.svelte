<script lang="ts">
  interface Props {
    className?: string;
    reducedMotion?: boolean;
    delay?: number;
    duration?: number;
    style?: string;
  }

  let {
    className = '',
    reducedMotion = false,
    delay = 0,
    duration = 1.8,
    style = '',
  }: Props = $props();
</script>

<div
  class={className}
  {style}
  style:animation={reducedMotion
    ? 'none'
    : `motion-pulse ${duration}s ease-in-out ${delay}s infinite`}
  style:opacity={reducedMotion ? 1 : undefined}
></div>

<style>
  @keyframes motion-pulse {
    0%,
    100% {
      opacity: 0.46;
    }
    50% {
      opacity: 0.92;
    }
  }
</style>
