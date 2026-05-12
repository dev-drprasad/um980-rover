import { Image as KonvaImage } from "react-konva";

import { useCallback, useMemo } from "react";

export function KonvaImageWrapper({
  image,
  stageWidth,
  stageHeight,
}: {
  image: HTMLImageElement;
  stageWidth: number;
  stageHeight: number;
}) {
  const calculateFitParams = useCallback(
    (stageWidth: number, stageHeight: number) => {
      const imageWidth = image.width;
      const imageHeight = image.height;

      // Calculate the scale to fit the image entirely within the canvas dimensions
      const scale = Math.min(
        stageWidth / imageWidth,
        stageHeight / imageHeight,
      );

      // Calculate the centered position
      const x = (stageWidth - imageWidth * scale) / 2;
      const y = (stageHeight - imageHeight * scale) / 2;

      return { x, y, scale: { x: scale, y: scale } };
    },
    [image],
  );

  const fitParams = useMemo(
    () => calculateFitParams(stageWidth, stageHeight),
    [calculateFitParams, stageWidth, stageHeight],
  );

  return <KonvaImage image={image} {...fitParams} />;
}
