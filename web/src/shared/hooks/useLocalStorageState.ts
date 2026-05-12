import { useCallback, useState } from "react";

export function useLocalStorageState<T>(key: string, defaultValue: T) {
  const [state, setState] = useState<T>(() => {
    const storedValue = localStorage.getItem(key);
    return storedValue ? JSON.parse(storedValue) : defaultValue;
  });

  const setLocalStorageState = useCallback(
    (value: T | ((prevState: T) => T)) => {
      const newValue =
        typeof value === "function"
          ? (value as (prevState: T) => T)(state)
          : value;
      setState(newValue);
      localStorage.setItem(key, JSON.stringify(newValue));
    },
    [key, state],
  );

  return [state, setLocalStorageState] as const;
}
