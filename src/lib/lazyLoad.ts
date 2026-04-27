import { getImageDataUrl } from './api';

export function lazyLoad(
  node: HTMLElement,
  {
    rootMargin = '150px',
    reducedMotion = false,
  }: { rootMargin?: string; reducedMotion?: boolean } = {}
) {
  let imageAnimation: Animation | null = null;
  let placeholderAnimation: Animation | null = null;
  let loadSeq = 0;

  const stopAnimations = () => {
    imageAnimation?.cancel();
    placeholderAnimation?.cancel();
  };

  const observer = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (!entry.isIntersecting) {
          return;
        }

        const src = node.dataset.src;
        if (!src) {
          observer.unobserve(node);
          return;
        }

        const img = node.querySelector('img');
        const placeholder = node.querySelector<HTMLElement>(
          '.album-cover-placeholder'
        );
        if (!img) {
          observer.unobserve(node);
          return;
        }

        const seq = ++loadSeq;

        void (async () => {
          try {
            const resolvedSrc = await getImageDataUrl(src);
            if (seq !== loadSeq) return;

            img.style.opacity = '0';
            img.style.transform = reducedMotion ? 'scale(1)' : 'scale(1.04)';
            img.onload = () => {
              stopAnimations();
              const duration = reducedMotion ? 0 : 180;
              if (placeholder) {
                placeholderAnimation = placeholder.animate(
                  [
                    { opacity: getComputedStyle(placeholder).opacity },
                    { opacity: '0' },
                  ],
                  { duration, easing: 'ease-out', fill: 'forwards' }
                );
              }
              imageAnimation = img.animate(
                [
                  { opacity: '0', transform: img.style.transform },
                  { opacity: '1', transform: 'scale(1)' },
                ],
                {
                  duration: reducedMotion ? 0 : 200,
                  easing: 'ease-out',
                  fill: 'forwards',
                }
              );
            };
            img.onerror = () => {
              stopAnimations();
              if (placeholder) {
                placeholder.style.opacity = '1';
              }
            };
            img.src = resolvedSrc;
          } catch {
            stopAnimations();
            if (placeholder) {
              placeholder.style.opacity = '1';
            }
          } finally {
            if (seq === loadSeq) {
              node.removeAttribute('data-src');
              observer.unobserve(node);
            }
          }
        })();
      });
    },
    { rootMargin, threshold: 0 }
  );

  observer.observe(node);

  return {
    update(next: { rootMargin?: string; reducedMotion?: boolean } = {}) {
      reducedMotion = next.reducedMotion ?? false;
    },
    destroy() {
      stopAnimations();
      observer.disconnect();
    },
  };
}
