'use client';

import { useEffect, useState } from 'react';
import { toast } from 'sonner';
import { getRoutingProjects, RoutingProject, saveRoutingProjects } from '@/lib/cornflake';

export function RoutingSettings() {
  const [projects, setProjects] = useState<RoutingProject[]>([]);

  useEffect(() => {
    getRoutingProjects().then(setProjects).catch(() => undefined);
  }, []);

  const update = (id: string, description: string) =>
    setProjects((ps) => ps.map((p) => (p.id === id ? { ...p, description } : p)));

  return (
    <div className="bg-white rounded-lg border border-gray-200 p-6 shadow-sm">
      <h3 className="text-lg font-semibold text-gray-900 mb-1">Routing to Claude Code projects</h3>
      <p className="text-sm text-gray-600 mb-4">
        Describe the projects meetings should be filed into. Only projects with a description are considered. Nothing is
        written until you approve a preview on a meeting.
      </p>
      <div className="space-y-3">
        {projects.map((p) => (
          <label key={p.id} className="block">
            <span className="text-sm font-medium text-gray-900">{p.name}</span>
            <input
              value={p.description}
              onChange={(e) => update(p.id, e.target.value)}
              placeholder="What this project is about, who is involved"
              className="mt-1 w-full text-sm border border-gray-300 rounded px-2 py-1.5"
            />
          </label>
        ))}
        {projects.length === 0 && <p className="text-sm text-gray-500">No Claude Code projects found on this computer.</p>}
      </div>
      <button
        onClick={() =>
          saveRoutingProjects(projects)
            .then(() => toast.success('Routing projects saved'))
            .catch((e) => toast.error(String(e)))
        }
        className="mt-4 text-sm font-medium px-3 py-1.5 rounded bg-gray-900 text-white"
      >
        Save
      </button>
    </div>
  );
}
