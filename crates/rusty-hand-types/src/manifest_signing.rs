//! Ed25519-based manifest signing for supply chain integrity.
//!
//! Agent manifests are TOML files that define an agent's capabilities,
//! tools, and configuration. A compromised or tampered manifest can grant
//! an agent elevated privileges. This module allows manifests to be
//! cryptographically signed so that the kernel can verify their integrity
//! and provenance before loading.
//!
//! The signing scheme:
//! 1. Compute SHA-256 of the manifest content.
//! 2. Sign the hash with Ed25519 (via `ed25519-dalek`).
//! 3. Bundle the signature, public key, and content hash into a
//!    `SignedManifest` envelope.
//!
//! Verification recomputes the hash and checks the Ed25519 signature.
//!
//! ## Trust model — read before relying on this
//!
//! [`SignedManifest::verify`] checks the signature against the public key
//! **embedded in the envelope**. That proves the envelope is internally
//! consistent and tamper-evident, but it does NOT establish authenticity: an
//! attacker who edits the manifest can simply re-sign it with their own key and
//! embed that key, and `verify()` still returns `Ok`. Treat `verify()` as an
//! integrity / corruption check only.
//!
//! For an actual trust decision — "was this signed by someone I trust?" — use
//! [`SignedManifest::verify_with_trusted_keys`] with an allowlist of trusted
//! public keys (a trust anchor). With no trust anchor, signing buys integrity,
//! not provenance.

use ed25519_dalek::{Signature, Signer, SigningKey, Verifier, VerifyingKey};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// A signed manifest envelope containing the original manifest text,
/// its content hash, the Ed25519 signature, and the signer's public key.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignedManifest {
    /// The raw manifest content (typically TOML).
    pub manifest: String,
    /// Hex-encoded SHA-256 hash of `manifest`.
    pub content_hash: String,
    /// Ed25519 signature bytes over `content_hash`.
    pub signature: Vec<u8>,
    /// The signer's Ed25519 public key bytes (32 bytes).
    pub signer_public_key: Vec<u8>,
    /// Human-readable identifier for the signer (e.g. email or key ID).
    pub signer_id: String,
}

/// Computes the hex-encoded SHA-256 hash of a manifest string.
pub fn hash_manifest(manifest: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(manifest.as_bytes());
    hex::encode(hasher.finalize())
}

impl SignedManifest {
    /// Signs a manifest with the given Ed25519 signing key.
    ///
    /// Returns a `SignedManifest` envelope ready for serialisation and
    /// distribution alongside (or instead of) the raw manifest file.
    pub fn sign(
        manifest: impl Into<String>,
        signing_key: &SigningKey,
        signer_id: impl Into<String>,
    ) -> Self {
        let manifest = manifest.into();
        let content_hash = hash_manifest(&manifest);
        let signature = signing_key.sign(content_hash.as_bytes());
        let verifying_key = signing_key.verifying_key();

        Self {
            manifest,
            content_hash,
            signature: signature.to_bytes().to_vec(),
            signer_public_key: verifying_key.to_bytes().to_vec(),
            signer_id: signer_id.into(),
        }
    }

    /// Verifies the **integrity** of this signed manifest (NOT authenticity).
    ///
    /// Checks:
    /// 1. The `content_hash` matches a fresh SHA-256 of `manifest`.
    /// 2. The `signature` is valid for `content_hash` under the *embedded*
    ///    `signer_public_key`.
    ///
    /// This is tamper-evidence only: it cannot tell a trusted signer from an
    /// attacker who re-signed with their own key. For trust decisions use
    /// [`Self::verify_with_trusted_keys`].
    ///
    /// Returns `Ok(())` on success, or `Err(description)` on failure.
    pub fn verify(&self) -> Result<(), String> {
        // Re-compute the hash and compare.
        let recomputed = hash_manifest(&self.manifest);
        if recomputed != self.content_hash {
            return Err(format!(
                "content hash mismatch: expected {} but manifest hashes to {}",
                self.content_hash, recomputed
            ));
        }

        // Reconstruct the public key.
        let pk_bytes: [u8; 32] = self
            .signer_public_key
            .as_slice()
            .try_into()
            .map_err(|_| "invalid public key length (expected 32 bytes)".to_string())?;
        let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
            .map_err(|e| format!("invalid public key: {}", e))?;

        // Reconstruct the signature.
        let sig_bytes: [u8; 64] = self
            .signature
            .as_slice()
            .try_into()
            .map_err(|_| "invalid signature length (expected 64 bytes)".to_string())?;
        let signature = Signature::from_bytes(&sig_bytes);

        // Verify.
        verifying_key
            .verify(self.content_hash.as_bytes(), &signature)
            .map_err(|e| format!("signature verification failed: {}", e))
    }

