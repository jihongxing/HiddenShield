CREATE OR REPLACE VIEW ai_public_confirmed_manifests AS
SELECT
    manifest.transparency_manifest_id,
    manifest.watermark_uid,
    manifest.manifest_version,
    manifest.status AS manifest_status,
    manifest.claim_type,
    manifest.generated_at,
    manifest.profile_status_json
FROM ai_transparency_manifests manifest
JOIN ai_marking_sessions session
  ON session.marking_session_id = manifest.marking_session_id
JOIN ai_platform_marking_submissions submission
  ON submission.marking_session_id = session.marking_session_id
WHERE session.status = 'confirmed'
  AND submission.status = 'confirmed';

CREATE OR REPLACE VIEW ai_public_confirmed_markers AS
SELECT
    marker.transparency_manifest_id,
    marker.marker_type,
    marker.marker_profile_id,
    marker.marker_version,
    marker.verify_status
FROM ai_marker_bindings marker
JOIN ai_public_confirmed_manifests manifest
  ON manifest.transparency_manifest_id = marker.transparency_manifest_id;

CREATE OR REPLACE VIEW ai_public_confirmed_evidence_summary AS
SELECT
    evidence.transparency_manifest_id,
    evidence.evidence_level,
    evidence.verification_status
FROM ai_claim_evidence evidence
JOIN ai_public_confirmed_manifests manifest
  ON manifest.transparency_manifest_id = evidence.transparency_manifest_id;
