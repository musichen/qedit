use crate::{Pane, PaneGroup, dock::PanelNavigationEntry};
use gpui::{App, Entity, SharedString};

const INITIAL_SESSION_ID: u64 = 1;

#[derive(Clone)]
pub struct WorkspaceTerminalSession {
    pub id: u64,
    pub title: SharedString,
    pub pinned: bool,
    pub center: PaneGroup,
    pub active_pane: Entity<Pane>,
}

impl WorkspaceTerminalSession {
    pub fn initial(center: PaneGroup, active_pane: Entity<Pane>) -> Self {
        Self {
            id: INITIAL_SESSION_ID,
            title: SharedString::from("Session 1"),
            pinned: false,
            center,
            active_pane,
        }
    }

    pub fn new(
        id: u64,
        title: SharedString,
        pinned: bool,
        center: PaneGroup,
        active_pane: Entity<Pane>,
    ) -> Self {
        Self {
            id,
            title,
            pinned,
            center,
            active_pane,
        }
    }

    fn label(&self, cx: &App) -> SharedString {
        let tab_count = self
            .center
            .panes()
            .iter()
            .map(|pane| pane.read(cx).items_len())
            .sum::<usize>();
        if tab_count <= 1 {
            self.title.clone()
        } else {
            SharedString::from(format!("{} ({tab_count})", self.title))
        }
    }
}

pub enum TerminalSessionCloseResult {
    BackgroundClosed,
    Switched(WorkspaceTerminalSession),
    NeedsReplacement,
}

pub struct TerminalSessionManager {
    sessions: Vec<WorkspaceTerminalSession>,
    active_session_id: u64,
    next_session_id: u64,
}

impl TerminalSessionManager {
    pub fn new(current_session: WorkspaceTerminalSession) -> Self {
        let next_session_id = current_session.id + 1;
        Self {
            active_session_id: current_session.id,
            sessions: vec![current_session],
            next_session_id,
        }
    }

    pub fn current_session_id(&self) -> u64 {
        self.active_session_id
    }

    fn current_session_index(&self) -> usize {
        self.sessions
            .iter()
            .position(|session| session.id == self.active_session_id)
            .expect("active terminal session must exist")
    }

    pub fn current_session(&self) -> &WorkspaceTerminalSession {
        &self.sessions[self.current_session_index()]
    }

    pub fn current_session_mut(&mut self) -> &mut WorkspaceTerminalSession {
        let index = self.current_session_index();
        &mut self.sessions[index]
    }

    pub fn replace_current_session(&mut self, session: WorkspaceTerminalSession) {
        let next_session_id = session.id + 1;
        if let Some(index) = self
            .sessions
            .iter()
            .position(|candidate| candidate.id == self.active_session_id)
        {
            self.sessions[index] = session.clone();
        } else {
            self.sessions.push(session.clone());
        }
        self.active_session_id = session.id;
        self.next_session_id = self.next_session_id.max(next_session_id);
    }

    fn make_new_session(
        &mut self,
        center: PaneGroup,
        active_pane: Entity<Pane>,
    ) -> WorkspaceTerminalSession {
        let session_id = self.next_session_id;
        self.next_session_id += 1;
        WorkspaceTerminalSession::new(
            session_id,
            SharedString::from(format!("Session {session_id}")),
            false,
            center,
            active_pane,
        )
    }

    pub fn create_session(
        &mut self,
        new_center: PaneGroup,
        new_active_pane: Entity<Pane>,
    ) -> WorkspaceTerminalSession {
        let session = self.make_new_session(new_center, new_active_pane);
        self.sessions.push(session.clone());
        self.active_session_id = session.id;
        session
    }

    pub fn activate_session_by_id(&mut self, session_id: u64) -> Option<WorkspaceTerminalSession> {
        if self.active_session_id == session_id {
            return None;
        }

        let session = self
            .sessions
            .iter()
            .find(|session| session.id == session_id)?
            .clone();
        self.active_session_id = session_id;
        Some(session)
    }

    pub fn close_session_by_id(&mut self, session_id: u64) -> TerminalSessionCloseResult {
        let Some(index) = self
            .sessions
            .iter()
            .position(|session| session.id == session_id)
        else {
            return TerminalSessionCloseResult::BackgroundClosed;
        };

        let was_active = self.active_session_id == session_id;
        self.sessions.remove(index);

        if !was_active {
            return TerminalSessionCloseResult::BackgroundClosed;
        }

        if self.sessions.is_empty() {
            return TerminalSessionCloseResult::NeedsReplacement;
        }

        let next_index = index.min(self.sessions.len() - 1);
        let next_session = self.sessions[next_index].clone();
        self.active_session_id = next_session.id;
        TerminalSessionCloseResult::Switched(next_session)
    }

    pub fn navigation_panes(&self) -> Vec<Entity<Pane>> {
        self.sessions
            .iter()
            .flat_map(|session| session.center.panes().into_iter().cloned())
            .collect()
    }

    pub fn navigation_entries(&self, cx: &App) -> Vec<PanelNavigationEntry> {
        self.sessions
            .iter()
            .map(|session| PanelNavigationEntry {
                id: SharedString::from(session.id.to_string()),
                label: session.title.clone(),
                detail: Some(session.label(cx)),
                is_pinned: session.pinned,
                is_selected: session.id == self.active_session_id,
            })
            .collect()
    }

    pub fn rename_session(&mut self, session_id: u64, title: SharedString) -> bool {
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        {
            session.title = title;
            return true;
        }

        false
    }

    pub fn set_session_pinned(&mut self, session_id: u64, pinned: bool) -> bool {
        if let Some(session) = self
            .sessions
            .iter_mut()
            .find(|session| session.id == session_id)
        {
            session.pinned = pinned;
            return true;
        }

        false
    }
}
