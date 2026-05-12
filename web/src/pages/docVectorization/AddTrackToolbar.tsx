import PolygonIcon from "../../assets/polygon.svg?react";
import CheckCircleIcon from "../../assets/check-circle.svg?react";
import UndoLeftSquareIcon from "../../assets/undo-left-square.svg?react";
import CloseCircleIcon from "../../assets/close-circle.svg?react";
import { IconButton } from "../../shared/button/Button";
import "./AddTrackToolbar.css";
import { Transition } from "../../shared/ui/Transition";
import { useTransition } from "../../shared/hooks/useTransition";
import type { XY } from "../../entities/LatLng";

interface AddTrackToolbarProps {
  draftTrack: XY[] | null;
  onUndo: () => void;
  onDone: () => void;
  className?: string;
}

export function AddTrackToolbar({
  onUndo,
  onDone,
  className,
  draftTrack,
}: AddTrackToolbarProps) {
  const [shouldShowToolbar, setShouldShowToolbar] = useTransition(false);

  return (
    <div className={`${className} add-tracking-toolbar-root`}>
      <Transition
        If={shouldShowToolbar}
        className="slide-up-down"
        enterAfter={100}
      >
        <div className={`add-tracking-toolbar`}>
          <IconButton
            onClick={onDone}
            disabled={!draftTrack || draftTrack.length < 3}
          >
            <CheckCircleIcon />
          </IconButton>
          <IconButton
            onClick={onUndo}
            disabled={!draftTrack || draftTrack.length === 0}
          >
            <UndoLeftSquareIcon />
          </IconButton>
          <IconButton onClick={() => setShouldShowToolbar(false)}>
            <CloseCircleIcon />
          </IconButton>
        </div>
      </Transition>
      <Transition
        Ifnot={shouldShowToolbar}
        className="slide-up-down"
        enterAfter={100}
      >
        <IconButton onClick={() => setShouldShowToolbar(true)}>
          <PolygonIcon />
        </IconButton>
      </Transition>
    </div>
  );
}
