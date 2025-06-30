use axum::{
    http::StatusCode,
    response::Json,
    routing::post,
    Router,
};
use serde::{Deserialize, Serialize};
use solana_sdk::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    signer::{keypair::Keypair, Signer},
    signature::Signature,
};
use solana_sdk::system_instruction;
use spl_token;
use spl_associated_token_account::instruction as spl_associated_token_instruction;
use tokio::net::TcpListener;
use std::str::FromStr;
use base64::{engine::general_purpose, Engine as _};


#[derive(Serialize)]
struct SuccessResponse<T> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct ErrorResponse {
    success: bool,
    error: String,
}

impl ErrorResponse {
    fn new(msg: &str) -> Self {
        ErrorResponse {
            success: false,
            error: msg.to_string(),
        }
    }
}


#[derive(Serialize)]
struct KeypairResponse {
    pubkey: String,
    secret: String,
}

#[derive(Deserialize)]
struct CreateTokenRequest {
    #[serde(rename = "mintAuthority")]
    mint_authority: String,
    mint: String,
    decimals: u8,
}

#[derive(Deserialize)]
struct MintTokenRequest {
    mint: String,
    destination: String,
    authority: String,
    amount: u64,
}

#[derive(Deserialize)]
struct SignMessageRequest {
    message: String,
    secret: String,
}

#[derive(Serialize)]
struct SignMessageResponse {
    signature: String,
    public_key: String,
    message: String,
}

#[derive(Deserialize)]
struct VerifyMessageRequest {
    message: String,
    signature: String,
    pubkey: String,
}

#[derive(Serialize)]
struct VerifyMessageResponse {
    valid: bool,
    message: String,
    pubkey: String,
}

#[derive(Deserialize)]
struct SendSolRequest {
    from: String,
    to: String,
    lamports: u64,
}

#[derive(Deserialize)]
struct SendTokenRequest {
    destination: String,
    mint: String,
    owner: String,
    amount: u64,
}

#[derive(Serialize)]
struct SerializableInstruction {
    program_id: String,
    accounts: Vec<SerializableAccountMeta>,
    instruction_data: String,
}

#[derive(Serialize)]
struct SerializableAccountMeta {
    pubkey: String,
    is_signer: bool,
    is_writable: bool,
}

impl From<Instruction> for SerializableInstruction {
    fn from(instruction: Instruction) -> Self {
        SerializableInstruction {
            program_id: instruction.program_id.to_string(),
            accounts: instruction
                .accounts
                .into_iter()
                .map(SerializableAccountMeta::from)
                .collect(),
            instruction_data: general_purpose::STANDARD.encode(&instruction.data),
        }
    }
}

impl From<AccountMeta> for SerializableAccountMeta {
    fn from(meta: AccountMeta) -> Self {
        SerializableAccountMeta {
            pubkey: meta.pubkey.to_string(),
            is_signer: meta.is_signer,
            is_writable: meta.is_writable,
        }
    }
}


async fn generate_keypair() -> Result<Json<SuccessResponse<KeypairResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let keypair = Keypair::new();
    let response = SuccessResponse {
        success: true,
        data: KeypairResponse {
            pubkey: keypair.pubkey().to_string(),
            secret: keypair.to_base58_string(),
        },
    };
    Ok(Json(response))
}

async fn create_token(
    Json(req): Json<CreateTokenRequest>,
) -> Result<Json<SuccessResponse<SerializableInstruction>>, (StatusCode, Json<ErrorResponse>)> {
    let mint_authority_pubkey = match Pubkey::from_str(&req.mint_authority) {
        Ok(pk) => pk,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid mint authority public key")),
            ))
        }
    };
    let mint_pubkey = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid mint public key")))),
    };

    match spl_token::instruction::initialize_mint(
        &spl_token::ID,
        &mint_pubkey,
        &mint_authority_pubkey,
        None,
        req.decimals,
    ) {
        Ok(instruction) => {
            let serializable_instruction: SerializableInstruction = instruction.into();
            Ok(Json(SuccessResponse {
                success: true,
                data: serializable_instruction,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(&format!("Failed to create instruction: {}", e))),
        )),
    }
}

async fn mint_token(
    Json(req): Json<MintTokenRequest>,
) -> Result<Json<SuccessResponse<SerializableInstruction>>, (StatusCode, Json<ErrorResponse>)> {
    let mint_pubkey = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid mint public key")))),
    };
    let destination_pubkey = match Pubkey::from_str(&req.destination) {
        Ok(pk) => pk,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid destination public key")),
            ))
        }
    };
    let authority_pubkey = match Pubkey::from_str(&req.authority) {
        Ok(pk) => pk,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid authority public key")),
            ))
        }
    };

    match spl_token::instruction::mint_to(
        &spl_token::ID,
        &mint_pubkey,
        &destination_pubkey,
        &authority_pubkey,
        &[],
        req.amount,
    ) {
        Ok(instruction) => {
            let serializable_instruction: SerializableInstruction = instruction.into();
            Ok(Json(SuccessResponse {
                success: true,
                data: serializable_instruction,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(&format!("Failed to create instruction: {}", e))),
        )),
    }
}

