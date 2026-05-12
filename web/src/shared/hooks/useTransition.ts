import { useCallback, useEffect, useState } from "react";

export function useTransition(initialValue: boolean = false) {
  const delay = 500;
  const [value, setValue] = useState(initialValue);
  const [tempValue, setTempValue] = useState(initialValue);

  const startTransition = useCallback((value: boolean) => {
    setTempValue(value);
    // setIsTransitionEnded(false);
  }, []);

  useEffect(() => {
    let timeoutId: number | undefined = undefined;

    timeoutId = setTimeout(() => {
      setValue(tempValue);
    }, delay);
    return () => {
      if (timeoutId) {
        clearTimeout(timeoutId);
      }
    };
  }, [tempValue]);

  return [
    { value, shouldTransition: tempValue, delay },
    startTransition,
  ] as const;
}
