#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckStatus {
    /// Everything this check needs is in place.
    Ok,
    /// It works, but something is not as it should be - or the check does not apply to the
    /// current configuration.
    Warning,
    /// The mail will not be delivered, or will be delivered and rejected.
    Failed,
}

impl CheckStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            CheckStatus::Ok => "ok",
            CheckStatus::Warning => "warning",
            CheckStatus::Failed => "failed",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CheckItem {
    pub title: String,
    pub status: CheckStatus,
    pub message: String,
    /// What has to be published or configured - the value to copy.
    pub expected: Option<String>,
    /// What is there right now.
    pub actual: Option<String>,
}

impl CheckItem {
    pub fn ok(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: CheckStatus::Ok,
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn warning(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: CheckStatus::Warning,
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn failed(title: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            status: CheckStatus::Failed,
            message: message.into(),
            expected: None,
            actual: None,
        }
    }

    pub fn with_expected(mut self, expected: impl Into<String>) -> Self {
        self.expected = Some(expected.into());
        self
    }

    pub fn with_actual(mut self, actual: impl Into<String>) -> Self {
        self.actual = Some(actual.into());
        self
    }
}

#[derive(Debug, Clone)]
pub struct CheckupReport {
    /// The ip address the recipients see the mail coming from.
    pub public_ip: Option<String>,
    pub items: Vec<CheckItem>,
}

impl CheckupReport {
    pub fn get_status(&self) -> CheckStatus {
        if self
            .items
            .iter()
            .any(|item| item.status == CheckStatus::Failed)
        {
            return CheckStatus::Failed;
        }

        if self
            .items
            .iter()
            .any(|item| item.status == CheckStatus::Warning)
        {
            return CheckStatus::Warning;
        }

        CheckStatus::Ok
    }
}
