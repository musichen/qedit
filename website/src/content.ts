export const siteContent = {
  githubUrl: 'https://github.com/musichen/qedit',
  releaseUrl: 'https://github.com/musichen/qedit/releases/tag/v0.1.0',
  donationUrl: 'https://github.com/sponsors/musichen',
  version: 'v0.1.0',
  downloadOptions: [
    { platform: 'macOS', detail: 'Apple silicon + Intel', icon: '⌘' },
    { platform: 'Windows', detail: 'Windows 10 and later', icon: '⊞' },
    { platform: 'Linux', detail: 'AppImage + deb', icon: '◈' },
  ],
} as const;

export const featureGroups = [
  {
    title: 'A workspace that stays out of the way',
    description:
      'Collapse the tree, keep the files you need close, and move through a project without losing the thread.',
    eyebrow: 'ORIENT',
    icon: 'tree',
  },
  {
    title: 'Search at the speed of thought',
    description:
      'Quick Open and keyboard-first commands make jumping between files feel immediate, even in a busy workspace.',
    eyebrow: 'MOVE',
    icon: 'search',
  },
  {
    title: 'Markdown when the words matter',
    description:
      'Switch between a clean editor and a calm preview for notes, docs, and README files without leaving qedit.',
    eyebrow: 'READ',
    icon: 'markdown',
  },
  {
    title: 'A terminal in the same rhythm',
    description:
      'Run the command, see the result, and keep your hands on the keyboard with a native integrated terminal.',
    eyebrow: 'SHIP',
    icon: 'terminal',
  },
] as const;
