import {
  createBrowserRouter,
  RouterProvider,
  type RouteObject,
} from "react-router-dom";
import "./App.css";
import { DevicesPage } from "./pages/devices";
import { HomePage } from "./pages/root/HomePage";
import { TerminalPage } from "./pages/terminal";

const routes: RouteObject[] = [
  { path: "", Component: HomePage },
  { path: "devices", Component: DevicesPage },
  { path: "terminal", Component: TerminalPage },
];

const router = createBrowserRouter(routes);

function App() {
  return <RouterProvider router={router} />;
}

export default App;
