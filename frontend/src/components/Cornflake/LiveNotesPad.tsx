'use client';

import { useEffect, useState } from 'react';
import { readLiveNotes, writeLiveNotes } from '@/lib/cornflake';
import { ConsentMessage } from './ConsentMessage';

export function LiveNotesPad({ isRecording }: { isRecording: boolean }) {
  const [notes, setNotes] = useState('');

  useEffect(() => {
    setNotes(readLiveNotes());
  }, [isRecording]);

  return (
    <section aria-label="My notes">
      <div className="mb-2 flex items-center justify-between">
        <span className="text-[12px] text-cf-muted">Your notes stay exactly as you type them</span>
        <ConsentMessage />
      </div>
      <textarea
        value={notes}
        onChange={(e) => {
          setNotes(e.target.value);
          writeLiveNotes(e.target.value);
        }}
        placeholder={'Write notes...\n- pricing 40k?\n- follow up on churn'}
        aria-label="My notes"
        spellCheck
        className="min-h-[50vh] w-full resize-none bg-transparent text-[16px] leading-relaxed text-cf-ink placeholder:text-cf-muted focus:outline-none"
      />
    </section>
  );
}
