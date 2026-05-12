import { cloneElement, isValidElement, useEffect, useState } from "react";

export function Transition({
  If,
  Ifnot,
  children,
  className,
  enterAfter,
}: {
  If?: { value: boolean; shouldTransition: boolean; delay: number };
  Ifnot?: { value: boolean; shouldTransition: boolean; delay: number };
  children: React.ReactElement;
  className: "slide-up-down" | "v-grow";
  enterAfter?: number;
}) {
  const [mounted, setMounted] = useState(false);

  const value = If ? If.value : Ifnot ? !Ifnot.value : true;
  const _shouldTransition = If
    ? If.shouldTransition
    : Ifnot
      ? !Ifnot.shouldTransition
      : false;
  const delay = If ? If.delay : Ifnot ? Ifnot.delay : 0;

  const [shouldTransition, setShouldTransition] = useState(_shouldTransition);

  useEffect(() => {
    let timerId: number | undefined = undefined;
    if (enterAfter !== undefined && _shouldTransition) {
      timerId = setTimeout(() => {
        setShouldTransition(_shouldTransition);
      }, enterAfter + delay);
    } else {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setShouldTransition(_shouldTransition);
    }
    return () => {
      if (timerId !== undefined) {
        clearTimeout(timerId);
      }
    };
  }, [_shouldTransition, delay, enterAfter]);

  useEffect(() => {
    if (!value && shouldTransition) {
      const requestId = requestAnimationFrame(() => {
        setMounted(true);
      });
      return () => {
        cancelAnimationFrame(requestId);
      };
    }
    if (!value && !shouldTransition) {
      // eslint-disable-next-line react-hooks/set-state-in-effect
      setMounted(false);
    }
  }, [shouldTransition, value]);

  if (!value && !shouldTransition) {
    return null;
  }
  if (!isValidElement<{ className?: string }>(children)) {
    return children;
  }

  if (!value && shouldTransition && !mounted) {
    return cloneElement(children, {
      className: `${children.props.className} tsn-${className} exit`,
    });
  }

  if (!value && shouldTransition && mounted) {
    return cloneElement(children, {
      className: `${children.props.className} tsn-${className} enter`,
    });
  }
  if (value && shouldTransition) {
    return cloneElement(children, {
      className: `${children.props.className} tsn-${className} enter`,
    });
  }
  if (value && !shouldTransition) {
    return cloneElement(children, {
      className: `${children.props.className} tsn-${className} exit`,
    });
  }
}
