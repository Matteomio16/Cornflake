import React from 'react';
import { Button } from '@/components/ui/button';
import { OnboardingContainer } from '../OnboardingContainer';
import { useOnboarding } from '@/contexts/OnboardingContext';
import { CalendarConnect, VocabularyEditor } from '@/components/Cornflake/CalendarConnect';

/// Step 2: calendar and personal vocabulary, in Granola's order (calendar right after the welcome).
export function SetupOverviewStep() {
  const { goNext } = useOnboarding();

  return (
    <OnboardingContainer
      title="Connect your calendar"
      description="See your next meetings on Home and start notes for them in one click. Attendee names also help Cornflake spell them right."
      step={2}
      totalSteps={3}
    >
      <div className="cf-surface mx-auto flex w-full max-w-md flex-col gap-8">
        <CalendarConnect />
        <div>
          <h3 className="mb-2 text-[14px] font-medium text-cf-ink">Words Cornflake should know</h3>
          <VocabularyEditor compact />
        </div>
        <div className="space-y-2">
          <Button onClick={goNext} className="h-11 w-full bg-cf-primary text-cf-primary-ink hover:opacity-90">
            Continue
          </Button>
          <p className="text-center text-[12px] text-cf-muted">Both are optional and can be changed later in Settings.</p>
        </div>
      </div>
    </OnboardingContainer>
  );
}
