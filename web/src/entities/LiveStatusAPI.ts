export interface HorizontalAccuracyAPI {
  lat_err: number;
  lon_err: number;
  drms: number;
  twice_drms: number;
}

export interface LiveStatusAPI {
  latitude: number | null;
  longitude: number | null;
  altitude: number | null;
  speed_over_ground: number | null;
  fix_type: string | null;
  satellites: number;
  fix_satellites: number | null;
  accuracy: HorizontalAccuracyAPI | null;
  hdop: number | null;
  vdop: number | null;
  pdop: number | null;
}
