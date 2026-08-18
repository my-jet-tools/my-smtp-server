use std::sync::Arc;

use crate::app::AppContext;

/// The struct the generated service trait is implemented for. The macro looks for it under
/// this exact name in the parent module, and reads the `app` field out of it.
#[derive(Clone)]
pub struct SdkGrpcService {
    pub app: Arc<AppContext>,
}

impl SdkGrpcService {
    pub fn new(app: Arc<AppContext>) -> Self {
        Self { app }
    }
}
