import { useEffect, useRef, useState } from "react";

export function useElementSize<T extends HTMLElement>() {
  const ref = useRef<T>(null);
  const [size, setSize] = useState({
    width: 0,
    height: 0,
  });

  useEffect(() => {
    if (!ref.current) return;

    const element = ref.current;

    const observer = new ResizeObserver((entries) => {
      const entry = entries[0];

      if (entry) {
        const { width, height } = entry.contentRect;
        setSize({ width, height });
      }
    });

    observer.observe(element);

    return () => {
      observer.disconnect();
    };
  }, []);

  return [size, ref] satisfies [typeof size, typeof ref];
}
