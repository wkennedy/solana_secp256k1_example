use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use libsecp256k1::{Message, PublicKey, SecretKey};
use log::info;
use solana_client::nonblocking::rpc_client::RpcClient;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::pubkey::Pubkey;
use solana_program::{keccak};
use solana_sdk::commitment_config::CommitmentConfig;
use solana_sdk::signature::{Keypair, Signer};
use solana_sdk::transaction::Transaction;
use std::str::FromStr;
use rand::thread_rng;

const PROGRAM_ID: &str = "4muvyr2m6AFioKUjuyMXyLTYztykfXTTUemg4ZnD38bi";
const RPC_URL: &str = "http://localhost:8899";

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SignaturePackage {
    pub verifier_signature: [u8; 64],
    pub recovery_id: u8,
    pub public_key: [u8; 65],
    pub data: [u8; 32],
}

#[derive(BorshSerialize, BorshDeserialize)]
pub enum ProgramInstruction {
    VerifySig(SignaturePackage)
}

#[tokio::main]
async fn main() {

    let client = RpcClient::new_with_commitment(RPC_URL.to_string(), CommitmentConfig::confirmed());

    // Load your Solana wallet keypair
    let payer = Keypair::new();
    let airdrop_amount = 1_000_000_000; // 1 SOL in lamports
    match request_airdrop(&client, &payer.pubkey(), airdrop_amount).await {
        Ok(_) => info!("Airdrop successful!"),
        Err(err) => info!("Airdrop failed: {}", err),
    }

    // Your program ID (replace with your actual program ID)
    let program_id = Pubkey::from_str(PROGRAM_ID).unwrap();

    // Create our secp256k1 secret. Normally, the secret is created and loaded elsewhere
    let rng = &mut thread_rng();
    let secret = SecretKey::random(rng).serialize();

    // Create some data we want to store on-chain. We'll use this to create our signature.
    let data = Pubkey::new_unique().to_bytes();

    // Use our data and secret to create a signed package to send to the Solana program.
    let commitment = create_and_sign_package(
        data,
        &secret).unwrap();

    // Create the instruction to call our program
    let instruction_data = to_vec(&ProgramInstruction::VerifySig(commitment)).unwrap();
    let instruction = Instruction::new_with_bytes(
        program_id,
        instruction_data.as_slice(),
        vec![
            AccountMeta::new(payer.pubkey(), true),
        ],
    );

    // Create the transaction
    let recent_blockhash = client.get_latest_blockhash().await.unwrap();
    let transaction = Transaction::new_signed_with_payer(
        &[instruction],
        Some(&payer.pubkey()),
        &[&payer],
        recent_blockhash,
    );

    // Send and confirm transaction
    match client.send_and_confirm_transaction(&transaction).await {
        Ok(signature) => {
            println!("Transaction succeeded: {:?}", &signature);
        }
        Err(err) => {
            println!("Error sending transaction: {}", err);
        }
    }

}

fn create_and_sign_package(
    message_data: [u8; 32],
    signer_secret_key: &[u8; 32],
) -> Result<SignaturePackage, Box<dyn std::error::Error>> {

    let message_hash = {
        let mut hasher = keccak::Hasher::default();
        hasher.hash(&message_data);
        hasher.result()
    };

    let message = Message::parse_slice(&message_hash.0)?;

    // Create secret key from input bytes
    let secret_key = SecretKey::parse(signer_secret_key)?;
    let public_key = PublicKey::from_secret_key(&secret_key).serialize();

    // Sign the message and get the signature and recovery ID
    let (signature, recovery_id) = libsecp256k1::sign(&message, &secret_key);

    // Combine signature and recovery ID into 64 bytes
    let mut signature_bytes = [0u8; 64];
    signature_bytes[..64].copy_from_slice(&signature.serialize());

    Ok(SignaturePackage {
        verifier_signature: signature_bytes,
        recovery_id: recovery_id.serialize(),
        public_key,
        data: message_data,
    })
}

async fn request_airdrop(
    client: &RpcClient,
    pubkey: &Pubkey,
    amount: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    let signature = client.request_airdrop(pubkey, amount).await?;

    // Wait for the transaction to be confirmed
    loop {
        let confirmation = client.confirm_transaction(&signature).await.unwrap();
        if confirmation {
            break;
        }
    }
    Ok(())
}
