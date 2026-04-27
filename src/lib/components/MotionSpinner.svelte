<script lang="ts">
  interface Props {
    className?: string;
    reducedMotion?: boolean;
    duration?: number;
    style?: string;
  }

  let {
    className = '',
    reducedMotion = false,
    duration = 0.9,
    style = '',
  }: Props = $props();
</script>

<div
  class={className}
  {style}
  style:animation={reducedMotion
    ? 'none'
    : `motion-spin ${duration}s linear infinite`}
></div>

<style>
  @keyframes motion-spin {
    from {
      transform: rotate(0deg);
    }
    to {
      transform: rotate(360deg);
    }
  }
</style>
