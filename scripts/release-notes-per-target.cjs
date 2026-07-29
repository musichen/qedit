const fs = require('node:fs');
const path = require('node:path');

const root = process.argv[2];
for (const target of ['macos-arm64', 'macos-x64', 'windows-arm64', 'windows-x64', 'linux-arm64', 'linux-x64']) {
  const manifest = JSON.parse(fs.readFileSync(path.join(root, target, 'manifest.json'), 'utf8'));
  console.log(`- ${target}: ${manifest.signing.status} — ${manifest.signing.reason}`);
}
