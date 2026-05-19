import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import fs from "fs";
import path from "path";
import { fileURLToPath } from "url";

const host = process.env.TAURI_DEV_HOST;

// Path to project-root templates/
const templatesDir = path.resolve(__dirname, "templates");

export default defineConfig({
  plugins: [
    vue(),
    tailwindcss(),
    {
      name: "serve-templates",
      configureServer(server) {
        server.middlewares.use("/api/templates", (req, res) => {
          const url = (req.url || "").split("?")[0];
          const templateId = url.replace("/api/templates", "").replace(/^\/+/, "");

          // Prevent path traversal attacks
          if (templateId.includes('/') || templateId.includes('\\') || templateId.includes('..') || templateId.includes('~')) {
            res.statusCode = 400;
            res.end("Invalid template ID");
            return;
          }

          if (!templateId) {
            const files = fs.readdirSync(templatesDir).filter(f => f.endsWith(".json"));
            const list = files.map(f => {
              const id = f.replace(".json", "");
              try {
                const raw = fs.readFileSync(path.join(templatesDir, f), "utf-8");
                const json = JSON.parse(raw);
                return { id, name: json.name || id, description: json.description || "" };
              } catch {
                return { id, name: id, description: "" };
              }
            });
            res.setHeader("Content-Type", "application/json");
            res.end(JSON.stringify(list));
          } else {
            const filePath = path.join(templatesDir, `${templateId}.json`);
            if (!fs.existsSync(filePath)) {
              res.statusCode = 404;
              res.end("Template not found");
              return;
            }
            res.setHeader("Content-Type", "application/json");
            res.end(fs.readFileSync(filePath, "utf-8"));
          }
        });
      },
    },
    // Resolve @tauri-apps/api .ts sources (package ships .ts only, no .js)
    {
      name: "resolve-tauri-api",
      enforce: "pre",
      resolveId(source) {
        if (source.startsWith("@tauri-apps/api/")) {
          const subpath = source.replace("@tauri-apps/api/", "");
          const tsPath = path.resolve(__dirname, "node_modules/@tauri-apps/api/src", subpath + ".ts");
          if (fs.existsSync(tsPath)) return tsPath;
        }
        return null;
      },
    },
  ],

  resolve: {
    alias: {
      // 使用 runtime-only 构建，避免 CSP unsafe-eval
      'vue': 'vue/dist/vue.runtime.esm-bundler.js',
      // vue-i18n runtime 构建不含 message compiler (new Function)，兼容 CSP
      'vue-i18n': 'vue-i18n/dist/vue-i18n.runtime.mjs',
      '@': path.resolve(__dirname, "src"),
    },
  },
  clearScreen: false,
  build: {
    outDir: "./dist",
    emptyOutDir: true,
  },
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
});
