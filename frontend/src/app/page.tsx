'use client';

import { Suspense, useEffect, useMemo, useState } from 'react';
import { useRouter, useSearchParams } from 'next/navigation';
import { invoke } from '@tauri-apps/api/core';
import { Video, CalendarDays } from 'lucide-react';
import { useSidebar, CurrentMeeting } from '@/components/Sidebar/SidebarProvider';
import { startNewNote } from '@/components/Cornflake/AppSidebar';
import { calendarIsConnected, calendarUpcoming, CalendarEvent, writePendingNote } from '@/lib/cornflake';

function dayLabel(d: Date, now = new Date()) {
  const startOf = (x: Date) => new Date(x.getFullYear(), x.getMonth(), x.getDate()).getTime();
  const diff = Math.round((startOf(now) - startOf(d)) / 86_400_000);
  if (diff === 0) return 'Today';
  if (diff === 1) return 'Yesterday';
  if (diff === -1) return 'Tomorrow';
  if (diff > 1 && diff < 7) return d.toLocaleDateString(undefined, { weekday: 'long' });
  return d.toLocaleDateString(undefined, { weekday: 'short', day: 'numeric', month: 'long', year: d.getFullYear() === now.getFullYear() ? undefined : 'numeric' });
}

const time = (d: Date) => d.toLocaleTimeString(undefined, { hour: '2-digit', minute: '2-digit' });

function attendeeLine(names: string[]) {
  if (!names.length) return '';
  const shown = names.slice(0, 3).join(', ');
  return names.length > 3 ? `${shown} +${names.length - 3}` : shown;
}

