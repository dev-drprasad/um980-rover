import type { LiveStatusAPI } from "../../entities/LiveStatusAPI";
import SatelliteDishIcon from "../../assets/satellite-dish.svg?react";
import LocationLockIcon from "../../assets/location-lock.svg?react";
import "./LiveStatus.css";
import { AltitudeIcon } from "../../assets/AltitudeIcon";
import { Transition } from "../../shared/ui/Transition";
import { useTransition } from "../../shared/hooks/useTransition";

interface LiveStatusProps {
  liveStatus: LiveStatusAPI;
  isLoading: boolean;
}

export function LiveStatus({ liveStatus, isLoading }: LiveStatusProps) {
  const [shouldShowContent, setShouldShowContent] = useTransition(false);
  return (
    <div className="live-status">
      <div
        className="header"
        role="button"
        onClick={() => setShouldShowContent(!shouldShowContent.value)}
      >
        <RTKStatus fixType={liveStatus.fix_type} />
        <span className="middle">
          <span>
            <SatelliteDishIcon className="label icon" />
            <span className="value">
              {liveStatus.fix_satellites || 0}/{liveStatus.satellites || 0}
            </span>
          </span>
          <span className="seperator">|</span>
          <span>
            <span className="label">R₉₅</span>
            <span className="value">
              {(liveStatus.accuracy?.twice_drms || 0).toFixed(2)}m
            </span>
          </span>
          <span className="seperator">|</span>
          <span>
            <span className="label">hdop </span>{" "}
            <span className="value">{(liveStatus.hdop || 0).toFixed(2)}</span>
          </span>
        </span>
        {isLoading && <span className="spinner"></span>}
      </div>
      <Transition If={shouldShowContent} className="v-grow">
        <div className="content">
          <span className="seperator">|</span>
          <AltitudeIcon className="icon" />
          {(liveStatus.altitude || 0).toFixed(2)}m
          <span>
            <span className="err">σₗₐₜ</span>
            {(liveStatus.accuracy?.lat_err || 0).toFixed(2)}m
          </span>
          <span>
            <span className="err">σₗₒₙ</span>
            {(liveStatus.accuracy?.lon_err || 0).toFixed(2)}m
          </span>
          <span className="seperator">|</span>
          <span className="hdop err">HDOP </span>{" "}
          {(liveStatus.hdop || 0).toFixed(2)}
          <span className="seperator">|</span>
          <span className="drms">r₆₈</span>
          {(liveStatus.accuracy?.drms || 0).toFixed(2)}m
          <span className="seperator">|</span>
          <span className="drms">r₉₅</span>
          {(liveStatus.accuracy?.twice_drms || 0).toFixed(2)}m
        </div>
      </Transition>
    </div>
  );
}

function RTKStatus({ fixType }: { fixType: string | null }) {
  let className = "gps";
  switch (fixType) {
    case "DGps":
      className = "dgps";
      break;
    case "Gps":
      className = "gps";
      break;
    case "Rtk":
      className = "rtk";
      break;
    case "FloatRtk":
      className = "float-rtk";
      break;
  }
  return <LocationLockIcon className={`location-lock-icon ${className}`} />;
}
