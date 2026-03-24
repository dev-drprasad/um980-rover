import { useState, useEffect, useCallback } from "react";
import Map, {
  Layer,
  Marker,
  Source,
  type LngLat,
  type MapLayerMouseEvent,
} from "react-map-gl/maplibre"; // Import from maplibre
import { API_HOST, deviceAPI } from "../../core";
import { useSearchParams } from "react-router-dom";
import "maplibre-gl/dist/maplibre-gl.css"; // Import the maplibre CSS
import "./HomePage.css";
import { useSaveTrack } from "../devices/hooks/useSaveTrack";
import { useTracks } from "../devices/hooks/useTracks";

const style = {
  version: 8,
  sources: {
    "osm-tiles": {
      type: "raster",
      tiles: ["https://tile.openstreetmap.org/{z}/{x}/{y}.png"],
      tileSize: 256,
      attribution:
        '© <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> contributors',
    },
  },
  layers: [
    {
      id: "osm-tiles",
      type: "raster",
      source: "osm-tiles",
      minzoom: 0,
      maxzoom: 19,
    },
  ],
};

const jsonString = JSON.stringify(style);
const blob = new Blob([jsonString], { type: "application/json" });
const styleURL = URL.createObjectURL(blob);

interface HorizontalAccuracy {
  lat_err: number;
  lon_err: number;
  drms: number;
  twice_drms: number;
}

interface LiveStatus {
  latitude: number | null;
  longitude: number | null;
  altitude: number | null;
  speed_over_ground: number | null;
  fix_type: string | null;
  satellites: number;
  fix_satellites: number | null;
  accuracy: HorizontalAccuracy | null;
  hdop: number | null;
  vdop: number | null;
  pdop: number | null;
}

