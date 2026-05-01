import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import fs from "fs";
import path from "path";

const host = process.env.TAURI_DEV_HOST;

// Path to project-root templates/
const templatesDir = path.resolve(__dirname, "..", "templates");

export default defineConfig({
  plugins: [
    vue(),
    tailwindcss(),
    {
      name: "serve-templates",
      configureServer(server) {
        // GET /api/templates          → template list [{ id, name, description }]
        // GET /api/templates/:id       → full template JSON
        server.middlewares.use("/api/templates", (req, res) => {
          // req.url is the original full path, e.g. "/api/templates" or "/api/templates/ai-summarize"
          const url = (req.url || "").split("?")[0];
          const templateId = url.replace("/api/templates", "").replace(/^\/+/, "");

          if (!templateId) {
            // List all templates
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
            // Get specific template by id (templateId already extracted above)
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
  ],

  clearScreen: false,
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
