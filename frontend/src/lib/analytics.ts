// Telemetry is removed. This stub keeps legacy call sites compiling and sends nothing.
const noop = (..._args: any[]): Promise<any> => Promise.resolve(undefined);

export const Analytics: Record<string, (...args: any[]) => Promise<any>> = new Proxy({}, { get: () => noop });

export default Analytics;