export function HomePage() {
  // You can load your GeoJSON data here, e.g., via an import or fetch
  const [geojsonData, setGeojsonData] =
    useState<GeoJSON.FeatureCollection | null>(null);
  const [viewState, setViewState] = useState({
    longitude: -0.09,
    latitude: 51.505,
    zoom: 18,
  });
  const { save, status } = useSaveTrack();
  const { data: tracks, refetch } = useTracks();

  const [draftTrack, setDraftTrack] = useState<LngLat[] | null>(null);
  const [liveStatus, setLiveStatus] = useState<LiveStatus>({
    latitude: null,
    longitude: null,
    altitude: null,
    speed_over_ground: null,
    fix_type: null,
    satellites: 0,
    fix_satellites: null,
    accuracy: null,
    hdop: null,
    vdop: null,
    pdop: null,
  });
  const [searchParams] = useSearchParams();
  const lpmNo = searchParams.get("lpm") || "";

  const handleMapClick = useCallback((event: MapLayerMouseEvent) => {
    console.log(event.lngLat);
    setDraftTrack((prev) => (prev ? [...prev, event.lngLat] : [event.lngLat]));
  }, []);

  const handleEndTrack = useCallback(() => {
    setDraftTrack((prev) => {
      if (!prev) return null;
      if (prev.length < 3) return prev; // Not enough points to form a track
      const firstPoint = prev.at(0);
      if (!firstPoint) return prev;

      return prev ? [...prev, firstPoint] : null;
    });
  }, []);

  useEffect(() => {
    if (!draftTrack) return;
    if (draftTrack.length < 3) return;
    if (draftTrack.at(0) === draftTrack.at(-1)) {
      save(draftTrack.map((lngLat) => [lngLat.lng, lngLat.lat]));
    }
  }, [draftTrack, save]);

  useEffect(() => {
    if (status === "success" && draftTrack) {
      // @eslint-disable-next-line
      setDraftTrack(null);
      refetch();
    }
  }, [draftTrack, refetch, status]);

  // Example of fetching GeoJSON data from a URL
  useEffect(() => {
    (async () => {
      const geoJSON = await deviceAPI
        .get<GeoJSON.FeatureCollection>(`layers?lpm=${lpmNo}`)
        .json();
      setGeojsonData(geoJSON);
      // const [lng, lat] = geoJSON
      //   ? geoJSON.features[0].geometry.type === "Polygon"
      //     ? geoJSON.features[0].geometry.coordinates[0][0]
      //     : [0, 0]
      //   : [0, 0];
      // console.log("geojsonData :>> ", geoJSON);
      // setViewState((prev) => ({ ...prev, longitude: lng, latitude: lat }));
    })();
  }, [lpmNo]);

  useEffect(() => {
    (async () => {
      const socket = new WebSocket(`ws://${API_HOST}/ws`);
      socket.addEventListener("message", (event) => {
        if (typeof event.data === "string") {
          try {
            const parsedData = JSON.parse(event.data);
            if ("data" in parsedData && "event" in parsedData) {
              switch (parsedData.event) {
                case "live_status":
                  setLiveStatus(parsedData.data as LiveStatus);
                  break;
                default:
                  console.warn("Unknown event type:", parsedData.event);
              }
            }
          } catch (error) {
            console.error("Error parsing WebSocket message:", error);
          }
        } else {
          console.warn("Received non-string message:", event.data);
        }
      });
    })();
  }, []);

  const draftTrackGeoJSON = draftTrack ? trackToGeoJSON(draftTrack) : null;
  const trackGeoJSON = tracks
    ? tracks.map((track) => trackToGeoJSON(track))
    : null;
  return (
    <div className="home-page-container">
      <div className="status">
        {(liveStatus.fix_type || "N/A").toUpperCase()}{" "}
        <span className="seperator">|</span>
        <span className="emoji">🛰️</span>
        {liveStatus.fix_satellites || 0}/{liveStatus.satellites || 0}
        <span className="seperator">|</span>
        <span className="emoji">🗻</span>
        {(liveStatus.altitude || 0).toFixed(2)}m
        <span className="seperator">|</span>
        <span className="err">σₗₐₜ</span>
        {(liveStatus.accuracy?.lat_err || 0).toFixed(2)}m
        <span className="seperator">|</span>
        <span className="err">σₗₒₙ</span>
        {(liveStatus.accuracy?.lon_err || 0).toFixed(2)}m
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

      <Map
        {...viewState}
        onMove={(evt) => setViewState(evt.viewState)}
        mapStyle={styleURL}
        onClick={handleMapClick}
        maxZoom={18}
      >
        <Source
          id="geojson-data"
          type="geojson"
          data={geojsonData || { type: "FeatureCollection", features: [] }}
        >
          <Layer
            id="geojson-layer"
            type="fill"
            paint={{
              "fill-color": "#007cbf",
              "fill-opacity": 0.1,
              "fill-outline-color": "#007cbf",
            }}
          />
          <Layer
            id="geojson-layer-line"
            type="line"
            paint={{
              "line-color": "#007cbf",
              "line-width": 2,
            }}
          />
        </Source>
        {location && (
          <Marker
            longitude={liveStatus.longitude || 0}
            latitude={liveStatus.latitude || 0}
            anchor="center" // Positions the bottom of the marker at the coordinate
          >
            {/* You can use a custom component for the marker, e.g., a blue dot SVG */}
            <div className="live-location-marker"></div>
          </Marker>
        )}
        {draftTrackGeoJSON && (
          <Source id="draft-track" type="geojson" data={draftTrackGeoJSON}>
            <Layer
              id="draft-track-layer"
              type="line"
              paint={{
                "line-color": "#ff0000",
                "line-width": 2,
              }}
            />
            <Layer
              id="draft-track-corners"
              type="circle"
              paint={{
                "circle-color": "#ffffff",
                "circle-stroke-color": "#ff0000",
                "circle-radius": 4,
                "circle-stroke-width": 2,
              }}
            />
          </Source>
        )}
        {trackGeoJSON?.map((geoJSON, index) => (
          <Source
            id={`track-${index}`}
            type="geojson"
            data={geoJSON}
            key={index}
          >
            <Layer
              id={`track-layer-${index}`}
              type="line"
              paint={{
                "line-color": "#0000ff",
                "line-width": 2,
              }}
            />
          </Source>
        ))}
      </Map>
      <button className="end-track-button" onClick={handleEndTrack}>
        end
      </button>
      <button
        onClick={() => {
          const latitude = liveStatus.latitude;
          const longitude = liveStatus.longitude;
          if (latitude !== null && longitude !== null) {
            setViewState((prev) => ({
              ...prev,
              longitude,
              latitude,
            }));
          }
        }}
        className="move-to-live-location"
      >
        ⌖
      </button>
    </div>
  );
}

function trackToGeoJSON(track: LngLat[]): GeoJSON.FeatureCollection {
  return {
    type: "FeatureCollection",
    features: [
      {
        type: "Feature",
        geometry: {
          type: "LineString",
          coordinates: track.map((lngLat) => [lngLat.lng, lngLat.lat]),
        },
        properties: {},
      },
    ],
  };
}
