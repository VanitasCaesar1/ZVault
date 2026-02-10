import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  base: "/app/",
  server: {
    port: 5173,
    proxy: {
      "/v1": "http://localhost:8200",
      "/auth": "http://localhost:8200",
    },
  },
  build: {
    outDir: "dist",
    emptyOutDir: true,
  },
});
