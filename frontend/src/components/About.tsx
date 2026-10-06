import React, { useEffect, useState } from "react";
import { getVersion } from '@tauri-apps/api/app';

export function About() {
    const [version, setVersion] = useState<string>('');

    useEffect(() => {
        getVersion().then(setVersion).catch(console.error);
    }, []);

    return (
        <div className="p-4 space-y-4 h-[80vh] overflow-y-auto">
            <div className="text-center">
                <h1 className="text-xl font-semibold text-gray-900">Open Cornflake</h1>
                {version && <p className="text-sm text-gray-500">Version {version}</p>}
            </div>
            <p className="text-sm text-gray-700">
                Local meeting notes. Audio is captured and transcribed on this computer. Nothing is sent anywhere
                except the notes requests you choose to make to your configured AI provider.
            </p>
            <p className="text-sm text-gray-700">
                Recording laws differ by country. In Germany and many other places you generally need the consent of
                all participants before recording a conversation. You are responsible for obtaining it.
            </p>
            <div className="pt-2 border-t border-gray-200 text-xs text-gray-500">
                Open Cornflake is MIT licensed and based on Meetily, copyright 2024 Zackriya Solutions.
            </div>
        </div>
    );
}
