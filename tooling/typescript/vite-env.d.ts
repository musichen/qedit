/**
 * Ambient typings for Vite's `import.meta.env`, shared across every workspace
 * package via `@qedit/tsconfig`'s `base.json` (`compilerOptions.types`).
 */
interface ImportMetaEnv {
  // Vite built-ins
  readonly MODE: string;
  readonly BASE_URL: string;
  readonly PROD: boolean;
  readonly DEV: boolean;
  readonly SSR: boolean;

  // App
  readonly VITE_PRODUCT_NAME?: string;
  readonly VITE_SITE_URL?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
