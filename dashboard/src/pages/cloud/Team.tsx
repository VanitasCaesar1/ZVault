import { useEffect, useState, useCallback } from "react";
import { Card, Th, Td, Badge } from "../../components/Table";
import { Modal, FormField, ModalActions } from "./Projects";
import {
  listOrgs,
  listMembers,
  inviteMember,
  type Organization,
  type OrgMember,
} from "../../lib/cloud-api";

export function CloudTeamPage() {
  const [orgs, setOrgs] = useState<Organization[]>([]);
  const [selectedOrg, setSelectedOrg] = useState<Organization | null>(null);
  const [members, setMembers] = useState<OrgMember[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState("");

  const [showInviteModal, setShowInviteModal] = useState(false);
  const [inviteForm, setInviteForm] = useState({ email: "", role: "developer" });
  const [formLoading, setFormLoading] = useState(false);
  const [formError, setFormError] = useState("");

  const fetchOrgs = useCallback(async () => {
    try {
      const data = await listOrgs();
      setOrgs(data);
      if (data.length > 0 && !selectedOrg) setSelectedOrg(data[0]);
    } catch (e: any) {
      setError(e.message);
    }
  }, [selectedOrg]);

  const fetchMembers = useCallback(async () => {
    if (!selectedOrg) return;
    setLoading(true);
    try {
      const data = await listMembers(selectedOrg.id);
      setMembers(data);
    } catch (e: any) {
      setError(e.message);
    } finally {
      setLoading(false);
    }
  }, [selectedOrg]);

  useEffect(() => { fetchOrgs(); }, [fetchOrgs]);
  useEffect(() => { fetchMembers(); }, [fetchMembers]);

  const handleInvite = async () => {
    if (!selectedOrg) return;
    if (!inviteForm.email.trim()) { setFormError("Email is required"); return; }
    setFormLoading(true);
    setFormError("");
    try {
      await inviteMember(selectedOrg.id, inviteForm.email.trim(), inviteForm.role);
      await fetchMembers();
      setShowInviteModal(false);
      setInviteForm({ email: "", role: "developer" });
    } catch (e: any) {
      setFormError(e.message);
    } finally {
      setFormLoading(false);
    }
  };

  const roleVariant = (role: string) => {
    if (role === "admin") return "warning";
    if (role === "developer") return "info";
    return "muted";
  };

  return (
    <>
      <div className="flex justify-between items-center mb-6">
        <div className="flex items-center gap-3">
          <p className="text-sm text-stone-500">Manage team members and roles for your organization.</p>
          {orgs.length > 1 && (
            <select
              value={selectedOrg?.id ?? ""}
              onChange={(e) => { const org = orgs.find((o) => o.id === e.target.value); if (org) setSelectedOrg(org); }}
              className="px-3 py-1.5 text-sm border border-stone-300/40 rounded-lg bg-glass focus:outline-none focus:border-amber-500"
            >
              {orgs.map((o) => <option key={o.id} value={o.id}>{o.name}</option>)}
            </select>
          )}
        </div>
        {selectedOrg && (
          <button
            onClick={() => { setShowInviteModal(true); setFormError(""); }}
            className="px-4 py-2 rounded-full bg-amber-500 text-amber-900 text-[13px] font-semibold hover:bg-amber-600 transition-all cursor-pointer"
          >
            + Invite Member
          </button>
        )}
      </div>

      {error && <p className="text-sm text-red-600 mb-4">{error}</p>}

      <Card title={`Team${selectedOrg ? ` — ${selectedOrg.name}` : ""}`} actions={<span className="text-xs text-stone-400">{members.length} member{members.length !== 1 ? "s" : ""}</span>}>
        <table className="w-full text-[13px] border-collapse">
          <thead>
            <tr><Th>Email</Th><Th>Role</Th><Th>Joined</Th><Th>Status</Th></tr>
          </thead>
          <tbody>
            {loading ? (
              <tr><td colSpan={4} className="px-[18px] py-4 text-center text-stone-400">Loading…</td></tr>
            ) : members.length === 0 ? (
              <tr><td colSpan={4} className="px-[18px] py-4 text-center text-stone-400">No members yet.</td></tr>
            ) : members.map((m) => (
              <tr key={m.id} className="hover:bg-stone-50">
                <Td><span className="font-medium text-stone-800">{m.email}</span></Td>
                <Td><Badge variant={roleVariant(m.role)}>{m.role}</Badge></Td>
                <Td>{new Date(m.invited_at).toLocaleDateString()}</Td>
                <Td>
                  {m.accepted_at
                    ? <Badge variant="success">Active</Badge>
                    : <Badge variant="muted">Pending</Badge>
                  }
                </Td>
              </tr>
            ))}
          </tbody>
        </table>
      </Card>

      {showInviteModal && (
        <Modal title="Invite Team Member" onClose={() => setShowInviteModal(false)}>
          <FormField label="Email" value={inviteForm.email} onChange={(v) => setInviteForm((f) => ({ ...f, email: v }))} placeholder="teammate@company.com" type="email" />
          <div>
            <label className="block text-xs font-semibold text-stone-500 mb-1.5">Role</label>
            <select
              value={inviteForm.role}
              onChange={(e) => setInviteForm((f) => ({ ...f, role: e.target.value }))}
              className="w-full px-3 py-2 text-sm border border-stone-300 rounded-lg focus:outline-none focus:border-amber-500"
            >
              <option value="admin">Admin — full access</option>
              <option value="developer">Developer — read/write secrets</option>
              <option value="viewer">Viewer — read-only</option>
            </select>
          </div>
          {formError && <p className="text-sm text-red-600">{formError}</p>}
          <ModalActions onCancel={() => setShowInviteModal(false)} onSubmit={handleInvite} loading={formLoading} submitLabel="Send Invite" />
        </Modal>
      )}
    </>
  );
}
