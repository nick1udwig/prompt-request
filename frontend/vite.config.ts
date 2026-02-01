import { defineConfig } from "vite";

export default defineConfig(({ command }) => ({
  // Use /h/ for production builds, but avoid dev-server redirecting / -> /h/.
  base: command === "build" ? "/h/" : "/"
}));
