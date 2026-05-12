import {
  Stage,
  Image as KonvaImage,
  Layer,
  Line,
  Circle,
  Text,
} from "react-konva";
import Konva from "konva";
import { useEffect, useRef, useState } from "react";
import "./DocVectorization.css";
import type { XY } from "../../entities/LatLng";

interface CanvasProps {
  width: number;
  height: number;
  draftTrack: XY[];
  imageUrl: string;
  defaultMeasurementScale: { value: number; unit: string } | null;
  setDraftTrack: React.Dispatch<React.SetStateAction<XY[]>>;
}

export function Canvas({
  width,
  draftTrack,
  defaultMeasurementScale,
  setDraftTrack,
  imageUrl,
}: CanvasProps) {
  // 🔒 Fixed aspect ratio (example: 16:9)
  const aspectRatio = 16 / 9;
  const stageWidth = width || 800;
  const stageHeight = stageWidth / aspectRatio;
  const [img] = useImage(imageUrl);
  const stageRef = useRef<Konva.Stage | null>(null);

  const [scale, setScale] = useState(1);

  // =============================
  // 🧠 Click → normalized canvas coords
  // =============================
  const handleClick = () => {
    const stage = stageRef.current;
    if (!stage) return;
    const pointer = stage.getPointerPosition();
    if (!pointer) return;

    const transform = stage.getAbsoluteTransform().copy();
    transform.invert();

    const pos = transform.point(pointer);

    const normX = pos.x / stageWidth;
    const normY = pos.y / stageHeight;

    setDraftTrack((prev) => [...prev, { x: normX, y: normY }]);
  };

  // =============================
  // 🔍 Zoom (mouse wheel)
  // =============================
  const handleWheel = (e: Konva.KonvaEventObject<WheelEvent>) => {
    e.evt.preventDefault();
    const stage = stageRef.current;
    if (!stage) return;

    const oldScale = scale;
    const scaleBy = 1.05;

    const pointer = stage.getPointerPosition();
    if (!pointer) return;

    const mousePointTo = {
      x: (pointer.x - stage.x()) / oldScale,
      y: (pointer.y - stage.y()) / oldScale,
    };

    const newScale = e.evt.deltaY > 0 ? oldScale / scaleBy : oldScale * scaleBy;

    setScale(newScale);

    stage.scale({ x: newScale, y: newScale });

    const newPos = {
      x: pointer.x - mousePointTo.x * newScale,
      y: pointer.y - mousePointTo.y * newScale,
    };

    stage.position(newPos);
    stage.batchDraw();
  };

  // =============================
  // 🖼 Fit image (contain + center)
  // =============================
  let imgX = 0,
    imgY = 0,
    imgW = 0,
    imgH = 0;

  if (img) {
    const scale = Math.min(stageWidth / img.width, stageHeight / img.height);

    imgW = img.width * scale;
    imgH = img.height * scale;

    imgX = (stageWidth - imgW) / 2;
    imgY = (stageHeight - imgH) / 2;
  }

  // =============================
  // 🔁 Convert normalized → canvas
  // =============================
  const canvasPoints = draftTrack.map((p) => ({
    x: p.x * stageWidth,
    y: p.y * stageHeight,
  }));

  return (
    <Stage
      width={stageWidth}
      height={stageHeight}
      ref={stageRef}
      onClick={handleClick}
      onWheel={handleWheel}
      draggable
      style={{ background: "#111" }}
    >
      <Layer>
        {/* Image */}
        {img && (
          <KonvaImage
            image={img}
            x={imgX}
            y={imgY}
            width={imgW}
            height={imgH}
          />
        )}

        {/* Points */}
        {canvasPoints.map((p, i) => (
          <Circle key={i} x={p.x} y={p.y} radius={4} fill="red" />
        ))}

        {/* Polygon */}
        {canvasPoints.length > 1 && (
          <>
            <Line
              points={canvasPoints.flatMap((p) => [p.x, p.y])}
              stroke="red"
              closed={false}
            />
            {canvasPoints.map((p, i, all) =>
              i < all.length - 1 ? (
                <Text
                  key={p.x + "-" + p.y}
                  text={
                    defaultMeasurementScale
                      ? `${(getDistance(p, all[i + 1]) * defaultMeasurementScale.value).toFixed(2)}${defaultMeasurementScale.unit}`
                      : ""
                  }
                  fill={"red"}
                  {...getTextPosAlongLine(canvasPoints[i], canvasPoints[i + 1])}
                />
              ) : null,
            )}
          </>
        )}
      </Layer>
    </Stage>
  );
}

function useImage(url: string): [HTMLImageElement | null] {
  const [image, setImage] = useState<HTMLImageElement | null>(null);

  useEffect(() => {
    const img = new Image();
    img.src = url;
    img.onload = () => setImage(img);
  }, [url]);

  return [image];
}

function getDistance(
  p1: { x: number; y: number },
  p2: { x: number; y: number },
) {
  const distance = Math.hypot(p1.x - p2.x, p1.y - p2.y);
  return distance;
}

function getTextPosAlongLine(
  p1: { x: number; y: number },
  p2: { x: number; y: number },
) {
  const midX = (p1.x + p2.x) / 2;
  const midY = (p1.y + p2.y) / 2;

  // angle in degrees
  const angle = (Math.atan2(p2.y - p1.y, p2.x - p1.x) * 180) / Math.PI;
  return { x: midX, y: midY, angle };
}
