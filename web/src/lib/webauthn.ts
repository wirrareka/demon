import {
  startRegistration,
  startAuthentication,
  type PublicKeyCredentialCreationOptionsJSON,
  type PublicKeyCredentialRequestOptionsJSON,
} from "@simplewebauthn/browser";
import { api } from "./api";

/** Register a platform passkey (Touch ID / Windows Hello) for the current operator. */
export async function registerPasskey(): Promise<void> {
  const opts = await api.webauthn.registerStart();
  const attestation = await startRegistration({
    optionsJSON: opts.publicKey as PublicKeyCredentialCreationOptionsJSON,
  });
  await api.webauthn.registerFinish(attestation);
}

/** Perform a fresh touch-per-op assertion scoped to one job, then mark it stepped-up. */
export async function stepUp(jobId: string): Promise<void> {
  const opts = await api.webauthn.stepupStart(jobId);
  const assertion = await startAuthentication({
    optionsJSON: opts.publicKey as PublicKeyCredentialRequestOptionsJSON,
  });
  await api.webauthn.stepupFinish(jobId, assertion);
}
