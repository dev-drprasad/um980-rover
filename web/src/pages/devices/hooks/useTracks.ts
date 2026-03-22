import { useCallback, useEffect, useState } from "react";
import { deviceAPI } from "../../../core";
import type { QueryData } from "../../../shared/utils/types/QueryData";
import { LngLat } from "maplibre-gl";

type TracksAPI = { [key: string]: [number, number][] };
type Tracks = LngLat[][];

export function useTracks() {
  const [{ data, status }, setState] = useState<QueryData<Tracks>>({
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
        const tracks = await deviceAPI.get<TracksAPI>("track").json();
        console.log("tracks :>> ", tracks);
        setState({
          data: tracks
            ? Object.values(tracks).map((p) =>
                p.map(([lng, lat]) => new LngLat(lng, lat)),
              )
            : null,
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
