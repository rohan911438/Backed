// Hash routes, so the site works on any static host (Pages, Vercel, a file).
//   #/            home
//   #/app         dashboard
//   #/app/<page>  checker | replays | build

import { useEffect, useState } from "react";

export type Page = "dashboard" | "checker" | "replays" | "build";
export type Route = { name: "home" } | { name: "app"; page: Page };

const PAGES: Page[] = ["dashboard", "checker", "replays", "build"];

export function parse(hash: string): Route {
  const parts = hash.replace(/^#\/?/, "").split(/[/?]/);
  if (parts[0] !== "app") return { name: "home" };
  const page = (PAGES as string[]).includes(parts[1] ?? "") ? (parts[1] as Page) : "dashboard";
  return { name: "app", page };
}

export function href(route: Route): string {
  return route.name === "home" ? "#/" : route.page === "dashboard" ? "#/app" : `#/app/${route.page}`;
}

export function navigate(route: Route) {
  window.location.hash = href(route);
  window.scrollTo({ top: 0 });
}

export function useRoute(): Route {
  const [route, setRoute] = useState(() => parse(window.location.hash));
  useEffect(() => {
    const on = () => setRoute(parse(window.location.hash));
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  return route;
}

/** Transaction hashes a browser wallet appends to the URL after signing. */
export function takeReturnedTxHashes(): string[] {
  const url = new URL(window.location.href);
  const fromQuery = url.searchParams.get("transactionHashes");
  const fromHash = new URLSearchParams(url.hash.split("?")[1] ?? "").get("transactionHashes");
  const raw = fromQuery ?? fromHash;
  if (!raw) return [];
  url.searchParams.delete("transactionHashes");
  url.searchParams.delete("errorCode");
  url.searchParams.delete("errorMessage");
  url.hash = url.hash.split("?")[0];
  window.history.replaceState(null, "", url.toString());
  return raw.split(",").filter(Boolean);
}
