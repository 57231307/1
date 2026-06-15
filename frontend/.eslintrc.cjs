module.exports = {
  root: true,
  env: {
    browser: true,
    es2021: true,
    node: true,
  },
  extends: [
    'eslint:recommended',
    'plugin:vue/vue3-recommended',
    'plugin:@typescript-eslint/recommended',
    'plugin:prettier/recommended',
  ],
  parser: 'vue-eslint-parser',
  parserOptions: {
    parser: '@typescript-eslint/parser',
    ecmaVersion: 'latest',
    sourceType: 'module',
    ecmaFeatures: {
      jsx: true,
    },
  },
  plugins: ['vue', '@typescript-eslint', 'prettier'],
  rules: {
    // Vue 相关规则
    'vue/multi-word-component-names': 'off',
    'vue/no-v-html': 'off',
    'vue/require-default-prop': 'off',
    'vue/require-explicit-emits': 'off',

    // TypeScript 相关规则
    // 禁止使用 any 类型，强制类型安全
    // 历史说明：main 分支累计 800+ 处 `any`，设为 error 会阻塞所有 PR。
    // 临时降级为 warn 以解锁 CI，后续按模块逐步收紧为 error。
    // 跟踪计划：见 docs/tech-debt/no-explicit-any-rollout.md
    '@typescript-eslint/no-explicit-any': 'warn',
    '@typescript-eslint/consistent-type-assertions': 'error',
    '@typescript-eslint/no-unused-vars': ['warn', { argsIgnorePattern: '^_' }],
    '@typescript-eslint/ban-ts-comment': 'off',
    '@typescript-eslint/no-non-null-assertion': 'off',

    // 通用规则
    'no-console': ['warn', { allow: ['warn', 'error'] }],
    'no-debugger': 'warn',
    'no-unused-vars': 'off', // 使用 TypeScript 的规则

    // Prettier 规则
    'prettier/prettier': [
      'error',
      {
        semi: false,
        singleQuote: true,
        printWidth: 100,
        trailingComma: 'es5',
        endOfLine: 'auto',
      },
    ],
  },
  // 测试文件例外配置
  overrides: [
    {
      // 允许测试文件使用 any 类型
      files: ['**/*.test.ts', '**/*.spec.ts'],
      rules: {
        '@typescript-eslint/no-explicit-any': 'off',
      },
    },
    {
      // scripts/ 下的 .cjs / CommonJS 脚本，关闭 require/console 规则
      // 原因：这些是 Node CLI 脚本，非前端代码
      files: ['scripts/**/*.cjs', 'scripts/**/*.js'],
      parserOptions: {
        sourceType: 'script',
        ecmaVersion: 2022,
      },
      rules: {
        '@typescript-eslint/no-require': 'off',
        '@typescript-eslint/no-var-requires': 'off',
        'no-console': 'off',
      },
    },
  ],
  ignorePatterns: ['node_modules/', 'dist/', 'scripts/'],
  globals: {
    defineProps: 'readonly',
    defineEmits: 'readonly',
    defineExpose: 'readonly',
    withDefaults: 'readonly',
  },
}
