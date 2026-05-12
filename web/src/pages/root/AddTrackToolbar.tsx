import { deviceAPI, SOUNDS } from "../../core";
import type { LiveStatusAPI } from "../../entities/LiveStatusAPI";
import type { LatLng } from "../../entities/LatLng";
import PolygonIcon from "../../assets/polygon.svg?react";
import LocationBookmarkIcon from "../../assets/location-bookmark.svg?react";
import AddLocationIcon from "../../assets/add-location2.svg?react";
import CheckCircleIcon from "../../assets/check-circle.svg?react";
import UndoLeftSquareIcon from "../../assets/undo-left-square.svg?react";
import CloseCircleIcon from "../../assets/close-circle.svg?react";
import { IconButton } from "../../shared/button/Button";
import "./AddTrackToolbar.css";
import { Transition } from "../../shared/ui/Transition";
import { useTransition } from "../../shared/hooks/useTransition";
import { useCallback, useState } from "react";
import { BookmarkFormDialog } from "./BookmarkFormDialog";
import type { TrackInfo } from "../../entities/types/TrackInfo";
import { TrackFormDialog } from "./TrackFormDialog";

interface AddTrackToolbarProps {
  draftTrack: TrackInfo | null;
  onUndo: () => void;
  onUpdate: (point: LatLng) => void;
  onBookMarkLocation: (name: string) => void;
  onSaveTrackConfirmation: (params: { name: string }) => void;
  className?: string;
}

export function AddTrackToolbar({
  onUndo,
  onUpdate,
  className,
  draftTrack,
  onBookMarkLocation,
  onSaveTrackConfirmation,
}: AddTrackToolbarProps) {
  const [shouldShowToolbar, setShouldShowToolbar] = useTransition(false);
  const [shouldShowBookmarkDialog, setShouldShowBookmarkDialog] =
    useState(false);
  const [shouldShowTrackDialog, setShouldShowTrackDialog] = useState(false);

  const handleBookMarkLocation = useCallback(
    (name: string) => {
      onBookMarkLocation(name);
      setShouldShowBookmarkDialog(false);
    },
    [onBookMarkLocation],
  );
  const handleTrackSaveConfirmation = useCallback(
    (params: { name: string }) => {
      onSaveTrackConfirmation(params);
      setShouldShowTrackDialog(false);
    },
    [onSaveTrackConfirmation],
  );
  const handleAddCorner = async () => {
    try {
      const liveStatus = await deviceAPI
        .get("latlng", { timeout: 3000 })
        .json<LiveStatusAPI>();
      if (liveStatus.latitude === null || liveStatus.longitude === null) {
        return;
      }
      const latlng = {
        lat: liveStatus.latitude,
        lng: liveStatus.longitude,
      } satisfies LatLng;
      onUpdate(latlng);
      SOUNDS.click.play();
    } catch (error) {
      console.error("Error fetching live status:", error);
      SOUNDS.error.play();
    }
  };

  return (
    <div className={`${className} add-tracking-toolbar-root`}>
      <Transition
        If={shouldShowToolbar}
        className="slide-up-down"
        enterAfter={100}
      >
        <div className={`add-tracking-toolbar`}>
          <IconButton onClick={handleAddCorner}>
            <AddLocationIcon />
          </IconButton>
          <IconButton
            onClick={() => {
              if (!draftTrack || draftTrack.points.length < 3) return;
              setShouldShowTrackDialog(true);
            }}
            disabled={!draftTrack || draftTrack.points.length < 3}
          >
            <CheckCircleIcon />
          </IconButton>
          <IconButton
            onClick={onUndo}
            disabled={!draftTrack || draftTrack.points.length === 0}
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
        <div className="main-toolbar">
          <IconButton onClick={() => setShouldShowToolbar(true)}>
            <PolygonIcon />
          </IconButton>
          <IconButton onClick={() => setShouldShowBookmarkDialog(true)}>
            <LocationBookmarkIcon />
          </IconButton>
        </div>
      </Transition>
      {shouldShowBookmarkDialog && (
        <BookmarkFormDialog
          onConfirm={handleBookMarkLocation}
          onClose={() => setShouldShowBookmarkDialog(false)}
        />
      )}
      {shouldShowTrackDialog && (
        <TrackFormDialog
          onConfirm={handleTrackSaveConfirmation}
          onClose={() => setShouldShowTrackDialog(false)}
        />
      )}
    </div>
  );
}
