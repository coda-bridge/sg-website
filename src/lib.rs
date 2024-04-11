use std::collections::HashMap;
use webhook_flows::{
    create_endpoint, request_handler, 
    route::{get, post, options, route, RouteError, Router},
    send_response
};
use store_flows as store;
use flowsnet_platform_sdk::logger;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;
use mysql_async::{prelude::*, Opts, OptsBuilder, Conn, Pool, PoolConstraints, PoolOpts, SslOpts};
use sendgrid::v3::*;
use ethers_signers::{LocalWallet, Signer, MnemonicBuilder, coins_bip39::English};
use ethers_core::types::{NameOrAddress, Bytes, U256, U64, H160, TransactionRequest, transaction::eip2718::TypedTransaction};
use ethers_core::abi::{Abi, Function, Token};
use ethers_core::utils::hex;
use ethers_core::rand;
use ethers_core::rand::Rng;
use std::str::FromStr;
use tokio::time::{sleep, Duration};
use time::PrimitiveDateTime;
use csv::Reader;
type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

mod sms;
use sms::SmsProvider;

// role: 0: user; 1: merchant
#[derive(Default, Serialize, Deserialize)]
struct AuthUser {
    user_id: u64,
    pin: String,
    phone_country: String,
    phone_number: String,
    phone_active: i32,
    email: String,
    email_active: i32,
    role: u64,
}
impl AuthUser {
    fn new(
        user_id: u64,
        pin: String,
        phone_country: String,
        phone_number: String,
        phone_active: i32,
        email: String,
        email_active: i32,
        role: u64,
    ) -> Self {
        Self {
            user_id,
            pin,
            phone_country,
            phone_number,
            phone_active,
            email,
            email_active,
            role,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct User {
    user_id: u64,
    firstname: String,
    lastname: String,
    address: String,
    country: String,
    dob: String,
    official_id: String,
    phone_country: String,
    phone_number: String,
    phone_active: i32,
    email: String,
    email_active: i32,
    payment_pin: String,
    eth_address: String,
    eth_pk: String,
}
impl User {
    fn new(
        user_id: u64,
        firstname: String,
        lastname: String,
        address: String,
        country: String,
        dob: String,
        official_id: String,
        phone_country: String,
        phone_number: String,
        phone_active: i32,
        email: String,
        email_active: i32,
        payment_pin: String,
        eth_address: String,
        eth_pk: String,
    ) -> Self {
        Self {
            user_id,
            firstname,
            lastname,
            address,
            country,
            dob,
            official_id,
            phone_country,
            phone_number,
            phone_active,
            email,
            email_active,
            payment_pin,
            eth_address,
            eth_pk,
        }
    }
}

#[derive(Default, Clone, Serialize, Deserialize)]
struct Merchant {
    user_id: u64,
    merchant_name: String,
    address: String,
    country: String,
    phone_country: String,
    phone_number: String,
    email: String,
    eth_address: String,
    eth_pk: String,
}
impl Merchant {
    fn new(
        user_id: u64,
        merchant_name: String,
        address: String,
        country: String,
        phone_country: String,
        phone_number: String,
        email: String,
        eth_address: String,
        eth_pk: String,
    ) -> Self {
        Self {
            user_id,
            merchant_name,
            address,
            country,
            phone_country,
            phone_number,
            email,
            eth_address,
            eth_pk,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Payment {
    payment_id: u64,
    created_on: PrimitiveDateTime,
    tx_hash: String,
    user_id: u64,
    user_name: String,
    user_phone: String,
    user_email: String,
    user_address: String,
    merchant_id: u64,
    merchant_name: String,
    merchant_address: String,
    pool_address: String,
    amount: f32,
}
impl Payment {
    fn new (
        payment_id: u64,
        created_on: PrimitiveDateTime,
        tx_hash: String,
        user_id: u64,
        user_name: String,
        user_phone: String,
        user_email: String,
        user_address: String,
        merchant_id: u64,
        merchant_name: String,
        merchant_address: String,
        pool_address: String,
        amount: f32,
    ) -> Self {
        Self {
            payment_id,
            created_on,
            tx_hash,
            user_id,
            user_name,
            user_phone,
            user_email,
            user_address,
            merchant_id,
            merchant_name,
            merchant_address,
            pool_address,
            amount,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Funding {
    funding_id: u64,
    created_on: PrimitiveDateTime,
    tx_hash: String,
    user_id: u64,
    user_name: String,
    user_phone: String,
    user_email: String,
    user_address: String,
    pool_address: String,
    amount: f32,
}
impl Funding {
    fn new (
        funding_id: u64,
        created_on: PrimitiveDateTime,
        tx_hash: String,
        user_id: u64,
        user_name: String,
        user_phone: String,
        user_email: String,
        user_address: String,
        pool_address: String,
        amount: f32,
    ) -> Self {
        Self {
            funding_id,
            created_on,
            tx_hash,
            user_id,
            user_name,
            user_phone,
            user_email,
            user_address,
            pool_address,
            amount,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
struct Redeem {
    redeem_id: u64,
    created_on: PrimitiveDateTime,
    tx_hash: String,
    merchant_id: u64,
    merchant_name: String,
    merchant_address: String,
    redeem_address: String,
    amount: f32,
}
impl Redeem {
    fn new (
        redeem_id: u64,
        created_on: PrimitiveDateTime,
        tx_hash: String,
        merchant_id: u64,
        merchant_name: String,
        merchant_address: String,
        redeem_address: String,
        amount: f32,
    ) -> Self {
        Self {
            redeem_id,
            created_on,
            tx_hash,
            merchant_id,
            merchant_name,
            merchant_address,
            redeem_address,
            amount,
        }
    }
}

#[no_mangle]
#[tokio::main(flavor = "current_thread")]
pub async fn on_deploy() {
    create_endpoint().await;
}

#[request_handler]
async fn handler() {
    let mut router = Router::new();
    router.insert("/register", vec![options(opt), post(register)]).unwrap();
    router.insert("/activate/:code", vec![options(opt), get(activate)]).unwrap();
    router.insert("/login_phone", vec![options(opt), post(login_phone)]).unwrap();
    router.insert("/login_email", vec![options(opt), post(login_email)]).unwrap();
    router.insert("/login/:code", vec![options(opt), get(login)]).unwrap();
    router.insert("/profile", vec![options(opt), get(profile)]).unwrap();
    router.insert("/profile_simple", vec![options(opt), get(profile_simple)]).unwrap();
    router.insert("/create_payment_pin", vec![options(opt), post(create_payment_pin)]).unwrap();
    router.insert("/pay", vec![options(opt), post(pay)]).unwrap();
    router.insert("/user_transactions", vec![options(opt), get(user_transactions)]).unwrap();
    router.insert("/merchant_transactions", vec![options(opt), get(merchant_transactions)]).unwrap();
    router.insert("/donor_report", vec![options(opt), post(donor_report)]).unwrap();
    router.insert("/add_fund", vec![options(opt), post(add_fund)]).unwrap();
    router.insert("/funding_transactions", vec![options(opt), get(funding_transactions)]).unwrap();
    router.insert("/merchant_redeem", vec![options(opt), post(merchant_redeem)]).unwrap();
    router.insert("/redeem_transactions", vec![options(opt), get(redeem_transactions)]).unwrap();

    if let Err(e) = route(router).await {
        match e {
            RouteError::NotFound => {
                send_response(404, vec![], b"No route matched".to_vec());
            }
            RouteError::MethodNotAllowed => {
                send_response(405, vec![], b"Method not allowed".to_vec());
            }
        }
    }
}

fn get_default_headers () -> Vec<(String, String)> {
    let resp_headers = vec![
        (String::from("content-type"), String::from("application/json")),
        (String::from("Access-Control-Allow-Origin"), std::env::var("ORIGIN_SITE").unwrap()),
        (String::from("Access-Control-Allow-Methods"), String::from("GET, POST, OPTIONS")),
        (String::from("Access-Control-Allow-Credentials"), String::from("true")),
        (String::from("Access-Control-Allow-Headers"), String::from("Set-Cookie,cookie,api,Keep-Alive,User-Agent,Content-Type"))
    ];
    return resp_headers;
}

fn get_conn_pool () -> Pool {
    let database_url = std::env::var("DATABASE_URL").unwrap();
    let opts = Opts::from_url(&database_url).unwrap();
    let mut builder = OptsBuilder::from_opts(opts);
    builder = builder.ssl_opts(SslOpts::default());
    let constraints = PoolConstraints::new(1, 2).unwrap();
    let pool_opts = PoolOpts::default().with_constraints(constraints);
    let pool = Pool::new(builder.pool_opts(pool_opts));
    return pool;
}

async fn opt (_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    send_response(200, get_default_headers(), "".as_bytes().to_vec());
}

async fn register (_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let origin_site = std::env::var("ORIGIN_SITE").unwrap_or("https://www.thecharityproject.xyz".to_string());

    let mut status = true;
    let mut message = "";
    let json: Value = serde_json::from_slice(&body).unwrap();
    log::info!("Input JSON: {}", serde_json::to_string_pretty(&json).unwrap());
    let firstname = json.get("firstname").expect("Must have firstname").as_str().unwrap();
    let lastname = json.get("lastname").expect("Must have lastname").as_str().unwrap();

    // let address = json.get("address").expect("Must have address").as_str().unwrap();
    // let dob = json.get("dob").expect("Must have dob").as_str().unwrap();
    // let email = json.get("email").expect("Must have email").as_str().unwrap();
    // let payment_pin = json.get("payment_pin").expect("Must have pin").as_str().unwrap();
    let address = "N/A";
    let dob = "N/A";
    let email = "";
    let payment_pin = "";

    let phone_country = json.get("phone_country").expect("Must have phone country code").as_str().unwrap();
    let phone_number = json.get("phone_number").expect("Must have phone number").as_str().unwrap();
    let country = match phone_country {
        "1" => "USA",
        "65" => "Singapore",
        "86" => "China",
        _ => "unknown",
    };
    let official_id = "N/A";

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let uids_email: Vec<u64> = "SELECT user_id FROM users WHERE email=:email AND email_active=1"
      .with(params! {
          "email" => email,
      })
      .map(&mut conn, |uid| uid).await.unwrap_or(Vec::new());

    let uids_phone: Vec<u64> = "SELECT user_id FROM users WHERE phone_country=:phone_country AND phone_number=:phone_number AND phone_active=1"
      .with(params! {
          "phone_country" => phone_country,
          "phone_number" => phone_number,
      })
      .map(&mut conn, |uid| uid).await.unwrap_or(Vec::new());

    let mut uid = 0;
    if uids_email.len() > 0 || uids_phone.len() > 0 {
        status = false;
        message = "The email or phone number is already taken!";
    } else {
        r"INSERT INTO users (firstname, lastname, address, country, dob, official_id, email, phone_country, phone_number, payment_pin)
          VALUES (:firstname, :lastname, :address, :country, :dob, :official_id, :email, :phone_country, :phone_number, :payment_pin)"
            .with(params! {
                "firstname" => firstname,
                "lastname" => lastname,
                "address" => address,
                "country" => country,
                "dob" => dob,
                "official_id" => official_id,
                "email" => email,
                "phone_country" => phone_country,
                "phone_number" => phone_number,
                "payment_pin" => payment_pin,
            }).ignore(&mut conn).await.unwrap();

        match conn.last_insert_id() {
            None => {
                status = false;
                message = "The database insert operation has failed";
            },
            Some(v) =>  {
                log::info!("The newly inserted User ID is: {}", v);
                uid = v;
            },
        }
    }

    drop(conn);
    pool.disconnect().await.unwrap();

    if email.trim().is_empty() && phone_number.trim().is_empty() {
        status = false;
        message = "You must provide a validate email address or a mobile phone number.";
    }

    if status {
        if !email.trim().is_empty() {
            let activation_code_email = Uuid::new_v4().to_string();
            // Put the activation code into the cache
            // store::set(&activation_code_email, serde_json::to_value(uid).unwrap(), None);
            store::set(&activation_code_email, json!({"uid":uid,"mode":"email", "email":email, "phone_country":"", "phone_number":""}), None);
            // send email
            let sg_sender = std::env::var("SENDGRID_FROM").unwrap();
            let sg_api_key = std::env::var("SENDGRID_AUTH_TOKEN").unwrap();
            
            let message_body = format!("Click on the link to activate: {}/activate.html?code={}", origin_site, activation_code_email);
            let p = Personalization::new(Email::new(email));
            let m = Message::new(Email::new(sg_sender))
                .set_subject("Please activate your CODA account")
                .add_content(Content::new().set_content_type("text/html").set_value(message_body))
                .add_personalization(p);

            let sender = Sender::new(sg_api_key);
            match sender.send(&m).await {
                Ok(resp) => {
                    log::info!("Sendgrid response: {:?}", resp);
                }
                Err(e) => {
                    status = false;
                    message = "Cannot send email message.";
                    log::info!("Sendgrid error: {:?}", e);
                }
            }
        }

        if !phone_number.trim().is_empty() {
            let activation_code_phone = Uuid::new_v4().to_string();
            // Put the activation code into the cache
            store::set(&activation_code_phone, json!({"uid":uid,"mode":"phone", "email":"", "phone_country":phone_country, "phone_number":phone_number}), None);
            // send SMS
            match SmsProvider::Twilio.deliver (
                format!("+{}{}", phone_country, phone_number),
                format!("Click on the link to activate: {}/activate.html?code={}", origin_site, activation_code_phone),
            ).await {
                Ok(b) => log::info!("SMS response: {:?}", b),
                Err(e) => {
                    status = false;
                    message = "Cannot send SMS message to your phone number.";
                    log::info!("SMS error: {:?}", e);
                }
            }
            // TEST ONLY: send email
            let sg_sender = std::env::var("SENDGRID_FROM").unwrap();
            let sg_api_key = std::env::var("SENDGRID_AUTH_TOKEN").unwrap();
            let message_body = format!("Click on the link to activate: {}/activate.html?code={}", origin_site, activation_code_phone);
            let m = Message::new(Email::new(sg_sender))
                .set_subject(format!("+{}{}", phone_country, phone_number).as_str())
                .add_content(Content::new().set_content_type("text/html").set_value(message_body))
                .add_personalization(Personalization::new(Email::new("michael@secondstate.io")))
                .add_personalization(Personalization::new(Email::new("vivian@secondstate.io")))
                .add_personalization(Personalization::new(Email::new("juyichen0413@gmail.com")));
            let sender = Sender::new(sg_api_key);
            let _ = sender.send(&m).await;
            // TEST ONLY
        }
    }

    let resp_json = json!({
        "status": status,
        "message": message
    });
    send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn activate(_headers: Vec<(String, String)>, qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    log::info!("ACTIVATION CALLED");
    let code =  qry.get("code").unwrap().as_str().unwrap();
    let code_data = store::get(code).unwrap();
    let user_id : u64 = code_data["uid"].as_u64().unwrap_or_default();
    let mode = code_data["mode"].as_str().unwrap_or_default();
    let email = code_data["email"].as_str().unwrap_or_default();
    let phone_country = code_data["phone_country"].as_str().unwrap_or_default();
    let phone_number = code_data["phone_number"].as_str().unwrap_or_default();
    let mut status = true;
    let mut message = "Activated successfully.";
    let mut resp_headers = get_default_headers();
    let mut tx_hash = "".to_string();
    log::info!("user_id={}, mode={}, email={}, phone_country={}, phone_number={}", user_id, mode, email, phone_country, phone_number);

    let mut initial_tokens_to_user : i32 = 0;
    let csv_data: &'static [u8] = include_bytes!("activation_amounts.csv");
    let mut rdr = Reader::from_reader(csv_data);
    for result in rdr.records() {
        let record = result.unwrap();
        log::info!("CSV: phone_country={}, phone_number={}, amount={}", record.get(2).unwrap(), record.get(3).unwrap(), record.get(4).unwrap());
        if record.get(2).unwrap() == phone_country && record.get(3).unwrap() == phone_number {
            initial_tokens_to_user = record.get(4).unwrap().parse().unwrap();
            break;
        }
    }

    let rpc_node_url = std::env::var("RPC_NODE_URL").unwrap_or("https://mainnet.cybermiles.io".to_string());
    let chain_id = std::env::var("CHAIN_ID").unwrap_or("18".to_string()).parse::<u64>().unwrap_or(18u64);
    let wei_to_eth = U256::from_dec_str("1000000000000000000").unwrap();
    let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());
    let contract = NameOrAddress::from(H160::from_str(&pool_address).expect("Failed to parse contract address"));
    let erc20_address = std::env::var("ERC20_ADDRESS").unwrap_or("0x69d388C32BCDC33D38846087317457A49f52dB20".to_string());
    let erc20 = NameOrAddress::from(H160::from_str(&erc20_address).expect("Failed to parse ERC20 address"));
    let admin_private_key = std::env::var("ADMIN_PRIVATE_KEY").unwrap_or("0x".to_string());
    let admin_wallet: LocalWallet = admin_private_key.parse::<LocalWallet>().unwrap().with_chain_id(chain_id);


    // 1 Generate a user address
    let (eth_address, eth_pk) = gen_key().await;
    let receiver = NameOrAddress::from(H160::from_str(&eth_address)
            .unwrap_or_else(|_| {status = false; message = "Failed to generate a valid ETH address."; H160::zero()}));

    if status {
        // Do the following as the PBM owner
        // 2 Add address to PBM user list
        let value = U256::from_dec_str("0").unwrap();
        let data = create_pbm_adduser_data(receiver.as_address().unwrap().clone()).unwrap();
        let params = json!([wrap_transaction(&rpc_node_url, chain_id, admin_wallet.clone(), contract.clone(), data, value).await.unwrap().as_str()]);
        let _resp = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await
                .unwrap_or_else(|_| {status = false; message = "Failed to add user to the PBM contract."; message.to_string()});
    }

    if status {
        // Do the following as the external token holder
        // 3 Transfer 0.1 CMT gas fee to the user address
        // let value = U256::from_dec_str("100000000000000000").unwrap();
        let value = wei_to_eth / U256::from(10);
        let data = Bytes::from(vec![0u8; 32]);
        let params = json!([wrap_transaction(&rpc_node_url, chain_id, admin_wallet.clone(), receiver.clone(), data, value).await.unwrap().as_str()]);
        let _resp = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await
                .unwrap_or_else(|_| {status = false; message = "Failed to send CMT for gas fee."; message.to_string()});
    }

    if status && initial_tokens_to_user > 0 {
        // 4 Call the ERC20 contract to approve transfer to the PBM contract
        let value = U256::from_dec_str("0").unwrap();
        let data = create_erc20_approve_data(contract.as_address().unwrap().clone(), U256::from(initial_tokens_to_user * 100)).unwrap();
        let params = json!([wrap_transaction(&rpc_node_url, chain_id, admin_wallet.clone(), erc20.clone(), data, value).await.unwrap().as_str()]);
        let _resp = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await
                .unwrap_or_else(|_| {status = false; message = "Failed to approve ERC20 for funding."; message.to_string()});
    }

    if status && initial_tokens_to_user > 0 {
        // 5 Call PBM.fundUser() to send tokens to the PBM contract and record it under the user.
        let value = U256::from_dec_str("0").unwrap();
        let data = create_pbm_funduser_data(receiver.as_address().unwrap().clone(), U256::from(initial_tokens_to_user * 100)).unwrap();
        let params = json!([wrap_transaction(&rpc_node_url, chain_id, admin_wallet.clone(), contract.clone(), data, value).await.unwrap().as_str()]);
        tx_hash = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await
                .unwrap_or_else(|_| {status = false; message = "Failed to fund user."; message.to_string()});
    }

    if status {
        let pool = get_conn_pool();
        let mut conn = pool.get_conn().await.unwrap();

        if mode == "email" {
            r"UPDATE users SET email_active=1, eth_address=:eth_address, eth_pk=:eth_pk WHERE user_id=:user_id"
                .with(params! {
                    "eth_address" => eth_address.clone(),
                    "eth_pk" => eth_pk,
                    "user_id" => user_id,
                }).ignore(&mut conn).await.unwrap_or_else(|_| { status = false; message = "Failed to update database."; });

        } else if mode == "phone" {
            r"UPDATE users SET phone_active=1, eth_address=:eth_address, eth_pk=:eth_pk WHERE user_id=:user_id"
                .with(params! {
                    "eth_address" => eth_address.clone(),
                    "eth_pk" => eth_pk,
                    "user_id" => user_id,
                }).ignore(&mut conn).await.unwrap_or_else(|_| { status = false; message = "Failed to update database."; });

        } else {
            status = false;
            message = "The mode is neither phone or email.";
            log::warn!("MISSED ETH PK is {}", eth_pk);
        }

        if status {
            store::del(code);

            r"INSERT INTO fundings (tx_hash, user_id, user_name, user_phone, user_email, user_address, pool_address, amount)
            VALUES (:tx_hash, :user_id, :user_name, :user_phone, :user_email, :user_address, :pool_address, :amount)"
                .with(params! {
                    "tx_hash" => tx_hash,
                    "user_id" => user_id,
                    "user_name" => "",
                    "user_phone" => "",
                    "user_email" => "",
                    "user_address" => eth_address.clone(),
                    "pool_address" => pool_address,
                    "amount" => initial_tokens_to_user,
                }).ignore(&mut conn).await.unwrap();

            match create_session_for_authuser(&mut conn, user_id, 0).await {
                None => {
                    status = false;
                    message = "Cannot create session for the user";
                },
                Some(sid) =>  {
                    let cookie_domain = std::env::var("COOKIE_DOMAIN").unwrap();
                    resp_headers.push((String::from("Set-Cookie"), format!("SESSIONID={}; Domain={}; Path=/; SameSite=None; Secure; HttpOnly", sid, cookie_domain)));
                },
            }
        }

        drop(conn);
        pool.disconnect().await.unwrap();
    }

    // Now, we wait for 10s for the transactions to be confirmed on the blockchain
    sleep(Duration::from_millis(10000)).await;

    let resp_json = json!({
        "status": status,
        "message": message,
    });
    send_response(200, resp_headers, serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn login_phone(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let empty_value = json!("");
    let origin_site = std::env::var("ORIGIN_SITE").unwrap_or("https://www.thecharityproject.xyz".to_string());

    let mut status = true;
    let mut message = "Success";
    let json: Value = serde_json::from_slice(&body).unwrap();
    log::info!("Input JSON: {}", serde_json::to_string_pretty(&json).unwrap());
    let phone_country = json.get("phone_country").expect("Must have phone country code").as_str().unwrap();
    let phone_number = json.get("phone_number").expect("Must have phone number").as_str().unwrap();
    let role = json.get("role").expect("Must have role").as_str().unwrap().parse::<u64>().unwrap_or_default();
    let redirect_url = json.get("redirect_url").unwrap_or(&empty_value).as_str().unwrap();

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let mut uids: Vec<u64> = Vec::new();
    if role == 0 {
        uids = "SELECT user_id FROM users WHERE phone_country=:phone_country AND phone_number=:phone_number AND phone_active=1"
          .with(params! {
              "phone_country" => phone_country,
              "phone_number" => phone_number,
          })
          .map(&mut conn, |uid| uid).await.unwrap_or(Vec::new());
    } else if role == 1 {
        uids = "SELECT user_id FROM merchants WHERE phone_country=:phone_country AND phone_number=:phone_number"
          .with(params! {
              "phone_country" => phone_country,
              "phone_number" => phone_number,
          })
          .map(&mut conn, |uid| uid).await.unwrap_or(Vec::new());
    }

    drop(conn);
    pool.disconnect().await.unwrap();

    if uids.len() == 1 {
        let uid = uids[0];
        let login_code = Uuid::new_v4().to_string();
        store::set(&login_code, json!({"uid": uid, "role": role, "redirect_url": redirect_url}), None);

        // send SMS
        match SmsProvider::Twilio.deliver (
            format!("+{}{}", phone_country, phone_number),
            format!("Click on the link to login: {}/login.html?code={}", origin_site, login_code),
        ).await {
            Ok(b) => log::info!("SMS response: {:?}", b),
            Err(e) => {
                status = false;
                message = "Failed to send login link! Please try again later.";
                log::info!("SMS error: {:?}", e);
            }
        }
        // TEST ONLY: send email
        let sg_sender = std::env::var("SENDGRID_FROM").unwrap();
        let sg_api_key = std::env::var("SENDGRID_AUTH_TOKEN").unwrap();
        let message_body = format!("Click on the link to login: {}/login.html?code={}", origin_site, login_code);
        let m = Message::new(Email::new(sg_sender))
            .set_subject(format!("+{}{}", phone_country, phone_number).as_str())
            .add_content(Content::new().set_content_type("text/html").set_value(message_body))
            .add_personalization(Personalization::new(Email::new("michael@secondstate.io")))
            .add_personalization(Personalization::new(Email::new("vivian@secondstate.io")))
            .add_personalization(Personalization::new(Email::new("juyichen0413@gmail.com")));
        let sender = Sender::new(sg_api_key);
        let _ = sender.send(&m).await;
        // TEST ONLY

    } else {
        status = false;
        message = "The phone number is not activated. Please sign up first.";
    }
    
    let resp_json = json!({
        "status": status,
        "message": message,
    });
    send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn login_email(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let empty_value = json!("");
    let origin_site = std::env::var("ORIGIN_SITE").unwrap_or("https://www.thecharityproject.xyz".to_string());

    let mut status = true;
    let mut message = "Success";
    let json: Value = serde_json::from_slice(&body).unwrap();
    log::info!("Input JSON: {}", serde_json::to_string_pretty(&json).unwrap());
    let email = json.get("email").expect("Must have email").as_str().unwrap();
    let role = json.get("role").expect("Must have role").as_str().unwrap().parse::<u64>().unwrap_or_default();
    let redirect_url = json.get("redirect_url").unwrap_or(&empty_value).as_str().unwrap();

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let mut uids: Vec<u64> = Vec::new();
    if role == 0 {
        uids = "SELECT user_id FROM users WHERE email=:email AND email_active=1"
          .with(params! {
              "email" => email,
          })
          .map(&mut conn, |uid| uid).await.unwrap_or(Vec::new());
    } else if role == 1 {
        uids = "SELECT user_id FROM merchants WHERE email=:email"
          .with(params! {
              "email" => email,
          })
          .map(&mut conn, |uid| uid).await.unwrap_or(Vec::new());
    }

    drop(conn);
    pool.disconnect().await.unwrap();

    if uids.len() == 1 {
        let uid : u64 = uids[0];
        let login_code = Uuid::new_v4().to_string();
        // store::set(&login_code, serde_json::to_value(uid).unwrap(), None);
        store::set(&login_code, json!({"uid": uid, "role": role, "redirect_url": redirect_url}), None);

        // send email
        let sg_sender = std::env::var("SENDGRID_FROM").unwrap();
        let sg_api_key = std::env::var("SENDGRID_AUTH_TOKEN").unwrap();

        let message_body = format!("Click on the link to login: {}/login.html?code={}", origin_site, login_code);
        let p = Personalization::new(Email::new(email));
        let m = Message::new(Email::new(sg_sender))
            .set_subject("Please login your CODA account")
            .add_content(Content::new().set_content_type("text/html").set_value(message_body))
            .add_personalization(p);

        let sender = Sender::new(sg_api_key);
        match sender.send(&m).await {
            Ok(resp) => {
                log::info!("Sendgrid response: {:?}", resp);
            }
            Err(e) => {
                status = false;
                message = "Failed to send login link! Please try again later.";
                log::info!("Sendgrid error: {:?}", e);
            }
        }

    } else {
        status = false;
        message = "The email is not activated. Please sign up first.";
    }

    let resp_json = json!({
        "status": status,
        "message": message,
    });
    send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn login(_headers: Vec<(String, String)>, qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    let code =  qry.get("code").unwrap().as_str().unwrap();
    // let role =  qry.get("role").unwrap().as_u64().unwrap_or_default();
    let mut user_id : u64 = 0;
    let mut role : u64 = 0;
    let mut redirect_url = "".to_string();
    let mut status = true;
    match store::get(code) {
        Some(v) => {
            log::info!("CODE={} and v={:?}", code, v);
            user_id = v.get("uid").expect("Must have uid").as_u64().unwrap_or_default();
            role = v.get("role").expect("Must have role").as_u64().unwrap_or_default();
            redirect_url = v.get("redirect_url").expect("Must have a redirect URL").to_string();
        },
        None => {
            status = false;
        },
    }
    log::info!("LOGIN UID={} and ROLE={}", user_id, role);
	
    let mut resp_headers = get_default_headers();

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    if user_id != 0 {
        store::del(code);
        match create_session_for_authuser(&mut conn, user_id, role).await {
            None => {
                status = false;
            },
            Some(sid) =>  {
                let cookie_domain = std::env::var("COOKIE_DOMAIN").unwrap();
                resp_headers.push((String::from("Set-Cookie"), format!("SESSIONID={}; Domain={}; Path=/; SameSite=None; Secure; HttpOnly", sid, cookie_domain)));
            },
        }
    }

    drop(conn);
    pool.disconnect().await.unwrap();

    let resp_json = json!({
        "status": status,
        "redirect_url": redirect_url
    });
    log::info!("Sent back HEADERS: {:?}", &resp_headers);
    send_response(200, resp_headers, serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn create_payment_pin (headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    let mut status = true;
    let mut au = get_authuser(headers.clone());
    if au.user_id == 0 {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    }

    let mut rng = rand::thread_rng();
    let mut pin = String::new();
    for _ in 0..4 {
        let r = rng.gen_range(0..10);
        pin.push_str(&r.to_string());
    }

    // send SMS
    match SmsProvider::Twilio.deliver (
        format!("+{}{}", au.phone_country, au.phone_number),
        format!("The payment pin code is {}", pin),
    ).await {
        Ok(b) => log::info!("SMS response: {:?}", b),
        Err(e) => {
            status = false;
            log::info!("SMS error: {:?}", e);
        }
    }
    // TEST ONLY: send email
    let sg_sender = std::env::var("SENDGRID_FROM").unwrap();
    let sg_api_key = std::env::var("SENDGRID_AUTH_TOKEN").unwrap();
    let message_body = format!("The payment pin code is {}", pin);
    let m = Message::new(Email::new(sg_sender))
        .set_subject(format!("+{}{}", au.phone_country, au.phone_number).as_str())
        .add_content(Content::new().set_content_type("text/html").set_value(message_body))
        .add_personalization(Personalization::new(Email::new("michael@secondstate.io")))
        .add_personalization(Personalization::new(Email::new("vivian@secondstate.io")))
        .add_personalization(Personalization::new(Email::new("juyichen0413@gmail.com")));
    let sender = Sender::new(sg_api_key);
    let _ = sender.send(&m).await;
    // TEST ONLY

    let session_id = get_session_id(headers);
    au.pin = pin;
    store::set(&session_id, serde_json::to_value(au).unwrap(), None);

    let resp_json = json!({
        "status": status,
    });
    send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn pay (headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let pin = json.get("pin").expect("Must have pin").as_str().unwrap();
    let to_address = json.get("to_address").expect("Must have to_address").as_str().unwrap();
    let amount = json.get("amount").expect("Must have amount").as_str().unwrap().parse::<f32>().unwrap();
    let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());

    let mut user = User::default();
    let mut merchant = Merchant::default();
    let mut status = true;
    let mut message = "The payment is successful.";
    let mut au = get_authuser(headers.clone());
    if amount > 50.0 && pin != au.pin {
        status = false;
        message = "The user entered the wrong PIN code.";
    } else if au.user_id == 0 {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    } else {
        let pool = get_conn_pool();
        let mut conn = pool.get_conn().await.unwrap();

        // let users = "SELECT firstname, lastname, eth_address, eth_pk FROM users WHERE user_id=:user_id AND payment_pin=:payment_pin"
        let users = "SELECT firstname, lastname, eth_address, eth_pk FROM users WHERE user_id=:user_id"
          .with(params! {
              "user_id" => au.user_id,
              // "payment_pin" => pin,
          }).map(&mut conn, |(firstname, lastname, eth_address, eth_pk)| 
              User::new(au.user_id, firstname, lastname, "".to_string(), "".to_string(), "".to_string(), "".to_string(), au.phone_country.clone(), au.phone_number.clone(), au.phone_active, au.email.clone(), au.email_active, "".to_string(), eth_address, eth_pk)
          ).await.unwrap();

        let merchants = "SELECT user_id, merchant_name FROM merchants WHERE eth_address=:eth_address"
          .with(params! {
              "eth_address" => to_address,
          }).map(&mut conn, |(user_id, merchant_name)| 
              Merchant::new(user_id, merchant_name, "".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string(), to_address.to_string(), "".to_string())
          ).await.unwrap();

        if users.len() == 1 && merchants.len() == 1 {
            user = users.get(0).unwrap().clone();
            merchant = merchants.get(0).unwrap().clone();
        } else {
            status = false;
            message = "Cannot get the user or merchant from the database.";
        }

        drop(conn);
        pool.disconnect().await.unwrap();

        let session_id = get_session_id(headers);
        au.pin = "PIN".to_string();
        store::set(&session_id, serde_json::to_value(au).unwrap(), None);
    }

    if status {
        let tx_hash = pbm_pay(&pool_address, to_address, &user.eth_pk, amount).await
                .unwrap_or_else(|_| {status = false; message = "The blockchain transaction failed."; message.to_string() });

        if status {
            let pool = get_conn_pool();
            let mut conn = pool.get_conn().await.unwrap();
            r"INSERT INTO payments (tx_hash, user_id, user_name, user_phone, user_email, user_address, merchant_id, merchant_name, merchant_address, pool_address, amount)
            VALUES (:tx_hash, :user_id, :user_name, :user_phone, :user_email, :user_address, :merchant_id, :merchant_name, :merchant_address, :pool_address, :amount)"
                .with(params! {
                    "tx_hash" => tx_hash,
                    "user_id" => user.user_id,
                    "user_name" => format!("{} {}", user.firstname, user.lastname),
                    "user_phone" => format!("+{} {}", user.phone_country, user.phone_number),
                    "user_email" => user.email,
                    "user_address" => user.eth_address,
                    "merchant_id" => merchant.user_id,
                    "merchant_name" => merchant.merchant_name,
                    "merchant_address" => to_address,
                    "pool_address" => pool_address,
                    "amount" => amount,
                }).ignore(&mut conn).await.unwrap();
            drop(conn);
            pool.disconnect().await.unwrap();
        }
    }

    let resp_json = json!({
        "status": status,
        "message": message,
    });
    send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn user_transactions(headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());

    let au = get_authuser(headers);
    log::info!("AU ID: {}", au.user_id);
    if au.user_id == 0  || au.role != 0 {

        send_response(401, get_default_headers(), "".as_bytes().to_vec());

    } else {

        let pool = get_conn_pool();
        let mut conn = pool.get_conn().await.unwrap();

        let payments = "SELECT payment_id, created_on, tx_hash, amount, merchant_id, merchant_name FROM payments WHERE user_id=:user_id ORDER BY created_on DESC"
          .with(params! {
              "user_id" => au.user_id,
          }).map(&mut conn,
               |(payment_id, created_on, tx_hash, amount, merchant_id, merchant_name)|
               Payment::new(payment_id, created_on, tx_hash, au.user_id, "".to_string(), "".to_string(), "".to_string(), "".to_string(), merchant_id, merchant_name, "".to_string(), pool_address.clone(), amount)
          ).await.unwrap();

        let fundings = "SELECT funding_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, user_address, amount FROM fundings WHERE user_id=:user_id ORDER BY created_on DESC"
          .with(params! {
              "user_id" => au.user_id,
          })
          .map(&mut conn,
               |(funding_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, user_address, amount)|
               Funding::new(funding_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, user_address, pool_address.clone(), amount)
          ).await.unwrap();

        drop(conn);
        pool.disconnect().await.unwrap();

        let mut txs: Vec<Value> = vec!();
        for funding in fundings.iter() {
            let tx = json!({
                "timestamp": format!("{}", funding.created_on),
                "tx_hash": funding.tx_hash,
                "merchant_name": "",
                "amount": funding.amount
            });
            txs.push(tx);
        }
        for payment in payments.iter() {
            let tx = json!({
                "timestamp": format!("{}", payment.created_on),
                "tx_hash": payment.tx_hash,
                "merchant_name": payment.merchant_name,
                "amount": (-1.0) * payment.amount
            });
            txs.push(tx);
        }
        let res_json:Value = txs.into();
        send_response(
            200, get_default_headers(),
            serde_json::to_vec_pretty(&res_json).unwrap(),
        );
    }
}

async fn merchant_transactions(headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());

    let au = get_authuser(headers);
    log::info!("AU ID: {}", au.user_id);
    if au.user_id == 0  || au.role != 1 {

        send_response(401, get_default_headers(), "".as_bytes().to_vec());

    } else {

        let pool = get_conn_pool();
        let mut conn = pool.get_conn().await.unwrap();

        let payments = "SELECT payment_id, created_on, tx_hash, amount, user_id, user_name, user_phone, user_email FROM payments WHERE merchant_id=:merchant_id ORDER BY created_on DESC"
          .with(params! {
              "merchant_id" => au.user_id,
          }).map(&mut conn,
               |(payment_id, created_on, tx_hash, amount, user_id, user_name, user_phone, user_email)|
               Payment::new(payment_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, "".to_string(), au.user_id, "".to_string(), "".to_string(), pool_address.clone(), amount)
          ).await.unwrap();

        let redeems = "SELECT redeem_id, created_on, tx_hash, merchant_id, merchant_name, merchant_address, redeem_address, amount FROM redeems WHERE merchant_id=:merchant_id ORDER BY created_on DESC"
          .with(params! {
              "merchant_id" => au.user_id,
          })
          .map(&mut conn,
               |(redeem_id, created_on, tx_hash, merchant_id, merchant_name, merchant_address, redeem_address, amount)|
               Redeem::new(redeem_id, created_on, tx_hash, merchant_id, merchant_name, merchant_address, redeem_address, amount)
          ).await.unwrap();

        drop(conn);
        pool.disconnect().await.unwrap();

        let mut txs: Vec<Value> = vec!();
        for redeem in redeems.iter() {
            let tx = json!({
                "timestamp": format!("{}", redeem.created_on),
                "tx_hash": redeem.tx_hash,
                "user_name": "",
                "user_phone": "",
                "user_email": "",
                "amount": (-1.0) * redeem.amount
            });
            txs.push(tx);
        }
        for payment in payments.iter() {
            let tx = json!({
                "timestamp": format!("{}", payment.created_on),
                "tx_hash": payment.tx_hash,
                "user_name": payment.user_name,
                "user_phone": payment.user_phone,
                "user_email": payment.user_email,
                "amount": payment.amount
            });
            txs.push(tx);
        }
        let res_json:Value = txs.into();
        send_response(
            200, get_default_headers(),
            serde_json::to_vec_pretty(&res_json).unwrap(),
        );
    }
}

async fn funding_transactions(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let password = json.get("password").expect("Must have password").as_str().unwrap();

    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap();
    if password != admin_password {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    }

    let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let fundings = "SELECT funding_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, user_address, amount FROM fundings ORDER BY created_on DESC"
          .with(())
          .map(&mut conn,
               |(funding_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, user_address, amount)|
               Funding::new(funding_id, created_on, tx_hash, user_id, user_name, user_phone, user_email, user_address, pool_address.clone(), amount)
          ).await.unwrap();

    drop(conn);
    pool.disconnect().await.unwrap();

    let mut txs: Vec<Value> = vec!();
    for funding in fundings.iter() {
        let tx = json!({
            "timestamp": format!("{}", funding.created_on),
            "tx_hash": funding.tx_hash,
            "user_name": funding.user_name,
            "user_phone": funding.user_phone,
            "user_email": funding.user_email,
            "user_address": funding.user_address,
            "amount": funding.amount
        });
        txs.push(tx);
    }
    let res_json:Value = txs.into();
    send_response(
        200, get_default_headers(),
        serde_json::to_vec_pretty(&res_json).unwrap(),
    );
}

async fn redeem_transactions(_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let password = json.get("password").expect("Must have password").as_str().unwrap();

    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap();
    if password != admin_password {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    }

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let redeems = "SELECT redeem_id, created_on, tx_hash, merchant_id, merchant_name, merchant_address,, redeem_address, amount FROM redeems ORDER BY created_on DESC"
          .with(())
          .map(&mut conn,
               |(redeem_id, created_on, tx_hash, merchant_id, merchant_name, merchant_address, redeem_address, amount)|
               Redeem::new(redeem_id, created_on, tx_hash, merchant_id, merchant_name, merchant_address, redeem_address, amount)
          ).await.unwrap();

    drop(conn);
    pool.disconnect().await.unwrap();

    let mut txs: Vec<Value> = vec!();
    for redeem in redeems.iter() {
        let tx = json!({
            "timestamp": format!("{}", redeem.created_on),
            "tx_hash": redeem.tx_hash,
            "merchant_name": redeem.merchant_name,
            "merchant_address": redeem.merchant_address,
            "redeem_address": redeem.redeem_address,
            "amount": redeem.amount
        });
        txs.push(tx);
    }
    let res_json:Value = txs.into();
    send_response(
        200, get_default_headers(),
        serde_json::to_vec_pretty(&res_json).unwrap(),
    );
}

async fn profile(headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    log::info!("HEADERS: {:?}", &headers);
    let mut status = true;
    let mut user = User::default();
    let mut merchant = Merchant::default();
    let mut profile_address : &str = "";

    let rpc_node_url = std::env::var("RPC_NODE_URL").unwrap_or("https://mainnet.cybermiles.io".to_string());
    let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());
    let erc20_address = std::env::var("ERC20_ADDRESS").unwrap_or("0x69d388c32bcdc33d38846087317457a49f52db20".to_string());
    // let contract = NameOrAddress::from(H160::from_str(&pool_address).expect("Failed to parse contract address"));
    // let wei_to_eth = U256::from_dec_str("1000000000000000000").unwrap();

    let au = get_authuser(headers);
    log::info!("AU ID: {}", au.user_id);
    if au.user_id == 0 {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    } else {

        if au.role == 0 {
            let pool = get_conn_pool();
            let mut conn = pool.get_conn().await.unwrap();

            let users = "SELECT firstname, lastname, eth_address FROM users WHERE user_id=:user_id"
              .with(params! {
                  "user_id" => au.user_id,
              }).map(&mut conn,
                   |(firstname, lastname, eth_address)|
                   User::new(au.user_id, firstname, lastname, "".to_string(), "".to_string(), "".to_string(), "".to_string(), au.phone_country.clone(), au.phone_number.clone(), au.phone_active, au.email.clone(), au.email_active, "".to_string(), eth_address, "na".to_string())
              ).await.unwrap();

            drop(conn);
            pool.disconnect().await.unwrap();

            if users.len() == 1 {
                user = users.get(0).unwrap().clone();
                log::info!("USER ID: {}", user.user_id);
                profile_address = &user.eth_address;

            } else {
                status = false;
            }

        } else if au.role == 1 {
            let pool = get_conn_pool();
            let mut conn = pool.get_conn().await.unwrap();

            let merchants = "SELECT merchant_name, eth_address FROM merchants WHERE user_id=:user_id"
              .with(params! {
                  "user_id" => au.user_id,
              }).map(&mut conn,
                   |(merchant_name, eth_address)|
                   Merchant::new(au.user_id, merchant_name, "".to_string(), "".to_string(), au.phone_country.clone(), au.phone_number.clone(), au.email.clone(), eth_address, "na".to_string())
              ).await.unwrap();

            drop(conn);
            pool.disconnect().await.unwrap();

            if merchants.len() == 1 {
                merchant = merchants.get(0).unwrap().clone();
                log::info!("MERCHANT ID: {}", user.user_id);
                profile_address = &merchant.eth_address;

            } else {
                status = false;
            }
        }
    }

    /*
    // get ETH balance from eth_address
    log::info!("PROFILE: {}", profile_address);
    let params = json!([profile_address, "latest"]);
    log::info!("DATA: {:?}", params);
    let result = json_rpc(&rpc_node_url, "eth_getBalance", params).await.expect("Failed to send json.");
    log::info!("RESULT: {}", result);
    // Must remove the leading 0x
    let wei_balance = U256::from_str_radix(&result[2..], 16).unwrap();
    log::info!("WEI BALANCE: {}", wei_balance);
    let eth_balance = wei_balance.as_u128() as f64 / wei_to_eth.as_u128() as f64;
    log::info!("ETH BALANCE: {}", eth_balance);
    */

    if au.role == 0 {
        // get balance from eth_address
        let data = create_pbm_balance_data(H160::from_str(profile_address).unwrap()).unwrap();
        let params = json!([{
            "from": profile_address, 
            "to": &pool_address, 
            "data": format!("{:}", data).as_str()}, "latest"]);
        let result = json_rpc(&rpc_node_url, "eth_call", params).await.expect("Failed to send json.");
        // Must remove the leading 0x
        // let balance = (U256::from_str_radix(&result[2..], 16).unwrap() / wei_to_eth).as_u64();
        let balance = u64::from_str_radix(&result[2..], 16).unwrap() as f64 / 100.;
        let resp_json = json!({
            "status": status,
            "firstname": user.firstname,
            "lastname": user.lastname,
            "role": au.role,
            "balance": balance,
        });
        send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());

    } else if au.role == 1 {
        // get balance from eth_address
        let data = create_erc20_balance_data(H160::from_str(profile_address).unwrap()).unwrap();
        let params = json!([{
            "from": profile_address, 
            "to": &erc20_address, 
            "data": format!("{:}", data).as_str()}, "latest"]);
        let result = json_rpc(&rpc_node_url, "eth_call", params).await.expect("Failed to send json.");
        // Must remove the leading 0x
        // let balance = (U256::from_str_radix(&result[2..], 16).unwrap() / wei_to_eth).as_u64();
        let balance = u64::from_str_radix(&result[2..], 16).unwrap() as f64 / 100.;
        let resp_json = json!({
            "status": status,
            "merchant_name": merchant.merchant_name,
            "role": au.role,
            "balance": balance,
            "eth_address": merchant.eth_address,
        });
        send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
    }
}

async fn profile_simple (headers: Vec<(String, String)>, _qry: HashMap<String, Value>, _body: Vec<u8>) {
    logger::init();
    // let mut user = User::default();
    // let mut merchant = Merchant::default();

    let mut status = true;
    let mut name = "".to_string();

    let au = get_authuser(headers);
    if au.user_id == 0 {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    } else {

        if au.role == 0 {
            let pool = get_conn_pool();
            let mut conn = pool.get_conn().await.unwrap();

            let users = "SELECT firstname, lastname, eth_address FROM users WHERE user_id=:user_id"
              .with(params! {
                  "user_id" => au.user_id,
              }).map(&mut conn,
                   |(firstname, lastname, eth_address)|
                   User::new(au.user_id, firstname, lastname, "".to_string(), "".to_string(), "".to_string(), "".to_string(), au.phone_country.clone(), au.phone_number.clone(), au.phone_active, au.email.clone(), au.email_active, "".to_string(), eth_address, "na".to_string())
              ).await.unwrap();

            drop(conn);
            pool.disconnect().await.unwrap();

            if users.len() == 1 {
                let user = users.get(0).unwrap();
                name = format!("{} {}", user.firstname, user.lastname);
            } else {
                status = false;
            }

        } else if au.role == 1 {
            let pool = get_conn_pool();
            let mut conn = pool.get_conn().await.unwrap();

            let merchants = "SELECT merchant_name, eth_address FROM merchants WHERE user_id=:user_id"
              .with(params! {
                  "user_id" => au.user_id,
              }).map(&mut conn,
                   |(merchant_name, eth_address)|
                   Merchant::new(au.user_id, merchant_name, "".to_string(), "".to_string(), au.phone_country.clone(), au.phone_number.clone(), au.email.clone(), eth_address, "na".to_string())
              ).await.unwrap();

            drop(conn);
            pool.disconnect().await.unwrap();

            if merchants.len() == 1 {
                let merchant = merchants.get(0).unwrap();
                name = merchant.merchant_name.clone();
            } else {
                status = false;
            }

        } else {
            status = false;
        }
    }

    let resp_json = json!({
        "status": status,
        "name": name,
        "role": au.role,
    });
    send_response(200, get_default_headers(), serde_json::to_string(&resp_json).unwrap().as_bytes().to_vec());
}

async fn donor_report (_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let password = json.get("password").expect("Must have password").as_str().unwrap();

    let donor_password = std::env::var("DONOR_PASSWORD").unwrap();
    if password != donor_password {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    }

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let users: Vec<u64> = "SELECT COUNT(*) FROM users WHERE phone_active=1"
      .with(())
      .map(&mut conn, |c| c).await.unwrap_or(Vec::new());
    let users = users[0];

    let payments: Vec<u64> = "SELECT COUNT(*) FROM payments"
      .with(())
      .map(&mut conn, |c| c).await.unwrap_or(Vec::new());
    let payments = payments[0];

    let amounts: Vec<f64> = "SELECT SUM(amount) FROM payments"
      .with(())
      .map(&mut conn, |c| c).await.unwrap_or(Vec::new());
    let amounts = amounts[0];

    drop(conn);
    pool.disconnect().await.unwrap();

    let res_json = json!({
        "users": users,
        "payments": payments,
        "amounts": amounts
    });

    send_response(200, get_default_headers(), serde_json::to_vec_pretty(&res_json).unwrap());
}

async fn add_fund (_headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let mut status = true;
    let json: Value = serde_json::from_slice(&body).unwrap();
    let password = json.get("password").expect("Must have password").as_str().unwrap();
    let phone_country = json.get("phone_country").expect("Must have country code").as_str().unwrap();
    let phone_number = json.get("phone_number").expect("Must have phone number").as_str().unwrap();
    let amount = json.get("amount").expect("Must have amount").as_f64().unwrap();
    let amountx100 = (amount * 100.) as u64;

    let admin_password = std::env::var("ADMIN_PASSWORD").unwrap();
    if password != admin_password {
        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;
    }

    let pool = get_conn_pool();
    let mut conn = pool.get_conn().await.unwrap();

    let mut user = User::default();
    let users = "SELECT user_id, firstname, lastname, email, eth_address FROM users WHERE phone_country=:phone_country AND phone_number=:phone_number AND phone_active=1"
        .with(params! {
            "phone_country" => phone_country,
            "phone_number" => phone_number,
        }).map(&mut conn, |(user_id, firstname, lastname, email, eth_address)|
            User::new(user_id, firstname, lastname, "".to_string(), "".to_string(), "".to_string(), "".to_string(), phone_country.to_string(), phone_number.to_string(), 0, email, 0, "".to_string(), eth_address, "".to_string())
        ).await.unwrap();

    if users.len() == 1 {
        user = users.get(0).unwrap().clone();
    } else {
        status = false;
    }

    drop(conn);
    pool.disconnect().await.unwrap();

    if status {
        let rpc_node_url = std::env::var("RPC_NODE_URL").unwrap_or("https://mainnet.cybermiles.io".to_string());
        let chain_id = std::env::var("CHAIN_ID").unwrap_or("18".to_string()).parse::<u64>().unwrap_or(18u64);
        // let wei_to_eth = U256::from_dec_str("1000000000000000000").unwrap();
        let pool_address = std::env::var("POOL_ADDRESS").unwrap_or("0x3b8535d3511d564c5B72Eb16977830543aC1b89c".to_string());
        let contract = NameOrAddress::from(H160::from_str(&pool_address).expect("Failed to parse contract address"));
        let erc20_address = std::env::var("ERC20_ADDRESS").unwrap_or("0x69d388C32BCDC33D38846087317457A49f52dB20".to_string());
        let erc20 = NameOrAddress::from(H160::from_str(&erc20_address).expect("Failed to parse ERC20 address"));
        let admin_private_key = std::env::var("ADMIN_PRIVATE_KEY").unwrap_or("0x".to_string());
        let admin_wallet: LocalWallet = admin_private_key.parse::<LocalWallet>().unwrap().with_chain_id(chain_id);
        let receiver = NameOrAddress::from(H160::from_str(user.eth_address.as_str()).expect("Failed to parse address"));

        // 1 Call the ERC20 contract to approve transfer to the PBM contract
        let value = U256::from_dec_str("0").unwrap();
        let data = create_erc20_approve_data(contract.as_address().unwrap().clone(), U256::from(amountx100)).unwrap();
        let params = json!([wrap_transaction(&rpc_node_url, chain_id, admin_wallet.clone(), erc20.clone(), data, value).await.unwrap().as_str()]);
        let resp = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await.expect("Failed to approve erc20");
        log::info!("resp for approve erc20: {:#?}", resp);

        // 2 Call PBM.fundUser() to send tokens to the PBM contract and record it under the user.
        let value = U256::from_dec_str("0").unwrap();
        let data = create_pbm_funduser_data(receiver.as_address().unwrap().clone(), U256::from(amountx100)).unwrap();
        let params = json!([wrap_transaction(&rpc_node_url, chain_id, admin_wallet.clone(), contract.clone(), data, value).await.unwrap().as_str()]);
        let tx_hash = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await.expect("Failed to fundUser");
        log::info!("resp for fundUser: {:#?}", tx_hash);

        let pool = get_conn_pool();
        let mut conn = pool.get_conn().await.unwrap();

        r"INSERT INTO fundings (tx_hash, user_id, user_name, user_phone, user_email, user_address, pool_address, amount)
        VALUES (:tx_hash, :user_id, :user_name, :user_phone, :user_email, :user_address, :pool_address, :amount)"
            .with(params! {
                "tx_hash" => tx_hash,
                "user_id" => user.user_id,
                "user_name" => format!("{} {}", user.firstname, user.lastname),
                "user_phone" => format!("+{} {}", user.phone_country, user.phone_number),
                "user_email" => user.email,
                "user_address" => user.eth_address,
                "pool_address" => pool_address,
                "amount" => amount,
            }).ignore(&mut conn).await.unwrap();

        drop(conn);
        pool.disconnect().await.unwrap();
    }

    let res_json = json!({
        "status": status
    });
    send_response(200, get_default_headers(), serde_json::to_vec_pretty(&res_json).unwrap());
}

async fn merchant_redeem (headers: Vec<(String, String)>, _qry: HashMap<String, Value>, body: Vec<u8>) {
    logger::init();
    let mut status = true;
    let mut message = "Success";
    let json: Value = serde_json::from_slice(&body).unwrap();
    let amount = json.get("amount").expect("Must have amount").as_f64().unwrap();
    let amountx100 = (amount * 100.) as u64;
    log::info!("amountx100: {}", amountx100);
    let mut merchant = Merchant::default();

    let au = get_authuser(headers);
    log::info!("AU ID: {}", au.user_id);
    if au.user_id == 0  || au.role != 1 {

        send_response(401, get_default_headers(), "".as_bytes().to_vec());
        return;

    } else {

        let pool = get_conn_pool();
        let mut conn = pool.get_conn().await.unwrap();

        let merchants = "SELECT merchant_name, eth_address, eth_pk FROM merchants WHERE user_id=:user_id"
          .with(params! {
              "user_id" => au.user_id,
          }).map(&mut conn, |(merchant_name, eth_address, eth_pk)| 
              Merchant::new(au.user_id, merchant_name, "".to_string(), "".to_string(), "".to_string(), "".to_string(), "".to_string(), eth_address, eth_pk)
          ).await.unwrap();

        if merchants.len() == 1 {
            merchant = merchants.get(0).unwrap().clone();
            log::info!("merchant_name: {}", merchant.merchant_name);
        } else {
            status = false;
            message = "Cannot find the merchant";
        }

        drop(conn);
        pool.disconnect().await.unwrap();

        if status {
            let rpc_node_url = std::env::var("RPC_NODE_URL").unwrap_or("https://mainnet.cybermiles.io".to_string());
            let chain_id = std::env::var("CHAIN_ID").unwrap_or("18".to_string()).parse::<u64>().unwrap_or(18u64);
            let erc20_address = std::env::var("ERC20_ADDRESS").unwrap_or("0x69d388C32BCDC33D38846087317457A49f52dB20".to_string());
            let erc20 = NameOrAddress::from(H160::from_str(&erc20_address).expect("Failed to parse ERC20 address"));
            let redeem_address = std::env::var("REDEEM_ADDRESS").unwrap_or("0x".to_string());
            let merchant_wallet: LocalWallet = merchant.eth_pk.parse::<LocalWallet>().unwrap().with_chain_id(chain_id);
            let receiver = NameOrAddress::from(H160::from_str(redeem_address.as_str()).expect("Failed to parse address"));

            // Call the ERC20 contract to transfer
            let value = U256::from_dec_str("0").unwrap();
            let data = create_erc20_transfer_data(receiver.as_address().unwrap().clone(), U256::from(amountx100)).unwrap();
            let params = json!([wrap_transaction(&rpc_node_url, chain_id, merchant_wallet.clone(), erc20.clone(), data, value).await.unwrap().as_str()]);
            let tx_hash = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await.expect("Failed to transfer erc20");
            log::info!("resp for erc20 transfer: {:#?}", tx_hash);

            let pool = get_conn_pool();
            let mut conn = pool.get_conn().await.unwrap();

            r"INSERT INTO redeems (tx_hash, merchant_id, merchant_name, merchant_address, redeem_address, amount)
            VALUES (:tx_hash, :merchant_id, :merchant_name, :merchant_address, :redeem_address, :amount)"
                .with(params! {
                    "tx_hash" => tx_hash,
                    "merchant_id" => merchant.user_id,
                    "merchant_name" => merchant.merchant_name,
                    "merchant_address" => merchant.eth_address,
                    "redeem_address" => redeem_address,
                    "amount" => amount,
                }).ignore(&mut conn).await.unwrap();

            drop(conn);
            pool.disconnect().await.unwrap();
        }
    }

    let res_json = json!({
        "status": status,
        "message": message,
    });
    send_response(200, get_default_headers(), serde_json::to_vec_pretty(&res_json).unwrap());
}

async fn create_session_for_authuser (conn: &mut Conn, user_id: u64, role: u64) -> Option<String> {
    let mut auth_users = Vec::new();
    if role == 0 {
        auth_users = "SELECT phone_country, phone_number, phone_active, email, email_active FROM users WHERE user_id=:user_id"
          .with(params! {
              "user_id" => user_id,
          }).map(conn,
               |(phone_country, phone_number, phone_active, email, email_active)|
               AuthUser::new(user_id, "PIN".to_string(), phone_country, phone_number, phone_active, email, email_active, role)
          ).await.unwrap();

    } else if role == 1 {
        auth_users = "SELECT phone_country, phone_number, email FROM merchants WHERE user_id=:user_id"
          .with(params! {
              "user_id" => user_id,
          }).map(conn,
               |(phone_country, phone_number, email)|
               AuthUser::new(user_id, "PIN".to_string(), phone_country, phone_number, -1, email, -1, role)
          ).await.unwrap();
    }

    if auth_users.len() == 1 {
        let auth_user = auth_users.get(0).unwrap();
        let session_id = Uuid::new_v4().to_string();
        log::info!("TRYING to set session {} for {}", &session_id, auth_user.user_id);
        store::set(&session_id, serde_json::to_value(auth_user).unwrap(), None);
        log::info!("DONE");
        Some(session_id)
    } else {
        None
    }
}

fn get_session_id (headers: Vec<(String, String)>) -> String {
    let mut session_id = String::new();
    for (k, v) in headers {
        if k.to_lowercase() == "cookie" {
            for cookie in v.trim().split(";") {
                let parts: Vec<&str> = cookie.trim().split("=").collect();
                if parts.len() == 2 && parts.get(0).unwrap().to_string() == "SESSIONID" {
                    session_id = parts.get(1).unwrap().trim().to_string();
                }
            }
        }
    }
    return session_id;
}

fn get_authuser (headers: Vec<(String, String)>) -> AuthUser {
    let session_id = get_session_id(headers);
    log::info!("TRYING to get session from store: {}", &session_id);
    let value = store::get(&session_id).unwrap_or_default();
    log::info!("GOT session from store: {:?}", &value);
    return serde_json::from_value(value).unwrap_or_default();
}

async fn gen_key () -> (String, String) {
    let wallet;
    let mut rng = rand::thread_rng();
    wallet = MnemonicBuilder::<English>::default()
      .word_count(24)
      .derivation_path("m/44'/60'/0'/2/1")
      .unwrap()
      .build_random(&mut rng)
      .unwrap();

    // let addr = wallet.address().to_string();
    let addr = format!("{:?}", wallet.address());
    let key = format!("0x{}", hex::encode(wallet.signer().to_bytes()));
    return (addr, key);
}

async fn pbm_pay (pool_addr: &str, to_addr: &str, private_key: &str, amount: f32) -> Result<String> {
    let rpc_node_url = std::env::var("RPC_NODE_URL").unwrap_or("https://mainnet.cybermiles.io".to_string());
    let chain_id = std::env::var("CHAIN_ID").unwrap_or("18".to_string()).parse::<u64>().unwrap_or(18u64);
    // let contract_addrss = NameOrAddress::from(H160::from_str(std::env::var("CONTRACT_ADDRESS").unwrap_or("0x2ba7EA93b29286CB1f65c151ea0ad97FcCD41C91".to_string()).as_str()).expect("Failed to parse contract address"));
    let contract = NameOrAddress::from(H160::from_str(pool_addr).expect("Failed to parse contract address"));

    let wallet: LocalWallet = private_key.parse::<LocalWallet>().unwrap().with_chain_id(chain_id);
    let receiver = NameOrAddress::from(H160::from_str(to_addr).expect("Failed to parse address"));
    let value = U256::from_dec_str("0").unwrap();
    // let wei_to_eth = U256::from_dec_str("1000000000000000000").unwrap();
    let amount_u256 = U256::from((amount * 100.) as u64);

    // Call the PBM pay() function
    let data = create_pbm_pay_data(receiver.as_address().unwrap().clone(), amount_u256).unwrap();
    log::info!("Parameter: {:#?} {:#?}", data, receiver);
    let params = json!([wrap_transaction(&rpc_node_url, chain_id, wallet, contract, data, value).await.unwrap().as_str()]);
    // let resp = json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await.expect("Failed to send raw transaction.");
    // log::info!("resp: {:#?}", resp);
    return json_rpc(&rpc_node_url, "eth_sendRawTransaction", params).await;
}

pub fn create_pbm_pay_data(receiver_address: H160, amount: U256) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "receiver", "type": "address"},
                {"internalType": "uint256", "name": "amount", "type": "uint256"}
            ],
            "name": "pay",
            "outputs": [],
            "stateMutability": "nonpayable",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "pay")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(receiver_address), Token::Uint(amount.into())];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub fn create_pbm_funduser_data(user_address: H160, amount: U256) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "user", "type": "address"},
                {"internalType": "uint256", "name": "amount", "type": "uint256"}
            ],
            "name": "fundUser",
            "outputs": [],
            "stateMutability": "nonpayable",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "fundUser")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(user_address), Token::Uint(amount.into())];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub fn create_pbm_adduser_data(user_address: H160) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "user", "type": "address"}
            ],
            "name": "addUser",
            "outputs": [],
            "stateMutability": "nonpayable",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "addUser")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(user_address)];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub fn create_erc20_approve_data (user_address: H160, amount: U256) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "user", "type": "address"},
                {"internalType": "uint256", "name": "amount", "type": "uint256"}
            ],
            "name": "approve",
            "outputs": [
                {"name": "", "type": "bool"}
            ],
            "stateMutability": "nonpayable",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "approve")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(user_address), Token::Uint(amount.into())];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub fn create_erc20_transfer_data (user_address: H160, amount: U256) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "user", "type": "address"},
                {"internalType": "uint256", "name": "amount", "type": "uint256"}
            ],
            "name": "transfer",
            "outputs": [
                {"name": "", "type": "bool"}
            ],
            "stateMutability": "nonpayable",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "transfer")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(user_address), Token::Uint(amount.into())];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub fn create_pbm_balance_data(user_address: H160) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "user", "type": "address"}
            ],
            "name": "balanceOf",
            "outputs": [
                {"internalType": "uint256", "name": "", "type": "uint256"}
            ],
            "stateMutability": "view",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "balanceOf")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(user_address)];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub fn create_erc20_balance_data(user_address: H160) -> Result<Bytes> {
    let contract_abi: &str = r#"[
        {
            "inputs": [
                {"internalType": "address", "name": "user", "type": "address"}
            ],
            "name": "balanceOf",
            "outputs": [
                {"internalType": "uint256", "name": "", "type": "uint256"}
            ],
            "stateMutability": "view",
            "type": "function"
        }
    ]"#;
    let abi: Abi = serde_json::from_str(contract_abi).unwrap();
    let function: &Function = abi
        .functions()
        .find(|&f| f.name == "balanceOf")
        .ok_or("Function not found in ABI")?;

    let tokens = vec![Token::Address(user_address)];
    let data = function.encode_input(&tokens).unwrap();

    Ok(Bytes::from(data))
}

