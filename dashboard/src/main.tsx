import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router";
import { ClerkProvider } from "@clerk/clerk-react";
import { App } from "./App";
import { CLERK_PUBLISHABLE_KEY, CLERK_ENABLED } from "./lib/clerk";
import "./index.css";

const root = document.getElementById("root")!;

const app = (
  <StrictMode>
    <BrowserRouter basename="/app">
      {CLERK_ENABLED ? (
        <ClerkProvider
          publishableKey={CLERK_PUBLISHABLE_KEY}
          afterSignOutUrl="/app/login"
        >
          <App />
        </ClerkProvider>
      ) : (
        <App />
      )}
    </BrowserRouter>
  </StrictMode>
);

createRoot(root).render(app);
