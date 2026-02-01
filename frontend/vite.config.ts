import { defineConfig } from "vite";

export default defineConfig(({ command }) => {
  const isBuild = command === "build";

  return {
    // Use /h/ for production builds, but keep dev at / so we can proxy / to the backend.
    base: isBuild ? "/h/" : "/",
    server: isBuild
      ? undefined
      : {
          proxy: {
            "^/(?!h(?:/|$)|@|__vite|src/|node_modules/|assets/).*": {
              target: "http://localhost:3000",
              changeOrigin: true
            }
          }
        }
  };
});
