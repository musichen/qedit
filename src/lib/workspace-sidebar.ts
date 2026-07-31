export function shouldOpenSidebarForWorkspace(
  previousWorkspaceRoot: string | null,
  workspaceRoot: string | null,
): boolean {
  return workspaceRoot !== null && workspaceRoot !== previousWorkspaceRoot;
}
