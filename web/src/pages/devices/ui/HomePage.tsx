import { useState, useEffect, useCallback } from "react";
import Map, {
  Layer,
  Marker,
  Source,
  type LngLat,
  type MapLayerMouseEvent,
} from "react-map-gl/maplibre"; // Import from maplibre
import { API_HOST, deviceAPI } from "../../../core";
import { useSearchParams } from "react-router-dom";
import "maplibre-gl/dist/maplibre-gl.css"; // Import the maplibre CSS
import "./HomePage.css";
import { useSaveTrack } from "../hooks/useSaveTrack";
import { useTracks } from "../hooks/useTracks";

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

interface LiveStatus {
  fixType: string | null;
  altitudeMtrs: number | null;
  satellites: number;
  latLng: { longitude: number; latitude: number } | null;
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
    fixType: null,
    altitudeMtrs: null,
    satellites: 0,
    latLng: null,
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
                case "latLngUpdate":
                  setLiveStatus((prev) => ({
                    ...prev,
                    latLng: {
                      longitude: parsedData.data.longitude,
                      latitude: parsedData.data.latitude,
                    },
                  }));
                  break;
                case "fixUpdate":
                  setLiveStatus((prev) => ({
                    ...prev,
                    fixType: parsedData.data.fixType,
                  }));
                  break;
                case "statusUpdate":
                  setLiveStatus((prev) => ({
                    ...prev,
                    satellites: parsedData.data.satellites,
                  }));
                  break;
                case "altitudeUpdate":
                  setLiveStatus((prev) => ({
                    ...prev,
                    altitudeMtrs: parsedData.data.altitudeMtrs,
                  }));
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
        {(liveStatus.fixType || "N/A").toUpperCase()}{" "}
        <span className="seperator">|</span>
        <span className="emoji">🛰️</span>
        {liveStatus.satellites || 0}
        <span className="seperator">|</span>
        <span className="emoji">🗻</span>
        {(liveStatus.altitudeMtrs || 0).toFixed(2)}m
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
            longitude={liveStatus.latLng?.longitude || 0}
            latitude={liveStatus.latLng?.latitude || 0}
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
          const latLng = liveStatus.latLng;
          if (latLng) {
            setViewState((prev) => ({
              ...prev,
              longitude: latLng.longitude,
              latitude: latLng.latitude,
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
