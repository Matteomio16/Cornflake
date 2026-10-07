'use client';

import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { calendarClear, calendarIsConnected, calendarSetUrl, getVocabulary, setVocabulary } from '@/lib/cornflake';

/// Paste-a-link calendar connection (Google: Settings > your calendar > Integrate calendar > Secret address in iCal format).
export function CalendarConnect({ onConnected }: { onConnected?: () => void }) {
  const [connected, setConnected] = useState(false);
  const [url, setUrl] = useState('');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    calendarIsConnected().then(setConnected).catch(() => undefined);
  }, []);

  const connect = async () => {
    setBusy(true);
    setError(null);
    try {
      const count = await calendarSetUrl(url);
      setConnected(true);
      setUrl('');
      toast.success(`Calendar connected: ${count} meeting${count === 1 ? '' : 's'} in the next two weeks`);
      onConnected?.();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  };

  if (connected) {
    return (
      <div className="flex items-center justify-between rounded-md border border-cf-line px-3 py-2.5 text-[14px]">
        <span className="text-cf-ink">Google Calendar is connected.</span>
        <button
          onClick={() => calendarClear().then(() => setConnected(false))}
          className="text-[13px] text-cf-muted underline underline-offset-4 hover:text-cf-ink"
        >
          Disconnect
        </button>
      </div>
    );
  }

  return (
    <div>
      <ol className="mb-3 list-decimal space-y-1 pl-5 text-[13px] text-cf-muted">
        <li>Open Google Calendar on the web, then Settings.</li>
        <li>Under &ldquo;Settings for my calendars&rdquo; pick your calendar and open &ldquo;Integrate calendar&rdquo;.</li>
        <li>Copy &ldquo;Secret address in iCal format&rdquo; and paste it here.</li>
      </ol>
      <div className="flex gap-2">
        <input
          value={url}
          onChange={(e) => setUrl(e.target.value)}
          placeholder="https://calendar.google.com/calendar/ical/.../basic.ics"
          aria-label="Secret iCal address"
          className="min-w-0 flex-1 rounded-md border border-cf-line bg-cf-bg px-2.5 py-1.5 text-[13px] text-cf-ink placeholder:text-cf-muted focus:outline-none"
        />
        <button
          onClick={connect}
          disabled={busy || !url.trim()}
          className="rounded-md bg-cf-primary px-3 py-1.5 text-[13px] font-medium text-cf-primary-ink disabled:opacity-50"
        >
          {busy ? 'Checking...' : 'Connect'}
        </button>
      </div>
      <p className="mt-2 text-[12px] text-cf-muted">
        Read-only. The address is stored in Windows Credential Manager, because anyone with it can read your calendar.
      </p>
      {error && <p role="alert" className="mt-2 text-[13px] text-cf-danger">{error}</p>}
    </div>
  );
}

export function VocabularyEditor({ compact = false }: { compact?: boolean }) {
  const [text, setText] = useState('');
  const [saved, setSaved] = useState('');

  useEffect(() => {
    getVocabulary()
      .then((v) => {
        setText(v);
        setSaved(v);
      })
      .catch(() => undefined);
  }, []);

  const save = async () => {
    try {
      const n = await setVocabulary(text);
      setSaved(text);
      toast.success(`${n} word${n === 1 ? '' : 's'} saved`);
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div>
      <textarea
        value={text}
        onChange={(e) => setText(e.target.value)}
        placeholder={'One per line, as they are written:\nscalia.studio\nScaleform\nKatrin Weber'}
        aria-label="Vocabulary"
        className={`w-full rounded-md border border-cf-line bg-cf-bg px-2.5 py-2 text-[13px] text-cf-ink placeholder:text-cf-muted focus:outline-none ${compact ? 'min-h-[96px]' : 'min-h-[140px]'}`}
      />
      <div className="mt-2 flex items-center gap-3">
        <button
          onClick={save}
          disabled={text === saved}
          className="rounded-md bg-cf-primary px-3 py-1.5 text-[13px] font-medium text-cf-primary-ink disabled:opacity-50"
        >
          Save words
        </button>
        <span className="text-[12px] text-cf-muted">
          Names, companies, products and websites. Cornflake fixes them when speech recognition mishears them, and keeps the original.
        </span>
      </div>
    </div>
  );
}
