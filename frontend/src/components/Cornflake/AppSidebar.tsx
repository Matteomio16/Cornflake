'use client';

import { useState } from 'react';
import { usePathname, useRouter, useSearchParams } from 'next/navigation';
import { Home, Plus, Search, Settings, FolderClosed, Radio } from 'lucide-react';
import { toast } from 'sonner';
import { useSidebar } from '@/components/Sidebar/SidebarProvider';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { clearPendingNote, createSpace } from '@/lib/cornflake';

/// Starts a fresh note: the live page picks up the flag and starts recording.
export function startNewNote(router: ReturnType<typeof useRouter>) {
  clearPendingNote();
  sessionStorage.setItem('autoStartRecording', 'true');
  router.push('/live');
}

function NavRow({
  active,
  onClick,
  icon: Icon,
  label,
  trailing,
}: {
  active?: boolean;
  onClick: () => void;
  icon: React.ComponentType<{ className?: string }>;
  label: string;
  trailing?: React.ReactNode;
}) {
  return (
    <button
      onClick={onClick}
      aria-current={active ? 'page' : undefined}
      className={`flex w-full items-center gap-2.5 rounded-md px-2.5 py-1.5 text-[14px] text-left transition-colors ${
        active ? 'bg-cf-hover text-cf-ink font-medium' : 'text-cf-muted hover:bg-cf-hover hover:text-cf-ink'
      }`}
    >
      <Icon className="h-4 w-4 shrink-0" />
      <span className="truncate">{label}</span>
      {trailing}
    </button>
  );
}

export function AppSidebar() {
  const router = useRouter();
  const pathname = usePathname();
  const params = useSearchParams();
  const { spaces, refetchMeetings } = useSidebar();
  const { isRecording } = useRecordingState();
  const [query, setQuery] = useState(params.get('q') ?? '');
  const [addingSpace, setAddingSpace] = useState(false);
  const [spaceName, setSpaceName] = useState('');
  const activeSpace = pathname === '/' ? params.get('space') : null;

  const submitSpace = async () => {
    const name = spaceName.trim();
    setAddingSpace(false);
    setSpaceName('');
    if (!name) return;
    try {
      const s = await createSpace(name);
      await refetchMeetings();
      router.push(`/?space=${s.id}`);
    } catch (e) {
      toast.error(`Could not create the space: ${e}`);
    }
  };

  return (
    <aside className="flex h-screen w-60 shrink-0 flex-col border-r border-cf-line bg-cf-side">
      <div className="flex items-center gap-2 px-4 pt-4 pb-3">
        <img src="/logo-collapsed.png" alt="" className="h-5 w-5 rounded" />
        <span className="text-[15px] font-semibold text-cf-ink">Cornflake</span>
      </div>

      <div className="px-3">
        {isRecording ? (
          <button
            onClick={() => router.push('/live')}
            className="flex w-full items-center justify-center gap-2 rounded-md border border-cf-line bg-cf-bg px-3 py-2 text-[14px] font-medium text-cf-ink hover:bg-cf-hover"
          >
            <span className="relative flex h-2 w-2">
              <span className="absolute inline-flex h-full w-full animate-ping rounded-full bg-cf-gold opacity-60" />
              <span className="relative inline-flex h-2 w-2 rounded-full bg-cf-gold" />
            </span>
            Back to live note
          </button>
        ) : (
          <button
            onClick={() => startNewNote(router)}
            className="flex w-full items-center justify-center gap-2 rounded-md bg-cf-primary px-3 py-2 text-[14px] font-medium text-cf-primary-ink hover:opacity-90"
            title="New note (Ctrl+Alt+R)"
          >
            <Plus className="h-4 w-4" />
            New note
          </button>
        )}

        <form
          className="mt-3 flex items-center gap-2 rounded-md border border-cf-line bg-cf-bg px-2.5 py-1.5"
          onSubmit={(e) => {
            e.preventDefault();
            router.push(query.trim() ? `/?q=${encodeURIComponent(query.trim())}` : '/');
          }}
        >
          <Search className="h-3.5 w-3.5 text-cf-muted" />
          <input
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search notes"
            aria-label="Search notes and transcripts"
            className="w-full bg-transparent text-[13px] text-cf-ink placeholder:text-cf-muted focus:outline-none"
          />
        </form>
      </div>

      <nav className="cf-scroll mt-4 flex-1 overflow-y-auto px-3" aria-label="Main">
        <NavRow icon={Home} label="Home" active={pathname === '/' && !activeSpace && !params.get('q')} onClick={() => router.push('/')} />
        {isRecording && <NavRow icon={Radio} label="Live note" active={pathname === '/live'} onClick={() => router.push('/live')} />}

        <div className="mt-5 mb-1 flex items-center justify-between px-2.5">
          <span className="text-[12px] font-medium text-cf-muted">Spaces</span>
          <button
            onClick={() => setAddingSpace(true)}
            className="rounded p-0.5 text-cf-muted hover:bg-cf-hover hover:text-cf-ink"
            aria-label="New space"
            title="New space"
          >
            <Plus className="h-3.5 w-3.5" />
          </button>
        </div>
        {spaces.map((s) => (
          <NavRow
            key={s.id}
            icon={FolderClosed}
            label={s.name}
            active={activeSpace === s.id}
            onClick={() => router.push(`/?space=${s.id}`)}
          />
        ))}
        {addingSpace && (
          <input
            autoFocus
            value={spaceName}
            onChange={(e) => setSpaceName(e.target.value)}
            onBlur={submitSpace}
            onKeyDown={(e) => {
              if (e.key === 'Enter') submitSpace();
              if (e.key === 'Escape') {
                setAddingSpace(false);
                setSpaceName('');
              }
            }}
            placeholder="Space name"
            aria-label="New space name"
            className="mt-1 w-full rounded-md border border-cf-line bg-cf-bg px-2.5 py-1.5 text-[14px] text-cf-ink focus:outline-none"
          />
        )}
      </nav>

      <div className="border-t border-cf-line px-3 py-3">
        <NavRow icon={Settings} label="Settings" active={pathname === '/settings'} onClick={() => router.push('/settings')} />
      </div>
    </aside>
  );
}
