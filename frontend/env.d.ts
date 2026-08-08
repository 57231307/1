/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue';
  const component: DefineComponent<Record<string, never>, Record<string, never>, any>;
  export default component;
}

// batch-20 P3: 环境变量类型声明
interface ImportMetaEnv {
  readonly VITE_API_BASE_URL: string;
  readonly VITE_API_TIMEOUT: string;
  readonly VITE_DEV_SERVER_PORT: string;
  readonly VITE_DEV_SERVER_HOST: string;
  readonly VITE_USE_MOCK: string;
  readonly VITE_MOCK_DELAY: string;
  readonly VITE_DEBUG: string;
  readonly VITE_SHOW_ERROR_DETAILS: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
