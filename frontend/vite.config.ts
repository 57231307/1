import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import AutoImport from 'unplugin-auto-import/vite';
import Components from 'unplugin-vue-components/vite';
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers';
import { visualizer } from 'rollup-plugin-visualizer';
import { resolve } from 'path';
import { readFileSync } from 'fs';

// 后端代理目标：默认 8082（常规后端/CI 34 分片）；Setup 向导真实链路 E2E
// 用独立后端实例（8083，专用空库），通过 VITE_PROXY_TARGET 环境变量切换。
// 测试端口同理：VITE_DEV_PORT 覆盖 dev server 监听端口，避免与常规 E2E 冲突
const PROXY_TARGET = process.env.VITE_PROXY_TARGET || 'http://localhost:8082';
const DEV_PORT = Number(process.env.VITE_DEV_PORT || 3000);

// 摄像头 getUserMedia 仅在安全上下文（HTTPS 或 localhost）可用：HTTP 域名下桌面/手机浏览器
// 均不会弹权限，navigator.mediaDevices 直接 undefined，扫码功能不可用。这里提供可选 HTTPS，
// 由 VITE_USE_HTTPS=1 门控，默认关闭 —— CI 的 build + preview E2E 不设该变量，维持 http/localhost，
// 不受影响。
// 启用 HTTPS 的两种方式（择一，二者均不在仓库硬编码证书路径）：
//  (A) 依赖无侵入：用 mkcert 生成合法自签证书，并以环境变量提供文件路径：
//      VITE_HTTPS_KEY=<key.pem 路径> VITE_HTTPS_CERT=<cert.pem 路径> VITE_USE_HTTPS=1 npm run dev
//  (B) 使用 @vitejs/plugin-basic-ssl 自动注入自签证书（该插件当前不在依赖树，启用需先
//      `npm i -D @vitejs/plugin-basic-ssl` 并把 basicSsl() 加入下方 plugins —— 属待用户确认项，
//      此处不擅自安装）。选 (B) 时不设 VITE_HTTPS_KEY/CERT，https 传 true，由插件提供证书。
// 若 VITE_USE_HTTPS=1 却既无证书也未装插件，Vite 会因缺少证书显式报错（不静默降级回 http）。
const USE_HTTPS = process.env.VITE_USE_HTTPS === '1';
const HTTPS_KEY = process.env.VITE_HTTPS_KEY;
const HTTPS_CERT = process.env.VITE_HTTPS_CERT;

function buildHttpsOption(): false | true | { key: Buffer; cert: Buffer } {
  if (!USE_HTTPS) return false;
  if (HTTPS_KEY && HTTPS_CERT) {
    return {
      key: readFileSync(resolve(__dirname, HTTPS_KEY)),
      cert: readFileSync(resolve(__dirname, HTTPS_CERT)),
    };
  }
  // 交由 @vitejs/plugin-basic-ssl 注入证书（需按上文 (B) 装插件并加入 plugins）
  return true;
}
const HTTPS_OPTION = buildHttpsOption();

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
    }),
    Components({
      resolvers: [ElementPlusResolver()],
    }),
    visualizer({ open: false, gzipSize: true, brotliSize: true }),
  ],
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
  server: {
    port: DEV_PORT,
    https: HTTPS_OPTION,
    allowedHosts: ['.monkeycode-ai.online'],
    proxy: {
      '/api/': {
        target: PROXY_TARGET,
        changeOrigin: true,
        // WebSocket 升级转发（/ws/notifications 通知通道）
        ws: true,
      },
    },
  },
  // vite preview（生产构建本地服务）复用同一 proxy 配置：
  // CI E2E 已切换为 build + preview（规避 dev server 按需编译挂起）
  preview: {
    port: DEV_PORT,
    https: HTTPS_OPTION,
    allowedHosts: ['.monkeycode-ai.online'],
    proxy: {
      '/api/': {
        target: PROXY_TARGET,
        changeOrigin: true,
        ws: true,
      },
    },
  },
  build: {
    outDir: 'dist',
    assetsDir: 'static',
    sourcemap: false,
    target: 'esnext',
    // V15 P1-20-3 chunk 分割策略：将大依赖拆分为独立 chunk，优化首屏加载
    chunkSizeWarningLimit: 1000,
    rollupOptions: {
      output: {
        // Vite 8 (Rolldown) 不支持对象格式 manualChunks，改为函数
        manualChunks(id: string) {
          if (id.includes('node_modules')) {
            if (id.includes('vue') || id.includes('vue-router') || id.includes('vue-i18n') || id.includes('pinia')) {
              return 'vue-vendor';
            }
            if (id.includes('element-plus') || id.includes('@element-plus/icons-vue')) {
              return 'element-plus';
            }
            if (id.includes('echarts')) {
              return 'echarts-vendor';
            }
            if (id.includes('axios')) {
              return 'utils-vendor';
            }
          }
        },
      },
    },
  },
  // V15 P1-20-3 预构建依赖优化（减少冷启动时间）
  optimizeDeps: {
    include: [
      'vue',
      'vue-router',
      'vue-i18n',
      'pinia',
      'element-plus',
      '@element-plus/icons-vue',
      'axios',
      'dayjs',
      'echarts/core',
      'echarts/charts',
      'echarts/components',
      'echarts/renderers',
    ],
    exclude: ['@playwright/test'],
  },
});