pub async fn wrap_transaction(rpc_node_url: &str, chain_id: u64, wallet: LocalWallet, address_to: NameOrAddress, data: Bytes, value: U256) -> Result<String> {
    let address_from = wallet.address();
    log::info!("NONCE");
    let nonce = get_nonce(&rpc_node_url, format!("{:?}", wallet.address()).as_str()).await.unwrap();
    log::info!("GAS PRICE");
    let gas_price = get_gas_price(&rpc_node_url).await.expect("Failed to get gas price.");
    // let gas_price = U256::from(21000);
    log::info!("ESTIMATE GAS");
    let estimate_gas = get_estimate_gas(
        &rpc_node_url, 
        format!("{:?}", address_from).as_str(),
        format!("{:?}", address_to.as_address().expect("Failed to transfer address")).as_str(),
        format!("0x{:x}", value).as_str(), format!("{:}", data).as_str()
    ).await.expect("Failed to gat estimate gas.") * U256::from(12) / U256::from(10);
    // let estimate_gas = U256::from(4000000);

    let tx: TypedTransaction = TransactionRequest::new()
      .from(address_from)
      .to(address_to)
      .nonce::<U256>(nonce.into())
      .gas_price::<U256>(gas_price.into())
      .gas::<U256>(estimate_gas.into())
      .chain_id::<U64>(chain_id.into())
      .data::<Bytes>(data.into())
      .value(value).into();

    log::info!("Tx: {:#?}", tx);
    let signature = wallet.sign_transaction(&tx).await.expect("Failed to sign.");
    Ok(format!("0x{}", hex::encode(tx.rlp_signed(&signature))))
}

