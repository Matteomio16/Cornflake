'use client';

import { CalendarConnect, VocabularyEditor } from './CalendarConnect';

export function CalendarSettings() {
  return (
    <>
      <div id="calendar" className="cf-surface rounded-lg border border-cf-line p-6">
        <h3 className="mb-1 text-lg font-semibold text-cf-ink">Calendar</h3>
        <p className="mb-4 text-sm text-cf-muted">Your next meetings on Home, with attendee names for better spelling.</p>
        <CalendarConnect />
      </div>
      <div id="vocabulary" className="cf-surface rounded-lg border border-cf-line p-6">
        <h3 className="mb-1 text-lg font-semibold text-cf-ink">Vocabulary</h3>
        <p className="mb-4 text-sm text-cf-muted">Words Cornflake should spell right in transcripts and notes.</p>
        <VocabularyEditor />
      </div>
    </>
  );
}
