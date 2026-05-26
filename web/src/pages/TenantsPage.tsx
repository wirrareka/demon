import { useEffect, useState } from "react";
import { api, type Tenant } from "../lib/api";
import { Card } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Table, THead, TBody, TR, TH, TD } from "../components/ui/table";

export function TenantsPage() {
  const [tenants, setTenants] = useState<Tenant[]>([]);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    api.tenants().then(setTenants).catch((e) => setError(String(e)));
  }, []);

  return (
    <div className="flex flex-col gap-4">
      {error && (
        <Card className="border-danger-border bg-danger-bg px-4 py-2 text-sm text-danger-fg">
          {error}
        </Card>
      )}
      <Card>
        <Table>
          <THead>
            <TR className="hover:bg-transparent">
              <TH>Name</TH>
              <TH>Tenant ID</TH>
              <TH>Lifecycle</TH>
              <TH>Plan</TH>
              <TH>Residency</TH>
            </TR>
          </THead>
          <TBody>
            {tenants.length === 0 && (
              <TR className="hover:bg-transparent">
                <TD colSpan={5} className="py-6 text-center text-muted">
                  No tenants.
                </TD>
              </TR>
            )}
            {tenants.map((t) => (
              <TR key={t.id}>
                <TD className="font-medium">{t.name}</TD>
                <TD className="font-mono text-[11px] text-muted">{t.id}</TD>
                <TD>
                  <Badge className="bg-surface-2 text-fg-2 border-border">{t.lifecycle_state}</Badge>
                </TD>
                <TD className="text-fg-2">{t.plan ?? "—"}</TD>
                <TD className="uppercase text-fg-2">{t.residency_group}</TD>
              </TR>
            ))}
          </TBody>
        </Table>
      </Card>
    </div>
  );
}
