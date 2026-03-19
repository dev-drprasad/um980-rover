import ky from "ky";

export const API_HOST = import.meta.env.VITE_API_HOST || window.location.host;

export const deviceAPI = ky.create({
  prefixUrl: `http://${API_HOST}/api`,
  timeout: 30 * 1000,
  headers: {
    "Content-Type": "application/json",
    accept: "application/json",
  },
});
