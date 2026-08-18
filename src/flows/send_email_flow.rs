use std::sync::Arc;

use my_logger::{LOGGER, LogEventCtx};

use crate::{
    app::AppContext,
    models::{DeliveryMode, SendEmailError, SendEmailModel},
    smtp_client::SmtpSubmitResult,
};

pub async fn send_email(
    app: &Arc<AppContext>,
    mut model: SendEmailModel,
) -> Result<SmtpSubmitResult, SendEmailError> {
    let relay_is_configured = app
        .settings_reader
        .use_settings(|settings| settings.smtp.relay.is_some())
        .await;

    if model.delivery_mode == DeliveryMode::Relay && !relay_is_configured {
        return Err(SendEmailError::InvalidEmailModel(
            "The delivery mode 'relay' is asked for, but no relay is configured in the settings"
                .to_string(),
        ));
    }

    // Without a relay everything is delivered directly anyway - asking for it explicitly
    // must not put our own header into the message.
    if !relay_is_configured {
        model.delivery_mode = DeliveryMode::AsConfigured;
    }

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
