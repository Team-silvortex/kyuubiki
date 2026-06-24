export function mergeLanguagePack<T extends Record<string, unknown>>(
  base: T,
  overrides?: Record<string, unknown> | null,
): T {
  if (!overrides) return base;

  const mergeValue = (left: unknown, right: unknown): unknown => {
    if (right === undefined) return left;
    if (Array.isArray(right)) {
      return left === undefined || Array.isArray(left) ? right.slice() : left;
    }
    if (
      left &&
      right &&
      typeof left === "object" &&
      typeof right === "object" &&
      !Array.isArray(left) &&
      !Array.isArray(right)
    ) {
      const result: Record<string, unknown> = { ...(left as Record<string, unknown>) };
      for (const [key, value] of Object.entries(right as Record<string, unknown>)) {
        result[key] = mergeValue(result[key], value);
      }
      return result;
    }
    if (left && typeof left === "object" && !Array.isArray(left)) return left;
    if (right && typeof right === "object" && !Array.isArray(right) && left !== undefined) return left;
    return right;
  };

  return mergeValue(base, overrides) as T;
}
