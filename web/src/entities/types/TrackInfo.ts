import type { LngLat } from "./LngLat";

interface SideAPI {
  side: [[number, number], [number, number]];
  distance_in_cm: number;
}

export interface TrackInfoAPI {
  id: string;
  name: string;
  area: number;
  area_in_cents: number;
  sides: SideAPI[];
  points: LngLat[];
}

export interface TrackInfo {
  id: string;
  name: string;
  areaInSqM: number;
  areaInCents: number;
  sides: { side: [LngLat, LngLat]; distanceInCM: number }[];
  points: LngLat[];
}
