import { db } from '@qedit/db';
import { describe, it, expect, beforeEach } from 'vitest';

describe('db client', () => {
  // The db singleton is an in-memory store — clear it between tests by
  // re-importing. Since it's a singleton, tests that mutate state must
  // be self-contained.
  beforeEach(() => {
    // Clear the in-memory store by removing all recent files
    while (db.getRecentFiles(100).length > 0) {
      const files = db.getRecentFiles(100);
      for (const f of files) {
        db.removeRecentFile(f.filePath);
      }
    }
    while (db.getRecentProjects(100).length > 0) {
      const projects = db.getRecentProjects(100);
      for (const project of projects) {
        db.removeRecentProject(project.projectPath);
      }
    }
  });

  it('starts with an empty recent file list', () => {
    expect(db.getRecentFiles()).toHaveLength(0);
  });

  it('adds a recent file entry', () => {
    const entry = db.addRecentFile('/home/test.ts', 'test.ts');

    expect(entry.filePath).toBe('/home/test.ts');
    expect(entry.displayName).toBe('test.ts');
    expect(entry.lastOpenedAt).toBeInstanceOf(Date);

    const recent = db.getRecentFiles();
    expect(recent).toHaveLength(1);
    expect(recent[0]?.filePath).toBe('/home/test.ts');
  });

  it('deduplicates recent files by path', () => {
    db.addRecentFile('/home/a.ts', 'a.ts');
    db.addRecentFile('/home/b.ts', 'b.ts');
    db.addRecentFile('/home/a.ts', 'a.ts'); // re-open

    const recent = db.getRecentFiles();
    expect(recent).toHaveLength(2);
    // Most recently opened should be first
    expect(recent[0]?.filePath).toBe('/home/a.ts');
    expect(recent[1]?.filePath).toBe('/home/b.ts');
  });

  it('capping at 50 entries', () => {
    for (let i = 0; i < 60; i++) {
      db.addRecentFile(`/home/file${i}.ts`, `file${i}.ts`);
    }

    const recent = db.getRecentFiles();
    expect(recent.length).toBeLessThanOrEqual(50);
  });

  it('removes a recent file', () => {
    db.addRecentFile('/home/a.ts', 'a.ts');
    db.addRecentFile('/home/b.ts', 'b.ts');

    db.removeRecentFile('/home/a.ts');

    const recent = db.getRecentFiles();
    expect(recent).toHaveLength(1);
    expect(recent[0]?.filePath).toBe('/home/b.ts');
  });

  it('getRecentFiles respects limit', () => {
    for (let i = 0; i < 10; i++) {
      db.addRecentFile(`/home/file${i}.ts`, `file${i}.ts`);
    }

    expect(db.getRecentFiles(5)).toHaveLength(5);
    expect(db.getRecentFiles(20)).toHaveLength(10);
  });

  it('tracks and deduplicates recent projects', () => {
    const first = db.addRecentProject('/home/project-a', 'project-a');
    db.addRecentProject('/home/project-b', 'project-b');
    db.addRecentProject('/home/project-a', 'project-a');

    expect(first.projectPath).toBe('/home/project-a');
    expect(db.getRecentProjects()).toHaveLength(2);
    expect(db.getRecentProjects()[0]?.projectPath).toBe('/home/project-a');
  });

  it('removes a recent project', () => {
    db.addRecentProject('/home/project-a', 'project-a');
    db.removeRecentProject('/home/project-a');

    expect(db.getRecentProjects()).toHaveLength(0);
  });
});
