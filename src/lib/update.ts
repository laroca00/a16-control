import { useCallback, useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

const LATEST_URL = "https://api.github.com/repos/laroca00/a16-control/releases/latest";
const AUTO_KEY = "gienemy.updateAuto";
const LAST_KEY = "gienemy.updateLast";
/** The start-up check runs at most this often. */
const CHECK_EVERY_MS = 24 * 60 * 60 * 1000;

export type Release = { version: string; url: string };

export type UpdateState =
  | { status: "idle" | "checking" | "latest" }
  | { status: "available"; release: Release }
  | { status: "error"; error: string };

/** Compares dotted versions number by number; a leading "v" is ignored. */
export function compareVersions(a: string, b: string): number {
  const parts = (v: string) => v.replace(/^v/i, "").split(/[.-]/).map((p) => parseInt(p, 10) || 0);
  const [x, y] = [parts(a), parts(b)];
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0);
    if (d !== 0) return Math.sign(d);
  }
  return 0;
}

export async function fetchLatest(): Promise<Release> {
  const res = await fetch(LATEST_URL, { headers: { Accept: "application/vnd.github+json" } });
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const json = (await res.json()) as { tag_name?: unknown; html_url?: unknown };
  if (typeof json.tag_name !== "string" || typeof json.html_url !== "string") throw new Error("unexpected reply");
  return { version: json.tag_name.replace(/^v/i, ""), url: json.html_url };
}

export function autoCheckEnabled(): boolean {
  return localStorage.getItem(AUTO_KEY) !== "0";
}

export function setAutoCheck(on: boolean) {
  localStorage.setItem(AUTO_KEY, on ? "1" : "0");
}

type Last = { at: number; release: Release };

function readLast(): Last | null {
  try {
    return JSON.parse(localStorage.getItem(LAST_KEY) ?? "null") as Last | null;
  } catch {
    return null;
  }
}

function stateFor(current: string, release: Release): UpdateState {
  return compareVersions(release.version, current) > 0 ? { status: "available", release } : { status: "latest" };
}

/**
 * The running version and what GitHub says about the newest one. With `auto`,
 * checks once a day when the app starts and otherwise reuses the last answer.
 */
export function useUpdate({ auto }: { auto: boolean }) {
  const [current, setCurrent] = useState<string | null>(null);
  const [state, setState] = useState<UpdateState>({ status: "idle" });

  const check = useCallback(async (version: string) => {
    setState({ status: "checking" });
    try {
      const release = await fetchLatest();
      localStorage.setItem(LAST_KEY, JSON.stringify({ at: Date.now(), release } satisfies Last));
      setState(stateFor(version, release));
    } catch (e) {
      setState({ status: "error", error: e instanceof Error ? e.message : String(e) });
    }
  }, []);

  useEffect(() => {
    let alive = true;
    // Outside the app (tests, a plain browser) there is no version to compare.
    getVersion()
      .then((version) => {
        if (!alive) return;
        setCurrent(version);
        if (!auto) return;
        const last = readLast();
        if (last && Date.now() - last.at < CHECK_EVERY_MS) setState(stateFor(version, last.release));
        else if (autoCheckEnabled()) void check(version);
      })
      .catch(() => {});
    return () => {
      alive = false;
    };
  }, [auto, check]);

  return { current, state, check: () => (current ? check(current) : Promise.resolve()) };
}
