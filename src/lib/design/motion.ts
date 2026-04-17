export const motionDuration = {
  fast: 140,
  base: 180,
  slow: 260,
} as const;

export const motionEase = {
  standard: [0.2, 0, 0, 1],
  decelerate: [0.16, 1, 0.3, 1],
} as const;
