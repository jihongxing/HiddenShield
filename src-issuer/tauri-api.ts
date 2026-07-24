import { invoke } from "@tauri-apps/api/core";

export type ActivationRequest = {
  status: "valid";
  payload: {
    requestId: string;
    installationId: string;
    createdAt: string;
  };
};

export type IssueLicenseOutput = {
  licensePath: string;
  auditPath: string;
  token: string;
  licenseId: string;
  customerReference: string;
  expiresAt: string;
};

export function issuerReadiness() {
  return invoke<void>("issuer_readiness");
}

export function inspectActivationRequest(requestPath: string) {
  return invoke<ActivationRequest>("inspect_activation_request", { requestPath });
}

export function issueLicense(requestPath: string) {
  return invoke<IssueLicenseOutput>("issue_license", { requestPath });
}
