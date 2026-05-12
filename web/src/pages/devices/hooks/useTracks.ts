import { useCallback, useEffect, useState } from "react";
import { deviceAPI } from "../../../core";
import type { QueryData } from "../../../shared/utils/types/QueryData";
import { LngLat } from "maplibre-gl";
import type {
  TrackInfo,
  TrackInfoAPI,
} from "../../../entities/types/TrackInfo";

export function useTracks() {
  const [{ data, status }, setState] = useState<QueryData<TrackInfo[]>>({
    data: null,
    status: "pending",
  });

  const refetch = useCallback(() => {
    setState((state) => ({ ...state, status: "pending" }));
  }, []);

  useEffect(() => {
    if (status !== "pending") return;
    (async () => {
      setState({ data: null, status: "fetching" });
      try {
        const tracks = await deviceAPI.get<TrackInfoAPI[]>("track").json();
        setState({
          data: tracks ? mapTrackInfoAPIToTrackInfo(tracks) : null,
          status: "success",
        });
      } catch (error) {
        console.error(error);
        setState({ data: null, status: "error" });
      }
    })();
  }, [status]);

  return { data: data, status, refetch };
}

function mapTrackInfoAPIToTrackInfo(trackInfoAPI: TrackInfoAPI[]): TrackInfo[] {
  return trackInfoAPI.map((track) => {
    return {
      id: track.id,
      name: track.name,
      areaInSqM: track.area,
      areaInCents: track.area_in_cents,
      sides: track.sides.map((side) => ({
        side: [
          new LngLat(side.side[0][1], side.side[0][0]),
          new LngLat(side.side[1][1], side.side[1][0]),
        ],
        distanceInCM: side.distance_in_cm,
      })),
      points: track.points,
    };
  });
}
