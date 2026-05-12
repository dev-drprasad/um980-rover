import ky from "ky";
import errorSound from "../assets/error.mp3";
import clickSound from "../assets/click-heavy.mp3";
import successSound from "../assets/success.mp3";

export const API_HOST = import.meta.env.VITE_API_HOST || window.location.host;

export const deviceAPI = ky.create({
  prefixUrl: `http://${API_HOST}/api`,
  timeout: 30 * 1000,
  headers: {
    "Content-Type": "application/json",
    accept: "application/json",
  },
});

export const SOUNDS = {
  click: new Audio(clickSound),
  error: new Audio(errorSound),
  success: new Audio(successSound),
};
