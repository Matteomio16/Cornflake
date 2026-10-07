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
    <section className="flex flex-col w-2/5 min-w-[280px] border-r border-gray-200 bg-white">
      <header className="px-5 pt-4 pb-2">
        <div className="flex items-baseline justify-between">
          <h2 className="text-sm font-semibold text-gray-900">My notes</h2>
          <span className="text-xs text-gray-500">Kept exactly as you type them</span>
        </div>
        <div className="mt-1">
          <ConsentMessage />
        </div>
      </header>
      <textarea
        value={notes}
        onChange={(e) => {
          setNotes(e.target.value);
          writeLiveNotes(e.target.value);
        }}
        placeholder={'Rough bullets, one per line.\n- pricing 40k?\n- follow up on churn'}
        aria-label="My notes"
        spellCheck
        className="flex-1 resize-none px-5 pb-5 text-[15px] leading-relaxed text-gray-900 placeholder:text-gray-400 focus:outline-none"
      />
    </section>
  );
}
