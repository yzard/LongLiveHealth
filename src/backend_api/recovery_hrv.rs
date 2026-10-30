//! Persisted Recovery HRV identity; projection is shared with all Apple quantities.
use serde_json::Value;
pub const KIND: &str = "HKQuantityTypeIdentifierHeartRateVariabilityRMSSD";
pub fn projection(payload: &Value) -> Option<Value> {
    crate::apple_health::projection(KIND, payload)
}
