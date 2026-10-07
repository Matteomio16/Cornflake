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
  names_corrected: number;
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

export interface TranslatedLines {
  lines: (string | null)[];
  starts: number[];
  cost_usd: number | null;
  model: string;
}

export const translateTexts = (texts: string[], target: string) =>
  invoke<TranslatedLines>('translate_texts', { texts, target });
export const translateMeeting = (meetingId: string, target: string, refresh = false) =>
  invoke<TranslatedLines>('translate_meeting', { meetingId, target, refresh });
export const getSetting = (key: string) => invoke<string | null>('setting_get', { key });
export const setSetting = (key: string, value: string) => invoke<void>('setting_set', { key, value });

export const TRANSLATION_TARGETS = ['English', 'German', 'French', 'Italian', 'Spanish'];

/// Translations attach to transcript lines by start time, which survives pagination.
export const timeKey = (seconds: number) => seconds.toFixed(2);

export interface CalendarEvent {
  uid: string;
  title: string;
  start: string;
  end: string;
  attendees: string[];
  location: string | null;
  video_url: string | null;
}

export const calendarUpcoming = (limit = 5, refresh = false) =>
  invoke<CalendarEvent[]>('calendar_upcoming', { limit, refresh });
export const calendarIsConnected = () => invoke<boolean>('calendar_is_connected');
export const calendarSetUrl = (url: string) => invoke<number>('calendar_set_url', { url });
export const calendarClear = () => invoke<void>('calendar_clear');
export const getVocabulary = () => invoke<string>('vocabulary_get');
export const setVocabulary = (text: string) => invoke<number>('vocabulary_set', { text });
export const setMeetingAttendees = (meetingId: string, attendees: string[]) =>
  invoke<void>('meeting_set_attendees', { meetingId, attendees });
export const getMeetingAttendees = (meetingId: string) => invoke<string[]>('meeting_get_attendees', { meetingId });
export const revertCorrections = (meetingId: string) => invoke<number>('transcript_revert_corrections', { meetingId });

// The calendar event a note is being taken for, kept until the recording is saved as a meeting
export interface PendingNote {
  title: string;
  attendees: string[];
  eventUid?: string;
}

const PENDING_NOTE_KEY = 'cornflake.pendingNote';

export function readPendingNote(): PendingNote | null {
  try {
    const raw = localStorage.getItem(PENDING_NOTE_KEY);
    return raw ? (JSON.parse(raw) as PendingNote) : null;
  } catch {
    return null;
  }
}

export function writePendingNote(note: PendingNote) {
  try {
    localStorage.setItem(PENDING_NOTE_KEY, JSON.stringify(note));
  } catch {
    // Storage unavailable: the note still records, with a generated title
  }
}

export function clearPendingNote() {
  try {
    localStorage.removeItem(PENDING_NOTE_KEY);
  } catch {
    // ignore
  }
}

export function formatNoteTitleDate(d = new Date()) {
  return d.toLocaleString(undefined, { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' });
}
