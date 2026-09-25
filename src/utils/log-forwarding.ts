import { api } from "../api";

function describe(value: unknown): string {
  if (value instanceof Error) return value.stack ?? `${value.name}: ${value.message}`;
  if (typeof value === "string") return value;
  try {
    return JSON.stringify(value);
  } catch {
    return String(value);
  }
}

function forward(level: "warn" | "error", message: string) {
  // Never log a failure to log, or a broken backend would loop forever.
  api.logFrontend(level, message).catch(() => {});
}

/** Sends uncaught errors and console warnings/errors to the backend log file. */
export function installLogForwarding() {
  window.addEventListener("error", (e) => {
    forward("error", `${e.message} (${e.filename}:${e.lineno}:${e.colno})`);
  });
  window.addEventListener("unhandledrejection", (e) => {
    forward("error", `unhandled rejection: ${describe(e.reason)}`);
  });

  for (const level of ["warn", "error"] as const) {
    const original = console[level].bind(console);
    console[level] = (...args: unknown[]) => {
      original(...args);
      forward(level, args.map(describe).join(" "));
    };
  }
}
