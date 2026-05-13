import {
  createBrowserRouter,
  RouterProvider,
  type RouteObject,
} from "react-router-dom";
import "./App.css";
import { DevicesPage } from "./pages/devices";
import { HomePage } from "./pages/root/HomePage";
import { TerminalPage } from "./pages/terminal";
import { DocVectorizationPage } from "./pages/docVectorization/DocVectorization";
import { NTRIPSettingsPage } from "./pages/ntrip/ui/NTRIPSettingsPage";
import { RoverSetupPage } from "./pages/roversetup/ui/RoverSetupPage";

const routes: RouteObject[] = [
  { path: "", Component: HomePage },
  { path: "ntrip", Component: NTRIPSettingsPage },
  { path: "devices", Component: DevicesPage },
  { path: "terminal", Component: TerminalPage },
  { path: "doc-vectorization", Component: DocVectorizationPage },
  { path: "rover-setup", Component: RoverSetupPage },
];

const router = createBrowserRouter(routes, {
  basename: "/um980-rover",
});

function App() {
  return <RouterProvider router={router} />;
}

export default App;
