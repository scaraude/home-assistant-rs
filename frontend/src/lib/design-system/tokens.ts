export const colors = {
  text: "#111827",
  textStrong: "#374151",
  textMuted: "#6b7280",
  textSubtle: "#9ca3af",
  background: "#f3f4f6",
  surfaceSoft: "#f3f4f6",
  surfaceMuted: "#f9fafb",
  cardBg: "#ffffff",
  cardBorder: "#e5e7eb",
  cardBorderHover: "#d1d5db",
  cardActive: "#3b82f6",
  cardActiveBg: "#eff6ff",
  badgeNeutralBg: "rgba(107, 114, 128, 0.1)",
  badgeNeutralBorder: "rgba(107, 114, 128, 0.2)",
  badgeNeutralText: "#6b7280",
  badgeSuccessBg: "rgba(34, 197, 94, 0.1)",
  badgeSuccessBorder: "rgba(34, 197, 94, 0.3)",
  badgeSuccessText: "#16a34a",
  badgeWarningBg: "rgba(251, 191, 36, 0.1)",
  badgeWarningBorder: "rgba(251, 191, 36, 0.3)",
  badgeWarningText: "#d97706",
  badgeDangerBg: "rgba(239, 68, 68, 0.1)",
  badgeDangerBorder: "rgba(239, 68, 68, 0.3)",
  badgeDangerText: "#dc2626",
} as const;

export const spacing = {
  xs: "0.25rem",
  sm: "0.5rem",
  md: "0.75rem",
  mdPlus: "0.875rem",
  lg: "1rem",
  xl: "1.25rem",
  xxl: "1.5rem",
  xxxl: "2rem",
} as const;

export const radii = {
  xs: "3px",
  sm: "6px",
  md: "8px",
  lg: "12px",
  pill: "999px",
} as const;

export const shadows = {
  xs: "0 1px 2px rgba(0, 0, 0, 0.05)",
  sm: "0 2px 4px rgba(0, 0, 0, 0.05)",
  md: "0 4px 6px rgba(0, 0, 0, 0.1)",
  lg: "0 10px 15px rgba(0, 0, 0, 0.1)",
  active: "0 2px 8px rgba(59, 130, 246, 0.15)",
} as const;

export const transitions = {
  fast: "0.15s ease",
  base: "0.2s ease",
} as const;
