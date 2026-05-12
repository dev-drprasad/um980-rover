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

const routes: RouteObject[] = [
  { path: "", Component: HomePage },
  { path: "ntrip", Component: NTRIPSettingsPage },
  { path: "devices", Component: DevicesPage },
  { path: "terminal", Component: TerminalPage },
  { path: "doc-vectorization", Component: DocVectorizationPage },
];

const router = createBrowserRouter(routes);

function App() {
  return <RouterProvider router={router} />;
}

export default App;
