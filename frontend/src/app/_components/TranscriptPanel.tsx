import { VirtualizedTranscriptView } from '@/components/VirtualizedTranscriptView';
import { PermissionWarning } from '@/components/PermissionWarning';
import { Button } from '@/components/ui/button';
import { ButtonGroup } from '@/components/ui/button-group';
import { Copy, GlobeIcon } from 'lucide-react';
import { useTranscripts } from '@/contexts/TranscriptContext';
import { useConfig } from '@/contexts/ConfigContext';
import { useRecordingState } from '@/contexts/RecordingStateContext';
import { usePermissionCheck } from '@/hooks/usePermissionCheck';
import { ModalType } from '@/hooks/useModalState';
import { useIsLinux } from '@/hooks/usePlatform';
import { useEffect, useMemo, useRef, useState } from 'react';
import { TranslateBar } from '@/components/Cornflake/TranslateBar';
import { translateTexts } from '@/lib/cornflake';

// Live translation batches new final lines this often, to keep calls few and cheap
const LIVE_TRANSLATE_INTERVAL_MS = 8000;

/**
 * TranscriptPanel Component
 *
 * Displays transcript content with controls for copying and language settings.
 * Uses TranscriptContext, ConfigContext, and RecordingStateContext internally.
 */

interface TranscriptPanelProps {
  // indicates stop-processing state for transcripts; derived from backend statuses.
  isProcessingStop: boolean;
  isStopping: boolean;
  showModal: (name: ModalType, message?: string) => void;
}

export function TranscriptPanel({
  isProcessingStop,
  isStopping,
  showModal
}: TranscriptPanelProps) {
  // Contexts
  const { transcripts, transcriptContainerRef, copyTranscript } = useTranscripts();
  const { transcriptModelConfig } = useConfig();
  const { isRecording, isPaused } = useRecordingState();
  const { checkPermissions, isChecking, hasSystemAudio, hasMicrophone } = usePermissionCheck();
  const isLinux = useIsLinux();

  const [liveTarget, setLiveTarget] = useState<string | null>(null);
  const [liveTranslations, setLiveTranslations] = useState<Map<string, string>>(new Map());
  const [liveCost, setLiveCost] = useState<number | null>(null);
  const inFlight = useRef(new Set<string>());
  // Refs so the timer below is not restarted by every new transcript line
  const transcriptsRef = useRef(transcripts);
  const translatedRef = useRef(liveTranslations);
  transcriptsRef.current = transcripts;
  translatedRef.current = liveTranslations;

  useEffect(() => {
    if (!liveTarget) return;
    const tick = async () => {
      const pending = transcriptsRef.current.filter(
        (t) => !t.is_partial && !translatedRef.current.has(t.id) && !inFlight.current.has(t.id)
      );
      if (!pending.length) return;
      pending.forEach((t) => inFlight.current.add(t.id));
      try {
        const res = await translateTexts(pending.map((t) => t.text), liveTarget);
        setLiveTranslations((prev) => {
          const next = new Map(prev);
          pending.forEach((t, i) => {
            const line = res.lines[i];
            if (line) next.set(t.id, line);
          });
          return next;
        });
        setLiveCost((c) => (res.cost_usd == null ? c : (c ?? 0) + res.cost_usd));
      } catch (e) {
        console.error('Live translation failed:', e);
      } finally {
        pending.forEach((t) => inFlight.current.delete(t.id));
      }
    };
    void tick();
    const timer = setInterval(tick, LIVE_TRANSLATE_INTERVAL_MS);
    return () => clearInterval(timer);
  }, [liveTarget]);

  // Convert transcripts to segments for virtualized view
  const segments = useMemo(() =>
    transcripts.map(t => ({
      id: t.id,
      timestamp: t.audio_start_time ?? 0,
      endTime: t.audio_end_time,
      text: t.text,
      confidence: t.confidence,
      speaker: t.speaker,
      translation: liveTranslations.get(t.id),
    })),
    [transcripts, liveTranslations]
  );

  return (
    <div ref={transcriptContainerRef} className="w-full border-r border-gray-200 bg-white flex flex-col overflow-y-auto">
      {/* Title area - Sticky header */}
      <div className="sticky top-0 z-10 bg-white p-4 border-gray-200">
        <div className="flex flex-col space-y-3">
          <div className="flex  flex-col space-y-2">
            <div className="flex justify-center  items-center space-x-2">
              <ButtonGroup>
                {transcripts?.length > 0 && (
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={copyTranscript}
                    title="Copy Transcript"
                  >
                    <Copy />
                    <span className='hidden md:inline'>
                      Copy
                    </span>
                  </Button>
                )}
                {transcriptModelConfig.provider === "localWhisper" &&
                  <Button
                    variant="outline"
                    size="sm"
                    onClick={() => showModal('languageSettings')}
                    title="Language"
                  >
                    <GlobeIcon />
                    <span className='hidden md:inline'>
                      Language
                    </span>
                  </Button>
                }
              </ButtonGroup>
            </div>
          </div>
        </div>
      </div>

      {transcripts.length > 0 && (
        <TranslateBar
          label="Translate live"
          busy={false}
          active={liveTarget !== null}
          cost={liveCost}
          onTranslate={(target) => setLiveTarget(target)}
          onStop={() => setLiveTarget(null)}
        />
      )}

      {/* Permission Warning - Not needed on Linux */}
      {!isRecording && !isChecking && !isLinux && (
        <div className="flex justify-center px-4 pt-4">
          <PermissionWarning
            hasMicrophone={hasMicrophone}
            hasSystemAudio={hasSystemAudio}
            onRecheck={checkPermissions}
            isRechecking={isChecking}
          />
        </div>
      )}

      {/* Transcript content */}
      <div className="pb-20">
        <div className="flex justify-center">
          <div className="w-2/3 max-w-[750px]">
            <VirtualizedTranscriptView
              segments={segments}
              isRecording={isRecording}
              isPaused={isPaused}
              isProcessing={isProcessingStop}
              isStopping={isStopping}
              enableStreaming={isRecording}
              showConfidence={true}
            />
          </div>
        </div>
      </div>
    </div>
  );
}
