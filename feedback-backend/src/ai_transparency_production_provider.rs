use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionCustodyOperation {
    IssueCredential,
    CreateMarkingSession,
    RotateCredential,
    RevokeCredential,
}

pub trait ProductionProviderReadiness: Send + Sync {
    fn ensure_ready(
        &self,
        operation: ProductionCustodyOperation,
    ) -> Result<(), ProductionProviderDeploymentError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProductionProviderDependency {
    IamReceipt,
    KmsHealth,
    ActivePepper,
}

pub trait ProductionProviderDependencyProbe: Send + Sync {
    fn check(
        &self,
        dependency: ProductionProviderDependency,
        config: &ProductionProviderDeploymentConfig,
    ) -> Result<(), ProductionProviderDeploymentError>;
}

#[derive(Debug, Default)]
pub struct UnavailableProductionProviderProbe;

impl ProductionProviderDependencyProbe for UnavailableProductionProviderProbe {
    fn check(
        &self,
        dependency: ProductionProviderDependency,
        _config: &ProductionProviderDeploymentConfig,
    ) -> Result<(), ProductionProviderDeploymentError> {
        Err(ProductionProviderDeploymentError::Unavailable(
            dependency_name(dependency),
        ))
    }
}

pub struct DeploymentProviderReadiness<P> {
    config: ProductionProviderDeploymentConfig,
    probe: P,
}

impl<P> DeploymentProviderReadiness<P> {
    pub fn new(config: ProductionProviderDeploymentConfig, probe: P) -> Self {
        Self { config, probe }
    }
}

impl<P> ProductionProviderReadiness for DeploymentProviderReadiness<P>
where
    P: ProductionProviderDependencyProbe,
{
    fn ensure_ready(
        &self,
        _operation: ProductionCustodyOperation,
    ) -> Result<(), ProductionProviderDeploymentError> {
        self.config.validate()?;
        for dependency in [
            ProductionProviderDependency::IamReceipt,
            ProductionProviderDependency::KmsHealth,
            ProductionProviderDependency::ActivePepper,
        ] {
            self.probe.check(dependency, &self.config)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProductionProviderDeploymentConfig {
    pub custody_enabled: bool,
    pub iam_receipt_url: String,
    pub iam_issuer: String,
    pub iam_audience: String,
    pub iam_jwks_url: String,
    pub kms_provider: String,
    pub kms_active_pepper_ref: String,
    pub kms_retained_pepper_refs: String,
    pub kms_workload_identity_ref: String,
    pub kms_health_url: String,
    pub recovery_runbook_ref: String,
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ProductionProviderDeploymentError {
    #[error("AI Transparency production custody provider configuration is missing: {0}")]
    Missing(&'static str),
    #[error("AI Transparency production custody provider configuration is invalid: {0}")]
    Invalid(&'static str),
    #[error("AI Transparency production custody provider is unavailable: {0}")]
    Unavailable(&'static str),
}

impl ProductionProviderDeploymentConfig {
    pub fn from_environment() -> Result<Option<Self>, ProductionProviderDeploymentError> {
        let environment = std::env::vars().collect::<BTreeMap<_, _>>();
        Self::from_map(&environment)
    }

    pub fn from_map(
        environment: &BTreeMap<String, String>,
    ) -> Result<Option<Self>, ProductionProviderDeploymentError> {
        let enabled = environment
            .get("HIDDENSHIELD_AI_TRANSPARENCY_CUSTODY_ENABLED")
            .map(String::as_str)
            .unwrap_or("false");
        match enabled {
            "false" | "0" | "" => Ok(None),
            "true" | "1" => {
                let config = Self {
                    custody_enabled: true,
                    iam_receipt_url: required(environment, "HIDDENSHIELD_AI_IAM_RECEIPT_URL")?,
                    iam_issuer: required(environment, "HIDDENSHIELD_AI_IAM_ISSUER")?,
                    iam_audience: required(environment, "HIDDENSHIELD_AI_IAM_AUDIENCE")?,
                    iam_jwks_url: required(environment, "HIDDENSHIELD_AI_IAM_JWKS_URL")?,
                    kms_provider: required(environment, "HIDDENSHIELD_AI_KMS_PROVIDER")?,
                    kms_active_pepper_ref: required(
                        environment,
                        "HIDDENSHIELD_AI_KMS_ACTIVE_PEPPER_REF",
                    )?,
                    kms_retained_pepper_refs: required(
                        environment,
                        "HIDDENSHIELD_AI_KMS_RETAINED_PEPPER_REFS",
                    )?,
                    kms_workload_identity_ref: required(
                        environment,
                        "HIDDENSHIELD_AI_KMS_WORKLOAD_IDENTITY_REF",
                    )?,
                    kms_health_url: required(environment, "HIDDENSHIELD_AI_KMS_HEALTH_URL")?,
                    recovery_runbook_ref: required(
                        environment,
                        "HIDDENSHIELD_AI_PROVIDER_RECOVERY_RUNBOOK_REF",
                    )?,
                };
                config.validate()?;
                Ok(Some(config))
            }
            _ => Err(ProductionProviderDeploymentError::Invalid(
                "HIDDENSHIELD_AI_TRANSPARENCY_CUSTODY_ENABLED",
            )),
        }
    }

    pub fn validate(&self) -> Result<(), ProductionProviderDeploymentError> {
        for (name, value) in [
            ("HIDDENSHIELD_AI_IAM_RECEIPT_URL", &self.iam_receipt_url),
            ("HIDDENSHIELD_AI_IAM_JWKS_URL", &self.iam_jwks_url),
            ("HIDDENSHIELD_AI_KMS_HEALTH_URL", &self.kms_health_url),
        ] {
            if !value.starts_with("https://") {
                return Err(ProductionProviderDeploymentError::Invalid(name));
            }
        }
        if !matches!(
            self.kms_provider.as_str(),
            "gcp_kms" | "aws_kms" | "azure_key_vault" | "pkcs11"
        ) {
            return Err(ProductionProviderDeploymentError::Invalid(
                "HIDDENSHIELD_AI_KMS_PROVIDER",
            ));
        }
        for (name, value) in [
            (
                "HIDDENSHIELD_AI_KMS_ACTIVE_PEPPER_REF",
                &self.kms_active_pepper_ref,
            ),
            (
                "HIDDENSHIELD_AI_KMS_WORKLOAD_IDENTITY_REF",
                &self.kms_workload_identity_ref,
            ),
            (
                "HIDDENSHIELD_AI_PROVIDER_RECOVERY_RUNBOOK_REF",
                &self.recovery_runbook_ref,
            ),
        ] {
            if contains_placeholder_or_secret(value) || value.len() < 8 {
                return Err(ProductionProviderDeploymentError::Invalid(name));
            }
        }
        for (name, value) in [
            ("HIDDENSHIELD_AI_IAM_RECEIPT_URL", &self.iam_receipt_url),
            ("HIDDENSHIELD_AI_IAM_ISSUER", &self.iam_issuer),
            ("HIDDENSHIELD_AI_IAM_AUDIENCE", &self.iam_audience),
            ("HIDDENSHIELD_AI_IAM_JWKS_URL", &self.iam_jwks_url),
            ("HIDDENSHIELD_AI_KMS_HEALTH_URL", &self.kms_health_url),
            (
                "HIDDENSHIELD_AI_KMS_RETAINED_PEPPER_REFS",
                &self.kms_retained_pepper_refs,
            ),
        ] {
            if contains_placeholder_or_secret(value) {
                return Err(ProductionProviderDeploymentError::Invalid(name));
            }
        }
        if !self.kms_retained_pepper_refs.trim_start().starts_with('[') {
            return Err(ProductionProviderDeploymentError::Invalid(
                "HIDDENSHIELD_AI_KMS_RETAINED_PEPPER_REFS",
            ));
        }
        Ok(())
    }
}

fn contains_placeholder_or_secret(value: &str) -> bool {
    value.contains("secret=")
        || value.contains("example.internal")
        || [
            "REPLACE_WITH",
            "PROJECT_ID",
            "LOCATION",
            "RETAINED_VERSION",
            "VERSION",
            "POOL",
            "PROVIDER",
        ]
        .iter()
        .any(|placeholder| value.contains(placeholder))
}

fn required(
    environment: &BTreeMap<String, String>,
    name: &'static str,
) -> Result<String, ProductionProviderDeploymentError> {
    environment
        .get(name)
        .filter(|value| !value.trim().is_empty())
        .cloned()
        .ok_or(ProductionProviderDeploymentError::Missing(name))
}

fn dependency_name(dependency: ProductionProviderDependency) -> &'static str {
    match dependency {
        ProductionProviderDependency::IamReceipt => "internal_iam_receipt",
        ProductionProviderDependency::KmsHealth => "kms_health",
        ProductionProviderDependency::ActivePepper => "active_pepper",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn production_map() -> BTreeMap<String, String> {
        BTreeMap::from([
            (
                "HIDDENSHIELD_AI_TRANSPARENCY_CUSTODY_ENABLED".to_string(),
                "true".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_IAM_RECEIPT_URL".to_string(),
                "https://iam.control.hiddenshield.internal/receipts".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_IAM_ISSUER".to_string(),
                "https://iam.control.hiddenshield.internal".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_IAM_AUDIENCE".to_string(),
                "hiddenshield-ai-custody".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_IAM_JWKS_URL".to_string(),
                "https://iam.control.hiddenshield.internal/.well-known/jwks.json".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_KMS_PROVIDER".to_string(),
                "gcp_kms".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_KMS_ACTIVE_PEPPER_REF".to_string(),
                "gcp-kms://projects/hiddenshield-prod/keys/ai-pepper/versions/2".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_KMS_RETAINED_PEPPER_REFS".to_string(),
                "[\"gcp-kms://projects/hiddenshield-prod/keys/ai-pepper/versions/1\"]".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_KMS_WORKLOAD_IDENTITY_REF".to_string(),
                "gcp-wif://projects/hiddenshield-prod/pools/hs/providers/custody-prod".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_KMS_HEALTH_URL".to_string(),
                "https://kms.control.hiddenshield.internal/health".to_string(),
            ),
            (
                "HIDDENSHIELD_AI_PROVIDER_RECOVERY_RUNBOOK_REF".to_string(),
                "runbook://ai-custody/provider-recovery-v1".to_string(),
            ),
        ])
    }

    #[test]
    fn disabled_custody_does_not_require_provider_configuration() {
        assert_eq!(
            ProductionProviderDeploymentConfig::from_map(&BTreeMap::new()).unwrap(),
            None
        );
    }

    #[test]
    fn enabled_custody_fails_closed_when_iam_configuration_is_missing() {
        let mut map = production_map();
        map.remove("HIDDENSHIELD_AI_IAM_JWKS_URL");
        assert_eq!(
            ProductionProviderDeploymentConfig::from_map(&map),
            Err(ProductionProviderDeploymentError::Missing(
                "HIDDENSHIELD_AI_IAM_JWKS_URL"
            ))
        );
    }

    #[test]
    fn enabled_custody_accepts_secret_references_but_not_literal_secrets() {
        let mut map = production_map();
        map.insert(
            "HIDDENSHIELD_AI_KMS_ACTIVE_PEPPER_REF".to_string(),
            "secret=literal-value".to_string(),
        );
        assert_eq!(
            ProductionProviderDeploymentConfig::from_map(&map),
            Err(ProductionProviderDeploymentError::Invalid(
                "HIDDENSHIELD_AI_KMS_ACTIVE_PEPPER_REF"
            ))
        );
    }

    #[test]
    fn enabled_custody_rejects_example_placeholder_configuration() {
        let mut map = production_map();
        map.insert(
            "HIDDENSHIELD_AI_IAM_RECEIPT_URL".to_string(),
            "https://iam.example.internal/receipts".to_string(),
        );
        assert_eq!(
            ProductionProviderDeploymentConfig::from_map(&map),
            Err(ProductionProviderDeploymentError::Invalid(
                "HIDDENSHIELD_AI_IAM_RECEIPT_URL"
            ))
        );
    }

    #[derive(Debug, Clone, Copy)]
    struct Probe {
        unavailable: Option<ProductionProviderDependency>,
    }

    impl ProductionProviderDependencyProbe for Probe {
        fn check(
            &self,
            dependency: ProductionProviderDependency,
            _config: &ProductionProviderDeploymentConfig,
        ) -> Result<(), ProductionProviderDeploymentError> {
            if self.unavailable == Some(dependency) {
                return Err(ProductionProviderDeploymentError::Unavailable(
                    dependency_name(dependency),
                ));
            }
            Ok(())
        }
    }

    #[test]
    fn readiness_rejects_unavailable_iam_before_custody_operation() {
        let config = ProductionProviderDeploymentConfig::from_map(&production_map())
            .unwrap()
            .unwrap();
        let readiness = DeploymentProviderReadiness::new(
            config,
            Probe {
                unavailable: Some(ProductionProviderDependency::IamReceipt),
            },
        );
        assert_eq!(
            readiness.ensure_ready(ProductionCustodyOperation::IssueCredential),
            Err(ProductionProviderDeploymentError::Unavailable(
                "internal_iam_receipt"
            ))
        );
    }

    #[test]
    fn readiness_accepts_all_dependencies_after_recovery() {
        let config = ProductionProviderDeploymentConfig::from_map(&production_map())
            .unwrap()
            .unwrap();
        let readiness = DeploymentProviderReadiness::new(config, Probe { unavailable: None });
        assert_eq!(
            readiness.ensure_ready(ProductionCustodyOperation::CreateMarkingSession),
            Ok(())
        );
    }
}
