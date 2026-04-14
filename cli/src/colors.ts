/**
 * Tiny zero-dependency ANSI color helper.
 *
 * Honours `NO_COLOR` (https://no-color.org) and disables itself when stdout is
 * not a TTY, so piping into files / pagers stays clean.
 */

const enabled =
  process.env.NO_COLOR === undefined &&
  process.env.TERM !== "dumb" &&
  Boolean(process.stdout.isTTY);

function wrap(open: number, close: number) {
  return (s: string | number): string =>
    enabled ? `\x1b[${open}m${s}\x1b[${close}m` : String(s);
}

export const c = {
  enabled,
  bold: wrap(1, 22),
  dim: wrap(2, 22),
  italic: wrap(3, 23),
  underline: wrap(4, 24),
  red: wrap(31, 39),
  green: wrap(32, 39),
  yellow: wrap(33, 39),
  blue: wrap(34, 39),
  magenta: wrap(35, 39),
  cyan: wrap(36, 39),
  gray: wrap(90, 39),
};

/** Color a log level / status string by its semantic. */
export function colorLevel(level: string): string {
  switch (level.toLowerCase()) {
    case "ok":
    case "done":
      return c.green(level);
    case "info":
      return c.blue(level);
    case "decision":
    case "shipping":
      return c.magenta(level);
    case "warn":
    case "queued":
      return c.yellow(level);
    case "error":
      return c.red(level);
    default:
      return c.gray(level);
  }
}
