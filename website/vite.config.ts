import react from '@vitejs/plugin-react';
import { defineConfig } from 'vite';

const configuredBase = process.env.VITE_BASE_PATH;
const repositoryName = process.env.GITHUB_REPOSITORY?.split('/')[1];
const base =
  configuredBase ?? (repositoryName ? `/${repositoryName}/` : '/qedit/');

export default defineConfig({
  base,
  plugins: [react()],
});
