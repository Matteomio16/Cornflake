'use client';

import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { getExportDir, setExportDir } from '@/lib/cornflake';

export function ExportSettings() {
  const [dir, setDir] = useState('');

  useEffect(() => {
    getExportDir().then(setDir).catch(() => undefined);
  }, []);

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-1">Markdown export</h3>
      <p className="text-sm text-gray-600 mb-4">
        Every meeting is written here as a markdown file with front matter, one folder per space, with the transcript
        in a separate file. Notes are exported each time they are generated.
      </p>
      <div className="flex gap-2">
        <input
          value={dir}
          onChange={(e) => setDir(e.target.value)}
          aria-label="Export folder"
          className="flex-1 text-sm font-mono border border-gray-300 rounded px-2 py-1.5"
        />
        <button
          onClick={() =>
            setExportDir(dir)
              .then(() => toast.success('Export folder saved'))
              .catch((e) => toast.error(String(e)))
          }
          className="text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white"
        >
          Save
        </button>
      </div>
    </div>
  );
}
