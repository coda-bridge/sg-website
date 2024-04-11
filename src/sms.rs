use tencentcloud_sdk_sms::{
    client::{Client as TcClient, SendSmsRequest},
    credentials::Credential,
};
use twilio_wasi::{Client, OutboundMessage};

#[allow(dead_code)]
pub enum SmsProvider {
    Tencent,
    Twilio,
}

impl SmsProvider {
    pub async fn deliver(&self, dest_phone: String, message: String) -> crate::Result<String> {
        match self {
            Self::Tencent => send_by_tencent(dest_phone, message).await,
            Self::Twilio => send_by_twilio(dest_phone, message).await,
        }
    }
}

async fn send_by_tencent(dest_phone: String, message_in_template: String) -> crate::Result<String> {
    let secret_id = std::env::var("TENCENTCLOUD_SECRET_ID").unwrap();
    let secret_key = std::env::var("TENCENTCLOUD_SECRET_KEY").unwrap();
    let sms_sdk_app_id = std::env::var("TENCENTCLOUD_SMS_APP_ID").unwrap();
    let template_id = std::env::var("TENCENTCLOUD_SMS_TEMPLATE_ID").unwrap();
    let region = std::env::var("TENCENTCLOUD_REGION").unwrap();

    let phone_number_set = vec![dest_phone.clone()];
    let template_param_set = vec![message_in_template];
    // build client
    let credential = Credential::new(secret_id, secret_key, None);
    let client = TcClient::new(credential, region);

    // build request
    let request = SendSmsRequest::new(
        phone_number_set,
        sms_sdk_app_id,
        template_id,
        "", // sign_name
        template_param_set,
    );
    // send
    let res = client
        .send_sms(request)
        .await
        .map_err(|e| format!("{:?}", e))?;

    if let Some(send_status_set) = res.response.send_status_set.clone() {
        for send_status in send_status_set.into_iter() {
            if send_status.phone_number == dest_phone {
                match send_status.code.as_str() {
                    "Ok" => return Ok(send_status.message),
                    _ => Err(send_status.message)?,
                }
            }
        }
    }

    Err(String::from("Deliver failed"))?
}

async fn send_by_twilio(dest_phone: String, message: String) -> crate::Result<String> {
    let account_id = std::env::var("TWILIO_ACCOUNT_ID").unwrap();
    let auth_token = std::env::var("TWILIO_AUTH_TOKEN").unwrap();
    let from = std::env::var("TWILIO_FROM").unwrap();
    let client = Client::new(account_id.as_str(), auth_token.as_str());
    client
        .send_message(OutboundMessage::new(
            from.as_str(),
            dest_phone.as_str(),
            message.as_str(),
        ))
        .await
        .map_err(|e| format!("{:?}", e))?;

    Ok(String::from("Sent"))
}
