'use client';

import { useCallback, useEffect, useRef, useState } from 'react';
import ReactMarkdown from 'react-markdown';
import { RoutingCard } from './RoutingCard';
import remarkGfm from 'remark-gfm';
import { toast } from 'sonner';
import {
  exportMeetingMarkdown,
  sendToGoldfish,
  formatCost,
  generateNotes,
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

export function NotesPanel({
  meetingId,
  onSpaceChanged,
  autoGenerate = false,
  onAutoGenerateStarted,
}: {
  meetingId: string;
  onSpaceChanged?: () => void;
  autoGenerate?: boolean;
  onAutoGenerateStarted?: () => void;
}) {
  const [spaces, setSpaces] = useState<Space[]>([]);
  const [templates, setTemplates] = useState<Template[]>([]);
  const [spaceId, setSpaceId] = useState<string>(NO_SPACE);
  const [templateId, setTemplateId] = useState<string>('');
  const [userNotes, setUserNotes] = useState('');
  const [versions, setVersions] = useState<NotesVersion[]>([]);
  const [selectedVersion, setSelectedVersion] = useState<string>('');
  const [generating, setGenerating] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [loaded, setLoaded] = useState(false);
  const autoStarted = useRef(false);

  const refreshVersions = useCallback(async () => {
    const v = await listNotesVersions(meetingId);
    setVersions(v);
    setSelectedVersion(v[0]?.id ?? '');
  }, [meetingId]);

  useEffect(() => {
    let cancelled = false;
    (async () => {
      try {
        const [s, t, space, notes] = await Promise.all([
          listSpaces(),
          listTemplates(),
          getMeetingSpace(meetingId),
          getUserNotes(meetingId),
        ]);
        if (cancelled) return;
        setSpaces(s);
        setTemplates(t);
        setSpaceId(space?.id ?? NO_SPACE);
        setTemplateId(space?.default_template ?? 'general');
        setUserNotes(notes);
        await refreshVersions();
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

  const onGenerate = async () => {
    setGenerating(true);
    setError(null);
    try {
      await saveUserNotes(meetingId, userNotes);
      const res = await generateNotes(meetingId, templateId);
      if (res.payment_required) {
        setError(res.markdown);
        return;
      }
      await refreshVersions();
      toast.success(`Notes ready (${formatCost(res.cost_usd)})`);
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

  return (
    <div className="flex flex-col h-full overflow-hidden bg-white">
      <div className="flex flex-wrap items-center gap-2 px-5 py-3 border-b border-gray-200">
        <label className="text-xs text-gray-600" htmlFor="space-select">Space</label>
        <select
          id="space-select"
          value={spaceId}
          onChange={(e) => onSpaceChange(e.target.value)}
          className="text-sm border border-gray-300 rounded px-2 py-1"
        >
          <option value={NO_SPACE}>No space</option>
          {spaces.map((s) => (
            <option key={s.id} value={s.id}>{s.name}</option>
          ))}
        </select>
        <label className="text-xs text-gray-600 ml-2" htmlFor="template-select">Template</label>
        <select
          id="template-select"
          value={templateId}
          onChange={(e) => setTemplateId(e.target.value)}
          className="text-sm border border-gray-300 rounded px-2 py-1"
        >
          {templates.map((t) => (
            <option key={t.id} value={t.id}>{t.name}</option>
          ))}
        </select>
        <button
          onClick={onGenerate}
          disabled={generating}
          className="ml-auto text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white disabled:opacity-50"
        >
          {generating ? 'Writing notes...' : versions.length ? 'Regenerate notes' : 'Generate notes'}
        </button>
      </div>

      <div className="flex-1 overflow-y-auto">
        <details className="px-5 pt-4" open={!versions.length}>
          <summary className="text-sm font-semibold text-gray-900 cursor-pointer">My notes</summary>
          <textarea
            value={userNotes}
            onChange={(e) => setUserNotes(e.target.value)}
            onBlur={() => saveUserNotes(meetingId, userNotes).catch(() => undefined)}
            placeholder="Your own bullets. They are kept word for word in the generated notes."
            aria-label="My notes"
            className="w-full min-h-[120px] mt-2 p-3 text-sm border border-gray-200 rounded resize-y focus:outline-none focus:border-gray-400"
          />
        </details>

        {error && (
          <p role="alert" className="mx-5 mt-4 p-3 text-sm text-red-800 bg-red-50 border border-red-200 rounded">{error}</p>
        )}

        {current ? (
          <article className="px-5 py-4">
            <div className="flex items-center gap-3 mb-3 text-xs text-gray-500">
              {versions.length > 1 && (
                <select
                  value={selectedVersion}
                  onChange={(e) => setSelectedVersion(e.target.value)}
                  aria-label="Notes version"
                  className="border border-gray-300 rounded px-1 py-0.5"
                >
                  {versions.map((v) => (
                    <option key={v.id} value={v.id}>
                      {new Date(v.created_at).toLocaleString()} ({v.template})
                    </option>
                  ))}
                </select>
              )}
              <span>{current.model}</span>
              <span>{formatCost(current.cost_usd)}</span>
              <button
                onClick={() => navigator.clipboard.writeText(current.markdown).then(() => toast.success('Copied'))}
                className="ml-auto underline"
              >
                Copy markdown
              </button>
              <button
                onClick={() =>
                  exportMeetingMarkdown(meetingId)
                    .then((p) => toast.success(`Exported to ${p.notes_path}`))
                    .catch((e) => toast.error(`Export failed: ${e}`))
                }
                className="underline"
              >
                Export
              </button>
              <button
                onClick={async () => {
                  try {
                    await sendToGoldfish(meetingId, true);
                    if (!window.confirm('Goldfish accepted a dry run. Import these notes into Goldfish now? Goldfish has no undo for imports.')) return;
                    await sendToGoldfish(meetingId, false);
                    toast.success('Sent to Goldfish');
                  } catch (e) {
                    toast.error(String(e));
                  }
                }}
                className="underline"
              >
                Send to Goldfish
              </button>
            </div>
            <div className="cornflake-notes prose prose-sm max-w-none">
              <ReactMarkdown remarkPlugins={[remarkGfm]}>{current.markdown}</ReactMarkdown>
            </div>
            <RoutingCard meetingId={meetingId} />
            <p className="mt-6 text-xs text-gray-500">
              Plain text is yours. Grey italic text was added from the transcript, with timestamps.
            </p>
          </article>
        ) : (
          !generating && (
            <p className="px-5 py-6 text-sm text-gray-500">
              No notes yet. Choose a template and generate them from your notes and the transcript.
            </p>
          )
        )}
      </div>
    </div>
  );
}
