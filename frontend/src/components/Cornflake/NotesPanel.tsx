'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkGfm from 'remark-gfm';
import { invoke } from '@tauri-apps/api/core';
import { toast } from 'sonner';
import { RefreshCw, Share2, ChevronDown } from 'lucide-react';
import { RoutingCard } from './RoutingCard';
import {
  exportMeetingMarkdown,
  sendToGoldfish,
  formatCost,
  generateNotes,
  getMeetingAttendees,
  getMeetingSpace,
  getUserNotes,
  listNotesVersions,
  listSpaces,
  listTemplates,
  NotesVersion,
  saveUserNotes,
  setMeetingSpace,
  Space,
  Template,
} from '@/lib/cornflake';

const NO_SPACE = '__none__';

type View = 'enhanced' | 'mine';

export function NotesPanel({
  meetingId,
  title,
  createdAt,
  onSpaceChanged,
  autoGenerate = false,
  onAutoGenerateStarted,
}: {
  meetingId: string;
  title: string;
  createdAt?: string;
  onSpaceChanged?: () => void;
  autoGenerate?: boolean;
  onAutoGenerateStarted?: () => void;
}) {
  const [spaces, setSpaces] = useState<Space[]>([]);
  const [templates, setTemplates] = useState<Template[]>([]);
  const [spaceId, setSpaceId] = useState<string>(NO_SPACE);
  const [templateId, setTemplateId] = useState<string>('');
  const [userNotes, setUserNotes] = useState('');
  const [attendees, setAttendees] = useState<string[]>([]);
  const [versions, setVersions] = useState<NotesVersion[]>([]);
  const [selectedVersion, setSelectedVersion] = useState<string>('');
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const [view, setView] = useState<View>('enhanced');
  const [shareOpen, setShareOpen] = useState(false);
  const [routingOpen, setRoutingOpen] = useState(false);
  const [noteTitle, setNoteTitle] = useState(title);
  const autoStarted = useRef(false);

  useEffect(() => setNoteTitle(title), [title]);

  const refreshVersions = useCallback(async () => {
    const v = await listNotesVersions(meetingId);
    setVersions(v);
    setSelectedVersion(v[0]?.id ?? '');
    return v;
  }, [meetingId]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [s, t, space, notes, people] = await Promise.all([
          listSpaces(),
          listTemplates(),
          getMeetingSpace(meetingId),
          getUserNotes(meetingId),
          getMeetingAttendees(meetingId),
        ]);
        if (cancelled) return;
        setSpaces(s);
        setTemplates(t);
        setSpaceId(space?.id ?? NO_SPACE);
        setTemplateId(space?.default_template ?? 'general');
        setUserNotes(notes);
        setAttendees(people);
        const v = await refreshVersions();
        setView(v.length ? 'enhanced' : 'mine');
        setLoaded(true);
      } catch (e) {
        setError(String(e));
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [meetingId, refreshVersions]);

  const onSpaceChange = async (id: string) => {
    setSpaceId(id);
    await setMeetingSpace(meetingId, id === NO_SPACE ? null : id);
    const space = spaces.find((s) => s.id === id);
    if (space) setTemplateId(space.default_template);
    onSpaceChanged?.();
  };

  const saveTitle = async () => {
    const t = noteTitle.trim();
    if (!t || t === title) return;
    try {
      await invoke('api_save_meeting_title', { meetingId, title: t });
      onSpaceChanged?.();
    } catch (e) {
      toast.error(`Could not rename: ${e}`);
    }
  };

  const onGenerate = async (template = templateId) => {
    setGenerating(true);
    setError(null);
    setView('enhanced');
    try {
      await saveUserNotes(meetingId, userNotes);
      const res = await generateNotes(meetingId, template);
      if (res.payment_required) {
        setError(res.markdown);
        return;
      }
      await refreshVersions();
      const fixed = res.names_corrected ? `, ${res.names_corrected} name${res.names_corrected === 1 ? '' : 's'} corrected` : '';
      toast.success(`Notes ready (${formatCost(res.cost_usd)}${fixed})`);
    } catch (e) {
      setError(String(e));
    } finally {
      setGenerating(false);
    }
  };

  // Right after a recording, write notes once without a click, like Granola does
  useEffect(() => {
    if (!autoGenerate || !loaded || autoStarted.current || versions.length > 0) return;
    autoStarted.current = true;
    onAutoGenerateStarted?.();
    void onGenerate();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [autoGenerate, loaded, versions.length]);

  const current = versions.find((v) => v.id === selectedVersion);
  const when = createdAt
    ? new Date(createdAt).toLocaleString(undefined, { weekday: 'short', day: 'numeric', month: 'short', hour: '2-digit', minute: '2-digit' })
    : '';

  return (
    <div className="mx-auto w-full max-w-[720px] px-8 pt-12 pb-40">
      <input
        value={noteTitle}
        onChange={(e) => setNoteTitle(e.target.value)}
        onBlur={saveTitle}
        onKeyDown={(e) => e.key === 'Enter' && (e.target as HTMLInputElement).blur()}
        aria-label="Note title"
        className="w-full bg-transparent text-[28px] font-semibold tracking-tight text-cf-ink focus:outline-none"
      />
      <div className="mt-2 flex flex-wrap items-center gap-x-3 gap-y-1 text-[13px] text-cf-muted">
        {when && <span className="cf-tabular">{when}</span>}
        {attendees.length > 0 && <span>{attendees.join(', ')}</span>}
        <label className="sr-only" htmlFor="space-select">Space</label>
        <select
          id="space-select"
          value={spaceId}
          onChange={(e) => onSpaceChange(e.target.value)}
          className="rounded-md border border-transparent bg-transparent py-0.5 text-[13px] text-cf-muted hover:border-cf-line focus:outline-none"
        >
          <option value={NO_SPACE}>No space</option>
          {spaces.map((s) => (
            <option key={s.id} value={s.id}>{s.name}</option>
          ))}
        </select>
      </div>

      <div className="mt-6 flex items-center gap-2 border-b border-cf-line pb-2">
        <div role="tablist" aria-label="Notes view" className="flex rounded-md bg-cf-hover p-0.5">
          {(['mine', 'enhanced'] as View[]).map((v) => (
            <button
              key={v}
              role="tab"
              aria-selected={view === v}
              onClick={() => setView(v)}
              className={`rounded px-3 py-1 text-[13px] ${view === v ? 'bg-cf-bg font-medium text-cf-ink shadow-[0_1px_2px_rgba(0,0,0,0.08)]' : 'text-cf-muted hover:text-cf-ink'}`}
            >
              {v === 'mine' ? 'My notes' : 'Enhanced'}
            </button>
          ))}
        </div>

        <label className="sr-only" htmlFor="template-select">Template</label>
        <select
          id="template-select"
          value={templateId}
          onChange={(e) => setTemplateId(e.target.value)}
          className="ml-2 rounded-md border border-cf-line bg-cf-bg px-2 py-1 text-[13px] text-cf-ink focus:outline-none"
        >
          {templates.map((t) => (
            <option key={t.id} value={t.id}>{t.name}</option>
          ))}
        </select>
        <button
          onClick={() => onGenerate()}
          disabled={generating}
          className="flex items-center gap-1.5 rounded-md px-2 py-1 text-[13px] text-cf-muted hover:bg-cf-hover hover:text-cf-ink disabled:opacity-50"
          title={versions.length ? 'Regenerate with this template' : 'Generate notes'}
        >
          <RefreshCw className={`h-3.5 w-3.5 ${generating ? 'animate-spin' : ''}`} />
          {generating ? 'Writing...' : versions.length ? 'Regenerate' : 'Generate notes'}
        </button>

        <div className="relative ml-auto">
          <button
            onClick={() => setShareOpen((o) => !o)}
            aria-expanded={shareOpen}
            className="flex items-center gap-1.5 rounded-md border border-cf-line px-2.5 py-1 text-[13px] text-cf-ink hover:bg-cf-hover"
          >
            <Share2 className="h-3.5 w-3.5" />
            Share
            <ChevronDown className="h-3 w-3" />
          </button>
          {shareOpen && (
            <div
              role="menu"
              className="absolute right-0 z-20 mt-1 w-56 rounded-lg border border-cf-line bg-cf-bg p-1 shadow-[0_8px_24px_rgba(0,0,0,0.12)]"
              onMouseLeave={() => setShareOpen(false)}
            >
              {[
                {
                  label: 'Copy notes as markdown',
                  disabled: !current,
                  run: () => current && navigator.clipboard.writeText(current.markdown).then(() => toast.success('Copied')),
                },
                {
                  label: 'Export to markdown folder',
                  run: () =>
                    exportMeetingMarkdown(meetingId)
                      .then((p) => toast.success(`Exported to ${p.notes_path}`))
                      .catch((e) => toast.error(`Export failed: ${e}`)),
                },
                { label: 'File in a Claude Code project', disabled: !current, run: () => setRoutingOpen(true) },
                {
                  label: 'Send to Goldfish',
                  disabled: !current,
                  run: async () => {
                    try {
                      await sendToGoldfish(meetingId, true);
                      if (!window.confirm('Goldfish accepted a dry run. Import these notes into Goldfish now? Goldfish has no undo for imports.')) return;
                      await sendToGoldfish(meetingId, false);
                      toast.success('Sent to Goldfish');
                    } catch (e) {
                      toast.error(String(e));
                    }
                  },
                },
              ].map((item) => (
                <button
                  key={item.label}
                  role="menuitem"
                  disabled={item.disabled}
                  onClick={() => {
                    setShareOpen(false);
                    void item.run();
                  }}
                  className="block w-full rounded-md px-2.5 py-1.5 text-left text-[13px] text-cf-ink hover:bg-cf-hover disabled:text-cf-muted disabled:hover:bg-transparent"
                >
                  {item.label}
                </button>
              ))}
            </div>
          )}
        </div>
      </div>

      {error && (
        <p role="alert" className="mt-4 rounded-md border border-cf-line px-3 py-2 text-[14px] text-cf-danger">{error}</p>
      )}

      {routingOpen && current && <RoutingCard meetingId={meetingId} />}

      {view === 'mine' ? (
        <textarea
          value={userNotes}
          onChange={(e) => setUserNotes(e.target.value)}
          onBlur={() => saveUserNotes(meetingId, userNotes).catch(() => undefined)}
          placeholder="Your own notes. They are kept word for word in the enhanced notes."
          aria-label="My notes"
          className="mt-5 min-h-[50vh] w-full resize-none bg-transparent text-[16px] leading-relaxed text-cf-ink placeholder:text-cf-muted focus:outline-none"
        />
      ) : generating && !current ? (
        <p className="mt-6 text-[15px] text-cf-muted">Writing your notes from what you typed and the transcript...</p>
      ) : current ? (
        <article className="mt-5">
          <div className="cornflake-notes prose max-w-none text-cf-ink prose-headings:text-cf-ink prose-strong:text-cf-ink prose-p:text-cf-ink prose-li:text-cf-ink">
            <ReactMarkdown remarkPlugins={[remarkGfm]}>{current.markdown}</ReactMarkdown>
          </div>
          <div className="mt-8 flex flex-wrap items-center gap-3 border-t border-cf-line pt-3 text-[12px] text-cf-muted">
            <span>Dark text is yours; grey italic was added from the transcript, with timestamps.</span>
            <span className="ml-auto">{current.model}</span>
            <span className="cf-tabular">{formatCost(current.cost_usd)}</span>
            {versions.length > 1 && (
              <select
                value={selectedVersion}
                onChange={(e) => setSelectedVersion(e.target.value)}
                aria-label="Notes version"
                className="rounded border border-cf-line bg-cf-bg px-1 py-0.5 text-[12px]"
              >
                {versions.map((v) => (
                  <option key={v.id} value={v.id}>
                    {new Date(v.created_at).toLocaleString()} ({v.template})
                  </option>
                ))}
              </select>
            )}
          </div>
        </article>
      ) : (
        <div className="mt-6 text-[15px] text-cf-muted">
          No enhanced notes yet.{' '}
          <button onClick={() => onGenerate()} className="text-cf-ink underline underline-offset-4">
            Generate them
          </button>{' '}
          from your notes and the transcript.
        </div>
      )}
    </div>
  );
}
