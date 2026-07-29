import {
  ArrowDownToLine,
  ArrowUpRight,
  Check,
  ChevronRight,
  Coffee,
  Command,
  FileCode,
  FolderTree,
  Gauge,
  GitBranch,
  Layers,
  LockKeyhole,
  Menu,
  MonitorDown,
  PanelBottom,
  Search,
  SquareTerminal,
  X,
  Zap,
  type LucideIcon,
} from 'lucide-react';
import { useState } from 'react';

import { featureGroups, siteContent } from './content';

const baseUrl = import.meta.env.BASE_URL;

const featureIcons: Record<(typeof featureGroups)[number]['icon'], LucideIcon> =
  {
    tree: FolderTree,
    search: Search,
    markdown: FileCode,
    terminal: SquareTerminal,
  };

function BrandMark({ size = 'small' }: { size?: 'small' | 'large' }) {
  return (
    <img
      className={`brand-mark brand-mark--${size}`}
      src={`${baseUrl}qedit-logo.svg`}
      alt=""
      aria-hidden="true"
    />
  );
}

function EditorWindow({ compact = false }: { compact?: boolean }) {
  return (
    <div
      className={`editor-window${compact ? ' editor-window--compact' : ''}`}
      aria-label="qedit editor preview"
    >
      <div className="window-bar">
        <div className="window-dots" aria-hidden="true">
          <span />
          <span />
          <span />
        </div>
        <span className="window-title">qedit / notes.md</span>
        <span className="window-menu">•••</span>
      </div>
      <div className="editor-body">
        <aside className="editor-sidebar">
          <div className="sidebar-label">WORKSPACE</div>
          <div className="sidebar-folder">
            <ChevronRight size={12} />
            <FolderTree size={14} /> qedit
          </div>
          <div className="sidebar-file sidebar-file--active">
            <FileCode size={13} /> notes.md
          </div>
          <div className="sidebar-file">
            <FileCode size={13} /> README.md
          </div>
          <div className="sidebar-file">
            <FileCode size={13} /> roadmap.md
          </div>
          <div className="sidebar-rule" />
          <div className="sidebar-label">OPEN EDITORS</div>
          <div className="sidebar-file sidebar-file--muted">
            <FileCode size={13} /> ideas.txt
          </div>
        </aside>
        <div className="editor-main">
          <div className="editor-tabs">
            <span className="editor-tab editor-tab--active">
              <FileCode size={13} /> notes.md <X size={12} />
            </span>
            <span className="editor-tab">README.md</span>
          </div>
          <div className="code-area">
            <div className="code-line">
              <span className="line-number">1</span>
              <span className="syntax syntax-heading">
                # Make room for good work
              </span>
            </div>
            <div className="code-line">
              <span className="line-number">2</span>
              <span />
            </div>
            <div className="code-line">
              <span className="line-number">3</span>
              <span className="syntax syntax-muted">
                A small editor is a form of care.
              </span>
            </div>
            <div className="code-line">
              <span className="line-number">4</span>
              <span className="syntax">It should open before the thought</span>
            </div>
            <div className="code-line">
              <span className="line-number">5</span>
              <span className="syntax">has time to disappear.</span>
            </div>
            <div className="code-line">
              <span className="line-number">6</span>
              <span />
            </div>
            <div className="code-line">
              <span className="line-number">7</span>
              <span className="syntax syntax-comment">
                // fewer layers, more focus
              </span>
            </div>
            <div className="code-line">
              <span className="line-number">8</span>
              <span className="syntax syntax-caret">|</span>
            </div>
          </div>
          <div className="editor-status">
            <span>Markdown</span>
            <span>UTF-8</span>
            <span>Ln 8, Col 1</span>
          </div>
        </div>
      </div>
      {!compact && (
        <div className="terminal-drawer">
          <div className="terminal-header">
            <span>
              <PanelBottom size={13} /> TERMINAL
            </span>
            <span>
              zsh <ChevronRight size={12} />
            </span>
          </div>
          <div className="terminal-line">
            <span className="terminal-prompt">~/qedit</span> git status
          </div>
          <div className="terminal-line terminal-output">
            On branch main · working tree clean
          </div>
          <div className="terminal-line">
            <span className="terminal-prompt">~/qedit</span>{' '}
            <span className="terminal-caret">▌</span>
          </div>
        </div>
      )}
    </div>
  );
}

