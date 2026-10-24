import { useEffect, useState } from "react";
import type { Reading } from "./near";

export type Snapshot = { block: number; checked: number; backed: number; under_backed: number; errors: number; readings: Reading[] };

let cache: Promise<Snapshot | null> | null = null;

/** The last full intents.near scan (refreshed by CI), loaded once per visit. */
export function useSnapshot(): Snapshot | null {
  const [snap, setSnap] = useState<Snapshot | null>(null);
  useEffect(() => {
    cache ??= fetch("./intents-snapshot.json")
      .then((r) => r.json() as Promise<Snapshot>)
      .catch(() => null);
    let live = true;
    cache.then((s) => live && setSnap(s));
    return () => {
      live = false;
    };
  }, []);
  return snap;
}
