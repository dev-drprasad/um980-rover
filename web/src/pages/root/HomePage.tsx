import {
  useState,
  useEffect,
  useCallback,
  useMemo,
  type ReactNode,
} from "react";
import Map, {
  Layer,
  Marker,
  Popup,
  Source,
  type MapLayerMouseEvent,
} from "react-map-gl/maplibre"; // Import from maplibre
import { API_HOST, deviceAPI, SOUNDS } from "../../core";
import { useSearchParams } from "react-router-dom";
import { useSaveTrack } from "../devices/hooks/useSaveTrack";
import { useTracks } from "../devices/hooks/useTracks";
import { AddTrackToolbar } from "./AddTrackToolbar";
import type { LiveStatusAPI } from "../../entities/LiveStatusAPI";
import type { LatLng } from "../../entities/LatLng";
import { FilledIconButton } from "../../shared/button/Button";
import LocationIcon from "../../assets/location.svg?react";
import "maplibre-gl/dist/maplibre-gl.css"; // Import the maplibre CSS
import "./HomePage.css";
import { LiveStatus } from "./LiveStatus";
import type { TrackInfo } from "../../entities/types/TrackInfo";
import { useDraftTrack } from "../../entities/hooks/useDraftTrack";
import { useUndoDraftPoint } from "../devices/hooks/useUndoDraftPoint";
import { useMoveDraftToTrack } from "../devices/hooks/useMoveDraftToTrack";

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

