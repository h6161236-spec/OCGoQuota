import { cpSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';

const generatedIcons = resolve('src-tauri/gen/icon-output/android');
const androidResources = resolve('src-tauri/gen/android/app/src/main/res');

if (!existsSync(generatedIcons)) {
  throw new Error(`Generated Android icons not found: ${generatedIcons}`);
}

if (!existsSync(androidResources)) {
  throw new Error('Android project is not initialized. Run `pnpm tauri android init --ci` first.');
}

cpSync(generatedIcons, androidResources, { recursive: true, force: true });
console.log(`Android launcher icons copied to ${androidResources}`);