pub async fn get_gas_price(rpc_node_url: &str) -> Result<U256> {
    let params = json!([]);
    let result = json_rpc(rpc_node_url, "eth_gasPrice", params).await.expect("Failed to send json.");

    Ok(U256::from_str(&result)?)
}

pub async fn get_nonce(rpc_node_url: &str, address: &str) -> Result<U256> {
    let params = json!([address, "pending"]);
    let result = json_rpc(rpc_node_url, "eth_getTransactionCount", params).await.expect("Failed to send json.");

    Ok(U256::from_str(&result)?)
}

pub async fn get_estimate_gas(rpc_node_url: &str, from: &str, to: &str, value: &str, data: &str) -> Result<U256> {
    let params = json!([{"from": from, "to": to, "value":value, "data":data}]);
    log::info!("ESTIMATE GAS: {:?}", params);
    let result = json_rpc(rpc_node_url, "eth_estimateGas", params).await.expect("Failed to send json.");
    log::info!("ESTIMATE GAS done");

    Ok(U256::from_str(&result)?)
}

pub async fn json_rpc(url: &str, method: &str, params: Value) -> Result<String> {
    let client = reqwest::Client::new();
    let res = client
        .post(url)
        .header("Content-Type","application/json")
        .body(json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }).to_string())
        .send()
        .await?;

    let body = res.text().await?;
    log::info!("JSPN-RPC result: {}", body.as_str());
    let map: HashMap<String, serde_json::Value> = serde_json::from_str(body.as_str())?;
    log::info!("JSON_RPC {} response body: {:#?}", method, map);

    if !map.contains_key("result"){
        log::error!("JSON_RPC {} request body: {:#?}", method, json!({
            "jsonrpc": "2.0",
            "method": method,
            "params": params,
            "id": 1
        }));
    }
    // Ok(map["result"].as_str().expect("Failed to parse json.").to_string())
    Ok(serde_json::to_string(&map["result"]).expect("Failed to parse str.").trim_matches('"').to_string())
}
