import js from "@eslint/js";
import tseslint from "typescript-eslint";
import pluginVue from "eslint-plugin-vue";

export default tseslint.config(
  // Global ignores
  { ignores: ["dist/**", "node_modules/**", "src-tauri/**"] },

  // Base JS recommended rules
  js.configs.recommended,

  // TypeScript recommended rules
  ...tseslint.configs.recommended,

  // Vue 3 recommended rules (flat config)
  ...pluginVue.configs["flat/recommended"],

  // Custom overrides
  {
    files: ["src/**/*.{ts,tsx,vue}", "*.{ts,js}"],
    rules: {
      // Disallow console.log — use proper logging instead
      "no-console": ["warn", { allow: ["warn", "error"] }],

      // Vue overrides for TypeScript
      "vue/multi-word-component-names": "off",
      "vue/require-default-prop": "off",
      "vue/no-v-html": "warn",

      // TypeScript relaxations for practical development
      "@typescript-eslint/no-explicit-any": "warn",
      "@typescript-eslint/no-unused-vars": [
        "warn",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
    },
  },
);
