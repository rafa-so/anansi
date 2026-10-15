import { defineConfig, mergeConfig } from 'vitest/config';
import { svelteTesting } from '@testing-library/svelte/vite';
import viteConfig from './vite.config.ts';

export default mergeConfig(
  viteConfig,
  defineConfig({
    plugins: [svelteTesting()],
    // Necessário para o Svelte 5 montar componentes nos testes (Seção 15.3).
    resolve: { conditions: ['browser'] },
    test: {
      environment: 'jsdom',
      include: ['src/**/*.test.ts'],
      setupFiles: ['./src/test-setup.ts'],
      restoreMocks: true,
    },
  }),
);
