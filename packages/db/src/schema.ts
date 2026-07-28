import { sqliteTable, text, integer } from 'drizzle-orm/sqlite-core';

/**
 * Sessions table - persists editor state per file.
 * Each row represents the last known state for a given file path.
 */
export const sessions = sqliteTable('sessions', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  filePath: text('file_path').notNull().unique(),
  cursorPosition: integer('cursor_position').notNull().default(0),
  scrollPosition: integer('scroll_position').notNull().default(0),
  openedAt: integer('opened_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
  updatedAt: integer('updated_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
});

/**
 * Recent files table - tracks recently opened files for quick access.
 */
export const recentFiles = sqliteTable('recent_files', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  filePath: text('file_path').notNull().unique(),
  displayName: text('display_name').notNull(),
  lastOpenedAt: integer('last_opened_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
});

/**
 * Recent workspace roots. The current client keeps this in memory until the
 * native SQLite persistence layer is introduced.
 */
export const recentProjects = sqliteTable('recent_projects', {
  id: integer('id').primaryKey({ autoIncrement: true }),
  projectPath: text('project_path').notNull().unique(),
  displayName: text('display_name').notNull(),
  lastOpenedAt: integer('last_opened_at', { mode: 'timestamp' })
    .notNull()
    .$defaultFn(() => new Date()),
});
