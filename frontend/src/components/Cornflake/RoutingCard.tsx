'use client';

import { useState } from 'react';
import { toast } from 'sonner';
import { RoutingSuggestion, suggestRouting, writeRouting } from '@/lib/cornflake';

export function RoutingCard({ meetingId }: { meetingId: string }) {
  const [busy, setBusy] = useState(false);
  const [suggestion, setSuggestion] = useState<RoutingSuggestion | null>(null);
  const [error, setError] = useState<string | null>(null);

  const preview = async () => {
    setBusy(true);
    setError(null);
    try {
      setSuggestion(await suggestRouting(meetingId));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const write = async () => {
    const project = suggestion?.decision.project;
    if (!project) return;
    setBusy(true);
    try {
      const path = await writeRouting(meetingId, project);
      toast.success(`Saved to ${path}`);
      setSuggestion(null);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  const d = suggestion?.decision;

  return (
    <section className="my-4 p-4 border border-gray-200 rounded">
      <div className="flex items-center gap-3">
        <h3 className="text-sm font-semibold text-gray-900">File in a Claude Code project</h3>
        <button onClick={preview} disabled={busy} className="ml-auto text-sm underline disabled:opacity-50">
          {busy ? 'Working...' : 'Preview'}
        </button>
      </div>
      {error && <p role="alert" className="mt-2 text-sm text-red-800">{error}</p>}
      {suggestion && d && (
        <div className="mt-3 text-sm">
          {suggestion.preview ? (
            <>
              <p className="text-gray-700">
                Suggested: <strong>{d.project}</strong>
                {d.source === 'space' ? ' (set by the space)' : ` (${Math.round(d.confidence * 100)}% sure)`}. {d.reason}
              </p>
              <p className="mt-2 text-xs text-gray-500">
                {suggestion.preview.replaces_existing ? 'Replaces' : 'Creates'} {suggestion.preview.file_name} in{' '}
                {suggestion.preview.memory_dir} and adds this line to MEMORY.md:
              </p>
              <pre className="mt-1 p-2 bg-gray-50 border border-gray-200 rounded text-xs whitespace-pre-wrap">
                {suggestion.preview.index_line}
              </pre>
              <details className="mt-2">
                <summary className="text-xs text-gray-600 cursor-pointer">Memory file</summary>
                <pre className="mt-1 p-2 bg-gray-50 border border-gray-200 rounded text-xs whitespace-pre-wrap">
                  {suggestion.preview.content}
                </pre>
              </details>
              <div className="mt-3 flex gap-2">
                <button
                  onClick={write}
                  disabled={busy}
                  className="text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white disabled:opacity-50"
                >
                  Write to this project
                </button>
                <button onClick={() => setSuggestion(null)} className="text-sm px-3 py-1.5 rounded border border-gray-300">
                  Cancel
                </button>
              </div>
            </>
          ) : (
            <p className="text-gray-700">No project fits this meeting. {d.reason}</p>
          )}
        </div>
      )}
    </section>
  );
}
