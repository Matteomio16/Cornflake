'use client';

import { useState } from 'react';
import { toast } from 'sonner';

const MESSAGES: Record<string, string> = {
  EN: "Quick heads-up: I'm taking notes with an app that transcribes this call on my computer. Nothing joins the call. Is that okay with everyone?",
  DE: 'Kurzer Hinweis: Ich mache Notizen mit einer App, die dieses Gespräch auf meinem Computer transkribiert. Es tritt niemand dem Call bei. Ist das für alle in Ordnung?',
  FR: "Petite précision : je prends des notes avec une application qui transcrit cet appel sur mon ordinateur. Rien ne rejoint l'appel. Est-ce que tout le monde est d'accord ?",
  IT: "Una breve nota: prendo appunti con un'app che trascrive questa chiamata sul mio computer. Nessuno si aggiunge alla chiamata. Va bene per tutti?",
};

/// Copies a short consent message to paste into the meeting chat.
export function ConsentMessage() {
  const [lang, setLang] = useState('EN');

  return (
    <div className="flex items-center gap-1 text-xs text-gray-600">
      <button
        onClick={() =>
          navigator.clipboard
            .writeText(MESSAGES[lang])
            .then(() => toast.success('Consent message copied. Paste it into the meeting chat.'))
            .catch(() => toast.error('Could not copy to the clipboard'))
        }
        className="underline"
        title={MESSAGES[lang]}
      >
        Copy consent message
      </button>
      <select
        value={lang}
        onChange={(e) => setLang(e.target.value)}
        aria-label="Consent message language"
        className="border border-gray-200 rounded px-1 py-0.5"
      >
        {Object.keys(MESSAGES).map((l) => (
          <option key={l}>{l}</option>
        ))}
      </select>
    </div>
  );
}
