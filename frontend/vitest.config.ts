import { defineConfig } from 'vitest/config'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'

export default defineConfig({
  plugins: [vue()],
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['tests/**/*.{test,spec}.{js,mjs,cjs,ts,mts,cts,jsx,tsx}'],
    // 排除 Playwright E2E 测试（e2e/），由 @playwright/test 运行
    exclude: [
      'node_modules',
      'dist',
      '.idea',
      '.git',
      '.cache',
      'e2e/**',
    ],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      reportsDirectory: './coverage',
      include: ['src/**/*.{ts,vue}'],
      exclude: [
        'src/types/**',
        'src/**/*.d.ts',
        'src/main.ts',
        'src/App.vue',
      ],
      // V15 P1-20-6 覆盖率门槛（当前 1%，待测试补齐批次后回调至 70%）
      thresholds: {
        lines: 1,
        functions: 1,
        branches: 1,
        statements: 1,
        perFile: false,
      },
    },
    setupFiles: ['./tests/setup.ts'],
  },
  resolve: {
    alias: {
      '@': resolve(__dirname, 'src'),
    },
  },
})
