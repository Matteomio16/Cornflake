'use client';

import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { getWebhooks, setWebhooks } from '@/lib/cornflake';

export function WebhookSettings() {
  const [urls, setUrls] = useState('');

  useEffect(() => {
    getWebhooks().then(setUrls).catch(() => undefined);
  }, []);

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-1">Webhooks</h3>
      <p className="text-sm text-gray-600 mb-4">
        When notes are generated, Cornflake sends them as JSON (title, summary, decisions, action items, markdown) to
        each URL below. One URL per line. This sends meeting content to those addresses, so only add ones you trust.
      </p>
      <textarea
        value={urls}
        onChange={(e) => setUrls(e.target.value)}
        aria-label="Webhook URLs"
        placeholder="https://hooks.example.com/cornflake"
        className="w-full min-h-[80px] text-sm font-mono border border-gray-300 rounded px-2 py-1.5"
      />
      <button
        onClick={() =>
          setWebhooks(urls)
            .then((n) => toast.success(`${n} webhook${n === 1 ? '' : 's'} saved`))
            .catch((e) => toast.error(String(e)))
        }
        className="mt-3 text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white"
      >
        Save
      </button>
    </div>
  );
}