async fn sign_message(
    Json(req): Json<SignMessageRequest>,
) -> Result<Json<SuccessResponse<SignMessageResponse>>, (StatusCode, Json<ErrorResponse>)> {
    if req.message.is_empty() || req.secret.is_empty() {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Missing required fields"))));
    }

    let keypair = match bs58::decode(&req.secret).into_vec() {
        Ok(bytes) => match Keypair::try_from(&bytes) {
            Ok(kp) => kp,
            Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid secret key")))),
        },
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid secret key format")))),
    };

    let signature = keypair.sign_message(req.message.as_bytes());

    Ok(Json(SuccessResponse {
        success: true,
        data: SignMessageResponse {
            signature: general_purpose::STANDARD.encode(signature.as_ref()),
            public_key: keypair.pubkey().to_string(),
            message: req.message,
        },
    }))
}

async fn verify_message(
    Json(req): Json<VerifyMessageRequest>,
) -> Result<Json<SuccessResponse<VerifyMessageResponse>>, (StatusCode, Json<ErrorResponse>)> {
    let pubkey = match Pubkey::from_str(&req.pubkey) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid public key")))),
    };

    let signature_bytes = match general_purpose::STANDARD.decode(&req.signature) {
        Ok(bytes) => bytes,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid signature format; must be base64")),
            ))
        }
    };

    let signature = match Signature::try_from(signature_bytes.as_slice()) {
        Ok(sig) => sig,
        Err(_) => {
            return Err((
                StatusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid signature length")),
            ))
        }
    };

    let valid = signature.verify(pubkey.as_ref(), req.message.as_bytes());

    Ok(Json(SuccessResponse {
        success: true,
        data: VerifyMessageResponse {
            valid,
            message: req.message,
            pubkey: req.pubkey,
        },
    }))
}

async fn send_sol(
    Json(req): Json<SendSolRequest>,
) -> Result<Json<SuccessResponse<SerializableInstruction>>, (StatusCode, Json<ErrorResponse>)> {
    let from_pubkey = match Pubkey::from_str(&req.from) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid 'from' public key")))),
    };
    let to_pubkey = match Pubkey::from_str(&req.to) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid 'to' public key")))),
    };

    if from_pubkey == to_pubkey {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new("Sender and recipient addresses cannot be the same.")),
        ));
    }
    if req.lamports == 0 {
        return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Cannot send 0 lamports."))));
    }

    let instruction = system_instruction::transfer(&from_pubkey, &to_pubkey, req.lamports);
    let serializable_instruction: SerializableInstruction = instruction.into();

    Ok(Json(SuccessResponse {
        success: true,
        data: serializable_instruction,
    }))
}

async fn send_token(
    Json(req): Json<SendTokenRequest>,
) -> Result<Json<SuccessResponse<SerializableInstruction>>, (StatusCode, Json<ErrorResponse>)> {
    let destination_pubkey = match Pubkey::from_str(&req.destination) {
        Ok(pk) => pk,
        Err(_) => {
            return Err((
                Status,tusCode::BAD_REQUEST,
                Json(ErrorResponse::new("Invalid destination public key")),
            ))
        }
    };
    let mint_pubkey = match Pubkey::from_str(&req.mint) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid mint public key")))),
    };
    let owner_pubkey = match Pubkey::from_str(&req.owner) {
        Ok(pk) => pk,
        Err(_) => return Err((StatusCode::BAD_REQUEST, Json(ErrorResponse::new("Invalid owner public key")))),
    };

    let source_token_account = spl_associated_token_instruction::get_associated_token_address(&owner_pubkey, &mint_pubkey);

    match spl_token::instruction::transfer(
        &spl_token::ID,
        &source_token_account,
        &destination_pubkey,
        &owner_pubkey,
        &[],
        req.amount,
    ) {
        Ok(instruction) => {
            let serializable_instruction: SerializableInstruction = instruction.into();
            Ok(Json(SuccessResponse {
                success: true,
                data: serializable_instruction,
            }))
        }
        Err(e) => Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse::new(&format!("Failed to create instruction: {}", e))),
        )),
    }
}

#[tokio::main]
async fn main() {

    let app = Router::new()
        .route("/keypair", post(generate_keypair))
        .nest("/token", Router::new()
            .route("/create", post(create_token))
            .route("/mint", post(mint_token)))
        .nest("/message", Router::new()
            .route("/sign", post(sign_message))
            .route("/verify", post(verify_message)))
        .nest("/send", Router::new()
            .route("/sol", post(send_sol))
            .route("/token", post(send_token)));

    let listener = TcpListener::bind("0.0.0.0:8080").await.unwrap();
    axum32::serve(listener, app).await.unwrap();
}