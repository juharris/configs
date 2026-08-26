import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const backendOrigin = "http://127.0.0.1:3000";

export default defineConfig({
  plugins: [react()],
  server: {
    host: "127.0.0.1",
    open: true,
    port: 5173,
    proxy: {
      "/bootstrap": {
        target: backendOrigin,
      },
      "/ws": {
        target: backendOrigin,
        ws: true,
      },
    },
    strictPort: true,
  },
});
