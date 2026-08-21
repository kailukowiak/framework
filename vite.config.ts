import { defineConfig, configDefaults } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: "127.0.0.1",
  },
  test: {
    // Agent worktrees under .claude/worktrees/ are complete checkouts of this
    // repository, so vitest's default discovery walks into them and runs a
    // second copy of the suite from whatever branch happens to be checked out
    // there. That copy is not the code under test: it inflates the counts,
    // roughly doubles the run, and can fail — or pass — for reasons that have
    // nothing to do with the working tree. `npm test` is the verification
    // command AGENTS.md points at, so it has to mean one unambiguous thing.
    exclude: [...configDefaults.exclude, "**/.claude/**"],
  },
});
