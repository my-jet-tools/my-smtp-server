use std::sync::Arc;

use my_logger::{LOGGER, LogEventCtx};

use crate::{
    app::AppContext,
    models::{SendEmailError, SendEmailModel},
    smtp_client::SmtpSubmitResult,
};

pub async fn send_email(
    app: &Arc<AppContext>,
    model: SendEmailModel,
) -> Result<SmtpSubmitResult, SendEmailError> {
    let (default_from_email, default_from_name) = app
        .settings_reader
        .use_settings(|settings| {
            (
                settings.smtp.default_from_email.clone(),
                settings.smtp.default_from_name.clone(),
            )
        })
        .await;

    let amount_of_recipients = model.to.len() + model.cc.len() + model.bcc.len();

    let message =
        crate::scripts::build_email_message(model, default_from_email, default_from_name)?;

    match app.smtp_client.send(message).await {
        Ok(result) => Ok(result),
        Err(err) => {
            LOGGER.write_error(
                "send_email",
                err.as_str(),
                LogEventCtx::new().add("recipients", amount_of_recipients.to_string()),
            );

            Err(SendEmailError::MailServerError(err))
        }
    }
}
