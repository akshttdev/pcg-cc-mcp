// vite.config.ts
import { defineConfig, Plugin } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";
import fs from "fs";
import { sentryVitePlugin } from "@sentry/vite-plugin";

function executorSchemasPlugin(): Plugin {
  const VIRTUAL_ID = "virtual:executor-schemas";
  const RESOLVED_VIRTUAL_ID = "\0" + VIRTUAL_ID;

  return {
    name: "executor-schemas-plugin",
    resolveId(id) {
      if (id === VIRTUAL_ID) return RESOLVED_VIRTUAL_ID; // keep it virtual
      return null;
    },
    load(id) {
      if (id !== RESOLVED_VIRTUAL_ID) return null;

      const schemasDir = path.resolve(__dirname, "../shared/schemas");
      const files = fs.existsSync(schemasDir)
        ? fs.readdirSync(schemasDir).filter((f) => f.endsWith(".json"))
        : [];

      const imports: string[] = [];
      const entries: string[] = [];

      files.forEach((file, i) => {
        const varName = `__schema_${i}`;
        const importPath = `shared/schemas/${file}`; // uses your alias
        const key = file.replace(/\.json$/, "").toUpperCase(); // claude_code -> CLAUDE_CODE
        imports.push(`import ${varName} from "${importPath}";`);
        entries.push(`  "${key}": ${varName}`);
      });

      // IMPORTANT: pure JS (no TS types), and quote keys.
      const code = `
${imports.join("\n")}

export const schemas = {
${entries.join(",\n")}
};

export default schemas;
`;
      return code;
    },
  };
}

const plugins: Plugin[] = [react(), executorSchemasPlugin()];

function getToposProjectDirectories() {
  const toposRoot = path.resolve(__dirname, "..", "..");
  if (!fs.existsSync(toposRoot)) {
    return [] as string[];
  }
  const entries = fs.readdirSync(toposRoot, { withFileTypes: true });
  return entries
    .filter((entry) => entry.isDirectory() && !entry.name.startsWith("."))
    .map((entry) => entry.name)
    .sort((a, b) => a.localeCompare(b));
}

const toposDirectories = getToposProjectDirectories();

const sentryIsExplicitlyDisabled = process.env.DISABLE_SENTRY?.toLowerCase() === "true";
const sentryHasCredentials = Boolean(
  process.env.SENTRY_AUTH_TOKEN && process.env.SENTRY_ORG && process.env.SENTRY_PROJECT
);

if (!sentryIsExplicitlyDisabled && sentryHasCredentials) {
  plugins.splice(1, 0, sentryVitePlugin({ org: process.env.SENTRY_ORG!, project: process.env.SENTRY_PROJECT! }));
} else {
  console.warn(
    "Skipping Sentry Vite plugin (set DISABLE_SENTRY=false and provide SENTRY_* env vars to enable)."
  );
}

export default defineConfig({
  plugins,
  define: {
    __TOPOS_PROJECTS__: JSON.stringify(toposDirectories),
  },
  resolve: {
    alias: {
      "@": path.resolve(__dirname, "./src"),
      shared: path.resolve(__dirname, "../shared"),
    },
  },
  server: {
    host: '0.0.0.0', // Listen on all network interfaces
    port: parseInt(process.env.FRONTEND_PORT || "3000"),
    allowedHosts: true, // Allow requests from any host
    proxy: {
      "/api": {
        target: `http://localhost:${process.env.BACKEND_PORT || "58297"}`,
        changeOrigin: true,
        ws: true,
      },
    },
    fs: {
      allow: [path.resolve(__dirname, "."), path.resolve(__dirname, "..")],
    },
    open: process.env.VITE_OPEN === "true",
  },
  build: { sourcemap: true },
});
