/**
 * Stellar address validation utilities.
 *
 * These are the canonical validation helpers for the whole monorepo.
 * Both the UI (PaymentForm) and CLI (batch validation) import from here so
 * the same rules are enforced everywhere.
 *
 * We use `stellar-sdk`'s `StrKey` rather than a hand-rolled regex so that
 * the full base-32 checksum is verified, not just the shape of the string.
 */

import { StrKey } from 'stellar-sdk';

// ── Core predicates ───────────────────────────────────────────────────────────

/**
 * Returns `true` when `address` is a valid Stellar ed25519 public key (G…).
 *
 * Uses `StrKey.isValidEd25519PublicKey` from `stellar-sdk` which verifies
 * the base-32 encoding and the embedded checksum — stricter than a regex.
 *
 * @example
 * isValidStellarPublicKey('GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFLBKYXRH7CL5BJM4A3') // true
 * isValidStellarPublicKey('not-a-key') // false
 */
export function isValidStellarPublicKey(address: string): boolean {
  if (!address) return false;
  try {
    return StrKey.isValidEd25519PublicKey(address);
  } catch {
    return false;
  }
}

/**
 * Returns `true` when `address` is a valid Soroban contract address (C…).
 *
 * Uses `StrKey.isValidContract` from `stellar-sdk`.
 *
 * @example
 * isValidStellarContractAddress('GBBD47IF6LWK7P7MDEVSCWR7DPUWV3NY3DTQEVFLBKYXRH7CL5BJM4A3') // false
 * isValidStellarContractAddress('CAHT...') // true (if valid)
 */
export function isValidStellarContractAddress(address: string): boolean {
  if (!address) return false;
  try {
    return StrKey.isValidContract(address);
  } catch {
    return false;
  }
}

/**
 * Returns `true` when `address` is either a valid ed25519 public key (G…)
 * or a valid Soroban contract address (C…).
 *
 * Use this for fields that accept both user accounts and contract addresses.
 *
 * @example
 * isValidStellarAddress('GBBD47...')  // true — ed25519 key
 * isValidStellarAddress('CAHT...')    // true — contract address
 * isValidStellarAddress('not-a-key')  // false
 */
export function isValidStellarAddress(address: string): boolean {
  return isValidStellarPublicKey(address) || isValidStellarContractAddress(address);
}