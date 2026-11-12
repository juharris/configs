import { get } from "node:http";
import { setTimeout as delay } from "node:timers/promises";
import react from "@vitejs/plugin-react";
import { createLogger, defineConfig } from "vite";

const backendOrigin = "http://127.0.0.1:3000";
const logger = createLogger();
const logError = logger.error;

logger.error = (message, options) => {
  const code = (options?.error as NodeJS.ErrnoException | undefined)?.code;
  // Backend diagnostics explain the failure while the browser retries.
  // Repeated bootstrap requests during shutdown must not fill the terminal with proxy stacks.
  if (
    (code === "ECONNREFUSED" || code === "ECONNRESET") &&
    message.includes("http proxy error: /bootstrap")
  ) {
    return;
  }
  logError(message, options);
};

export default defineConfig({
  customLogger: logger,
  plugins: [
    react(),
    {
      name: "wait-for-backend",
      async configureServer(server) {
        server.config.logger.info(
          `Waiting for Personal Dashboard at ${backendOrigin} before starting Vite.`,
        );
        while (true) {
          const ready = await new Promise<boolean>((resolve, reject) => {
            const request = get(
              `${backendOrigin}/bootstrap`,
              { headers: { Host: "127.0.0.1:5173" }, timeout: 1000 },
              (response) => {
                response.resume();
                if (response.statusCode === 200) {
                  resolve(true);
                } else {
                  reject(
                    new Error(
                      `Personal Dashboard returned HTTP ${response.statusCode} at ${backendOrigin}/bootstrap.`,
                    ),
                  );
                }
              },
            );
            request.on("error", () => resolve(false));
            request.on("timeout", () => request.destroy());
          });
          if (ready) return;
          // The backend owns startup deadlines and recovery diagnostics.
          // concurrently stops Vite if that process exits.
          await delay(100);
        }
      },
    },
  ],
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
