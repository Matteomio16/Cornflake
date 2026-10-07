'use client';

import { useEffect, useState } from 'react';
import { formatCost, getSetting, setSetting, TRANSLATION_TARGETS } from '@/lib/cornflake';

/// Target language picker plus an action button. The parent decides what "translate" means
/// (whole saved meeting, or live lines while recording).
export function TranslateBar({
  label,
  busy,
  active,
  cost,
  onTranslate,
  onStop,
}: {
  label: string;
  busy: boolean;
  active: boolean;
  cost: number | null;
  onTranslate: (target: string) => void;
  onStop?: () => void;
}) {
  const [target, setTarget] = useState('English');

  useEffect(() => {
    getSetting('translation_target').then((t) => t && setTarget(t)).catch(() => undefined);
  }, []);

  return (
    <div className="flex items-center gap-2 px-4 py-2 border-b border-gray-100 text-sm">
      <label htmlFor="translate-target" className="text-gray-600">Translate to</label>
      <select
        id="translate-target"
        value={target}
        onChange={(e) => {
          setTarget(e.target.value);
          setSetting('translation_target', e.target.value).catch(() => undefined);
        }}
        className="border border-gray-300 rounded px-1.5 py-0.5"
      >
        {TRANSLATION_TARGETS.map((t) => (
          <option key={t}>{t}</option>
        ))}
      </select>
      {active && onStop ? (
        <button onClick={onStop} className="underline">Stop</button>
      ) : (
        <button onClick={() => onTranslate(target)} disabled={busy} className="underline disabled:opacity-50">
          {busy ? 'Translating...' : label}
        </button>
      )}
      {cost != null && <span className="ml-auto text-xs text-gray-500">{formatCost(cost)}</span>}
    </div>
  );
}
