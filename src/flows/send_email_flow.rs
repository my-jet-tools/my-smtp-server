use std::sync::Arc;

use my_logger::{LOGGER, LogEventCtx};

use crate::{
    app::AppContext,
    models::{DeliveryMode, SendEmailError, SendEmailModel},
    settings::MailgunHttpSettingsModel,
    smtp_client::SmtpSubmitResult,
};

pub async fn send_email(
    app: &Arc<AppContext>,
    mut model: SendEmailModel,
) -> Result<SmtpSubmitResult, SendEmailError> {
    let (relay_is_configured, mailgun_http, default_delivery_mode) = app
        .settings_reader
        .use_settings(|settings| {
            (
                settings.relay.is_some(),
                settings.mailgun_http.clone(),
                settings.get_default_delivery_mode(),
            )
        })
        .await;

    // A request which says nothing about the route takes the one the settings imply.
    if model.delivery_mode == DeliveryMode::AsConfigured {
        model.delivery_mode = default_delivery_mode;
    }

    if model.delivery_mode == DeliveryMode::Relay && !relay_is_configured {
        return Err(SendEmailError::InvalidEmailModel(
            "The delivery mode 'relay' is asked for, but no relay is configured in the settings"
                .to_string(),
        ));
    }

    if model.delivery_mode == DeliveryMode::MailgunHttp && mailgun_http.is_none() {
        return Err(SendEmailError::InvalidEmailModel(
            "The delivery mode 'mailgun_http' is asked for, but there is no mailgun_http section in the settings"
                .to_string(),
        ));
    }

    // Without a relay everything is delivered directly anyway - asking for it explicitly
    // must not put our own header into the message.
    if !relay_is_configured && model.delivery_mode == DeliveryMode::Direct {
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

    let delivery_mode = model.delivery_mode;
    let recipients = model.get_all_recipients();
    let amount_of_recipients = recipients.len();

    let message =
        crate::scripts::build_email_message(model, default_from_email, default_from_name)?;

    let result = match delivery_mode {
        // The http api of mailgun instead of the mail server of the container. There is no
        // local queue on this route: what the api answers is the final word.
        DeliveryMode::MailgunHttp => {
            send_via_mailgun_http(mailgun_http.as_ref().unwrap(), &message, &recipients).await
        }
        _ => app.smtp_client.send(message).await,
    };

    match result {
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

async fn send_via_mailgun_http(
    settings: &MailgunHttpSettingsModel,
    message: &lettre::Message,
    recipients: &[String],
) -> Result<SmtpSubmitResult, String> {
    let result = crate::scripts::send_via_mailgun_http(settings, message, recipients).await?;

    Ok(SmtpSubmitResult::from_parts(
        result.message_id,
        result.response,
    ))
}
