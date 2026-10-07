import React, { useState } from 'react';
import { Lock, FileText, Users } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';

export function WelcomeStep() {
  const { goNext } = useOnboarding();
  const [consentAcknowledged, setConsentAcknowledged] = useState(false);

  const points = [
    {
      icon: Lock,
      text: 'Audio and transcripts stay on this computer. No bot joins your call.',
    },
    {
      icon: FileText,
      text: 'Notes are written by the AI provider you choose, only when you generate them. That sends the transcript and your notes to that provider.',
    },
    {
      icon: Users,
      text: 'Recording laws differ by country. In Germany and many other places you generally need everyone in the conversation to agree before you record it. This is general information, not legal advice.',
    },
  ];

  return (
    <OnboardingContainer
      title="Welcome to Cornflake"
      description="Meeting notes from your own bullets and a local transcript."
      step={1}
      hideProgress={true}
    >
      <div className="flex flex-col items-center space-y-8">
        <div className="w-full max-w-md bg-white rounded-lg border border-gray-200 shadow-sm p-6 space-y-4">
          {points.map((point, index) => {
            const Icon = point.icon;
            return (
              <div key={index} className="flex items-start gap-3">
                <div className="flex-shrink-0 mt-0.5">
                  <div className="w-5 h-5 rounded-full bg-gray-100 flex items-center justify-center">
                    <Icon className="w-3 h-3 text-gray-700" />
                  </div>
                </div>
                <p className="text-sm text-gray-700 leading-relaxed">{point.text}</p>
              </div>
            );
          })}
        </div>

        <label className="w-full max-w-md flex items-start gap-3 text-sm text-gray-800">
          <input
            type="checkbox"
            checked={consentAcknowledged}
            onChange={(e) => setConsentAcknowledged(e.target.checked)}
            className="mt-1"
          />
          <span>I will tell the people I record and get their consent where the law requires it.</span>
        </label>

        <div className="w-full max-w-xs space-y-3">
          <Button
            onClick={goNext}
            disabled={!consentAcknowledged}
            className="w-full h-11 bg-gray-900 hover:bg-gray-800 text-white disabled:opacity-50"
          >
            Get started
          </Button>
        </div>
      </div>
    </OnboardingContainer>
  );
}
