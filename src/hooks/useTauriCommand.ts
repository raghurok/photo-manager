import { invoke } from "@tauri-apps/api/core";
import { useCallback, useEffect, useRef, useState } from "react";

export function useTauriCommand<T>(cmd: string, args?: object, deps?: unknown[]) {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const argsKey = JSON.stringify(args);
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const depsKey = JSON.stringify(deps ?? [argsKey]);

  useEffect(() => {
    let cancelled = false;
    setLoading(true);
    setError(null);
    invoke<T>(cmd, args)
      .then((d) => { if (!cancelled) setData(d); })
      .catch((e) => { if (!cancelled) setError(String(e)); })
      .finally(() => { if (!cancelled) setLoading(false); });
    return () => { cancelled = true; };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cmd, depsKey]);

  const refetch = useCallback(() => {
    setLoading(true);
    invoke<T>(cmd, args)
      .then(setData)
      .catch((e) => setError(String(e)))
      .finally(() => setLoading(false));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [cmd, argsKey]);

  return { data, loading, error, refetch };
}

export function usePolling<T>(cmd: string, intervalMs: number, enabled = true) {
  const [data, setData] = useState<T | null>(null);
  const timerRef = useRef<ReturnType<typeof setInterval> | null>(null);

  useEffect(() => {
    if (!enabled) return;
    const fetch = () => invoke<T>(cmd).then(setData).catch(() => null);
    fetch();
    timerRef.current = setInterval(fetch, intervalMs);
    return () => { if (timerRef.current) clearInterval(timerRef.current); };
  }, [cmd, intervalMs, enabled]);

  return data;
}
