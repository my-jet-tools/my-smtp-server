use base64::{Engine, engine::general_purpose::STANDARD};
use lettre::{
    Message,
    message::{Attachment, Mailbox, MultiPart, SinglePart, header::ContentType},
};

use crate::models::{EmailAttachmentModel, SendEmailError, SendEmailModel};

pub fn build_email_message(
    model: SendEmailModel,
    default_from_email: String,
    default_from_name: Option<String>,
) -> Result<Message, SendEmailError> {
    if model.to.is_empty() {
        return Err(SendEmailError::InvalidEmailModel(
            "Field 'to' can not be empty".to_string(),
        ));
    }

    let from = compile_from_mailbox(&model, default_from_email, default_from_name)?;

    let mut builder = Message::builder()
        .from(from)
        .subject(model.subject.as_str());

    for email in model.to.iter() {
        builder = builder.to(parse_mailbox(email, "to")?);
    }

    for email in model.cc.iter() {
        builder = builder.cc(parse_mailbox(email, "cc")?);
    }

    for email in model.bcc.iter() {
        builder = builder.bcc(parse_mailbox(email, "bcc")?);
    }

    let body = if model.is_html {
        SinglePart::html(model.body)
    } else {
        SinglePart::plain(model.body)
    };

    let result = if model.attachments.is_empty() {
        builder.singlepart(body)
    } else {
        let mut multi_part = MultiPart::mixed().singlepart(body);

        for attachment in model.attachments.iter() {
            multi_part = multi_part.singlepart(compile_attachment(attachment)?);
        }

        builder.multipart(multi_part)
    };

    match result {
        Ok(message) => Ok(message),
        Err(err) => Err(SendEmailError::InvalidEmailModel(format!(
            "Can not build the email message. Err: {}",
            err
        ))),
    }
}

fn compile_from_mailbox(
    model: &SendEmailModel,
    default_from_email: String,
    default_from_name: Option<String>,
) -> Result<Mailbox, SendEmailError> {
    let (from_email, from_name) = match &model.from_email {
        Some(from_email) => (from_email.clone(), model.from_name.clone()),
        None => (default_from_email, default_from_name),
    };

    let mailbox = parse_mailbox(from_email.as_str(), "from_email")?;

    let from_name = match from_name {
        Some(from_name) => from_name,
        None => return Ok(mailbox),
    };

    if from_name.trim().is_empty() {
        return Ok(mailbox);
    }

    Ok(Mailbox::new(Some(from_name), mailbox.email))
}

/// Accepts both `user@domain.com` and `User Name <user@domain.com>`.
fn parse_mailbox(value: &str, field_name: &str) -> Result<Mailbox, SendEmailError> {
    match value.trim().parse::<Mailbox>() {
        Ok(mailbox) => Ok(mailbox),
        Err(err) => Err(SendEmailError::InvalidEmailModel(format!(
            "Invalid email address '{}' in the field '{}'. Err: {}",
            value, field_name, err
        ))),
    }
}

fn compile_attachment(attachment: &EmailAttachmentModel) -> Result<SinglePart, SendEmailError> {
    let content = match STANDARD.decode(attachment.base64_content.as_bytes()) {
        Ok(content) => content,
        Err(err) => {
            return Err(SendEmailError::InvalidEmailModel(format!(
                "Attachment '{}' has invalid base64 content. Err: {}",
                attachment.file_name, err
            )));
        }
    };

    let content_type = match ContentType::parse(attachment.content_type.as_str()) {
        Ok(content_type) => content_type,
        Err(err) => {
            return Err(SendEmailError::InvalidEmailModel(format!(
                "Attachment '{}' has invalid content type '{}'. Err: {}",
                attachment.file_name, attachment.content_type, err
            )));
        }
    };

    Ok(Attachment::new(attachment.file_name.clone()).body(content, content_type))
}
