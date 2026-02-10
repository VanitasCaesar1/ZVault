import { Routes, Route, Navigate } from "react-router";
import { DashboardLayout } from "./layouts/DashboardLayout";
import { LoginPage } from "./pages/Login";
import { DashboardPage } from "./pages/Dashboard";
import { InitPage } from "./pages/Init";
import { UnsealPage } from "./pages/Unseal";
import { SecretsPage } from "./pages/Secrets";
import { PoliciesPage } from "./pages/Policies";
import { AuditPage } from "./pages/Audit";
import { LeasesPage } from "./pages/Leases";
import { AuthMethodsPage } from "./pages/AuthMethods";

export function App() {
  return (
    <Routes>
      <Route path="/login" element={<LoginPage />} />
      <Route element={<DashboardLayout />}>
        <Route index element={<DashboardPage />} />
        <Route path="/init" element={<InitPage />} />
        <Route path="/unseal" element={<UnsealPage />} />
        <Route path="/secrets" element={<SecretsPage />} />
        <Route path="/policies" element={<PoliciesPage />} />
        <Route path="/audit" element={<AuditPage />} />
        <Route path="/leases" element={<LeasesPage />} />
        <Route path="/auth" element={<AuthMethodsPage />} />
      </Route>
      <Route path="*" element={<Navigate to="/" replace />} />
    </Routes>
  );
}