function ComingUp() {
  const router = useRouter();
  const [connected, setConnected] = useState<boolean | null>(null);
  const [events, setEvents] = useState<CalendarEvent[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    calendarIsConnected()
      .then(async (c) => {
        setConnected(c);
        if (c) setEvents(await calendarUpcoming(5));
      })
      .catch((e) => setError(String(e)));
  }, []);

  const takeNotes = (e: CalendarEvent) => {
    writePendingNote({ title: e.title, attendees: e.attendees, eventUid: e.uid });
    sessionStorage.setItem('autoStartRecording', 'true');
    router.push('/live');
  };

  return (
    <section aria-labelledby="coming-up" className="mb-10">
      <h2 id="coming-up" className="mb-3 text-[13px] font-semibold text-cf-muted">Coming up</h2>
      {connected === false && (
        <button
          onClick={() => router.push('/settings#calendar')}
          className="flex w-full items-center gap-3 rounded-lg border border-dashed border-cf-line px-4 py-3 text-left text-[14px] text-cf-muted hover:bg-cf-hover"
        >
          <CalendarDays className="h-4 w-4" />
          Connect your Google Calendar to see your next meetings here.
        </button>
      )}
      {error && <p className="text-[14px] text-cf-danger">Calendar could not be read: {error}</p>}
      {connected && !error && events.length === 0 && (
        <p className="text-[14px] text-cf-muted">Nothing in the next two weeks.</p>
      )}
      <ul className="divide-y divide-cf-line">
        {events.map((e) => {
          const start = new Date(e.start);
          const live = start.getTime() <= Date.now() && new Date(e.end).getTime() > Date.now();
          return (
            <li key={`${e.uid}-${e.start}`} className="group flex items-center gap-4 py-2.5">
              <div className="w-24 shrink-0 cf-tabular">
                <div className="text-[13px] text-cf-muted">{dayLabel(start)}</div>
                <div className="text-[14px] font-medium text-cf-ink">{time(start)}</div>
              </div>
              <button onClick={() => takeNotes(e)} className="min-w-0 flex-1 text-left">
                <div className="flex items-center gap-2">
                  {live && <span className="h-1.5 w-1.5 rounded-full bg-cf-gold" aria-label="Happening now" />}
                  <span className="truncate text-[15px] text-cf-ink group-hover:underline underline-offset-4">{e.title}</span>
                </div>
                {e.attendees.length > 0 && (
                  <div className="truncate text-[13px] text-cf-muted">{attendeeLine(e.attendees)}</div>
                )}
              </button>
              {e.video_url && (
                <button
                  onClick={() => invoke('open_external_url', { url: e.video_url })}
                  className="rounded-md p-1.5 text-cf-muted hover:bg-cf-hover hover:text-cf-ink"
                  aria-label={`Open call link for ${e.title}`}
                  title="Open call link"
                >
                  <Video className="h-4 w-4" />
                </button>
              )}
              <button
                onClick={() => takeNotes(e)}
                className="rounded-md border border-cf-line px-2.5 py-1 text-[13px] text-cf-ink opacity-0 transition-opacity hover:bg-cf-hover focus:opacity-100 group-hover:opacity-100"
              >
                Take notes
              </button>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function NoteRows({ notes, spaceNames }: { notes: CurrentMeeting[]; spaceNames: Map<string, string> }) {
  const router = useRouter();
  const groups = useMemo(() => {
    const map = new Map<string, CurrentMeeting[]>();
    for (const n of notes) {
      const label = n.created_at ? dayLabel(new Date(n.created_at)) : 'Earlier';
      map.set(label, [...(map.get(label) ?? []), n]);
    }
    return [...map.entries()];
  }, [notes]);

  return (
    <>
      {groups.map(([label, items]) => (
        <div key={label} className="mb-6">
          <h3 className="mb-1 text-[13px] text-cf-muted">{label}</h3>
          <ul>
            {items.map((n) => (
              <li key={n.id}>
                <button
                  onClick={() => router.push(`/meeting-details?id=${n.id}`)}
                  className="flex w-full items-baseline gap-3 rounded-md px-2 py-2 -mx-2 text-left hover:bg-cf-hover"
                >
                  <span className="min-w-0 flex-1 truncate text-[15px] text-cf-ink">{n.title}</span>
                  {n.space_id && spaceNames.get(n.space_id) && (
                    <span className="shrink-0 text-[12px] text-cf-muted">{spaceNames.get(n.space_id)}</span>
                  )}
                  {n.created_at && (
                    <span className="w-14 shrink-0 text-right text-[13px] text-cf-muted cf-tabular">{time(new Date(n.created_at))}</span>
                  )}
                </button>
              </li>
            ))}
          </ul>
        </div>
      ))}
    </>
  );
}

function HomeContent() {
  const router = useRouter();
  const params = useSearchParams();
  const q = params.get('q');
  const spaceId = params.get('space');
  const { meetings, spaces, searchTranscripts, searchResults, isSearching } = useSidebar();
  const spaceNames = useMemo(() => new Map(spaces.map((s) => [s.id, s.name])), [spaces]);
  const space = spaces.find((s) => s.id === spaceId);

  useEffect(() => {
    if (q) searchTranscripts(q);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [q]);

  const notes = spaceId ? meetings.filter((m) => m.space_id === spaceId) : meetings;

  return (
    <div className="cf-scroll h-screen overflow-y-auto">
      <div className="mx-auto max-w-[720px] px-8 pt-12 pb-24">
        {q ? (
          <>
            <h1 className="mb-6 text-[28px] font-semibold tracking-tight text-cf-ink">Results for &ldquo;{q}&rdquo;</h1>
            {isSearching && <p className="text-[14px] text-cf-muted">Searching...</p>}
            {!isSearching && searchResults.length === 0 && <p className="text-[14px] text-cf-muted">No notes or transcripts mention that.</p>}
            <ul className="divide-y divide-cf-line">
              {searchResults.map((r) => (
                <li key={r.id}>
                  <button onClick={() => router.push(`/meeting-details?id=${r.id}`)} className="w-full py-3 text-left">
                    <div className="text-[15px] text-cf-ink">{r.title}</div>
                    <div className="mt-0.5 line-clamp-2 text-[13px] text-cf-muted">{r.matchContext}</div>
                  </button>
                </li>
              ))}
            </ul>
          </>
        ) : (
          <>
            <div className="mb-8 flex items-center justify-between">
              <h1 className="text-[28px] font-semibold tracking-tight text-cf-ink">{space ? space.name : 'Home'}</h1>
              {space && (
                <span className="text-[13px] text-cf-muted">Default template: {space.default_template.replace('_', ' ')}</span>
              )}
            </div>
            {!space && <ComingUp />}
            <section aria-labelledby="your-notes">
              <h2 id="your-notes" className="mb-3 text-[13px] font-semibold text-cf-muted">
                {space ? 'Notes in this space' : 'Your notes'}
              </h2>
              {notes.length === 0 ? (
                <div className="rounded-lg border border-dashed border-cf-line px-5 py-8 text-center">
                  <p className="text-[15px] text-cf-ink">No notes yet.</p>
                  <p className="mt-1 text-[14px] text-cf-muted">
                    Start one with New note, or press Ctrl+Alt+R from any app.
                  </p>
                  <button
                    onClick={() => startNewNote(router)}
                    className="mt-4 rounded-md bg-cf-primary px-3 py-1.5 text-[14px] font-medium text-cf-primary-ink hover:opacity-90"
                  >
                    New note
                  </button>
                </div>
              ) : (
                <NoteRows notes={notes} spaceNames={spaceNames} />
              )}
            </section>
          </>
        )}
      </div>
    </div>
  );
}

export default function HomePage() {
  return (
    <Suspense fallback={null}>
      <HomeContent />
    </Suspense>
  );
}