function App() {
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  return (
    <div className="site-shell">
      <a className="skip-link" href="#main-content">
        Skip to content
      </a>
      <header className="site-header">
        <div className="container header-inner">
          <a className="brand" href="#top" aria-label="qedit home">
            <BrandMark />
            <span>qedit</span>
          </a>
          <nav
            className={`site-nav${mobileMenuOpen ? ' site-nav--open' : ''}`}
            aria-label="Primary navigation"
          >
            <a href="#features" onClick={() => setMobileMenuOpen(false)}>
              Features
            </a>
            <a href="#workflow" onClick={() => setMobileMenuOpen(false)}>
              Workflow
            </a>
            <a href="#download" onClick={() => setMobileMenuOpen(false)}>
              Download
            </a>
            <a
              className="nav-github"
              href={siteContent.githubUrl}
              target="_blank"
              rel="noreferrer"
            >
              <GitBranch size={15} /> GitHub
            </a>
          </nav>
          <a
            className="button button--small button--light header-download"
            href={siteContent.releaseUrl}
            target="_blank"
            rel="noreferrer"
          >
            Download <ArrowDownToLine size={14} />
          </a>
          <button
            className="menu-button"
            type="button"
            aria-label="Toggle navigation"
            aria-expanded={mobileMenuOpen}
            onClick={() => setMobileMenuOpen((open) => !open)}
          >
            {mobileMenuOpen ? <X size={20} /> : <Menu size={20} />}
          </button>
        </div>
      </header>

      <main id="main-content">
        <section id="top" className="hero-section">
          <div className="hero-grid" aria-hidden="true" />
          <div className="container hero-layout">
            <div className="hero-copy">
              <p className="eyebrow">
                <span className="eyebrow-dot" /> A desktop editor for focused
                work
              </p>
              <h1>
                Your files,
                <br />
                <em>open fast.</em>
              </h1>
              <p className="hero-lede">
                qedit is a small, thoughtful editor for the files you already
                have. Native speed, familiar tools, and nothing between you and
                the work.
              </p>
              <div className="hero-actions">
                <a
                  className="button button--accent"
                  href={siteContent.releaseUrl}
                  target="_blank"
                  rel="noreferrer"
                >
                  Download {siteContent.version} <ArrowDownToLine size={16} />
                </a>
                <a
                  className="text-link text-link--hero"
                  href={siteContent.githubUrl}
                  target="_blank"
                  rel="noreferrer"
                >
                  View on GitHub <ArrowUpRight size={15} />
                </a>
              </div>
              <div className="hero-notes">
                <span>
                  <Check size={14} /> Free forever
                </span>
                <span>
                  <LockKeyhole size={14} /> Local files
                </span>
                <span>
                  <MonitorDown size={14} /> macOS · Windows · Linux
                </span>
              </div>
            </div>
            <div className="hero-visual">
              <EditorWindow />
            </div>
          </div>
          <div className="container hero-foot">
            <span>BUILT FOR THE LONG HAUL</span>
            <span className="hero-foot-rule" />
            <span>TAURI · RUST · REACT · MONACO</span>
          </div>
        </section>

        <section id="features" className="section section--paper">
          <div className="container">
            <div className="section-heading section-heading--split">
              <div>
                <p className="eyebrow eyebrow--dark">The useful parts</p>
                <h2>
                  Everything you need.
                  <br />
                  <em>Nothing to get in the way.</em>
                </h2>
              </div>
              <p className="section-intro">
                A clear workspace, a capable editor, and the little details that
                make a tool feel like yours.
              </p>
            </div>
            <div className="feature-grid">
              {featureGroups.map((feature, index) => {
                const Icon = featureIcons[feature.icon];
                return (
                  <article className="feature-card" key={feature.title}>
                    <div className="feature-card-top">
                      <span className="feature-index">0{index + 1}</span>
                      <Icon size={19} strokeWidth={1.5} />
                    </div>
                    <p className="card-eyebrow">{feature.eyebrow}</p>
                    <h3>{feature.title}</h3>
                    <p>{feature.description}</p>
                    <a
                      className="card-arrow"
                      href="#download"
                      aria-label={`Learn more about ${feature.title}`}
                    >
                      <ChevronRight size={17} />
                    </a>
                  </article>
                );
              })}
            </div>
          </div>
        </section>

        <section id="workflow" className="section section--ink">
          <div className="container workflow-layout">
            <div className="workflow-copy">
              <p className="eyebrow">The whole loop, in one place</p>
              <h2>
                Open. Edit.
                <br />
                <em>Get on with it.</em>
              </h2>
              <p>
                qedit brings the editor and terminal together in a calm,
                familiar workspace. Your files stay on your machine, and the
                interface stays close to the work.
              </p>
              <div className="workflow-points">
                <span>
                  <Command size={16} /> Keyboard-first
                </span>
                <span>
                  <Layers size={16} /> Tabs that persist
                </span>
                <span>
                  <PanelBottom size={16} /> Integrated terminal
                </span>
              </div>
              <a
                className="text-link text-link--accent"
                href={siteContent.githubUrl}
                target="_blank"
                rel="noreferrer"
              >
                Explore the source <ArrowUpRight size={15} />
              </a>
            </div>
            <div className="workflow-visual">
              <EditorWindow compact />
            </div>
          </div>
        </section>

        <section id="why-qedit" className="section section--warm">
          <div className="container">
            <div className="section-heading section-heading--center">
              <p className="eyebrow eyebrow--dark">Small by design</p>
              <h2>
                More room for the work
                <br />
                <em>that brought you here.</em>
              </h2>
            </div>
            <div className="metrics-grid">
              <div className="metric">
                <Gauge size={21} />
                <strong>Fast startup</strong>
                <span>Open a file and begin before the coffee cools.</span>
              </div>
              <div className="metric">
                <Zap size={21} />
                <strong>Low overhead</strong>
                <span>A focused desktop app that respects your machine.</span>
              </div>
              <div className="metric">
                <LockKeyhole size={21} />
                <strong>Local-first</strong>
                <span>No account, no upload, no cloud hop in the way.</span>
              </div>
            </div>
            <div className="stack-line">
              <span>Made with</span>
              <span className="stack-pill">Tauri</span>
              <span className="stack-pill">Rust</span>
              <span className="stack-pill">React</span>
              <span className="stack-pill">TanStack</span>
              <span className="stack-pill">Monaco</span>
              <span className="stack-pill">Xterm.js</span>
            </div>
          </div>
        </section>

        <section
          id="download"
          className="section section--paper download-section"
        >
          <div className="container">
            <div className="section-heading section-heading--split download-heading">
              <div>
                <p className="eyebrow eyebrow--dark">Start with qedit</p>
                <h2>
                  Pick your platform.
                  <br />
                  <em>Keep your focus.</em>
                </h2>
              </div>
              <p className="section-intro">
                The first public release is ready. Download the build for your
                machine, open a folder, and make it yours.
              </p>
            </div>
            <div className="download-grid">
              {siteContent.downloadOptions.map((download) => (
                <a
                  className="download-card"
                  key={download.platform}
                  href={siteContent.releaseUrl}
                  target="_blank"
                  rel="noreferrer"
                >
                  <span className="platform-glyph" aria-hidden="true">
                    {download.icon}
                  </span>
                  <span>
                    <strong>{download.platform}</strong>
                    <small>{download.detail}</small>
                  </span>
                  <ArrowDownToLine size={18} />
                </a>
              ))}
            </div>
            <p className="download-note">
              All downloads live on the{' '}
              <a
                className="inline-link"
                href={siteContent.releaseUrl}
                target="_blank"
                rel="noreferrer"
              >
                v0.1.0 release page <ArrowUpRight size={13} />
              </a>
              . qedit is free to use and open source.
            </p>
          </div>
        </section>

        <section className="support-section">
          <div className="container support-card">
            <div className="support-icon">
              <Coffee size={22} />
            </div>
            <div>
              <p className="eyebrow eyebrow--dark">
                Optional, always appreciated
              </p>
              <h2>Enjoying the quiet?</h2>
              <p>
                qedit is free. If it earns a place in your toolkit, you can buy
                the people behind it a beer.
              </p>
            </div>
            <a
              className="button button--outline"
              href={siteContent.donationUrl}
              target="_blank"
              rel="noreferrer"
            >
              Buy us a beer <ArrowUpRight size={15} />
            </a>
          </div>
        </section>
      </main>

      <footer className="site-footer">
        <div className="container footer-inner">
          <a className="brand" href="#top">
            <BrandMark />
            <span>qedit</span>
          </a>
          <p>A small editor for serious work.</p>
          <div className="footer-links">
            <a href="#features">Features</a>
            <a href={siteContent.githubUrl} target="_blank" rel="noreferrer">
              GitHub <ArrowUpRight size={13} />
            </a>
            <a href={siteContent.releaseUrl} target="_blank" rel="noreferrer">
              Releases <ArrowUpRight size={13} />
            </a>
          </div>
          <span className="footer-copy">© 2026 qedit</span>
        </div>
      </footer>
    </div>
  );
}

export default App;
