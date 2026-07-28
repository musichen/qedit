import type { InferSelectModel } from 'drizzle-orm';

import { recentFiles, sessions } from './schema';

type Session = InferSelectModel<typeof sessions>;
type RecentFile = InferSelectModel<typeof recentFiles>;

/**
 * In-memory database client for qedit.
 *
 * Uses an in-memory store that mirrors the Drizzle schema.
 * In production (Tauri), this will be backed by the Rust-side SQLite
 * accessed through Tauri commands.
 */
class DbClient {
  private sessionStore = new Map<string, Session>();
  private recentFileStore: RecentFile[] = [];
  private nextId = 1;

  // ── Sessions ──

  getSession(filePath: string): Session | undefined {
    return this.sessionStore.get(filePath);
  }

  upsertSession(
    filePath: string,
    cursorPosition: number,
    scrollPosition: number,
  ): Session {
    const existing = this.sessionStore.get(filePath);
    const now = new Date();
    const session: Session = existing
      ? {
          ...existing,
          cursorPosition,
          scrollPosition,
          updatedAt: now,
        }
      : {
          id: this.nextId++,
          filePath,
          cursorPosition,
          scrollPosition,
          openedAt: now,
          updatedAt: now,
        };

    this.sessionStore.set(filePath, session);

    return session;
  }

  // ── Recent Files ──

  getRecentFiles(limit = 20): RecentFile[] {
    return [...this.recentFileStore]
      .sort(
        (a, b) =>
          new Date(b.lastOpenedAt).getTime() -
          new Date(a.lastOpenedAt).getTime(),
      )
      .slice(0, limit);
  }

  addRecentFile(filePath: string, displayName: string): RecentFile {
    const existing = this.recentFileStore.find((f) => f.filePath === filePath);
    const now = new Date();

    if (existing) {
      existing.lastOpenedAt = now;

      return existing;
    }

    const entry: RecentFile = {
      id: this.nextId++,
      filePath,
      displayName,
      lastOpenedAt: now,
    };

    this.recentFileStore.push(entry);

    // Cap at 50 entries
    if (this.recentFileStore.length > 50) {
      this.recentFileStore = this.recentFileStore.slice(-50);
    }

    return entry;
  }

  removeRecentFile(filePath: string): void {
    this.recentFileStore = this.recentFileStore.filter(
      (f) => f.filePath !== filePath,
    );
  }

  // ── Lifecycle ──

  /** Initialize the database. Called on app startup. */
  async init(): Promise<void> {
    // In production, this would open the SQLite database file
    // and run migrations. For now, the in-memory store is ready.
  }
}

/** Singleton database client instance */
export const db = new DbClient();