    /// Verifies integrity AND authenticity against a trust anchor.
    ///
    /// In addition to [`Self::verify`], this requires the embedded
    /// `signer_public_key` to be one of `trusted_keys` (each a 32-byte Ed25519
    /// public key). An empty `trusted_keys` is rejected — without a trust anchor
    /// there is nothing to authenticate against, so accepting would be the same
    /// self-attestation hole `verify()` has.
    pub fn verify_with_trusted_keys(&self, trusted_keys: &[Vec<u8>]) -> Result<(), String> {
        if trusted_keys.is_empty() {
            return Err(
                "no trusted signing keys configured — cannot authenticate the signer".to_string(),
            );
        }
        // Integrity + internal signature first.
        self.verify()?;
        // Then require the signer key to be explicitly trusted.
        if trusted_keys.iter().any(|k| k == &self.signer_public_key) {
            Ok(())
        } else {
            Err(format!(
                "signer '{}' public key is not in the trusted set",
                self.signer_id
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::OsRng;

    #[test]
    fn test_sign_and_verify() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let manifest = r#"
[agent]
name = "hello-world"
description = "A simple test agent"

[capabilities]
shell = false
network = false
"#;

        let signed = SignedManifest::sign(manifest, &signing_key, "test@rustyhand.dev");
        assert_eq!(signed.content_hash, hash_manifest(manifest));
        assert_eq!(signed.signer_id, "test@rustyhand.dev");
        assert!(signed.verify().is_ok());
    }

    #[test]
    fn test_verify_with_trusted_keys() {
        let trusted = SigningKey::generate(&mut OsRng);
        let attacker = SigningKey::generate(&mut OsRng);
        let manifest = "[agent]\nname = \"a\"\n";

        let signed = SignedManifest::sign(manifest, &trusted, "trusted@rustyhand.dev");
        let trusted_pk = trusted.verifying_key().to_bytes().to_vec();
        let attacker_pk = attacker.verifying_key().to_bytes().to_vec();

        // Trusted key accepted.
        assert!(signed
            .verify_with_trusted_keys(std::slice::from_ref(&trusted_pk))
            .is_ok());
        // Unknown signer rejected even though verify() (integrity) passes.
        assert!(signed.verify().is_ok());
        assert!(signed
            .verify_with_trusted_keys(std::slice::from_ref(&attacker_pk))
            .unwrap_err()
            .contains("not in the trusted set"));
        // No trust anchor at all is rejected (closes the self-attestation hole).
        assert!(signed.verify_with_trusted_keys(&[]).is_err());

        // An attacker who re-signs with their own key passes verify() but is
        // rejected by the trust-anchored check.
        let forged = SignedManifest::sign(manifest, &attacker, "attacker");
        assert!(forged.verify().is_ok());
        assert!(forged
            .verify_with_trusted_keys(std::slice::from_ref(&trusted_pk))
            .is_err());
    }

    #[test]
    fn test_tampered_fails() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let manifest = "[agent]\nname = \"secure-agent\"\n";

        let mut signed = SignedManifest::sign(manifest, &signing_key, "signer-1");

        // Tamper with the manifest content after signing.
        signed.manifest = "[agent]\nname = \"evil-agent\"\nshell = true\n".to_string();

        let result = signed.verify();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("content hash mismatch"));
    }

    #[test]
    fn test_wrong_key_fails() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let wrong_key = SigningKey::generate(&mut OsRng);

        let manifest = "[agent]\nname = \"test\"\n";
        let mut signed = SignedManifest::sign(manifest, &signing_key, "signer-a");

        // Replace the public key with a different key's public key.
        signed.signer_public_key = wrong_key.verifying_key().to_bytes().to_vec();

        let result = signed.verify();
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .contains("signature verification failed"));
    }
}
