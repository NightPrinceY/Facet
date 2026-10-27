import { createContext, useCallback, useContext, useEffect, useState, type ReactNode } from "react";

// One sentence, past tense, 3.8 s, polite live region (DESIGN-SYSTEM.md Toast).
const ToastContext = createContext<(message: string) => void>(() => {});

export function ToastProvider({ children }: { children: ReactNode }) {
  const [message, setMessage] = useState<string | null>(null);
  const [key, setKey] = useState(0);
  const show = useCallback((m: string) => {
    setMessage(m);
    setKey((k) => k + 1);
  }, []);
  useEffect(() => {
    if (!message) return;
    const t = setTimeout(() => setMessage(null), 3800);
    return () => clearTimeout(t);
  }, [message, key]);
  return (
    <ToastContext.Provider value={show}>
      {children}
      <div aria-live="polite" role="status">
        {message && (
          <div className="toast" key={key}>
            {message}
          </div>
        )}
      </div>
    </ToastContext.Provider>
  );
}

export function useToast() {
  return useContext(ToastContext);
}
