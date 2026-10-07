'use client';

import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { getSetting, setSetting } from '@/lib/cornflake';

const SUGGESTED = [
  { id: '', label: 'Same as the notes model' },
  { id: 'z-ai/glm-5.3-flash', label: 'GLM 5.3 Flash (paid, very cheap)' },
  { id: 'inclusionai/ling-3.1-flash', label: 'Ling 3.1 Flash (free; the provider may log or train on what you send)' },
];

export function TranslationSettings() {
  const [model, setModel] = useState('');

  useEffect(() => {
    getSetting('translation_model').then((m) => setModel(m ?? '')).catch(() => undefined);
  }, []);

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-1">Translation</h3>
      <p className="text-sm text-gray-600 mb-4">
        Model used to translate transcripts, through the same provider as your notes.
      </p>
      <select
        value={SUGGESTED.some((s) => s.id === model) ? model : '__custom'}
        onChange={(e) => e.target.value !== '__custom' && setModel(e.target.value)}
        aria-label="Translation model"
        className="w-full text-sm border border-gray-300 rounded px-2 py-1.5"
      >
        {SUGGESTED.map((s) => (
          <option key={s.id} value={s.id}>{s.label}</option>
        ))}
        <option value="__custom">Other model id</option>
      </select>
      <input
        value={model}
        onChange={(e) => setModel(e.target.value)}
        placeholder="Model id, e.g. z-ai/glm-5.3-flash"
        aria-label="Translation model id"
        className="mt-2 w-full text-sm font-mono border border-gray-300 rounded px-2 py-1.5"
      />
      <button
        onClick={() =>
          setSetting('translation_model', model)
            .then(() => toast.success('Translation model saved'))
            .catch((e) => toast.error(String(e)))
        }
        className="mt-3 text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white"
      >
        Save
      </button>
    </div>
  );
}
