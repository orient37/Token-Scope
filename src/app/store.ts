import { create } from "zustand";
export type Page = "overview" | "sessions" | "timeline" | "alerts" | "settings";
export const useUI = create<{ page: Page; setPage: (page: Page) => void }>(
  (set) => ({ page: "overview", setPage: (page) => set({ page }) }),
);
