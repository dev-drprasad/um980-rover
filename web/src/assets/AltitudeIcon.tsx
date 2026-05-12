import MountainIcon from "../assets/mountain.svg?react";
import UpArrowIcon from "../assets/up-arrow.svg?react";

export function AltitudeIcon({ className }: { className?: string }) {
  return (
    <span
      className={className}
      style={{
        position: "relative",
        display: "inline-block",
        width: "1em",
        height: "1em",
      }}
    >
      <MountainIcon style={{ fontSize: "0.9em" }} />
      <UpArrowIcon
        style={{ position: "absolute", top: 0, right: 0, fontSize: "0.4em" }}
      />
    </span>
  );
}
