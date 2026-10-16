export type CheckpointAuthorityOptions = {
  scope?: () => string;
  captureAuthority?: () => () => boolean;
};

export function captureCheckpointAuthority(options: CheckpointAuthorityOptions) {
  const readScope = options.scope ?? (() => "");
  const scope = readScope(), isCurrent = options.captureAuthority?.() ?? (() => true);
  let retired = false;
  const check = () => {
    try {
      if (retired || readScope() !== scope || !isCurrent()) throw new Error("checkpoint:context_changed");
    } catch (error) { retired = true; throw error; }
  };
  return { scope, check };
}
