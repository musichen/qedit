const docsIndex = [
  { id: 'getting-started', section: 'Start here', title: 'Getting started', description: 'Install Qedit and learn the essential workflow.' },
  { id: 'editing', section: 'Core workflow', title: 'Editing and navigation', description: 'Buffers, selections, key bindings, and navigation.' },
  { id: 'customize', section: 'Make it yours', title: 'Customize Qedit', description: 'Themes, languages, extensions, and settings.' },
  { id: 'development', section: 'Build from source', title: 'Development', description: 'Build Qedit and contribute to the project.' },
  { id: 'installation', section: 'Guides', title: 'Installation', description: 'Set up Qedit on your machine.', href: 'https://github.com/musichen/qedit/blob/main/docs/src/installation.md' },
  { id: 'quick-start', section: 'Guides', title: 'Quick start', description: 'Open a project and begin editing.', href: 'https://github.com/musichen/qedit/blob/main/docs/src/quick-start.md' },
  { id: 'terminal', section: 'Guides', title: 'Terminal', description: 'Run commands beside your code.', href: 'https://github.com/musichen/qedit/blob/main/docs/src/terminal.md' },
  { id: 'extensions', section: 'Reference', title: 'Extensions', description: 'Add languages and workflows.', href: 'https://github.com/musichen/qedit/blob/main/docs/src/extensions.md' },
  { id: 'settings', section: 'Reference', title: 'All settings', description: 'Browse every available setting.', href: 'https://github.com/musichen/qedit/blob/main/docs/src/reference/all-settings.md' }
];

function docsApp() {
  return {
    mobileMenu: false,
    searchOpen: false,
    query: '',
    copied: false,
    activeSection: 'getting-started',
    sections: [
      { id: 'start', label: 'Start here', items: [{ id: 'getting-started', title: 'Getting started' }] },
      { id: 'workflow', label: 'Core workflow', items: [{ id: 'editing', title: 'Editing and navigation' }] },
      { id: 'customize', label: 'Make it yours', items: [{ id: 'customize', title: 'Customize Qedit' }] },
      { id: 'development', label: 'Build from source', items: [{ id: 'development', title: 'Development' }] }
    ],
    get searchResults() {
      const query = this.query.trim().toLowerCase();
      if (!query) return [];
      const terms = query.split(/\s+/).filter(Boolean);
      return docsIndex.filter((item) => terms.every((term) => `${item.title} ${item.description} ${item.section}`.toLowerCase().includes(term)));
    },
    openSearch() {
      this.searchOpen = true;
      this.$nextTick(() => this.$refs.searchInput?.focus());
    },
    handleKeydown(event) {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        this.openSearch();
      }
      if (event.key === 'Escape' && this.mobileMenu) this.mobileMenu = false;
    },
    copyCommand() {
      navigator.clipboard?.writeText('git clone https://github.com/musichen/qedit.git\ncd qedit\ncargo run -p zed');
      this.copied = true;
      window.setTimeout(() => { this.copied = false; }, 1600);
    }
  };
}

document.addEventListener('DOMContentLoaded', () => {
  const sections = [...document.querySelectorAll('[data-section]')];
  const observer = new IntersectionObserver((entries) => {
    const visible = entries.filter((entry) => entry.isIntersecting).sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];
    if (visible && window.Alpine) window.Alpine.$data(document.querySelector('[x-data]')).activeSection = visible.target.dataset.section;
  }, { rootMargin: '-20% 0px -65% 0px', threshold: [0.1, 0.4, 0.8] });
  sections.forEach((section) => observer.observe(section));
});
