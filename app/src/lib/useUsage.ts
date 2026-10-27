import { useCallback, useEffect, useRef, useState } from "react";
import { errorText } from "./api";
import { useApi } from "./useFacet";
import type { UsageReport } from "./usage";

/** Usage for the last `days` days; refreshes every minute while shown. */
export function useUsage(days: number) {
  const api = useApi();
  const [report, setReport] = useState<UsageReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const live = useRef(true);

  const refresh = useCallback(async () => {
    setLoading(true);
    try {
      const r = await api.usageReport(days);
      if (live.current) {
        setReport(r);
        setError(null);
      }
    } catch (e) {
      if (live.current) setError(errorText(e));
    } finally {
      if (live.current) setLoading(false);
    }
  }, [api, days]);

  useEffect(() => {
    live.current = true;
    void refresh();
    const t = setInterval(() => void refresh(), 60_000);
    return () => {
      live.current = false;
      clearInterval(t);
    };
  }, [refresh]);

  return { report, error, loading, refresh };
}
