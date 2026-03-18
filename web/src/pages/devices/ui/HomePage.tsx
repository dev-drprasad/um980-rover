import { useState, useEffect } from "react";
import Map, { Layer, Source } from "react-map-gl/maplibre"; // Import from maplibre
import { deviceAPI } from "../../../core";
import { useSearchParams } from "react-router-dom";
import "maplibre-gl/dist/maplibre-gl.css"; // Import the maplibre CSS
import "./HomePage.css";

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
    longitude: -0.09,
    latitude: 51.505,
    zoom: 18,
  });
  const [searchParams] = useSearchParams();
  const lpmNo = searchParams.get("lpm") || "";

  // Example of fetching GeoJSON data from a URL
  useEffect(() => {
    (async () => {
      const geoJSON = await deviceAPI
        .get<GeoJSON.FeatureCollection>(`layers?lpm=${lpmNo}`)
        .json();
      setGeojsonData(geoJSON);
      const [lng, lat] = geoJSON
        ? geoJSON.features[0].geometry.type === "Polygon"
          ? geoJSON.features[0].geometry.coordinates[0][0]
          : [0, 0]
        : [0, 0];
      console.log("geojsonData :>> ", geoJSON);
      setViewState((prev) => ({ ...prev, longitude: lng, latitude: lat }));
    })();
  }, [lpmNo]);

  return (
    <div className="home-page-container">
      <Map
        {...viewState}
        onMove={(evt) => setViewState(evt.viewState)}
        mapStyle={styleURL}
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
      </Map>
    </div>
  );
}
