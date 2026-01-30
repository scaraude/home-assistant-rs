export type BadgeVariant = "good" | "medium" | "low";

export function getLqiBadgeVariant(lqi: number): BadgeVariant {
  if (lqi >= 100) return "good";
  if (lqi >= 50) return "medium";
  return "low";
}
