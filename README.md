# Solana secp256k1 tutorial

### Overview

This tutorial is largely based from examples and documentation provided by https://docs.rs/solana-program/latest/solana_program/secp256k1_recover/fn.secp256k1_recover.html. 
Also, as noted in that documentation, is to use the native secp256k1 program which has many benefits over doing it the way this tutorial suggests. See the excellent documentation here: https://docs.rs/solana-sdk/latest/solana_sdk/secp256k1_instruction/index.html

In this tutorial we'll use secp256k1 to create and verify digital signatures. This allows the Solana program to interact with or verify signatures that might have originated from other blockchains or sources that use this curve, enabling potential cross-chain functionalities.

Why do we need to create a signature?

The purpose of cryptographically signing data is to verify the authenticity of data by proving that it originated from a specific source and hasn't been altered in transit, essentially providing a way to digitally "sign" a document or transaction to ensure its integrity and prevent tampering, all while allowing the recipient to confirm the sender's identity.

What is secp256k1? secp256k1 is an elliptic curve used in cryptography and is used frequently in blockchains due to:

- being efficient for digital signature generation and verification
- offers strong security with relatively small key sizes

### Let's Begin

We'll start by creating a secret key. This secret key is used to sign messages or derive a public key for use in the secp256k1 cryptographic operations, such as creating digital signatures that the Solana program will verify.
```rust
let rng = &mut thread_rng();
let secret = SecretKey::random(rng).serialize();
```

Now that we have that, we'll create some data that we want to send to the Solana program. We'll just use a Pubkey as arbitrary data.

```rust
let data = Pubkey::new_unique().to_bytes();
```

Let's take a look at this function:

```rust
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
```

Let's hash our data using Keccak-256, as demonstrated in the Solana documentation. The reason to hash the data before signing it is for security and efficiency reasons. In this example, our data is small, but if signing larger data, it will become computationally more expensive. So, with hashing we get a fixed size that will be performed on-chain.
By using Keccak-256, the developers are likely aiming to create a system that can interact seamlessly with Ethereum-style signatures or data structures. This choice facilitates interoperability between different blockchain ecosystems, which is an important consideration in the development of dapps and cross-chain protocols.
It's worth pointing out you can use different hash systems with secp256k1. The choice of hash function is independent of the elliptic curve cryptography used for signing. SHA-256, SHA-3, etc... may be used as well depending on your use case.
```rust
let message_hash = {
    let mut hasher = keccak::Hasher::default();
    hasher.hash(&message_data);
    hasher.result()
};
let message = Message::parse_slice(&message_hash.0)?;
```

Now we'll create the signature and recovery ID. The recovery ID allows for public key recovery from the signature, which we'll see in a bit when we look at the Solana program.
```rust
let (signature, recovery_id) = libsecp256k1::sign(&message, &secret_key);
```

Finally, we'll put everything we need to send to the Solana program in a tidy struct.

```rust
Ok(SignaturePackage {
    verifier_signature: signature_bytes,
    recovery_id: recovery_id.serialize(),
    public_key,
    data: message_data,
})
```

### Contract side of things

This is the function in the contract that does the verification.
```rust
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
```

You'll notice that we perform the hashing of our data again. Here is the explanation from the Solana documentation:
```rust
// The secp256k1 recovery operation accepts a cryptographically-hashed
// message only. Passing it anything else is insecure and allows signatures
// to be forged.
//
// This means that the code calling `secp256k1_recover` must perform the hash
// itself, and not assume that data passed to it has been properly hashed.
```

We can recover the public key from the signature and recovery ID, a feature of secp256k1.
```rust
let recovered_pubkey = secp256k1_recover(&message_hash.0, signature_package.recovery_id, &signature_package.verifier_signature).expect("Error recovering public key");
```

With the public key we were given and the recovered public key, we can determine if the signature was valid if the two keys are equal.
```rust
if recovered_pubkey != expected_pubkey {
    msg!("Signature verification failed");
    return Err(ProgramError::MissingRequiredSignature.into());
}
```

There you have it. With the functions provided by Solana, it's easy to verify secp256k1 signatures on-chain. Remember to check out the official documentation for more details!