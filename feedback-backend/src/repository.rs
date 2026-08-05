use crate::schema::{
    AccountDevice, AccountDevicesResponse, AuthChallengeRequest, AuthChallengeResponse,
    AuthLogoutRequest, AuthLogoutResponse, AuthRefreshRequest, AuthSessionRequest,
    CloudAccountSession, CloudAccountSnapshot, CloudEntitlement, CloudSyncBatchRequest,
    CloudSyncBatchResult, CloudSyncChangesResult, ContinueAccountRequest, RevokeDeviceResponse,
    SyncPreferencesRequest, SyncPreferencesResponse, UpdateDeviceRequest,
    WatermarkIdConfirmRequest, WatermarkIdReconcileRequest, WatermarkIdRegistryResponse,
    WatermarkIdReissueRequest, WatermarkIdReissueResponse, WatermarkIdReserveRequest,
};
use crate::storage::{Storage, StorageError};

pub trait AuthRepository: Send + Sync {
    fn continue_account(
        &self,
        request: &ContinueAccountRequest,
    ) -> Result<CloudAccountSession, StorageError>;

    fn create_auth_challenge(
        &self,
        request: &AuthChallengeRequest,
    ) -> Result<AuthChallengeResponse, StorageError>;

    fn create_auth_session(
        &self,
        request: &AuthSessionRequest,
    ) -> Result<CloudAccountSession, StorageError>;

    fn refresh_auth_session(
        &self,
        request: &AuthRefreshRequest,
    ) -> Result<CloudAccountSession, StorageError>;

    fn logout_auth_session(
        &self,
        request: &AuthLogoutRequest,
    ) -> Result<AuthLogoutResponse, StorageError>;

    fn current_account_snapshot(
        &self,
        access_token: &str,
    ) -> Result<CloudAccountSnapshot, StorageError>;

    fn update_sync_preferences(
        &self,
        access_token: &str,
        request: &SyncPreferencesRequest,
    ) -> Result<SyncPreferencesResponse, StorageError>;

    fn list_devices(&self, access_token: &str) -> Result<AccountDevicesResponse, StorageError>;

    fn update_device(
        &self,
        access_token: &str,
        device_id: &str,
        request: &UpdateDeviceRequest,
    ) -> Result<AccountDevice, StorageError>;

    fn revoke_device(
        &self,
        access_token: &str,
        device_id: &str,
    ) -> Result<RevokeDeviceResponse, StorageError>;

    fn grant_cloud_sync_for_qa(
        &self,
        account_id: &str,
        workspace_id: &str,
    ) -> Result<CloudEntitlement, StorageError>;
}

pub trait CloudSyncRepository: Send + Sync {
    fn push_cloud_events_batch(
        &self,
        access_token: &str,
        request: &CloudSyncBatchRequest,
    ) -> Result<CloudSyncBatchResult, StorageError>;

    fn get_cloud_changes(
        &self,
        access_token: &str,
        workspace_id: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, StorageError>;
}

pub trait WatermarkRegistryRepository: Send + Sync {
    fn reserve_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReserveRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError>;

    fn confirm_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdConfirmRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError>;

    fn reconcile_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReconcileRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError>;

    fn reissue_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReissueRequest,
    ) -> Result<WatermarkIdReissueResponse, StorageError>;
}

impl AuthRepository for Storage {
    fn continue_account(
        &self,
        request: &ContinueAccountRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        Storage::continue_account(self, request)
    }

    fn create_auth_challenge(
        &self,
        request: &AuthChallengeRequest,
    ) -> Result<AuthChallengeResponse, StorageError> {
        Storage::create_auth_challenge(self, request)
    }

    fn create_auth_session(
        &self,
        request: &AuthSessionRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        Storage::create_auth_session(self, request)
    }

    fn refresh_auth_session(
        &self,
        request: &AuthRefreshRequest,
    ) -> Result<CloudAccountSession, StorageError> {
        Storage::refresh_auth_session(self, request)
    }

    fn logout_auth_session(
        &self,
        request: &AuthLogoutRequest,
    ) -> Result<AuthLogoutResponse, StorageError> {
        Storage::logout_auth_session(self, request)
    }

    fn current_account_snapshot(
        &self,
        access_token: &str,
    ) -> Result<CloudAccountSnapshot, StorageError> {
        Storage::current_account_snapshot(self, access_token)
    }

    fn update_sync_preferences(
        &self,
        access_token: &str,
        request: &SyncPreferencesRequest,
    ) -> Result<SyncPreferencesResponse, StorageError> {
        Storage::update_sync_preferences(self, access_token, request)
    }

    fn list_devices(&self, access_token: &str) -> Result<AccountDevicesResponse, StorageError> {
        Storage::list_devices(self, access_token)
    }

    fn update_device(
        &self,
        access_token: &str,
        device_id: &str,
        request: &UpdateDeviceRequest,
    ) -> Result<AccountDevice, StorageError> {
        Storage::update_device(self, access_token, device_id, request)
    }

    fn revoke_device(
        &self,
        access_token: &str,
        device_id: &str,
    ) -> Result<RevokeDeviceResponse, StorageError> {
        Storage::revoke_device(self, access_token, device_id)
    }

    fn grant_cloud_sync_for_qa(
        &self,
        _account_id: &str,
        _workspace_id: &str,
    ) -> Result<CloudEntitlement, StorageError> {
        Err(StorageError::BadRequest(
            "qa_entitlement_grant_requires_postgres_http_gate".to_string(),
        ))
    }
}

impl CloudSyncRepository for Storage {
    fn push_cloud_events_batch(
        &self,
        access_token: &str,
        request: &CloudSyncBatchRequest,
    ) -> Result<CloudSyncBatchResult, StorageError> {
        Storage::push_cloud_events_batch(self, access_token, request)
    }

    fn get_cloud_changes(
        &self,
        access_token: &str,
        workspace_id: Option<&str>,
        cursor: Option<&str>,
    ) -> Result<CloudSyncChangesResult, StorageError> {
        Storage::get_cloud_changes(self, access_token, workspace_id, cursor)
    }
}

impl WatermarkRegistryRepository for Storage {
    fn reserve_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReserveRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        Storage::reserve_watermark_id(self, access_token, request)
    }

    fn confirm_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdConfirmRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        Storage::confirm_watermark_id(self, access_token, request)
    }

    fn reconcile_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReconcileRequest,
    ) -> Result<WatermarkIdRegistryResponse, StorageError> {
        Storage::reconcile_watermark_id(self, access_token, request)
    }

    fn reissue_watermark_id(
        &self,
        access_token: &str,
        request: &WatermarkIdReissueRequest,
    ) -> Result<WatermarkIdReissueResponse, StorageError> {
        Storage::reissue_watermark_id(self, access_token, request)
    }
}
