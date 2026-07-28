import { createFileRoute } from '@tanstack/react-router';

import { Editor } from '#/components/Editor';
import { EditorProvider } from '#/components/EditorContext';
import { FileTree } from '#/components/FileTree';
import { StatusBar } from '#/components/StatusBar';
import { TabBar } from '#/components/TabBar';

export const Route = createFileRoute('/')({
  component: Index,
});

function Index() {
  return (
    <EditorProvider>
      <div className="grid h-screen w-screen overflow-hidden bg-background text-foreground">
        {/* Main layout: sidebar | (tabbar + editor) */}
        <div className="grid grid-cols-[240px_1fr] grid-rows-[auto_1fr_auto] h-screen">
          {/* FileTree sidebar - spans all rows on the left */}
          <div className="col-start-1 row-span-3 row-start-1 overflow-hidden">
            <FileTree />
          </div>

          {/* Tab bar - top of right panel */}
          <div className="col-start-2 row-start-1">
            <TabBar />
          </div>

          {/* Editor area - center of right panel */}
          <div className="col-start-2 row-start-2 overflow-hidden">
            <Editor />
          </div>

          {/* Status bar - bottom, spans both columns */}
          <div className="col-span-2 row-start-3">
            <StatusBar />
          </div>
        </div>
      </div>
    </EditorProvider>
  );
}
