import { useEffect } from "react";

interface ToastProps {
  message: string | null;
  onDone: () => void;
}

export function Toast({ message, onDone }: ToastProps) {
  useEffect(() => {
    if (!message) return;
    const timer = window.setTimeout(onDone, 3800);
    return () => window.clearTimeout(timer);
  }, [message, onDone]);

  if (!message) return null;
  return (
    <div className="toast" role="status" aria-live="polite">
      {message}
    </div>
  );
}