export function HomePage() {
  // You can load your GeoJSON data here, e.g., via an import or fetch
  const [geojsonData, setGeojsonData] =
    useState<GeoJSON.FeatureCollection | null>(null);
  const [viewState, setViewState] = useState({
    longitude: -0.09151008366981728,
    latitude: 51.504856420091755,
    zoom: 18, // should be less than osm maxzoom
  });

  const { save, status } = useSaveTrack();
  const { data: tracks, refetch } = useTracks();
  const { data: draftTrack, refetch: refectDraft } = useDraftTrack();
  const { save: moveDraftToTrack, status: moveDraftToTrackStatus } =
    useMoveDraftToTrack();
  const { save: undoDraftPoint, status: undoDraftPointStatus } =
    useUndoDraftPoint();

  // const [draftTrack, setDraftTrack] = useDraftTrack();
  const [liveStatus, setLiveStatus] = useState<LiveStatusAPI>({
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
  const [liveStatusLoading, setLiveStatusLoading] = useState(true);
  const [searchParams] = useSearchParams();
  const lpmNo = searchParams.get("lpm") || "";
  const [popupInfo, setPopupInfo] = useState<{
    coordinates: LatLng;
    content: ReactNode;
  } | null>(null);

  const handleAddDraftPoint = useCallback(
    (latLng: LatLng) => {
      (async function () {
        await save({
          name: "draft",
          type: "draft",
          coordinates: [
            ...(draftTrack?.points || []).map(
              (point) => [point.lat, point.lng] satisfies [number, number],
            ),
            [latLng.lat, latLng.lng],
          ],
        });
        refectDraft();
      })();
    },
    [draftTrack, refectDraft, save],
  );

  const handleBookMarkLocation = useCallback(
    (name: string) => {
      if (liveStatus.latitude && liveStatus.longitude) {
        save({
          name,
          type: "bookmark",
          coordinates: [liveStatus.latitude, liveStatus.longitude],
        });
      }
    },
    [liveStatus.latitude, liveStatus.longitude, save],
  );

  const handleMapClick = useCallback((event: MapLayerMouseEvent) => {
    const { lngLat, features } = event;

    if (!features?.length) return;

    // there will be multiple `features` if one polygon inside another
    // or line overlaps with polygon.
    // TODO: need to handle this case
    if (features[0].layer.id.startsWith("track-polygon-layer-")) {
      const properties = features[0].properties || {};
      setPopupInfo({
        coordinates: { lat: lngLat.lat, lng: lngLat.lng },
        content: (
          <div>
            {Object.entries(properties).map(([key, value]) => (
              <div key={key}>
                {key}: {value}
              </div>
            ))}
          </div>
        ),
      });
    }
  }, []);

  const handleUndoPoint = useCallback(() => {
    undoDraftPoint();
  }, [undoDraftPoint]);

  const handleTrackSaveConfirmation = useCallback(
    (params: { name: string }) => {
      if (!draftTrack) return;
      moveDraftToTrack({ name: params.name });
    },
    [draftTrack, moveDraftToTrack],
  );

  // useEffect(() => {
  //   if (!draftTrack) return;
  //   if (draftTrack.points.length < 3) return;

  //   save({
  //     name: "Track",
  //     type: "track",
  //     coordinates: draftTrack.points.map((lngLat) => [lngLat.lng, lngLat.lat]),
  //   });
  // }, [draftTrack, save]);

  useEffect(() => {
    if (status === "success" && draftTrack) {
      // @eslint-disable-next-line
      // setDraftTrack(null);
      refetch();
      SOUNDS.success.play();
    }
    if (status === "error") {
      SOUNDS.error.play();
    }
  }, [draftTrack, refetch, status]);

  useEffect(() => {
    if (moveDraftToTrackStatus === "success") {
      refetch();
      refectDraft();
      SOUNDS.success.play();
    }
    if (moveDraftToTrackStatus === "error") {
      SOUNDS.error.play();
    }
  }, [moveDraftToTrackStatus, refetch, refectDraft]);

  useEffect(() => {
    if (undoDraftPointStatus === "success") {
      refetch();
      refectDraft();
      SOUNDS.success.play();
    }
    if (undoDraftPointStatus === "error") {
      SOUNDS.error.play();
    }
  }, [undoDraftPointStatus, refetch, refectDraft]);

  // Example of fetching GeoJSON data from a URL
  useEffect(() => {
    (async () => {
      const geoJSON = await deviceAPI
        .get<GeoJSON.FeatureCollection>(`layers?lpm=${lpmNo}`)
        .json();
      setGeojsonData(geoJSON);
    })();
  }, [lpmNo]);

  useEffect(() => {
    let timerId: number | undefined = undefined;
    (async () => {
      const socket = new WebSocket(`ws://${API_HOST}/ws`);
      socket.addEventListener("message", (event) => {
        if (typeof event.data === "string") {
          try {
            const parsedData = JSON.parse(event.data);
            if ("data" in parsedData && "event" in parsedData) {
              clearTimeout(timerId);
              setLiveStatusLoading(false);
              switch (parsedData.event) {
                case "live_status": {
                  const data = parsedData.data as LiveStatusAPI;
                  if (data.latitude && data.longitude) {
                    setLiveStatus(data);
                  }
                  break;
                }
                default:
                  console.warn("Unknown event type:", parsedData.event);
              }
              timerId = setTimeout(() => setLiveStatusLoading(true), 1500);
            }
          } catch (error) {
            console.error("Error parsing WebSocket message:", error);
          }
        } else {
          console.warn("Received non-string message:", event.data);
        }
      });
    })();

    return () => clearTimeout(timerId);
  }, []);

  const draftTrackGeoJSON = draftTrack ? trackToGeoJSON(draftTrack) : null;

  const trackGeoJSON = tracks
    ? tracks.map((track) => trackToGeoJSON(track))
    : null;
  const trackPolygonGeoJSON = tracks
    ? tracks.map((track) => trackToPolygonGeoJSON(track))
    : null;

  const interactiveLayerIds = useMemo(() => {
    const ids = ["draft-track-corners"];
    if (tracks) {
      ids.push(...tracks.map((_, index) => `track-polygon-layer-${index}`));
    }
    return ids;
  }, [tracks]);

  return (
    <div className="home-page-container">
      <LiveStatus liveStatus={liveStatus} isLoading={liveStatusLoading} />
      <Map
        {...viewState}
        onMove={(evt) => setViewState(evt.viewState)}
        onClick={handleMapClick}
        mapStyle={styleURL}
        maxZoom={30}
        interactiveLayerIds={interactiveLayerIds}
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
          <Source
            id="draft-track"
            type="geojson"
            data={draftTrackGeoJSON}
            tolerance={0}
            buffer={128}
          >
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
        {trackPolygonGeoJSON?.map((geoJSON, index) => (
          <Source
            id={`track-polygon-${index}`}
            type="geojson"
            data={geoJSON}
            key={index}
            tolerance={0}
            buffer={128}
          >
            <Layer
              id={`track-polygon-layer-${index}`}
              type="fill"
              paint={{
                "fill-color": "tomato",
                "fill-opacity": 0.2,
              }}
            />
            <Layer
              id={`track-polygon-outline-${index}`}
              type="line"
              paint={{
                "line-color": "tomato",
                "line-width": 2,
              }}
            />
          </Source>
        ))}
        {trackGeoJSON?.map((geoJSON, index) => (
          <Source
            id={`track-${index}`}
            type="geojson"
            data={geoJSON}
            key={index}
            tolerance={0}
            buffer={128}
          >
            <Layer
              id={`track-symbol-layer-${index}`}
              type="symbol"
              layout={{
                "symbol-placement": "line-center",
                "text-field": "{distanceInCM}",
                "text-font": ["Inter Bold"],
                "text-size": 14,
                "text-offset": [0, -1],
                "text-anchor": "bottom",
                "text-allow-overlap": true,
                "text-ignore-placement": true,
              }}
              paint={{ "text-color": "slateblue" }}
            />
          </Source>
        ))}
        {popupInfo && (
          <Popup
            longitude={popupInfo.coordinates.lng}
            latitude={popupInfo.coordinates.lat}
            anchor="bottom"
            onClose={() => setPopupInfo(null)}
          >
            {popupInfo.content}
          </Popup>
        )}
      </Map>
      <AddTrackToolbar
        draftTrack={draftTrack}
        className="add-track-toolbar"
        onUpdate={handleAddDraftPoint}
        onUndo={handleUndoPoint}
        onBookMarkLocation={handleBookMarkLocation}
        onSaveTrackConfirmation={handleTrackSaveConfirmation}
      />
      <FilledIconButton
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
        <LocationIcon />
      </FilledIconButton>
    </div>
  );
}

function trackToPolygonGeoJSON(
  trackInfo: TrackInfo,
): GeoJSON.FeatureCollection {
  return {
    type: "FeatureCollection",
    features: [
      {
        type: "Feature",
        geometry: {
          type: "Polygon",
          coordinates: [
            trackInfo.sides.flatMap(({ side }) =>
              side.map((lngLat) => [lngLat.lng, lngLat.lat]),
            ),
          ],
        },
        properties: {
          Name: trackInfo.name,
          "Area (sq. m)": trackInfo.areaInSqM.toFixed(2),
          "Area (cents)": trackInfo.areaInCents.toFixed(2),
        },
      },
    ],
  };
}

function trackToGeoJSON(trackInfo: TrackInfo): GeoJSON.FeatureCollection {
  return {
    type: "FeatureCollection",
    features: trackInfo.sides.map(({ side, distanceInCM }) => {
      return {
        type: "Feature",
        geometry: {
          type: "LineString",
          coordinates: side.map((lngLat) => [lngLat.lng, lngLat.lat]),
        },
        properties: {
          distanceInCM: `${distanceInCM.toFixed(2)}cm`,
        },
      };
    }),
  };
}
