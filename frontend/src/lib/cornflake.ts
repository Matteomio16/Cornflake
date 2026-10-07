import { invoke } from '@tauri-apps/api/core';

export interface Space {
  id: string;
  name: string;
  default_template: string;
  routing_project: string | null;
  position: number;
}

export interface Template {
  id: string;
  name: string;
  description: string;
  sections: { heading: string; guidance: string }[];
}

export interface GeneratedNotes {
  version_id: string;
  template: string;
  markdown: string;
  model: string;
  cost_usd: number | null;
  prompt_tokens: number;
  completion_tokens: number;
  payment_required: boolean;
}

export interface NotesVersion {
  id: string;
  meeting_id: string;
  template: string;
  model: string;
  prompt_version: string;
  markdown: string;
  doc_json: string;
  cost_usd: number | null;
  created_at: string;
}

export const listSpaces = () => invoke<Space[]>('spaces_list');
export const createSpace = (name: string, defaultTemplate?: string) =>
  invoke<Space>('spaces_create', { name, defaultTemplate });
export const updateSpace = (
  id: string,
  changes: { name?: string; defaultTemplate?: string; routingProject?: string }
) => invoke<void>('spaces_update', { id, ...changes });
export const deleteSpace = (id: string) => invoke<void>('spaces_delete', { id });
export const setMeetingSpace = (meetingId: string, spaceId: string | null) =>
  invoke<void>('meeting_set_space', { meetingId, spaceId });
export const getMeetingSpace = (meetingId: string) => invoke<Space | null>('meeting_get_space', { meetingId });
export const listTemplates = () => invoke<Template[]>('notes_list_templates');
export const getUserNotes = (meetingId: string) => invoke<string>('user_notes_get', { meetingId });
export const saveUserNotes = (meetingId: string, markdown: string) =>
  invoke<void>('user_notes_save', { meetingId, markdown });
export const generateNotes = (meetingId: string, templateId?: string, outputLanguage?: string) =>
  invoke<GeneratedNotes>('notes_generate', { meetingId, templateId, outputLanguage });
export const listNotesVersions = (meetingId: string) =>
  invoke<NotesVersion[]>('notes_versions_list', { meetingId });

// Notes typed during a recording live here until the meeting row exists, then move to SQLite.
const LIVE_NOTES_KEY = 'cornflake.liveNotes';

export function readLiveNotes(): string {
  try {
    return localStorage.getItem(LIVE_NOTES_KEY) ?? '';
  } catch {
    return '';
  }
}

export function writeLiveNotes(markdown: string) {
  try {
    localStorage.setItem(LIVE_NOTES_KEY, markdown);
  } catch {
    // Storage unavailable: notes stay in component state for this session
  }
}

export function clearLiveNotes() {
  try {
    localStorage.removeItem(LIVE_NOTES_KEY);
  } catch {
    // ignore
  }
}

export function formatCost(usd: number | null | undefined): string {
  if (usd == null) return 'cost not reported';
  if (usd < 0.01) return `$${usd.toFixed(4)}`;
  return `$${usd.toFixed(2)}`;
}

export const exportMeetingMarkdown = (meetingId: string) =>
  invoke<{ notes_path: string; transcript_path: string }>('export_meeting_markdown', { meetingId });
export const getExportDir = () => invoke<string>('export_get_dir');
export const setExportDir = (dir: string) => invoke<void>('export_set_dir', { dir });

export interface RoutingProject {
  id: string;
  name: string;
  description: string;
}

export interface MemoryPreview {
  memory_dir: string;
  file_name: string;
  content: string;
  index_line: string;
  replaces_existing: boolean;
}

export interface RoutingSuggestion {
  decision: { project: string | null; confidence: number; reason: string; source: string; cost_usd: number | null };
  preview: MemoryPreview | null;
}

export const getRoutingProjects = () => invoke<RoutingProject[]>('routing_projects_get');
export const saveRoutingProjects = (projects: RoutingProject[]) => invoke<void>('routing_projects_save', { projects });
export const suggestRouting = (meetingId: string) => invoke<RoutingSuggestion>('routing_suggest', { meetingId });
export const writeRouting = (meetingId: string, projectId: string) =>
  invoke<string>('routing_write', { meetingId, projectId });

export const getWebhooks = () => invoke<string>('webhooks_get');
export const setWebhooks = (urls: string) => invoke<number>('webhooks_set', { urls });
export const sendToGoldfish = (meetingId: string, dryRun: boolean) =>
  invoke<{ ok: boolean; source?: string }>('goldfish_import', { meetingId, dryRun });
