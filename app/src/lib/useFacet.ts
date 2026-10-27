import { createContext, useCallback, useContext, useEffect, useState } from "react";
import type { Account, AccountState, Api, AppInfo } from "./api";

export const ApiContext = createContext<Api | null>(null);

export function useApi(): Api {
  const api = useContext(ApiContext);
  if (!api) throw new Error("ApiContext missing");
  return api;
}

export interface FacetData {
  accounts: Account[];
  states: Record<string, AccountState>;
  info: AppInfo | null;
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

/** Accounts, their live state (pushed by the core every 2 s on change) and app info. */
export function useFacetData(api: Api): FacetData {
  const [accounts, setAccounts] = useState<Account[]>([]);
  const [states, setStates] = useState<Record<string, AccountState>>({});
  const [info, setInfo] = useState<AppInfo | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const applyStates = useCallback((list: AccountState[]) => {
    setStates(Object.fromEntries(list.map((s) => [s.id, s])));
  }, []);

  const refresh = useCallback(async () => {
    try {
      const [a, s] = await Promise.all([api.listAccounts(), api.accountStates()]);
      setAccounts(a);
      applyStates(s);
      setError(null);
    } catch (e) {
      setError(
        typeof e === "string"
          ? `Facet could not read its accounts file: ${e} Check that ~/.config/facet/profiles.toml is readable, then try again.`
          : "Facet could not read its accounts file (~/.config/facet/profiles.toml). Check that it is readable, then try again.",
      );
    } finally {
      setLoading(false);
    }
  }, [api, applyStates]);

  useEffect(() => {
    void refresh();
    void api.appInfo().then(setInfo, () => setInfo(null));
    let unlisten: (() => void) | undefined;
    let cancelled = false;
    void api.onAccountStates(applyStates).then((fn) => {
      if (cancelled) fn();
      else unlisten = fn;
    });
    return () => {
      cancelled = true;
      unlisten?.();
    };
  }, [api, refresh, applyStates]);

  return { accounts, states, info, loading, error, refresh };
}
