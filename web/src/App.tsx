import { useEffect } from "react";
import { useRoute } from "./router";
import { WalletProvider } from "./wallet";
import { useTheme } from "./ui";
import Home from "./pages/Home";
import AppShell from "./pages/AppShell";

function Routes() {
  const route = useRoute();
  useTheme(); // applies the saved theme on every page
  useEffect(() => {
    document.title = route.name === "home" ? "Backed" : `Backed · ${route.page[0].toUpperCase()}${route.page.slice(1)}`;
  }, [route]);
  return route.name === "home" ? <Home /> : <AppShell page={route.page} />;
}

export default function App() {
  return (
    <WalletProvider>
      <div className="grain" aria-hidden />
      <Routes />
    </WalletProvider>
  );
}
