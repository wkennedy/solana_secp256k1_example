use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::program_error::ProgramError;
use solana_program::secp256k1_recover::{secp256k1_recover, Secp256k1Pubkey};
use solana_program::{account_info::AccountInfo, entrypoint, entrypoint::ProgramResult, keccak, msg, pubkey::Pubkey};

#[derive(BorshDeserialize, BorshSerialize)]
pub struct SignaturePackage {
    pub verifier_signature: [u8; 64],
    pub recovery_id: u8,
    pub public_key: [u8; 65],
    pub data: [u8; 32],
}

entrypoint!(process_instruction);

#[derive(BorshSerialize, BorshDeserialize)]
pub enum ProgramInstruction {
    VerifySig(SignaturePackage)
}

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = ProgramInstruction::try_from_slice(instruction_data)?;

    match instruction {
        ProgramInstruction::VerifySig(signature_package) => verify_signature_with_recover(&signature_package)
    }
}

fn verify_signature_with_recover(
    signature_package: &SignaturePackage
) -> ProgramResult {
    msg!("Attempting to verify signature");

    // Verify the signature
    let message_hash = {
        let mut hasher = keccak::Hasher::default();
        hasher.hash(&signature_package.data);
        hasher.result()
    };

    // Perform the secp256k1 recovery
    let recovered_pubkey = secp256k1_recover(&message_hash.0, signature_package.recovery_id, &signature_package.verifier_signature).expect("Error recovering public key");

    // In this example we got the public key from the data we passed to the program, but it would also be possible to load it from an account.
    let expected_pubkey = Secp256k1Pubkey::new(&signature_package.public_key[1..65]);
    // Check if the recovered public key matches the expected one
    if recovered_pubkey != expected_pubkey {
        msg!("Signature verification failed");
        return Err(ProgramError::MissingRequiredSignature.into());
    }

    msg!("Signature valid!");
    update_on_chain_state(&signature_package.data).expect("Error updating on chain state.");
    
    Ok(())
}


fn update_on_chain_state(message_data: &[u8; 32]) -> ProgramResult {
    msg!("Updating state with data {:?}", &message_data);

    Ok(())
}