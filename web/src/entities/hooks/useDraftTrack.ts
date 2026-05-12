import { useAPI } from "../../shared/hooks/useAPI";
import { deviceAPI } from "../../core";
import type { TrackInfo, TrackInfoAPI } from "../types/TrackInfo";
import type { LngLat } from "../types/LngLat";

export function useDraftTrack() {
  const { data, status, refetch } = useAPI({
    queryFn: () => deviceAPI.get("draft").json<TrackInfoAPI | null>(),
  });

  return {
    data: data
      ? ({
          id: data.id,
          name: data.name,
          areaInSqM: data.area,
          areaInCents: data.area_in_cents,
          points: data.points,
          sides: data.sides.map(({ side, distance_in_cm }) => ({
            side: [
              { lat: side[0][0], lng: side[0][1] },
              { lat: side[1][0], lng: side[1][1] },
            ] satisfies [LngLat, LngLat],
            distanceInCM: distance_in_cm,
          })),
        } satisfies TrackInfo)
      : null,
    status,
    refetch,
  };
}
