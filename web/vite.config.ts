import { defineConfig } from "vite";

// The API is a separate process on :8080. Proxying keeps the frontend on one
// origin, so EventSource and fetch need no CORS round trip and no absolute
// URLs baked into the build.
export default defineConfig({
  server: {
    port: 5173,
    proxy: {
      "/health": "http://127.0.0.1:8080",
      "/events": { target: "http://127.0.0.1:8080", changeOrigin: true },
      "/inbound-sms": "http://127.0.0.1:8080",
      "/transfers": "http://127.0.0.1:8080",
      "/transactions": "http://127.0.0.1:8080",
    },
  },
});
