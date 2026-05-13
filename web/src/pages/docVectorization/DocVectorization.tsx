import { useStrictMode } from "react-konva";
import { useElementSize } from "../../shared/hooks/useElementSize";
import {
  useCallback,
  useEffect,
  useRef,
  useState,
  type SubmitEvent,
} from "react";
import "./DocVectorization.css";
import { AddTrackToolbar } from "./AddTrackToolbar";
import type { XY } from "../../entities/LatLng";
import MeasurementScaleIcon from "../../assets/measure-map.svg?react";
import CloseIcon from "../../assets/close.svg?react";
import { FilledButton, IconButton } from "../../shared/button/Button";
import { FormInput } from "../../shared/formInputText";
import { Canvas } from "./Canvas";

export function DocVectorizationCanvas({ fileUrl }: { fileUrl: string }) {
  const [size, ref] = useElementSize<HTMLDivElement>();
  const dialogRef = useRef<HTMLDialogElement>(null);
  useStrictMode(true);
  const [draftTrack, setDraftTrack] = useState<XY[]>([]);
  const [isSettingMeasurementScale, setIsSettingMeasurementScale] =
    useState(false);
  const [defaultMeasurementScale, setDefaultMeasurementScale] = useState<{
    value: number;
    unit: string;
  } | null>(null);

  const handleEndTrack = useCallback(() => {
    console.log("Track completed:", draftTrack);
    setDraftTrack((prev) => [...prev, prev[0]]);
  }, [draftTrack]);

  const handleUndoPoint = useCallback(() => {
    setDraftTrack((prev) => prev.slice(0, -1));
  }, []);

  const handleDialogClose = useCallback(() => {
    setIsSettingMeasurementScale(false);
    setDraftTrack([]);
  }, []);

  const handleSubmit = useCallback(
    (event: SubmitEvent<HTMLFormElement>) => {
      event.preventDefault();
      const formData = new FormData(event.currentTarget);
      const value = formData.get("scale");
      const unit = formData.get("unit");

      if (
        typeof value === "string" &&
        typeof unit === "string" &&
        draftTrack.length === 2
      ) {
        const aspectRatio = 16 / 9;
        const stageWidth = size.width || 800;
        const stageHeight = stageWidth / aspectRatio;
        const diff = getDistance(
          { x: draftTrack[0].x * stageWidth, y: draftTrack[0].y * stageHeight },
          { x: draftTrack[1].x * stageWidth, y: draftTrack[1].y * stageHeight },
        );
        const scaleValue = parseFloat(value) / diff;
        setDefaultMeasurementScale({ value: scaleValue, unit });
        setDraftTrack([]);
      }
    },
    [draftTrack, size.width],
  );

  useEffect(() => {
    if (isSettingMeasurementScale) {
      if (draftTrack.length === 2) dialogRef.current?.showModal();
    } else {
      dialogRef.current?.close();
    }
  }, [isSettingMeasurementScale, draftTrack.length]);

  return (
    <div ref={ref} className="doc-vectorization-page">
      <Canvas
        imageUrl={fileUrl}
        width={size.width}
        height={size.height}
        draftTrack={draftTrack}
        setDraftTrack={setDraftTrack}
        defaultMeasurementScale={defaultMeasurementScale}
      />
      <AddTrackToolbar
        draftTrack={draftTrack}
        className="add-track-toolbar"
        onDone={handleEndTrack}
        onUndo={handleUndoPoint}
      />
      <IconButton
        className="measuremen-scale-button"
        selected={isSettingMeasurementScale}
        onClick={() => setIsSettingMeasurementScale((prev) => !prev)}
      >
        <MeasurementScaleIcon />
      </IconButton>
      {isSettingMeasurementScale && draftTrack.length === 2 && (
        <dialog className="dialog" ref={dialogRef} onClose={handleDialogClose}>
          <form className="set-scale-dialog-content" onSubmit={handleSubmit}>
            <div className="inputs">
              <label>
                <span className="input-label">Scale:</span>
                <input
                  tabIndex={0}
                  name="scale"
                  type="number"
                  autoComplete="off"
                  autoFocus
                ></input>
              </label>
              <label>
                <span className="input-label">Units:</span>
                <select name="unit" tabIndex={1}>
                  <option value="meters">meters</option>
                  <option value="feet">feet</option>
                </select>
              </label>
              <FilledButton type="submit" tabIndex={2}>
                Confirm
              </FilledButton>
            </div>
          </form>
          <IconButton
            className="close-dialog-button"
            onClick={handleDialogClose}
            tabIndex={4}
          >
            <CloseIcon />
          </IconButton>
        </dialog>
      )}
    </div>
  );
}

function getDistance(
  p1: { x: number; y: number },
  p2: { x: number; y: number },
) {
  const distance = Math.hypot(p1.x - p2.x, p1.y - p2.y);
  return distance;
}

export function DocVectorizationPage() {
  const [fileUrl, setFileUrl] = useState<string | null>(null);

  const handleSubmit = (e: SubmitEvent<HTMLFormElement>) => {
    e.preventDefault();
    const formData = new FormData(e.currentTarget);
    const file = formData.get("image") as File;
    setFileUrl(URL.createObjectURL(file));
  };

  useEffect(() => {
    return () => {
      if (fileUrl) {
        URL.revokeObjectURL(fileUrl);
      }
    };
  }, [fileUrl]);

  if (fileUrl) {
    return <DocVectorizationCanvas fileUrl={fileUrl} />;
  }
  return (
    <form className="docVectorizationUploadForm" onSubmit={handleSubmit}>
      <FormInput
        label="Image File"
        name="image"
        type="file"
        accept="image/*"
        required
      />
      <FilledButton type="submit">Upload</FilledButton>
    </form>
  );
}
