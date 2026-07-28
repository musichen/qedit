import { createFileRoute } from '@tanstack/react-router';
import { useCallback, useEffect } from 'react';

import { Editor } from '#/components/Editor';
import { EditorProvider, useEditor } from '#/components/EditorContext';
import { FileTree } from '#/components/FileTree';
import { StatusBar } from '#/components/StatusBar';
import { TabBar } from '#/components/TabBar';

export const Route = createFileRoute('/')({
  component: Index,
});

function Index() {
  return (
    <EditorProvider>
      <EditorLayout />
    </EditorProvider>
  );
}

function EditorLayout() {
  const { saveActiveFile } = useEditor();

  const handleKeyDown = useCallback(
    (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === 's') {
        e.preventDefault();
        void saveActiveFile();
      }
    },
    [saveActiveFile],
  );

  useEffect(() => {
    window.addEventListener('keydown', handleKeyDown);

    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [handleKeyDown]);

  return (
    <div className="grid h-screen w-screen overflow-hidden bg-background text-foreground">
      <div className="grid grid-cols-[240px_1fr] grid-rows-[auto_1fr_auto] h-screen">
        <div className="col-start-1 row-span-3 row-start-1 overflow-hidden">
          <FileTree />
        </div>

        <div className="col-start-2 row-start-1">
          <TabBar />
        </div>

        <div className="col-start-2 row-start-2 overflow-hidden">
          <Editor />
        </div>

        <div className="col-span-2 row-start-3">
          <StatusBar />
        </div>
      </div>
    </div>
  );
}
